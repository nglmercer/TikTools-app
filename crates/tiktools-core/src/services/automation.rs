use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, RwLock,
    },
};

use serde_json::Value;

use super::ScriptService;

const MAX_COOLDOWN_MS: u64 = 24 * 60 * 60 * 1_000;

/// Automation orchestration boundary. The workflow persistence and event
/// routing stay in `AppCore`; script evaluation is isolated here so it can be
/// tested without a desktop object or a plugin runtime.
pub struct AutomationService {
    script: ScriptService,
    actions: RwLock<BTreeMap<String, Value>>,
    events: RwLock<BTreeMap<String, Value>>,
    trigger_owners: RwLock<BTreeMap<String, String>>,
    enabled_plugins: RwLock<std::collections::BTreeSet<String>>,
    cooldowns: Mutex<BTreeMap<String, u64>>,
    runs: Mutex<Vec<Value>>,
    sequence: AtomicU64,
}

impl Default for AutomationService {
    fn default() -> Self {
        Self {
            script: ScriptService,
            actions: RwLock::new(BTreeMap::new()),
            events: RwLock::new(BTreeMap::new()),
            trigger_owners: RwLock::new(BTreeMap::new()),
            enabled_plugins: RwLock::new(std::collections::BTreeSet::new()),
            cooldowns: Mutex::new(BTreeMap::new()),
            runs: Mutex::new(Vec::new()),
            sequence: AtomicU64::new(0),
        }
    }
}

impl AutomationService {
    pub fn replace_snapshot(&self, snapshot: &Value) {
        let actions = records_by_id(snapshot.get("actions"));
        let events = records_by_id(snapshot.get("events"));
        *self.actions.write().expect("automation actions poisoned") = actions;
        *self.events.write().expect("automation events poisoned") = events;
        let (trigger_owners, enabled_plugins) = plugin_trigger_ownership(snapshot);
        *self
            .trigger_owners
            .write()
            .expect("automation trigger owners poisoned") = trigger_owners;
        *self
            .enabled_plugins
            .write()
            .expect("automation enabled plugins poisoned") = enabled_plugins;
    }

    pub fn upsert_action(&self, action: Value) {
        if let Some(id) = action.get("id").and_then(Value::as_str) {
            self.actions
                .write()
                .expect("automation actions poisoned")
                .insert(id.to_owned(), action);
        }
    }

    pub fn remove_action(&self, id: &str) {
        self.actions
            .write()
            .expect("automation actions poisoned")
            .remove(id);
    }

    pub fn upsert_event(&self, event: Value) {
        if let Some(id) = event.get("id").and_then(Value::as_str) {
            self.events
                .write()
                .expect("automation events poisoned")
                .insert(id.to_owned(), event);
        }
    }

    pub fn remove_event(&self, id: &str) {
        self.events
            .write()
            .expect("automation events poisoned")
            .remove(id);
        self.cooldowns
            .lock()
            .expect("automation cooldowns poisoned")
            .retain(|key, _| !key.starts_with(&format!("{id}:")));
    }

