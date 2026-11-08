//! Automation dispatch: test harnesses and live event fan-out.

use super::context::ExecutionContext;
use super::samples::{cohere_gift_pair, pin_event_to_record};
use crate::live_events::AutomationEvent;
use crate::*;

impl AppCore {
    /// Event the test harness replays for `trigger`: the last live envelope
    /// of that type when it satisfies the record under test (real user, real
    /// gift, real counts), else the declaring plugin's manifest sample, else
    /// the per-type sample (which the caller then pins to the record).
    /// Replayed live envelopes keep their payload and get a fresh timestamp.
    /// Returns the event plus where it came from (`live` or `sample`) so the
    /// UI can tell the operator what the test actually ran against.
    ///
    /// The compatibility gate matters: without it, one remembered Rose
    /// would replay forever and a Galaxy draft could never match — every
    /// probe after the first would show the same stale gift.
    fn test_event_for(&self, trigger: &str, record: Option<&Value>) -> (Value, &'static str) {
        if let Some(mut event) = self.last_event_for(trigger) {
            let compatible = record.is_none_or(|record| {
                record
                    .get("trigger")
                    .and_then(Value::as_str)
                    .is_none_or(|record_trigger| record_trigger == trigger)
                    && self.automation.runtime_event_record_matches(record, &event)
            });
            if compatible {
                event["timestamp"] = Value::from(now_millis());
                return (event, "live");
            }
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
    /// trigger when it satisfies the draft, else the per-type sample pinned
    /// to the draft), then real fan-out — domain subscribers, enrichment,
    /// cooldowns, and execution. Fired events are not stored as last-live.
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
        let manual = match record {
            Some(record) => match self.automation.manual_record(record) {
                Some(matched) => Some(matched),
                None => {
                    return json!({"status":"error","error":"authority_required","summary":"Definition requires explicit owner adoption or delegated execution approval.","matched":0,"draftMatched":false})
                }
            },
            None => None,
        };
        let started = now_millis();
        let (mut event, source) = match event_override {
            Some(custom) if custom.is_object() => (custom.clone(), "custom"),
            _ => self.test_event_for(trigger, record),
        };
        // A draft pins its own `eq` filters onto a sample envelope, so firing
        // "giftName = Galaxy" fires a Galaxy with the catalog's id — never
        // the generic Rose. Live and custom data are never reshaped.
        let pinned = record.map_or_else(Vec::new, |record| {
            self.pin_sample_to_record(&mut event, source, record)
        });
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
        let event = AutomationEvent::owner(event);
        if !self.publish_live_domain_event(&event) {
            return json!({"status":"error","error":"authority_required","matched":0,"draftMatched":false});
        }
        let envelope = match self.enrich_automation_event(event).await {
            Ok(event) => event,
            Err(_) => {
                return json!({"status":"error","error":"authority_required","matched":0,"draftMatched":false})
            }
        };
        let enriched = envelope.payload();
        let incoming = envelope.context();
        if incoming.depth() >= 3 {
            return json!({
                "trigger": trigger,
                "eventSource": source,
                "pinned": pinned,
                "matched": 0,
                "draftMatched": false,
                "status": "error",
                "summary": format!("Fired {trigger}: refused, emit depth limit reached."),
                "durationMs": now_millis().saturating_sub(started),
            });
        }
        let draft_id = record.and_then(|record| record.get("id").and_then(Value::as_str));
        let saved: Vec<_> = self
            .automation
            .matching_records(enriched)
            .into_iter()
            .filter(|matched| {
                draft_id.is_none_or(|id| {
                    matched.definition.get("id").and_then(Value::as_str) != Some(id)
                })
            })
            .collect();
        let draft_name = record
            .and_then(|record| record.get("name").and_then(Value::as_str))
            .unwrap_or("Event");
        let draft_matched = record.is_some_and(|record| {
            record.get("trigger").and_then(Value::as_str) == Some(trigger)
                && self
                    .automation
                    .runtime_event_record_matches(record, enriched)
        });
        // Deliberately NOT remembered as last-live: a fired synthetic must
        // never become "live" truth, or the next probe would replay this
        // fire's gift instead of resolving fresh data for its own draft.
        let matched = saved.len() + usize::from(draft_matched);
        if matched == 0 {
            return fire_no_match(
                trigger,
                source,
                started,
                record.map(|_| draft_name),
                &pinned,
                enriched,
            );
        }
        self.run_matched_records(saved, enriched, incoming).await;
        if draft_matched {
            if let Some(record) = manual {
                self.run_matched_records(vec![record], enriched, incoming)
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
        if !pinned.is_empty() {
            summary.push_str(&format!(" (pinned from filters: {})", pinned.join(", ")));
        }
        json!({
            "trigger": trigger,
            "eventSource": source,
            "pinned": pinned,
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
        let (event, source) = self.test_event_for(trigger, None);
        let mut run = self.execute_action(action, &event, None, true).await;
        if let Some(object) = run.as_object_mut() {
            object.insert("eventSource".to_owned(), Value::String(source.to_owned()));
        }
        run
    }

    /// Pins a sample envelope to a record's `eq` filters and coheres the
    /// gift pair with the catalog, so testing or firing a record exercises
    /// its own configuration. Live and custom envelopes pass through
    /// untouched — only synthetic placeholders are shaped. Returns
    /// display-ready pinned entries for honest reporting.
    fn pin_sample_to_record(&self, event: &mut Value, source: &str, record: &Value) -> Vec<String> {
        if source != "sample" {
            return Vec::new();
        }
        let pinned = pin_event_to_record(event, record);
        if !pinned.is_empty() {
            cohere_gift_pair(event, &pinned, &self.gift_catalog());
        }
        pinned
    }

    pub(crate) async fn test_event(self: &Arc<Self>, record: &Value) -> Value {
        let Some(matched) = self.automation.manual_record(record) else {
            return json!({"status":"error","test":true,"error":"authority_required","summary":"Definition requires explicit owner adoption or delegated execution approval."});
        };
        let started = now_millis();
        let trigger = record
            .get("trigger")
            .and_then(Value::as_str)
            .unwrap_or("tiktok.chat");
        let (mut event, source) = self.test_event_for(trigger, Some(record));
        let pinned = self.pin_sample_to_record(&mut event, source, record);

        if !self.automation.runtime_event_record_matches(record, &event) {
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
                "pinned": pinned,
                "logs": [],
                "error": summary
            });
        }

        let actions = &matched.actions;
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
                "pinned": pinned,
                "logs": [],
                "error": summary
            });
        }

        let mut runs = Vec::with_capacity(actions.len());
        let Ok(context) = self
            .record_execution_context(&matched, &ExecutionContext::owner())
            .await
        else {
            return json!({"status":"error","test":true,"error":"authority_required","summary":"Saved execution approval is unavailable."});
        };
        for action in actions {
            if !self.action_still_current(&matched, action).await {
                runs.push(json!({"status":"error","error":"authority_required","summary":"Saved definition or execution approval has changed."}));
                break;
            }
            runs.push(
                self.execute_action_with_context(
                    &action.definition,
                    &event,
                    Some(&matched.definition),
                    &context,
                    true,
                )
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
            "pinned": pinned,
            "logs": [],
            "actions": runs
        });
        if failed {
            result["error"] = Value::String(summary.to_owned());
        }
        result
    }

