//! Host-side synchronization for the global hotkey process plugin.
//!
//! Wayland's portal only receives shortcuts that have been registered ahead of
//! time. Behavior records are the source of truth, so this module projects
//! enabled `hotkey.pressed` filters into the plugin's `hotkey.bind` contract.
//! Filters that cannot describe one complete portal chord deliberately keep
//! the raw-input backend enabled.

use super::*;
use std::collections::BTreeSet;

const HOTKEY_PLUGIN_ID: &str = "hotkeys";
const HOTKEY_BIND_ACTION: &str = "hotkey.bind";

impl AppCore {
    /// Marks the persisted Behavior projection dirty. The next background
    /// plugin tick performs the process call; callers remain synchronous.
    pub(super) fn request_hotkey_sync(&self) {
        self.hotkey_sync_revision.fetch_add(1, Ordering::AcqRel);
    }

    /// Synchronizes configured hotkey filters with the hotkey process plugin.
    /// This runs on the plugin polling task, never on the webview IPC handler.
    pub(crate) async fn sync_hotkey_bindings(self: &Arc<Self>) {
        let Some(plugin) = self.plugins.get(HOTKEY_PLUGIN_ID) else {
            return;
        };
        if plugin.manifest.id != HOTKEY_PLUGIN_ID || !self.plugin_ready(HOTKEY_PLUGIN_ID) {
            return;
        }
        if !plugin_has_action(&plugin.manifest.action_types, HOTKEY_BIND_ACTION) {
            tracing::warn!(
                plugin = HOTKEY_PLUGIN_ID,
                action = HOTKEY_BIND_ACTION,
                "hotkey plugin does not declare its host synchronization action"
            );
            return;
        }

        let revision = self.hotkey_sync_revision.load(Ordering::Acquire);
        if revision == self.hotkey_synced_revision.load(Ordering::Acquire)
            && self.plugins.is_running(HOTKEY_PLUGIN_ID)
        {
            return;
        }
        if !self.plugin_retry_allowed(HOTKEY_PLUGIN_ID) {
            return;
        }

        let snapshot = self.load_behavior_snapshot_for_hotkey_sync();
        let action = json!({
            "typeId": HOTKEY_BIND_ACTION,
            "config": desired_hotkey_bind_config(&snapshot),
        });
        let mut logs = Vec::new();
        match self
            .execute_plugin_action(HOTKEY_BIND_ACTION, &action, &json!({}), &mut logs, false)
            .await
        {
            Ok(summary) => {
                self.hotkey_synced_revision
                    .store(revision, Ordering::Release);
                self.record_plugin_success(HOTKEY_PLUGIN_ID);
                tracing::debug!(
                    plugin = HOTKEY_PLUGIN_ID,
                    revision,
                    summary = %summary,
                    "hotkey behavior projection synchronized"
                );
            }
            Err(error) => {
                self.record_plugin_failure(HOTKEY_PLUGIN_ID, error.clone());
                tracing::warn!(
                    plugin = HOTKEY_PLUGIN_ID,
                    revision,
                    %error,
                    "could not synchronize hotkey behavior projection"
                );
            }
        }
    }
}

/// Builds the process plugin's stable bind payload from a persisted Behavior
/// snapshot. A full `key eq` + non-empty `modifiers eq` pair becomes a portal
/// chord; every other enabled hotkey behavior keeps raw input enabled.
pub(crate) fn desired_hotkey_bind_config(snapshot: &Value) -> Value {
    let mut shortcuts = BTreeSet::new();
    let mut sequences_needed = false;

    for event in snapshot
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|event| {
            event.get("enabled").and_then(Value::as_bool) == Some(true)
                && event.get("trigger").and_then(Value::as_str) == Some("hotkey.pressed")
        })
    {
        let filters = event
            .get("filters")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let key = exact_filter_value(&filters, "event.data.key");
        let modifiers =
            exact_filter_value(&filters, "event.data.modifiers").and_then(normalize_modifiers);

        let Some(key) = key.and_then(normalize_key) else {
            sequences_needed = true;
            continue;
        };
        let Some(modifiers) = modifiers else {
            sequences_needed = true;
            continue;
        };
        if modifiers.is_empty() || !portal_key_supported(&key) {
            sequences_needed = true;
            continue;
        }
        if filters_need_raw_input(&filters) {
            sequences_needed = true;
        }
        shortcuts.insert((key, modifiers));
    }

    json!({
        "shortcuts": shortcuts
            .into_iter()
            .map(|(key, modifiers)| json!({"key": key, "modifiers": modifiers}))
            .collect::<Vec<_>>(),
        "sequencesNeeded": sequences_needed,
    })
}

