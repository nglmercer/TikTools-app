//! Automation dispatch: test harnesses and live event fan-out.

use crate::*;

impl AppCore {
    /// Event the test harness replays for `trigger`: the last live envelope
    /// of that type when one was observed (real user, real gift, real counts),
    /// else the declaring plugin's manifest sample, else the per-type sample.
    /// Replayed live envelopes keep their payload and get a fresh timestamp.
    /// Returns the event plus where it came from (`live` or `sample`) so the
    /// UI can tell the operator what the test actually ran against.
    fn test_event_for(&self, trigger: &str) -> (Value, &'static str) {
        if let Some(mut event) = self.last_event_for(trigger) {
            event["timestamp"] = Value::from(now_millis());
            return (event, "live");
        }
        let event = self
            .plugin_event_sample(trigger)
            .unwrap_or_else(|| sample_automation_event(trigger));
        (event, "sample")
    }

    /// Fires a synthetic event through the full live pipeline so it really
    /// triggers matching events and executes their actions (no dry run).
    /// This is the manual "make it happen" twin of the dry-run harness:
    /// same event resolution (custom JSON, else last live envelope of the
    /// trigger, else the per-type sample), then real fan-out — domain
    /// subscribers, enrichment, last-event memory, cooldowns, and execution.
    ///
    /// `record` is the editor draft being fired: it runs when its trigger
    /// and filters match, even when unsaved, edited-but-unsaved, or
    /// disabled (an explicit fire tests what the operator sees, like the
    /// dry run does). Its saved twin, if any, is skipped so one fire never
    /// executes the same event twice.
    pub(crate) async fn fire_synthetic_event(
        self: &Arc<Self>,
        trigger: &str,
        event_override: Option<&Value>,
        record: Option<&Value>,
    ) -> Value {
        let started = now_millis();
        let (mut event, source) = match event_override {
            Some(custom) if custom.is_object() => (custom.clone(), "custom"),
            _ => self.test_event_for(trigger),
        };
        // A fired event is new work: fresh identity and timestamp, and the
        // fired trigger wins over whatever type a pasted envelope carried.
        event["id"] = Value::String(format!(
            "fired-{}-{}",
            trigger.replace('.', "-"),
            self.next_sequence()
        ));
        event["timestamp"] = Value::from(now_millis());
        event["type"] = Value::String(trigger.to_owned());
        // Same order as the live pipeline: domain fan-out first (widgets and
        // subscribers always see a fired event), then enrichment, then the
        // match that decides whether any action executes.
        self.publish_live_domain_event(&event);
        let enriched = self.enrich_automation_event(event).await;
        if self.automation.emit_depth(&enriched) >= 3 {
            return json!({
                "trigger": trigger,
                "eventSource": source,
                "matched": 0,
                "draftMatched": false,
                "status": "error",
                "summary": format!("Fired {trigger}: refused, emit depth limit reached."),
                "durationMs": now_millis().saturating_sub(started),
            });
        }
        let draft_id = record.and_then(|record| record.get("id").and_then(Value::as_str));
        let saved: Vec<Value> = self
            .automation
            .matching_events(&enriched)
            .into_iter()
            .filter(|matched| {
                draft_id.is_none_or(|id| matched.get("id").and_then(Value::as_str) != Some(id))
            })
            .collect();
        let draft_name = record
            .and_then(|record| record.get("name").and_then(Value::as_str))
            .unwrap_or("Event");
        let draft_matched = record.is_some_and(|record| {
            record.get("trigger").and_then(Value::as_str) == Some(trigger)
                && self.automation.event_record_matches(record, &enriched)
        });
        self.remember_automation_event(&enriched);
        let matched = saved.len() + usize::from(draft_matched);
        if matched == 0 {
            return fire_no_match(
                trigger,
                source,
                started,
                record.map(|_| draft_name),
                &enriched,
            );
        }
        self.run_matched_records(saved, &enriched).await;
        if draft_matched {
            if let Some(record) = record {
                self.run_matched_records(vec![record.clone()], &enriched)
                    .await;
            }
        }
        let mut summary = format!(
            "Fired {trigger}: {matched} event{} matched, actions executed.",
            if matched == 1 { "" } else { "s" },
        );
        if draft_matched {
            summary.push_str(&format!(" Including '{draft_name}' (editor draft)."));
        }
        json!({
            "trigger": trigger,
            "eventSource": source,
            "matched": matched,
            "draftMatched": draft_matched,
            "status": "ok",
            "summary": summary,
            "durationMs": now_millis().saturating_sub(started),
        })
    }

