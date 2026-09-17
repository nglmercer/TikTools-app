//! Host-side dynamic options for action and settings fields.
//!
//! A field that names `plugin-action-options:<actionType>:<field>` gets its
//! choices from the declaring plugin's option source: the host fetches the
//! manifest-declared endpoint (same origin, auth, and transport policy as
//! declarative actions), maps the JSON body to `{value, label}` pairs, and
//! caches the result briefly. The WebView only ever sees the mapped labels;
//! endpoint URLs, tokens, and raw bodies stay host-side.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use serde_json::{json, Value};

/// Source-id prefix for plugin dynamic options. The remainder is
/// `<actionType>:<field>`; action type ids never contain a colon, so the
/// split is unambiguous.
pub const OPTION_SOURCE_PREFIX: &str = "plugin-action-options:";
/// Cache lifetime for one fetched option list.
pub const OPTION_SOURCE_TTL: Duration = Duration::from_secs(60);
pub(crate) const MAX_OPTION_ITEMS: usize = 500;
const MAX_OPTION_VALUE_LEN: usize = 256;
const MAX_OPTION_LABEL_LEN: usize = 256;
/// Object keys tried, in order, for the item array when a source declares
/// no `itemsPath`.
const DEFAULT_ITEMS_KEYS: [&str; 3] = ["items", "voices", "data"];

struct CachedOptions {
    fetched_at: Instant,
    options: Vec<Value>,
}

/// TTL cache for fetched option lists. Entries are keyed by full source id;
/// any settings save clears the cache because a server URL or token may
/// have changed what the endpoints return.
pub struct OptionSourceService {
    cache: Mutex<HashMap<String, CachedOptions>>,
}

impl Default for OptionSourceService {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionSourceService {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn cached(&self, source: &str) -> Option<Vec<Value>> {
        let mut cache = self.cache.lock().expect("option cache lock poisoned");
        let entry = cache.get(source)?;
        if entry.fetched_at.elapsed() > OPTION_SOURCE_TTL {
            cache.remove(source);
            return None;
        }
        Some(entry.options.clone())
    }

    pub fn store(&self, source: &str, options: Vec<Value>) {
        self.cache
            .lock()
            .expect("option cache lock poisoned")
            .insert(
                source.to_owned(),
                CachedOptions {
                    fetched_at: Instant::now(),
                    options,
                },
            );
    }

    pub fn clear(&self) {
        self.cache
            .lock()
            .expect("option cache lock poisoned")
            .clear();
    }
}

/// Splits `plugin-action-options:<actionType>:<field>` into its parts.
/// Anything else (legacy or unknown sources) yields `None`.
pub fn parse_option_source(source: &str) -> Option<(String, String)> {
    let rest = source.strip_prefix(OPTION_SOURCE_PREFIX)?;
    let (action_type, field) = rest.split_once(':')?;
    if action_type.trim().is_empty() || field.trim().is_empty() || field.contains(':') {
        return None;
    }
    Some((action_type.to_owned(), field.to_owned()))
}

/// Maps one fetched document to `{value, label}` option items.
///
/// Item lookup: explicit `itemsPath` (dotted), else a root array, else the
/// first of `items`/`voices`/`data` that holds an array. Value lookup per
/// item: explicit `valuePath`, else `id`, else `value`. Label lookup:
/// explicit `labelPath`, else `name`, else `label`, else the value. Plain
/// strings become identical value/label pairs. Entries without a usable
/// value are skipped; the list is capped at [`MAX_OPTION_ITEMS`].
pub fn map_option_items(
    body: &Value,
    items_path: Option<&str>,
    value_path: Option<&str>,
    label_path: Option<&str>,
) -> Result<Vec<Value>, String> {
    let items = find_option_items(body, items_path)?;
    let mut options = Vec::new();
    for item in items {
        let Some((value, label)) = map_option_item(item, value_path, label_path) else {
            continue;
        };
        if value.is_empty() || value.len() > MAX_OPTION_VALUE_LEN {
            continue;
        }
        let label = if label.len() > MAX_OPTION_LABEL_LEN {
            label.chars().take(MAX_OPTION_LABEL_LEN).collect()
        } else {
            label
        };
        options.push(json!({"value": value, "label": label}));
        if options.len() >= MAX_OPTION_ITEMS {
            break;
        }
    }
    Ok(options)
}

fn find_option_items<'a>(
    body: &'a Value,
    items_path: Option<&str>,
) -> Result<Vec<&'a Value>, String> {
    if let Some(path) = items_path {
        let target = read_dotted_path(body, path)
            .ok_or_else(|| "The option list is missing its items path.".to_owned())?;
        return target
            .as_array()
            .map(|items| items.iter().collect())
            .ok_or_else(|| "The option list items path is not an array.".to_owned());
    }
    if let Some(items) = body.as_array() {
        return Ok(items.iter().collect());
    }
    if let Some(object) = body.as_object() {
        for key in DEFAULT_ITEMS_KEYS {
            if let Some(items) = object.get(key).and_then(Value::as_array) {
                return Ok(items.iter().collect());
            }
        }
    }
    Err("The option response holds no option list.".to_owned())
}