    pub fn matching_events(&self, event: &Value) -> Vec<Value> {
        let event_type = event.get("type").and_then(Value::as_str);
        let trigger_owners = self
            .trigger_owners
            .read()
            .expect("automation trigger owners poisoned");
        let enabled_plugins = self
            .enabled_plugins
            .read()
            .expect("automation enabled plugins poisoned");
        self.events
            .read()
            .expect("automation events poisoned")
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

    pub fn claim_event(&self, record: &Value, event: &Value, now: u64) -> bool {
        if !event_record_matches(record, event) {
            return false;
        }
        let cooldown_ms = record
            .get("cooldownMs")
            .and_then(number_u64)
            .unwrap_or_default()
            .min(MAX_COOLDOWN_MS);
        if cooldown_ms == 0 {
            return true;
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or("event");
        let scope = if record.get("cooldownScope").and_then(Value::as_str) == Some("global") {
            "global".to_owned()
        } else {
            event
                .get("user")
                .and_then(|user| user.get("uniqueId"))
                .and_then(Value::as_str)
                .unwrap_or("anonymous")
                .to_owned()
        };
        let key = format!("{id}:{scope}");
        let mut cooldowns = self
            .cooldowns
            .lock()
            .expect("automation cooldowns poisoned");
        cooldowns.retain(|_, previous| now.saturating_sub(*previous) <= MAX_COOLDOWN_MS);
        if cooldowns
            .get(&key)
            .is_some_and(|previous| now.saturating_sub(*previous) < cooldown_ms)
        {
            return false;
        }
        cooldowns.insert(key, now);
        true
    }

    pub fn actions_for_event(&self, record: &Value) -> Vec<Value> {
        let ids = record
            .get("actionIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        let actions = self.actions.read().expect("automation actions poisoned");
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

    pub fn next_run_id(&self, prefix: &str, at: u64) -> String {
        format!(
            "{}-{}-{}",
            prefix,
            self.sequence.fetch_add(1, Ordering::AcqRel) + 1,
            at
        )
    }

    pub fn record_run(&self, run: Value) -> Vec<Value> {
        let mut runs = self.runs.lock().expect("automation runs poisoned");
        runs.insert(0, run);
        runs.truncate(60);
        runs.clone()
    }

    pub fn recent_runs(&self) -> Vec<Value> {
        self.runs.lock().expect("automation runs poisoned").clone()
    }

    pub fn clear_runs(&self) {
        self.runs.lock().expect("automation runs poisoned").clear();
    }

    pub fn emit_depth(&self, event: &Value) -> u64 {
        let tracked = event
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|event_type| {
                event_type == "plugin.emit"
                    || self
                        .trigger_owners
                        .read()
                        .expect("automation trigger owners poisoned")
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

    pub fn evaluate_script(
        &self,
        source: &str,
        event: &Value,
        inputs: &Value,
    ) -> Result<Value, String> {
        self.script.evaluate(source, event, inputs)
    }

    pub fn validate_script(&self, source: &str) -> Result<(), String> {
        self.script.validate(source)
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
fn trigger_allowed(
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

fn event_record_matches(record: &Value, event: &Value) -> bool {
    let Some(filters) = record.get("filters").and_then(Value::as_array) else {
        return true;
    };
    filters.iter().all(|filter| matches_filter(filter, event))
}

fn matches_filter(filter: &Value, event: &Value) -> bool {
    let path = filter
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let raw = read_event_path(event, path);
    let left = raw.map(value_to_string).unwrap_or_default();
    let right = filter
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let left_number = left.parse::<f64>().ok();
    let right_number = right.parse::<f64>().ok();
    let numeric = left_number.is_some() && right_number.is_some() && !right.trim().is_empty();
    match filter
        .get("operator")
        .and_then(Value::as_str)
        .unwrap_or("eq")
    {
        "gte" => numeric && left_number >= right_number,
        "gt" => numeric && left_number > right_number,
        "lte" => numeric && left_number <= right_number,
        "lt" => numeric && left_number < right_number,
        "eq" => {
            if numeric {
                left_number == right_number
            } else {
                left == right
            }
        }
        "neq" => {
            if numeric {
                left_number != right_number
            } else {
                left != right
            }
        }
        "contains" => left
            .to_ascii_lowercase()
            .contains(&right.to_ascii_lowercase()),
        "starts-with" => left
            .to_ascii_lowercase()
            .starts_with(&right.to_ascii_lowercase()),
        "in" => filter
            .get("values")
            .and_then(Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|value| value.trim().eq_ignore_ascii_case(left.trim()))
            }),
        "is-true" => raw == Some(&Value::Bool(true)) || left == "true" || left == "1",
        "is-false" => {
            raw == Some(&Value::Bool(false)) || left == "false" || left == "0" || left.is_empty()
        }
        _ => false,
    }
}

/// Reads one `event.*` path with the shared filter/template path language
/// (a leading `event.` prefix addresses the envelope root).
pub(crate) fn read_event_path<'a>(event: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path
        .trim()
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim();
    let path = path.strip_prefix("event.").unwrap_or(path);
    let path = path.strip_prefix("event").unwrap_or(path);
    let mut current = event;
    for part in path.trim_matches('.').split('.') {
        if part.is_empty() {
            continue;
        }
        current = current.get(part)?;
    }
    Some(current)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn number_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| {
            value
                .as_f64()
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value as u64)
        })
        .or_else(|| value.as_str()?.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn enriched_intel_paths_match_generic_filters() {
        let service = AutomationService::default();
        let record = json!({
            "id": "event",
            "enabled": true,
            "trigger": "tiktok.chat",
            "filters": [
                {"path": "event.intel.comment.composition.emojiOnly", "operator": "is-true"},
                {"path": "event.intel.comment.spam.score", "operator": "lt", "value": "0.70"},
                {"path": "event.intel.comment.language.top", "operator": "eq", "value": "es"},
            ],
        });
        let matching = json!({
            "type": "tiktok.chat",
            "user": {"uniqueId": "alice"},
            "data": {"comment": "😂😂😂"},
            "intel": {"comment": {
                "composition": {"emojiOnly": true},
                "spam": {"score": 0.04},
                "language": {"top": "es", "confidence": 0.9},
            }},
        });
        assert!(service.event_record_matches(&record, &matching));
        // Raw events without enrichment never match intel filters.
        let raw = json!({
            "type": "tiktok.chat",
            "user": {"uniqueId": "alice"},
            "data": {"comment": "😂😂😂"},
        });
        assert!(!service.event_record_matches(&record, &raw));
        // Score above the threshold fails the numeric filter.
        let spammy = json!({
            "type": "tiktok.chat",
            "user": {"uniqueId": "alice"},
            "data": {"comment": "buy now"},
            "intel": {"comment": {
                "composition": {"emojiOnly": true},
                "spam": {"score": 0.91},
                "language": {"top": "es", "confidence": 0.9},
            }},
        });
        assert!(!service.event_record_matches(&record, &spammy));
    }

    #[test]
    fn matches_filters_and_enforces_user_cooldowns() {
        let service = AutomationService::default();
        service.replace_snapshot(&json!({
            "actions": [{"id":"action","enabled":true}],
            "events": [{
                "id":"event","enabled":true,"trigger":"tiktok.chat",
                "filters":[{"path":"event.data.comment","operator":"contains","value":"hello"}],
                "cooldownMs":1000,"cooldownScope":"user","actionIds":["action"]
            }]
        }));
        let event = json!({"type":"tiktok.chat","user":{"uniqueId":"alice"},"data":{"comment":"Hello there"}});
        let record = service
            .matching_events(&event)
            .pop()
            .expect("event should match");
        assert!(service.claim_event(&record, &event, 100));
        assert!(!service.claim_event(&record, &event, 500));
        assert!(service.claim_event(&record, &event, 1_100));
        assert_eq!(service.actions_for_event(&record).len(), 1);
    }
    #[test]
    fn plugin_triggers_match_only_while_their_plugin_is_active() {
        let service = AutomationService::default();
        service.replace_snapshot(&json!({
            "actions": [{"id":"action","enabled":true}],
            "events": [{
                "id":"event","enabled":true,"trigger":"hotkey.pressed",
                "filters":[{"path":"event.data.key","operator":"eq","value":"ctrl+k"}],
                "cooldownMs":0,"cooldownScope":"user","actionIds":["action"]
            }],
            "eventTypes": [{
                "type":"hotkey.pressed","title":{"default":"Hotkey pressed"},
                "source":{"kind":"plugin","pluginId":"hotkeys"}
            }],
            "plugins": [{
                "descriptor":{"id":"hotkeys"},"installed":true,"enabled":true,"available":true
            }]
        }));
        let event =
            json!({"type":"hotkey.pressed","user":{"uniqueId":"alice"},"data":{"key":"ctrl+k"}});
        assert_eq!(service.matching_events(&event).len(), 1);

        // Disabled plugin pauses its triggers; built-ins keep matching.
        service.replace_snapshot(&json!({
            "actions": [{"id":"action","enabled":true}],
            "events": [
                {"id":"event","enabled":true,"trigger":"hotkey.pressed","filters":[],"actionIds":["action"]},
                {"id":"builtin","enabled":true,"trigger":"tiktok.chat","filters":[],"actionIds":["action"]}
            ],
            "eventTypes": [{
                "type":"hotkey.pressed","title":{"default":"Hotkey pressed"},
                "source":{"kind":"plugin","pluginId":"hotkeys"}
            }],
            "plugins": [{
                "descriptor":{"id":"hotkeys"},"installed":true,"enabled":false,"available":true
            }]
        }));
        assert!(service.matching_events(&event).is_empty());
        let chat = json!({"type":"tiktok.chat","user":{"uniqueId":"alice"},"data":{}});
        assert_eq!(service.matching_events(&chat).len(), 1);
    }

    #[test]
    fn poll_responses_keep_only_declared_typed_events() {
        let declared = vec!["hotkey.pressed".to_owned()];
        let response = tiktools_plugin_sdk::decode_plugin_result(json!({"events": [
            {"type": "hotkey.pressed", "data": {"key": "ctrl+k"}},
            {"type": "tiktok.chat", "data": {}},
            {"type": "hotkey.pressed", "data": "nope"},
            {"type": "other.thing", "data": {}},
        ]}))
        .unwrap();
        let events = crate::parse_polled_events(&declared, &response);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "hotkey.pressed");
        assert_eq!(events[0].1, json!({"key": "ctrl+k"}));

        // Oversized payloads are dropped.
        let big = "x".repeat(70 * 1024);
        let response = tiktools_plugin_sdk::decode_plugin_result(
            json!({"events": [{"type": "hotkey.pressed", "data": {"blob": big}}]}),
        )
        .unwrap();
        let events = crate::parse_polled_events(&declared, &response);
        assert!(events.is_empty());
    }
}
