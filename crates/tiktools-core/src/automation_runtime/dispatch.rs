//! Automation dispatch: test harnesses and live event fan-out.

use crate::*;

impl AppCore {
    pub(crate) async fn test_action(
        self: &Arc<Self>,
        action: &Value,
        trigger: Option<&str>,
    ) -> Value {
        let event = recover_rwlock_read(&self.automation_state.last_event, "automation event")
            .clone()
            .unwrap_or_else(|| sample_automation_event(trigger.unwrap_or("tiktok.chat")));
        self.execute_action(action, &event, None, true).await
    }

    pub(crate) async fn test_event(self: &Arc<Self>, record: &Value) -> Value {
        let started = now_millis();
        let trigger = record
            .get("trigger")
            .and_then(Value::as_str)
            .unwrap_or("tiktok.chat");
        let event = recover_rwlock_read(&self.automation_state.last_event, "automation event")
            .clone()
            .filter(|event| event.get("type").and_then(Value::as_str) == Some(trigger))
            .unwrap_or_else(|| {
                self.plugin_event_sample(trigger)
                    .unwrap_or_else(|| sample_automation_event(trigger))
            });

        if !self.automation.event_record_matches(record, &event) {
            // Name the sample data so a mismatch against a plugin sample
            // (hotkey.pressed ships key "k") reads as a data problem, not a
            // broken trigger.
            let preview = event.get("data").map(|data| {
                const MAX_PREVIEW: usize = 120;
                let text = serde_json::to_string(data).unwrap_or_default();
                if text.len() > MAX_PREVIEW {
                    format!("{}…", &text[..MAX_PREVIEW])
                } else {
                    text
                }
            });
            let summary = match preview {
                Some(preview) if !preview.is_empty() => {
                    format!(
                        "Event filters did not match the sample event (sample data: {preview})."
                    )
                }
                _ => "Event filters did not match the sample event.".to_owned(),
            };
            return json!({
                "id": self.automation.next_run_id("test-event", started),
                "at": started,
                "status": "error",
                "eventName": trigger,
                "actionName": record.get("name").and_then(Value::as_str).unwrap_or("Event"),
                "summary": summary.clone(),
                "durationMs": now_millis().saturating_sub(started),
                "test": true,
                "logs": [],
                "error": summary
            });
        }

        let actions = self.automation.actions_for_event(record);
        if actions.is_empty() {
            let summary = "The event has no saved actions to test.";
            return json!({
                "id": self.automation.next_run_id("test-event", started),
                "at": started,
                "status": "error",
                "eventName": trigger,
                "actionName": record.get("name").and_then(Value::as_str).unwrap_or("Event"),
                "summary": summary,
                "durationMs": now_millis().saturating_sub(started),
                "test": true,
                "logs": [],
                "error": summary
            });
        }

        let mut runs = Vec::with_capacity(actions.len());
        for action in &actions {
            runs.push(
                self.execute_action(action, &event, Some(record), true)
                    .await,
            );
        }
        let failed = runs
            .iter()
            .any(|run| run.get("status") == Some(&Value::String("error".to_owned())));
        let summary = if failed {
            "One or more actions failed."
        } else {
            "All referenced actions passed."
        };
        let mut result = json!({
            "id": self.automation.next_run_id("test-event", started),
            "at": started,
            "status": if failed { "error" } else { "ok" },
            "eventName": trigger,
            "actionName": record.get("name").and_then(Value::as_str).unwrap_or("Event"),
            "summary": summary,
            "durationMs": now_millis().saturating_sub(started),
            "test": true,
            "logs": [],
            "actions": runs
        });
        if failed {
            result["error"] = Value::String(summary.to_owned());
        }
        result
    }

    pub(crate) async fn run_automation_event(self: &Arc<Self>, event: Value) {
        if self.automation.emit_depth(&event) >= 3 {
            tracing::warn!(event_type = ?event.get("type"), "automation emit depth limit reached");
            return;
        }
        let matched = self.automation.matching_events(&event);
        // Match outcome only (counts), never event contents: key data must
        // not end up in logs.
        tracing::debug!(
            event_type = ?event.get("type"),
            matched = matched.len(),
            "automation matching completed"
        );
        for record in matched {
            if !self.automation.claim_event(&record, &event, now_millis()) {
                continue;
            }
            for action in self.automation.actions_for_event(&record) {
                self.execute_action(&action, &event, Some(&record), false)
                    .await;
            }
        }
    }
}