fn map_option_item(
    item: &Value,
    value_path: Option<&str>,
    label_path: Option<&str>,
) -> Option<(String, String)> {
    if let Some(text) = item.as_str() {
        return Some((text.to_owned(), text.to_owned()));
    }
    if !item.is_object() {
        return None;
    }
    let value_paths: Vec<&str> = value_path.into_iter().chain(["id", "value"]).collect();
    let value = value_paths
        .iter()
        .filter_map(|path| read_dotted_path(item, path))
        .find_map(scalar_text)?;
    let label_paths: Vec<&str> = label_path.into_iter().chain(["name", "label"]).collect();
    let label = label_paths
        .iter()
        .filter_map(|path| read_dotted_path(item, path))
        .find_map(scalar_text)
        .unwrap_or_else(|| value.clone());
    Some((value, label))
}

fn read_dotted_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }
        current = current.get(part)?;
    }
    Some(current)
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

impl crate::AppCore {
    /// Resolves one `get-action-options` source to its items plus an optional
    /// display-safe error. Cache hits skip the fetch; failures yield empty
    /// options with an explanatory error instead of failing the form.
    pub(crate) async fn resolve_action_options(
        self: &std::sync::Arc<Self>,
        source: &str,
    ) -> (Vec<Value>, Option<String>) {
        let fail = |message: String| (Vec::new(), Some(message));
        if let Some(options) = self.option_sources.cached(source) {
            return (options, None);
        }
        let Some((action_type, field)) = parse_option_source(source) else {
            return fail(format!("Unknown option source `{source}`."));
        };
        let Some((plugin, _)) = self.plugin_for_action(&action_type) else {
            return fail(format!("Action type `{action_type}` is not available."));
        };
        if !self.plugin_ready(&plugin.manifest.id) {
            return fail(format!(
                "Plugin `{}` is not installed, enabled, or available.",
                plugin.manifest.id
            ));
        }
        if plugin.manifest.schema_version < 3 {
            return fail(format!(
                "Action type `{action_type}` declares no dynamic options."
            ));
        }
        let descriptor = plugin.manifest.action_types.iter().find(|descriptor| {
            descriptor.get("id").and_then(Value::as_str) == Some(action_type.as_str())
        });
        let option = descriptor
            .and_then(|descriptor| descriptor.get("optionSources"))
            .and_then(|sources| sources.get(&field));
        let Some(option) = option.and_then(Value::as_object) else {
            return fail(format!(
                "Action type `{action_type}` declares no options for `{field}`."
            ));
        };
        let path = option
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if path.trim().is_empty() {
            return fail(format!(
                "Action type `{action_type}` declares no options endpoint for `{field}`."
            ));
        }
        let settings = match self.capabilities.load_plugin_settings_raw(&plugin.manifest) {
            Ok(settings) => settings,
            Err(error) => return fail(error.to_string()),
        };
        let scope = json!({"settings": settings});
        let body = match self
            .fetch_declarative_json(
                &plugin.manifest,
                "GET",
                path,
                option.get("timeoutMs").and_then(Value::as_u64),
                &scope,
            )
            .await
        {
            Ok(body) => body,
            Err(error) => return fail(error),
        };
        let options = match map_option_items(
            &body,
            option.get("itemsPath").and_then(Value::as_str),
            option.get("valuePath").and_then(Value::as_str),
            option.get("labelPath").and_then(Value::as_str),
        ) {
            Ok(options) => options,
            Err(error) => return fail(error),
        };
        self.option_sources.store(source, options.clone());
        (options, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plugin_option_sources() {
        assert_eq!(
            parse_option_source("plugin-action-options:sonicboom.server.speak:voice"),
            Some(("sonicboom.server.speak".to_owned(), "voice".to_owned()))
        );
        assert_eq!(parse_option_source("voices"), None);
        assert_eq!(parse_option_source("plugin-action-options:no-field"), None);
        assert_eq!(parse_option_source("plugin-action-options:a:b:c"), None);
    }

    #[test]
    fn maps_voice_list_shapes() {
        // SonicBoom-style `{voices: [{id, name}]}` with no declared paths.
        let options = map_option_items(
            &json!({"voices": [{"id": "M1", "name": "Marcus"}, {"id": "F2", "name": "Freya"}]}),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            options,
            vec![
                json!({"value": "M1", "label": "Marcus"}),
                json!({"value": "F2", "label": "Freya"}),
            ]
        );
        // Root arrays and plain strings work too.
        let options = map_option_items(&json!(["a", "b"]), None, None, None).unwrap();
        assert_eq!(
            options,
            vec![
                json!({"value": "a", "label": "a"}),
                json!({"value": "b", "label": "b"}),
            ]
        );
        // Explicit dotted paths win over the defaults.
        let options = map_option_items(
            &json!({"data": {"list": [{"code": "x1", "title": "X One"}]}}),
            Some("data.list"),
            Some("code"),
            Some("title"),
        )
        .unwrap();
        assert_eq!(options, vec![json!({"value": "x1", "label": "X One"})]);
        // Entries without a value are skipped, labels fall back to values.
        let options = map_option_items(
            &json!([{"id": "k1"}, {"name": "nameless"}, "solo"]),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            options,
            vec![
                json!({"value": "k1", "label": "k1"}),
                json!({"value": "solo", "label": "solo"}),
            ]
        );
    }

    #[test]
    fn rejects_responses_without_a_list() {
        assert!(map_option_items(&json!({"status": "ok"}), None, None, None).is_err());
        assert!(map_option_items(&json!({"items": {}}), None, None, None).is_err());
        assert!(map_option_items(&json!({}), Some("missing"), None, None).is_err());
    }

    #[test]
    fn cache_serves_until_cleared() {
        let service = OptionSourceService::new();
        assert!(service.cached("s").is_none());
        service.store("s", vec![json!({"value": "v", "label": "V"})]);
        assert_eq!(service.cached("s").unwrap().len(), 1);
        service.clear();
        assert!(service.cached("s").is_none());
    }

    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
    }

    #[tokio::test]
    async fn resolution_errors_stay_display_safe() {
        let core = std::sync::Arc::new(crate::AppCore::new(std::sync::Arc::new(Emitter)));
        let (options, error) = core.resolve_action_options("not-a-source").await;
        assert!(options.is_empty());
        assert!(error.unwrap().contains("Unknown option source"));
        let (options, error) = core
            .resolve_action_options("plugin-action-options:missing.action:field")
            .await;
        assert!(options.is_empty());
        assert!(error.unwrap().contains("not available"));
    }
}