    pub(crate) async fn run_automation_event(self: &Arc<Self>, event: AutomationEvent) {
        let (event, incoming) = event.into_parts();
        if self.execution_context_guard(&incoming).is_err() {
            tracing::warn!("automation execution context rejected");
            return;
        }
        if incoming.depth() >= 3 {
            tracing::warn!(event_type = ?event.get("type"), "automation emit depth limit reached");
            return;
        }
        let matched = self.automation.matching_records(&event);
        // Match outcome only (counts), never event contents: key data must
        // not end up in logs.
        tracing::debug!(
            event_type = ?event.get("type"),
            matched = matched.len(),
            "automation matching completed"
        );
        // Live revalidation: matched records waited behind bounded
        // admission, and a record (or its owning plugin) may have been
        // disabled or removed meanwhile. Stale clones never execute here.
        // Explicit manual fires bypass this via `run_matched_records`.
        let started = now_millis();
        let live: Vec<_> = matched
            .into_iter()
            .filter(|record| self.automation.is_match_live(record))
            .collect();
        self.run_matched_records(live, &event, &incoming).await;
        self.automation_state
            .events_completed
            .fetch_add(1, Ordering::Relaxed);
        self.automation_state
            .exec_latency_sum_ms
            .fetch_add(now_millis().saturating_sub(started), Ordering::Relaxed);
    }

