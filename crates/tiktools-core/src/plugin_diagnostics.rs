//! In-memory plugin runtime diagnostics: last received event per plugin,
//! host-side poll drop counters, and hotkey binding synchronization state.
//!
//! Everything here is process-local and minimal: timestamps, counters, and
//! the small hotkey chord projection. No keyboard history is persisted and
//! no key contents are logged.

use super::*;

/// Last validated event received from one plugin. The `detail` projection
/// carries only the small hotkey chord fields (`key`, `modifiers`,
/// `backend`, `sequence`) when present as short strings; anything else is
/// omitted so arbitrary plugin payloads never accumulate here.
#[derive(Debug, Clone, Default)]
pub(crate) struct PluginLastEvent {
    pub(crate) event_type: String,
    pub(crate) at: u64,
    pub(crate) detail: BTreeMap<String, String>,
}

/// Host-side hotkey binding synchronization state. `desired_revision` is the
/// persisted Behavior projection revision, `applied_revision` the revision
/// last confirmed by the plugin process; equality plus `last_error == None`
/// means the plugin runs the current bindings.
#[derive(Debug, Clone, Default)]
pub(crate) struct HotkeySyncState {
    pub(crate) last_config: Option<Value>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_sync_at: Option<u64>,
}

impl AppCore {
    /// Records one validated plugin event for diagnostics. Called on the
    /// control-plane path (before automation), so the record exists even
    /// when automation has no match, fails, or is saturated.
    pub(crate) fn record_plugin_event(&self, plugin_id: &str, event: &Value) {
        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let mut detail = BTreeMap::new();
        if let Some(data) = event.get("data").and_then(Value::as_object) {
            for key in ["key", "modifiers", "backend", "sequence"] {
                if let Some(value) = data.get(key).and_then(Value::as_str) {
                    if !value.is_empty() && value.len() <= 64 {
                        detail.insert(key.to_owned(), value.to_owned());
                    }
                }
            }
        }
        self.plugin_last_events
            .lock()
            .expect("plugin last-event lock poisoned")
            .insert(
                plugin_id.to_owned(),
                PluginLastEvent {
                    event_type,
                    at: now_millis(),
                    detail,
                },
            );
    }

    /// Adds host-side poll drops for one plugin. Every drop is also logged
    /// at the call site with its reason; the counter makes totals visible
    /// through diagnostics and health RPCs.
    pub(crate) fn record_plugin_drops(&self, plugin_id: &str, dropped: u64) {
        if dropped == 0 {
            return;
        }
        *self
            .plugin_event_drops
            .lock()
            .expect("plugin drop lock poisoned")
            .entry(plugin_id.to_owned())
            .or_default() += dropped;
    }

    pub(crate) fn plugin_drop_total(&self) -> u64 {
        self.plugin_event_drops
            .lock()
            .expect("plugin drop lock poisoned")
            .values()
            .sum()
    }

    /// Read-only diagnostics snapshot for the `plugins.diagnostics` RPC:
    /// per-plugin last event, host-side drops, and hotkey sync state.
    pub fn plugin_diagnostics(&self) -> Value {
        let last_events = self
            .plugin_last_events
            .lock()
            .expect("plugin last-event lock poisoned")
            .clone();
        let drops = self
            .plugin_event_drops
            .lock()
            .expect("plugin drop lock poisoned")
            .clone();
        let sync = self
            .hotkey_sync_state
            .lock()
            .expect("hotkey sync lock poisoned")
            .clone();
        let desired = self.hotkey_sync_revision.load(Ordering::Acquire);
        let applied = self.hotkey_synced_revision.load(Ordering::Acquire);
        let mut plugins: Vec<Value> = self
            .plugins
            .list()
            .into_iter()
            .map(|plugin| {
                let id = plugin.manifest.id.clone();
                let last = last_events.get(&id);
                json!({
                    "pluginId": id,
                    "running": plugin.running,
                    "available": plugin.available,
                    "lastEventAt": last.map(|event| event.at),
                    "lastEventType": last.map(|event| event.event_type.clone()),
                    "lastEvent": last.map(|event| event.detail.clone()),
                    "droppedEvents": drops.get(&id).copied().unwrap_or(0),
                })
            })
            .collect();
        plugins.sort_by(|left, right| {
            left.get("pluginId")
                .and_then(Value::as_str)
                .cmp(&right.get("pluginId").and_then(Value::as_str))
        });
        json!({
            "plugins": plugins,
            "hotkeySync": {
                "desiredRevision": desired,
                "appliedRevision": applied,
                "inSync": desired == applied && sync.last_error.is_none(),
                "lastSyncAt": sync.last_sync_at,
                "lastError": sync.last_error,
                "appliedConfig": sync.last_config,
            },
        })
    }
}
