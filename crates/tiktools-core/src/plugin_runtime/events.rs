//! Plugin event samples for automation triggers.

use crate::*;

impl AppCore {
    /// Sample event for a plugin-owned trigger, taken from the declaring
    /// plugin's manifest sample so `test-event` previews realistic data.
    pub(crate) fn plugin_event_sample(&self, trigger: &str) -> Option<Value> {
        for plugin in self.plugins.list() {
            for entry in &plugin.manifest.event_types {
                if tiktools_plugin_api::manifest::validate_event_type(entry).is_err() {
                    continue;
                }
                if entry.get("type").and_then(Value::as_str) != Some(trigger) {
                    continue;
                }
                let data = entry
                    .get("sample")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                return Some(json!({
                    "id": "sample-event",
                    "type": trigger,
                    "timestamp": now_millis(),
                    "user": {"uniqueId": "viewer_demo", "nickname": "Viewer Demo", "userId": "1"},
                    "data": Value::Object(data),
                }));
            }
        }
        None
    }
}
