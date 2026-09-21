//! Behavior snapshot index and plugin trigger ownership.

use super::*;

impl AutomationService {
    pub fn replace_snapshot(&self, snapshot: &Value) {
        let actions = records_by_id(snapshot.get("actions"));
        let events = records_by_id(snapshot.get("events"));
        *recover_rwlock_write(&self.actions, "automation actions") = actions;
        *recover_rwlock_write(&self.events, "automation events") = events;
        let (trigger_owners, enabled_plugins) = plugin_trigger_ownership(snapshot);
        *recover_rwlock_write(&self.trigger_owners, "automation trigger owners") = trigger_owners;
        *recover_rwlock_write(&self.enabled_plugins, "automation enabled plugins") =
            enabled_plugins;
    }

    pub fn upsert_action(&self, action: Value) {
        if let Some(id) = action.get("id").and_then(Value::as_str) {
            recover_rwlock_write(&self.actions, "automation actions").insert(id.to_owned(), action);
        }
    }

    pub fn remove_action(&self, id: &str) {
        recover_rwlock_write(&self.actions, "automation actions").remove(id);
    }

    pub fn upsert_event(&self, event: Value) {
        if let Some(id) = event.get("id").and_then(Value::as_str) {
            recover_rwlock_write(&self.events, "automation events").insert(id.to_owned(), event);
        }
    }

    pub fn remove_event(&self, id: &str) {
        recover_rwlock_write(&self.events, "automation events").remove(id);
        recover_mutex(&self.cooldowns, "automation cooldowns")
            .retain(|key, _| !key.starts_with(&format!("{id}:")));
    }
}

/// Plugin-owned triggers from a behavior snapshot: trigger type -> plugin id,
/// plus the set of plugins that are installed, enabled, and available.
fn plugin_trigger_ownership(
    snapshot: &Value,
) -> (BTreeMap<String, String>, std::collections::BTreeSet<String>) {
    let mut trigger_owners = BTreeMap::new();
    if let Some(entries) = snapshot.get("eventTypes").and_then(Value::as_array) {
        for entry in entries {
            let event_type = entry.get("type").and_then(Value::as_str);
            let plugin_id = entry
                .get("source")
                .and_then(|source| source.get("pluginId"))
                .and_then(Value::as_str);
            if let (Some(event_type), Some(plugin_id)) = (event_type, plugin_id) {
                trigger_owners.insert(event_type.to_owned(), plugin_id.to_owned());
            }
        }
    }
    let mut enabled_plugins = std::collections::BTreeSet::new();
    if let Some(plugins) = snapshot.get("plugins").and_then(Value::as_array) {
        for plugin in plugins {
            let id = plugin
                .get("descriptor")
                .and_then(|descriptor| descriptor.get("id"))
                .and_then(Value::as_str)
                .or_else(|| plugin.get("id").and_then(Value::as_str));
            let active = plugin.get("installed").and_then(Value::as_bool) == Some(true)
                && plugin.get("enabled").and_then(Value::as_bool) == Some(true)
                && plugin.get("available").and_then(Value::as_bool) != Some(false);
            if let Some(id) = id {
                if active {
                    enabled_plugins.insert(id.to_owned());
                }
            }
        }
    }
    (trigger_owners, enabled_plugins)
}

/// Built-in triggers always match; plugin triggers only match while their
/// owning plugin is active, so disabling a plugin pauses its events.
pub(crate) fn trigger_allowed(
    trigger: Option<&str>,
    trigger_owners: &BTreeMap<String, String>,
    enabled_plugins: &std::collections::BTreeSet<String>,
) -> bool {
    let Some(trigger) = trigger else {
        return false;
    };
    match trigger_owners.get(trigger) {
        None => true,
        Some(plugin_id) => enabled_plugins.contains(plugin_id),
    }
}

fn records_by_id(value: Option<&Value>) -> BTreeMap<String, Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            value
                .get("id")
                .and_then(Value::as_str)
                .map(|id| (id.to_owned(), value.clone()))
        })
        .collect()
}