    pub(crate) async fn test_action(
        self: &Arc<Self>,
        action: &Value,
        trigger: Option<&str>,
    ) -> Value {
        let trigger = trigger.unwrap_or("tiktok.chat");
        let (event, source) = self.test_event_for(trigger);
        let mut run = self.execute_action(action, &event, None, true).await;
        if let Some(object) = run.as_object_mut() {
            object.insert("eventSource".to_owned(), Value::String(source.to_owned()));
        }
        run
    }

    pub(crate) async fn test_event(self: &Arc<Self>, record: &Value) -> Value {
        let started = now_millis();
        let trigger = record
            .get("trigger")
            .and_then(Value::as_str)
            .unwrap_or("tiktok.chat");
        let (event, source) = self.test_event_for(trigger);

        if !self.automation.event_record_matches(record, &event) {
            // Name the replayed data so a mismatch against a plugin sample
            // (hotkey.pressed ships key "k") or a stale live event reads as
            // a data problem, not a broken trigger.
            let preview = event.get("data").map(|data| {
                const MAX_PREVIEW: usize = 120;
                let text = serde_json::to_string(data).unwrap_or_default();
                if text.len() > MAX_PREVIEW {
                    format!("{}…", &text[..MAX_PREVIEW])
                } else {
                    text
                }
            });
            let origin = if source == "live" {
                "the last live event"
            } else {
                "the sample event"
            };
            let summary = match preview {
                Some(preview) if !preview.is_empty() => {
                    format!("Event filters did not match {origin} (event data: {preview}).")
                }
                _ => format!("Event filters did not match {origin}."),
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
                "eventSource": source,
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
                "eventSource": source,
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
            "eventSource": source,
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
        self.run_matched_records(matched, &event).await;
    }

    /// Claim + execute loop shared by live fan-out and manual fire: one
    /// cooldown claim per record, then every referenced action, for real.
    async fn run_matched_records(self: &Arc<Self>, records: Vec<Value>, event: &Value) {
        for record in records {
            if !self.automation.claim_event(&record, event, now_millis()) {
                continue;
            }
            for action in self.automation.actions_for_event(&record) {
                self.execute_action(&action, event, Some(&record), false)
                    .await;
            }
        }
    }
}

/// Zero-match fire outcome. Always names the fired data (truncated) so a
/// mismatch reads as a data problem — e.g. sample giftName 'Rose' against
/// a filter for 'Galaxy' — and says so explicitly when a draft was tested.
fn fire_no_match(
    trigger: &str,
    source: &str,
    started: u64,
    draft_name: Option<&str>,
    event: &Value,
) -> Value {
    const MAX_PREVIEW: usize = 120;
    let preview = event.get("data").map(|data| {
        let text = serde_json::to_string(data).unwrap_or_default();
        let truncated: String = text.chars().take(MAX_PREVIEW).collect();
        if text.chars().count() > MAX_PREVIEW {
            format!("{truncated}…")
        } else {
            truncated
        }
    });
    let fired = match preview {
        Some(preview) if !preview.is_empty() => format!(" (fired data: {preview})"),
        _ => String::new(),
    };
    let summary = match draft_name {
        Some(name) => format!("Fired {trigger}: '{name}' did not match{fired}."),
        None => format!("Fired {trigger}: no enabled event matched{fired}."),
    };
    json!({
        "trigger": trigger,
        "eventSource": source,
        "matched": 0,
        "draftMatched": false,
        "status": "error",
        "summary": summary,
        "durationMs": now_millis().saturating_sub(started),
    })
}
