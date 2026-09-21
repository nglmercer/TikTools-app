//! Trigger matching, action lookup, and emit-depth tracking.

use super::filters::{event_record_matches, number_u64};
use super::snapshot::trigger_allowed;
use super::*;

impl AutomationService {
    pub fn matching_events(&self, event: &Value) -> Vec<Value> {
        let event_type = event.get("type").and_then(Value::as_str);
        let trigger_owners = recover_rwlock_read(&self.trigger_owners, "automation trigger owners");
        let enabled_plugins =
            recover_rwlock_read(&self.enabled_plugins, "automation enabled plugins");
        recover_rwlock_read(&self.events, "automation events")
            .values()
            .filter(|record| {
                record.get("enabled").and_then(Value::as_bool) == Some(true)
                    && record.get("trigger").and_then(Value::as_str) == event_type
                    && trigger_allowed(
                        record.get("trigger").and_then(Value::as_str),
                        &trigger_owners,
                        &enabled_plugins,
                    )
                    && event_record_matches(record, event)
            })
            .cloned()
            .collect()
    }

    pub fn event_record_matches(&self, record: &Value, event: &Value) -> bool {
        event_record_matches(record, event)
    }

    pub fn actions_for_event(&self, record: &Value) -> Vec<Value> {
        let ids = record
            .get("actionIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        let actions = recover_rwlock_read(&self.actions, "automation actions");
        let mut selected = ids
            .into_iter()
            .filter_map(|id| actions.get(id))
            .filter(|action| action.get("enabled").and_then(Value::as_bool) != Some(false))
            .cloned()
            .collect::<Vec<_>>();
        if record.get("runMode").and_then(Value::as_str) == Some("random") && selected.len() > 1 {
            let index = fastrand::usize(..selected.len());
            selected = vec![selected.swap_remove(index)];
        }
        selected
    }

    pub fn emit_depth(&self, event: &Value) -> u64 {
        let tracked = event
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|event_type| {
                event_type == "plugin.emit"
                    || recover_rwlock_read(&self.trigger_owners, "automation trigger owners")
                        .contains_key(event_type)
            });
        if !tracked {
            return 0;
        }
        event
            .get("data")
            .and_then(|data| data.get("depth"))
            .and_then(number_u64)
            .unwrap_or_default()
    }
}
