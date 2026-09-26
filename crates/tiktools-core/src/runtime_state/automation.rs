//! Automation runtime-state operations: sequence numbers, last-event
//! memory, and the context projection consumed by panels and test previews.

use crate::*;

impl AppCore {
    pub(crate) fn next_sequence(&self) -> u64 {
        self.automation_state
            .sequence
            .fetch_add(1, Ordering::AcqRel)
            + 1
    }

    pub(crate) fn remember_automation_event(&self, event: &serde_json::Value) {
        let now = now_millis();
        *recover_rwlock_write(&self.automation_state.last_event, "automation event") =
            Some(event.clone());
        *recover_rwlock_write(&self.automation_state.last_event_at, "automation timestamp") =
            Some(now);
        if let Some(event_type) = event.get("type").and_then(Value::as_str) {
            recover_rwlock_write(
                &self.automation_state.last_events,
                "automation event by type",
            )
            .insert(event_type.to_owned(), event.clone());
            recover_rwlock_write(
                &self.automation_state.last_events_at,
                "automation timestamp by type",
            )
            .insert(event_type.to_owned(), now);
        }
        if let Some(event_type) = event.get("type").and_then(Value::as_str) {
            // Any `<namespace>.status` plugin event carries listener health:
            // publish it on the generic status topic. The legacy
            // `hotkey-status` push stays as a compatibility path.
            if event_type.ends_with(".status") {
                if let Some(plugin_id) = plugin_owner(event) {
                    self.events
                        .publish_domain(crate::events::DomainEvent::PluginStatus {
                            plugin_id,
                            status: event.get("data").cloned().unwrap_or_else(|| json!({})),
                        });
                }
            }
            if event_type == "hotkey.status" {
                self.emit(HostMessage::HotkeyStatus {
                    status: event.get("data").cloned().unwrap_or_else(|| json!({})),
                });
            }
        }
        // DomainEvent delivery happens up front in `publish_live_domain_event`
        // (before automation slots), never here, so saturation cannot drop
        // control/UI events.
        let now = now_millis();
        let last = self
            .automation_state
            .last_context_emit_at
            .load(std::sync::atomic::Ordering::Acquire);
        if (last == 0 || now.saturating_sub(last) >= 100)
            && self
                .automation_state
                .last_context_emit_at
                .compare_exchange(
                    last,
                    now,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Acquire,
                )
                .is_ok()
        {
            self.emit(HostMessage::AutomationContext {
                event: Some(event.clone()),
                captured_at: *recover_rwlock_read(
                    &self.automation_state.last_event_at,
                    "automation timestamp",
                ),
            });
        }
    }

    /// Last automation event observed, for context panels and test previews.
    pub fn automation_context(&self) -> (Option<Value>, Option<u64>) {
        let event =
            recover_rwlock_read(&self.automation_state.last_event, "automation event").clone();
        let captured_at =
            *recover_rwlock_read(&self.automation_state.last_event_at, "automation timestamp");
        (event, captured_at)
    }

    /// Last observed envelope of one event type, if any. The test harness
    /// prefers this over the generic sample so emulation replays real data.
    pub(crate) fn last_event_for(&self, event_type: &str) -> Option<Value> {
        recover_rwlock_read(
            &self.automation_state.last_events,
            "automation event by type",
        )
        .get(event_type)
        .cloned()
    }

    /// Per-type variant of [`AppCore::automation_context`]: the last envelope
    /// of `event_type` plus when it was captured.
    pub fn automation_context_for(&self, event_type: &str) -> (Option<Value>, Option<u64>) {
        let event = self.last_event_for(event_type);
        let captured_at = recover_rwlock_read(
            &self.automation_state.last_events_at,
            "automation timestamp by type",
        )
        .get(event_type)
        .copied();
        (event, captured_at)
    }
}