fn plugin_has_action(action_types: &[Value], action_id: &str) -> bool {
    action_types
        .iter()
        .any(|entry| entry.get("id").and_then(Value::as_str) == Some(action_id))
}

fn exact_filter_value<'a>(filters: &'a [Value], path: &str) -> Option<&'a str> {
    filters.iter().find_map(|filter| {
        (filter.get("path").and_then(Value::as_str) == Some(path)
            && filter.get("operator").and_then(Value::as_str) == Some("eq"))
        .then(|| filter.get("value").and_then(Value::as_str))
        .flatten()
    })
}

fn normalize_key(raw: &str) -> Option<String> {
    let key = raw.trim().to_ascii_lowercase();
    if key.is_empty() || matches!(key.as_str(), "ctrl" | "shift" | "alt" | "meta") {
        return None;
    }
    Some(key)
}

fn normalize_modifiers(raw: &str) -> Option<String> {
    let mut seen = BTreeSet::new();
    for part in raw
        .split('+')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty())
    {
        match part.as_str() {
            "ctrl" | "control" => {
                seen.insert("ctrl");
            }
            "shift" => {
                seen.insert("shift");
            }
            "alt" | "altgr" => {
                seen.insert("alt");
            }
            "meta" | "super" | "win" | "mod4" => {
                seen.insert("meta");
            }
            _ => return None,
        }
    }
    Some(
        ["ctrl", "shift", "alt", "meta"]
            .into_iter()
            .filter(|modifier| seen.contains(*modifier))
            .collect::<Vec<_>>()
            .join("+"),
    )
}

fn portal_key_supported(key: &str) -> bool {
    if key.len() == 1 && key.as_bytes()[0].is_ascii_graphic() {
        return true;
    }
    matches!(
        key,
        "space"
            | "enter"
            | "tab"
            | "esc"
            | "backspace"
            | "delete"
            | "insert"
            | "home"
            | "end"
            | "pageup"
            | "pagedown"
            | "up"
            | "down"
            | "left"
            | "right"
            | "capslock"
            | "numlock"
            | "printscreen"
            | "pause"
            | "f1"
            | "f2"
            | "f3"
            | "f4"
            | "f5"
            | "f6"
            | "f7"
            | "f8"
            | "f9"
            | "f10"
            | "f11"
            | "f12"
    )
}

fn filters_need_raw_input(filters: &[Value]) -> bool {
    filters.iter().any(|filter| {
        let path = filter
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if path == "event.data.sequence" {
            return true;
        }
        if path != "event.data.backend" {
            return false;
        }
        filter.get("operator").and_then(Value::as_str) != Some("eq")
            || filter.get("value").and_then(Value::as_str) != Some("portal")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(event: Value) -> Value {
        desired_hotkey_bind_config(&json!({"events": [event]}))
    }

    fn event(filters: Value) -> Value {
        json!({
            "enabled": true,
            "trigger": "hotkey.pressed",
            "filters": filters,
        })
    }

    #[test]
    fn bare_key_requires_raw_input_instead_of_a_portal_binding() {
        let actual = config(event(json!([
            {"path": "event.data.key", "operator": "eq", "value": "a"}
        ])));
        assert_eq!(actual, json!({"shortcuts": [], "sequencesNeeded": true}));
    }

    #[test]
    fn complete_chord_is_registered_without_requesting_raw_input() {
        let actual = config(event(json!([
            {"path": "event.data.key", "operator": "eq", "value": "A"},
            {"path": "event.data.modifiers", "operator": "eq", "value": "shift+ctrl"}
        ])));
        assert_eq!(
            actual,
            json!({
                "shortcuts": [{"key": "a", "modifiers": "ctrl+shift"}],
                "sequencesNeeded": false
            })
        );
    }

    #[test]
    fn sequence_and_backend_filters_keep_raw_input_enabled() {
        let actual = config(event(json!([
            {"path": "event.data.key", "operator": "eq", "value": "a"},
            {"path": "event.data.modifiers", "operator": "eq", "value": "ctrl"},
            {"path": "event.data.sequence", "operator": "contains", "value": "a b"}
        ])));
        assert_eq!(
            actual["shortcuts"],
            json!([{"key": "a", "modifiers": "ctrl"}])
        );
        assert_eq!(actual["sequencesNeeded"], true);
    }

    #[test]
    fn disabled_events_are_not_projected() {
        let actual = desired_hotkey_bind_config(&json!({
            "events": [{
                "enabled": false,
                "trigger": "hotkey.pressed",
                "filters": [{"path": "event.data.key", "operator": "eq", "value": "a"}]
            }]
        }));
        assert_eq!(actual, json!({"shortcuts": [], "sequencesNeeded": false}));
    }
}