    /// Claim + execute loop shared by live fan-out and manual fire: one
    /// cooldown claim per record, then every referenced action, for real.
    /// Outcomes are counted so overload (rejected admissions) stays
    /// distinguishable from plugin failures.
    pub(super) async fn run_matched_records(
        self: &Arc<Self>,
        records: Vec<crate::services::MatchedRecord>,
        event: &Value,
        incoming: &ExecutionContext,
    ) {
        for record in records {
            if self.execution_context_guard(incoming).is_err() {
                break;
            }
            if !self.claim_matched_record(&record, event).await {
                continue;
            }
            let Ok(context) = self.record_execution_context(&record, incoming).await else {
                continue;
            };
            let mut action_occurrences = std::collections::BTreeMap::<&str, u32>::new();
            for action in &record.actions {
                if self.execution_context_guard(incoming).is_err() {
                    break;
                }
                if !self.action_still_current(&record, action).await {
                    break;
                }
                let Some(action_id) = action.definition.get("id").and_then(Value::as_str) else {
                    break;
                };
                let occurrence = action_occurrences.entry(action_id).or_default();
                let slot = *occurrence;
                let Some(next) = occurrence.checked_add(1) else {
                    break;
                };
                *occurrence = next;
                // Stable saved identities plus a per-action occurrence keep
                // retries/replays on the original key when unrelated actions
                // are inserted or reordered. JSON event IDs never participate.
                let Ok(action_context) = context.effect_named_slot(
                    record
                        .definition
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    action_id,
                    slot,
                ) else {
                    break;
                };
                let run = self
                    .execute_action_with_context(
                        &action.definition,
                        event,
                        Some(&record.definition),
                        &action_context,
                        false,
                    )
                    .await;
                if run.get("status").and_then(Value::as_str) == Some("ok") {
                    self.automation_state
                        .actions_ok
                        .fetch_add(1, Ordering::Relaxed);
                } else {
                    self.automation_state
                        .actions_failed
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    async fn claim_matched_record(
        &self,
        record: &crate::services::MatchedRecord,
        event: &Value,
    ) -> bool {
        if !self
            .stable_automation(|| self.automation.is_match_live(record))
            .await
        {
            return false;
        }
        self.automation
            .claim_event(&record.definition, event, now_millis())
    }
    pub(super) async fn action_still_current(
        &self,
        record: &crate::services::MatchedRecord,
        action: &crate::services::MatchedAction,
    ) -> bool {
        self.stable_automation(|| self.automation.is_step_live(record, action))
            .await
    }
    async fn stable_automation(&self, check: impl Fn() -> bool) -> bool {
        loop {
            let settled = self.automation_write_settled.notified();
            tokio::pin!(settled);
            settled.as_mut().enable();
            if self.is_shutdown() || self.automation_mutations.is_poisoned() {
                return false;
            }
            let epoch = self.automation_write_epoch.load(Ordering::Acquire);
            if epoch >= u64::MAX - 1 {
                return false;
            }
            if !epoch.is_multiple_of(2) {
                settled.await;
                continue;
            }
            let valid = check();
            if self.automation_write_epoch.load(Ordering::Acquire) == epoch {
                return valid;
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
    pinned: &[String],
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
    let mut summary = match draft_name {
        Some(name) => format!("Fired {trigger}: '{name}' did not match{fired}."),
        None => format!("Fired {trigger}: no enabled event matched{fired}."),
    };
    if !pinned.is_empty() {
        summary.push_str(&format!(" (pinned from filters: {})", pinned.join(", ")));
    }
    json!({
        "trigger": trigger,
        "eventSource": source,
        "pinned": pinned,
        "matched": 0,
        "draftMatched": false,
        "status": "error",
        "summary": summary,
        "durationMs": now_millis().saturating_sub(started),
    })
}
