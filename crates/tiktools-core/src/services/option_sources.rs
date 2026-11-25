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
use tiktools_plugin_api::sync::recover_mutex;

/// Source-id prefix for plugin dynamic options. The remainder is
/// `<actionType>:<field>`; action type ids never contain a colon, so the
/// split is unambiguous.
pub const OPTION_SOURCE_PREFIX: &str = "plugin-action-options:";
/// Host-owned, read-only gift catalog source. Resolved internally from the
/// persisted `gifts.list` catalog — never through a plugin-controlled fetch.
/// This is the only `host.*` source; anything else under the prefix is
/// rejected by validation so plugins cannot probe for host internals.
pub const HOST_GIFTS_SOURCE: &str = "host.gifts";
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
    selected: Option<String>,
}

/// TTL cache for fetched option lists. Entries are keyed by full source id;
/// any settings save clears the cache because a server URL or token may
/// have changed what the endpoints return.
pub struct OptionSourceService {
    cache: Mutex<HashMap<String, CachedOptions>>,
    generation: std::sync::atomic::AtomicU64,
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
            generation: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn cached(&self, source: &str) -> Option<(Vec<Value>, Option<String>)> {
        let mut cache = recover_mutex(&self.cache, "option cache");
        let entry = cache.get(source)?;
        if entry.fetched_at.elapsed() > OPTION_SOURCE_TTL {
            cache.remove(source);
            return None;
        }
        Some((entry.options.clone(), entry.selected.clone()))
    }

    pub fn store(&self, source: &str, options: Vec<Value>, selected: Option<String>) {
        recover_mutex(&self.cache, "option cache").insert(
            source.to_owned(),
            CachedOptions {
                fetched_at: Instant::now(),
                options,
                selected,
            },
        );
    }

    pub fn clear(&self) {
        let mut cache = recover_mutex(&self.cache, "option cache");
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        cache.clear();
    }

    fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Acquire)
    }

    /// An in-flight request from old settings must never repopulate the cache.
    fn store_if_current(
        &self,
        source: &str,
        options: Vec<Value>,
        selected: Option<String>,
        generation: u64,
    ) -> bool {
        let mut cache = recover_mutex(&self.cache, "option cache");
        if self.generation() != generation {
            return false;
        }
        cache.insert(
            source.to_owned(),
            CachedOptions {
                fetched_at: Instant::now(),
                options,
                selected,
            },
        );
        true
    }

    /// Drops one cached option list. Explicit refresh after a
    /// state-changing action calls this so the next read re-fetches instead
    /// of serving the pre-mutation selection until the TTL expires.
    /// Contextual entries (`source\u{1f}…`) are dropped with their source.
    pub fn invalidate(&self, source: &str) {
        let mut cache = recover_mutex(&self.cache, "option cache");
        cache.remove(source);
        let prefix = format!("{source}\u{1f}");
        cache.retain(|key, _| !key.starts_with(&prefix));
    }

    /// Drops every cached option list owned by one action type (all fields).
    /// A successful live execution may have changed what the server reports,
    /// so its own option sources are never served stale afterwards. Other
    /// actions keep their cached entries.
    pub fn invalidate_action(&self, action_type: &str) {
        let owned: Vec<String> = recover_mutex(&self.cache, "option cache")
            .keys()
            .filter_map(|source| {
                // Contextual cache keys carry a `\u{1f}`-suffixed query; the
                // source id itself never contains that separator.
                let base = source.split('\u{1f}').next().unwrap_or(source);
                parse_option_source(base)
                    .filter(|(owner, _)| owner == action_type)
                    .map(|_| base.to_owned())
            })
            .collect();
        for source in owned {
            self.invalidate(&source);
        }
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

/// True for allowlisted host-owned sources. Only [`HOST_GIFTS_SOURCE`]
/// exists today; the allowlist (not a prefix rule) keeps future `host.*`
/// names from resolving until they are deliberately added here.
pub fn is_host_option_source(source: &str) -> bool {
    source == HOST_GIFTS_SOURCE
}

/// Maps the persisted gift catalog to `{value, label, meta}` option items.
///
/// Values are gift names, matching what event filters store. Duplicate
/// names collapse first-wins in catalog order, exactly like the event
/// `GiftPicker`, so both surfaces agree on which entry a name means.
/// Entries without a usable name are skipped; the list is capped at
/// [`MAX_OPTION_ITEMS`]. An empty catalog yields empty options, never an
/// error — callers render their empty state plus manual entry.
pub fn map_host_gift_options(catalog: &[Value]) -> Vec<Value> {
    let mut seen = std::collections::HashSet::new();
    let mut options = Vec::new();
    for gift in catalog {
        let Some(entry) = gift.as_object() else {
            continue;
        };
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if name.is_empty() || name.len() > MAX_OPTION_VALUE_LEN || !seen.insert(name.to_owned()) {
            continue;
        }
        let mut meta = serde_json::Map::new();
        if let Some(id) = entry
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        {
            meta.insert("giftId".to_owned(), Value::String(id.to_owned()));
        }
        if let Some(diamonds) = entry.get("diamondCount").and_then(Value::as_u64) {
            meta.insert("diamondCount".to_owned(), Value::Number(diamonds.into()));
        }
        if let Some(icon) = entry
            .get("iconUrl")
            .and_then(Value::as_str)
            .filter(|icon| !icon.is_empty())
        {
            meta.insert("iconUrl".to_owned(), Value::String(icon.to_owned()));
        }
        let mut option = serde_json::Map::new();
        option.insert("value".to_owned(), Value::String(name.to_owned()));
        option.insert("label".to_owned(), Value::String(name.to_owned()));
        option.insert("meta".to_owned(), Value::Object(meta));
        options.push(Value::Object(option));
        if options.len() >= MAX_OPTION_ITEMS {
            break;
        }
    }
    options
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
        let mut option = json!({"value": value, "label": label});
        if let Some(disabled) = item.get("disabled").and_then(Value::as_bool) {
            option["disabled"] = Value::Bool(disabled);
        }
        options.push(option);
        if options.len() >= MAX_OPTION_ITEMS {
            break;
        }
    }
    Ok(options)
}

/// Bounds caller-supplied option context to flat scalar entries. Nested
/// objects, arrays and oversized strings are dropped — option templates only
/// ever read top-level `{{ config.<key> }}` scalars.
pub(crate) fn sanitize_option_context(context: Option<&Value>) -> Value {
    let Some(object) = context.and_then(Value::as_object) else {
        return json!({});
    };
    let mut clean = serde_json::Map::new();
    for (key, value) in object.iter().take(32) {
        if key.is_empty() || key.len() > 128 {
            continue;
        }
        let scalar = match value {
            Value::String(text) if text.len() <= 512 => Value::String(text.clone()),
            Value::Number(_) | Value::Bool(_) => value.clone(),
            _ => continue,
        };
        clean.insert(key.clone(), scalar);
    }
    Value::Object(clean)
}

fn url_encode(text: &str) -> String {
    url::form_urlencoded::byte_serialize(text.as_bytes()).collect()
}

/// Renders a source's declared `params` map into one sorted, URL-encoded
/// query string. Every value is a `{{ config.* }}`/`{{ settings.* }}`
/// template resolved against the fetch scope; empty renders are dropped.
/// Manifest validation bounds the map (≤8 entries, ≤1024 chars each), so
/// the result stays small enough to double as the cache-key suffix.
fn render_option_params(params: Option<&Value>, scope: &Value) -> String {
    let Some(object) = params.and_then(Value::as_object) else {
        return String::new();
    };
    let mut pairs: Vec<(String, String)> = object
        .iter()
        .filter_map(|(key, raw)| {
            let rendered =
                crate::services::declarative_http::render_scoped_template(raw.as_str()?, scope);
            if rendered.is_empty() {
                return None;
            }
            Some((key.clone(), rendered))
        })
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", url_encode(key), url_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
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
    let value = option_item_value(item, value_path)?;
    let label_paths: Vec<&str> = label_path.into_iter().chain(["name", "label"]).collect();
    let label = label_paths
        .iter()
        .filter_map(|path| read_dotted_path(item, path))
        .find_map(scalar_text)
        .unwrap_or_else(|| value.clone());
    Some((value, label))
}

/// Value half of [`map_option_item`], shared with [`selected_option_value`]
/// so the reported selection always matches a mapped option value.
fn option_item_value(item: &Value, value_path: Option<&str>) -> Option<String> {
    let value_paths: Vec<&str> = value_path.into_iter().chain(["id", "value"]).collect();
    value_paths
        .iter()
        .filter_map(|path| read_dotted_path(item, path))
        .find_map(scalar_text)
}

/// Reports the server-selected option value from one fetched document, if
/// the server advertises one. A top-level scalar `selected` wins; otherwise
/// the first item carrying a literal `is_selected: true` contributes its
/// mapped value. Documents without either (voice lists, plain arrays) yield
/// `None`, and the caller falls back to its own state. Values obey the same
/// bounds as mapped options so a hostile document cannot smuggle an
/// oversized selection past the UI.
pub fn selected_option_value(
    body: &Value,
    items_path: Option<&str>,
    value_path: Option<&str>,
) -> Option<String> {
    if let Some(selected) = body.get("selected").and_then(scalar_text) {
        return bounded_selected(selected);
    }
    let items = find_option_items(body, items_path).ok()?;
    for item in items {
        if item.get("is_selected").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let value = match item.as_str() {
            Some(text) => text.to_owned(),
            None if item.is_object() => {
                let Some(value) = option_item_value(item, value_path) else {
                    continue;
                };
                value
            }
            None => continue,
        };
        // A flagged item without a usable value must not shadow a later
        // flagged item that has one.
        if let Some(selected) = bounded_selected(value) {
            return Some(selected);
        }
    }
    None
}

fn bounded_selected(value: String) -> Option<String> {
    if value.is_empty() || value.len() > MAX_OPTION_VALUE_LEN {
        return None;
    }
    Some(value)
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
    /// Resolves one `get-action-options` source to its items, the
    /// server-reported selection (if the document advertises one), and an
    /// optional display-safe error. Cache hits skip the fetch unless
    /// `force_refresh` bypasses the cache (manual Refresh, post-mutation
    /// re-read); failures yield empty options with an explanatory error
    /// instead of failing the form.
    pub(crate) async fn resolve_action_options(
        self: &std::sync::Arc<Self>,
        source: &str,
        force_refresh: bool,
        context: Option<&Value>,
    ) -> (Vec<Value>, Option<String>, Option<String>) {
        let fail = |message: String| (Vec::new(), None, Some(message));
        let generation = self.option_sources.generation();
        if !force_refresh {
            if let Some((options, selected)) = self.option_sources.cached(source) {
                return (options, selected, None);
            }
        }
        if is_host_option_source(source) {
            // Host-owned data: the same persisted catalog `gifts.list`
            // serves, mapped to the standard option envelope. No plugin
            // lookup, no fetch, no selection to report.
            let options = map_host_gift_options(&self.gift_catalog());
            self.option_sources.store(source, options.clone(), None);
            return (options, None, None);
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
        let config = sanitize_option_context(context);
        let scope = json!({"settings": settings, "config": config});
        let has_params = option
            .get("params")
            .and_then(Value::as_object)
            .is_some_and(|params| !params.is_empty());
        let query = render_option_params(option.get("params"), &scope);
        // Contextual sources cache per rendered query: provider=A and
        // provider=B must never share a list. The `\u{1f}` separator keeps
        // contextual keys distinct from the bare source id, so the early
        // `cached(source)` probe above can only ever hit non-contextual
        // entries.
        let cache_key = if has_params {
            format!("{source}\u{1f}{query}")
        } else {
            source.to_owned()
        };
        if has_params && !force_refresh {
            if let Some((options, selected)) = self.option_sources.cached(&cache_key) {
                return (options, selected, None);
            }
        }
        let path = if query.is_empty() {
            path.to_owned()
        } else {
            format!(
                "{}{}{}",
                path,
                if path.contains('?') { "&" } else { "?" },
                query
            )
        };
        let body = match self
            .fetch_declarative_json(
                &plugin.manifest,
                "GET",
                &path,
                option.get("timeoutMs").and_then(Value::as_u64),
                &scope,
            )
            .await
        {
            Ok(body) => body,
            Err(error) => return fail(error),
        };
        let items_path = option.get("itemsPath").and_then(Value::as_str);
        let value_path = option.get("valuePath").and_then(Value::as_str);
        let options = match map_option_items(
            &body,
            items_path,
            value_path,
            option.get("labelPath").and_then(Value::as_str),
        ) {
            Ok(options) => options,
            Err(error) => return fail(error),
        };
        let selected = selected_option_value(&body, items_path, value_path);
        if !self.option_sources.store_if_current(
            &cache_key,
            options.clone(),
            selected.clone(),
            generation,
        ) {
            return fail("Connection settings changed; reload options.".to_owned());
        }
        (options, selected, None)
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
    fn host_source_allowlist_holds_one_entry() {
        assert!(is_host_option_source(HOST_GIFTS_SOURCE));
        assert!(!is_host_option_source("host.gifts2"));
        assert!(!is_host_option_source("host.state"));
        assert!(!is_host_option_source("plugin-action-options:a:b"));
        assert!(!is_host_option_source("voices"));
    }

    #[test]
    fn maps_gift_catalog_to_host_options() {
        let catalog = vec![
            json!({"id": "1", "name": "Rose", "diamondCount": 1, "iconUrl": "https://example.com/rose.png"}),
            json!({"id": "2", "name": "Galaxy", "diamondCount": 1000}),
        ];
        let options = map_host_gift_options(&catalog);
        assert_eq!(
            options,
            vec![
                json!({
                    "value": "Rose",
                    "label": "Rose",
                    "meta": {"giftId": "1", "diamondCount": 1, "iconUrl": "https://example.com/rose.png"},
                }),
                json!({
                    "value": "Galaxy",
                    "label": "Galaxy",
                    "meta": {"giftId": "2", "diamondCount": 1000},
                }),
            ]
        );
        // Empty catalogs yield empty options, never an error.
        assert!(map_host_gift_options(&[]).is_empty());
    }

    #[test]
    fn host_gift_options_dedupe_first_wins_and_skip_bad_entries() {
        // Duplicate names collapse first-wins in catalog order, exactly
        // like the event GiftPicker, so both surfaces agree on one entry.
        let catalog = vec![
            json!({"id": "1", "name": "Rose", "diamondCount": 1}),
            json!({"id": "9", "name": "Rose", "diamondCount": 50}),
            json!({"id": "2", "name": "  ", "diamondCount": 5}),
            json!({"id": "3", "diamondCount": 5}),
            json!("not-an-object"),
            json!({"id": "4", "name": "x".repeat(300), "diamondCount": 5}),
            json!({"id": "5", "name": " GG ", "diamondCount": 2}),
        ];
        let options = map_host_gift_options(&catalog);
        assert_eq!(
            options,
            vec![
                json!({
                    "value": "Rose",
                    "label": "Rose",
                    "meta": {"giftId": "1", "diamondCount": 1},
                }),
                json!({
                    "value": "GG",
                    "label": "GG",
                    "meta": {"giftId": "5", "diamondCount": 2},
                }),
            ]
        );
    }

    #[test]
    fn host_gift_options_cap_at_max_items() {
        let catalog: Vec<Value> = (0..MAX_OPTION_ITEMS + 50)
            .map(|index| json!({"id": index.to_string(), "name": format!("Gift {index}"), "diamondCount": 1}))
            .collect();
        assert_eq!(map_host_gift_options(&catalog).len(), MAX_OPTION_ITEMS);
    }

    #[test]
    fn cache_serves_until_cleared() {
        let service = OptionSourceService::new();
        assert!(service.cached("s").is_none());
        service.store(
            "s",
            vec![json!({"value": "v", "label": "V"})],
            Some("v".to_owned()),
        );
        let (options, selected) = service.cached("s").unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(selected.as_deref(), Some("v"));
        service.clear();
        assert!(service.cached("s").is_none());
    }

    #[test]
    fn invalidate_drops_one_source_and_keeps_the_rest() {
        let service = OptionSourceService::new();
        let outputs = "plugin-action-options:sonicboom.server.set-output-device:device";
        let voices = "plugin-action-options:sonicboom.server.speak:voice";
        service.store(
            outputs,
            vec![json!({"value": "default", "label": "System Default"})],
            Some("default".to_owned()),
        );
        service.store(
            voices,
            vec![json!({"value": "M1", "label": "Marcus"})],
            None,
        );
        // Regression: after a device switch the outputs re-read must not
        // serve the pre-mutation `selected=default`.
        service.invalidate(outputs);
        assert!(service.cached(outputs).is_none());
        assert!(service.cached(voices).is_some());
        // Unknown sources are a no-op, never a panic.
        service.invalidate("plugin-action-options:missing.action:field");
        assert!(service.cached(voices).is_some());
    }

    #[test]
    fn invalidate_action_drops_only_that_actions_sources() {
        let service = OptionSourceService::new();
        let device = "plugin-action-options:sonicboom.server.set-output-device:device";
        let voices = "plugin-action-options:sonicboom.server.speak:voice";
        service.store(device, vec![], Some("default".to_owned()));
        service.store(voices, vec![], None);
        service.invalidate_action("sonicboom.server.set-output-device");
        assert!(service.cached(device).is_none());
        assert!(service.cached(voices).is_some());
        // A sibling prefix must not match: `speak` shares no owner here,
        // and unknown actions change nothing.
        service.invalidate_action("sonicboom.server.unknown");
        assert!(service.cached(voices).is_some());
    }

    #[test]
    fn reports_server_selected_option() {
        // SonicBoom-style devices document: top-level `selected` wins over
        // per-item flags, and values follow the declared value path.
        let body = json!({
            "devices": [
                {"id": "default", "name": "System Default", "is_default": true, "is_selected": false},
                {"id": "CABLE Input", "name": "CABLE Input", "is_default": false, "is_selected": true}
            ],
            "selected": "CABLE Input"
        });
        assert_eq!(
            selected_option_value(&body, Some("devices"), Some("id")).as_deref(),
            Some("CABLE Input")
        );
        // Without the top-level key the flagged item contributes its value.
        let body = json!({
            "devices": [
                {"id": "default", "name": "System Default"},
                {"id": "CABLE Input", "name": "CABLE Input", "is_selected": true}
            ]
        });
        assert_eq!(
            selected_option_value(&body, Some("devices"), Some("id")).as_deref(),
            Some("CABLE Input")
        );
        // A flagged item without a usable value does not shadow a later one.
        let body = json!({
            "devices": [
                {"name": "ghost", "is_selected": true},
                {"id": "real", "name": "Real", "is_selected": true}
            ]
        });
        assert_eq!(
            selected_option_value(&body, Some("devices"), Some("id")).as_deref(),
            Some("real")
        );
        // Voice lists and plain shapes advertise no selection.
        assert_eq!(
            selected_option_value(
                &json!({"voices": [{"id": "M1", "name": "Marcus"}]}),
                None,
                None
            ),
            None
        );
        assert_eq!(selected_option_value(&json!(["a", "b"]), None, None), None);
        // Only a literal `true` flag counts; oversized values are dropped.
        assert_eq!(
            selected_option_value(
                &json!({"items": [{"id": "x", "is_selected": "yes"}]}),
                None,
                None
            ),
            None
        );
        assert_eq!(
            selected_option_value(&json!({"selected": "x".repeat(300)}), None, None),
            None
        );
    }

    /// A core with no discovered plugins, independent of the user's local
    /// plugin directory. Without this, the forced lookup can reach a real
    /// installed plugin instead of failing at the expected "not available".
    fn empty_plugin_core() -> std::sync::Arc<crate::AppCore> {
        let root = std::env::temp_dir().join(format!(
            "tiktools-option-sources-test-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = crate::db::test_templates::materialize(&root);
        std::sync::Arc::new(crate::AppCore::with_paths(
            std::sync::Arc::new(Emitter),
            std::sync::Arc::new(crate::NoopMediaHost),
            paths,
        ))
    }

    #[tokio::test]
    async fn force_refresh_bypasses_the_cache_for_one_read() {
        let core = empty_plugin_core();
        let source = "plugin-action-options:sonicboom.server.set-output-device:device";
        core.option_sources.store(
            source,
            vec![json!({"value": "default", "label": "System Default"})],
            Some("default".to_owned()),
        );
        // Normal read: cache hit, no plugin lookup, no fetch.
        let (options, selected, error) = core.resolve_action_options(source, false, None).await;
        assert!(error.is_none());
        assert_eq!(options.len(), 1);
        assert_eq!(selected.as_deref(), Some("default"));
        // Forced read: the cache is skipped, so resolution proceeds to the
        // plugin lookup and fails there (no plugins registered in this
        // core) instead of returning the stale cached selection.
        let (options, selected, error) = core.resolve_action_options(source, true, None).await;
        assert!(options.is_empty());
        assert!(selected.is_none());
        assert!(error.unwrap().contains("not available"));
        // The bypass reads through without poisoning the stored entry.
        assert!(core.option_sources.cached(source).is_some());
    }

    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
    }

    #[tokio::test]
    async fn resolution_errors_stay_display_safe() {
        let core = std::sync::Arc::new(crate::AppCore::new(std::sync::Arc::new(Emitter)));
        let (options, selected, error) = core
            .resolve_action_options("not-a-source", false, None)
            .await;
        assert!(options.is_empty());
        assert!(selected.is_none());
        assert!(error.unwrap().contains("Unknown option source"));
        let (options, selected, error) = core
            .resolve_action_options("plugin-action-options:missing.action:field", false, None)
            .await;
        assert!(options.is_empty());
        assert!(selected.is_none());
        assert!(error.unwrap().contains("not available"));
    }

    #[tokio::test]
    async fn resolves_host_gifts_without_a_plugin_lookup() {
        // The source resolves from the persisted catalog without an error
        // and without requiring any plugin to be installed. The catalog
        // contents are machine state, so only the envelope is asserted
        // here; the mapping itself is covered by the pure tests above.
        let core = std::sync::Arc::new(crate::AppCore::new(std::sync::Arc::new(Emitter)));
        let (options, selected, error) = core
            .resolve_action_options(HOST_GIFTS_SOURCE, false, None)
            .await;
        assert!(error.is_none());
        assert!(selected.is_none());
        assert_eq!(options, map_host_gift_options(&core.gift_catalog()));
        // Anything else under the host namespace still fails closed.
        let (options, _, error) = core.resolve_action_options("host.state", false, None).await;
        assert!(options.is_empty());
        assert!(error.unwrap().contains("Unknown option source"));
    }

    #[tokio::test]
    async fn host_gifts_share_the_option_cache_and_refresh() {
        let core = std::sync::Arc::new(crate::AppCore::new(std::sync::Arc::new(Emitter)));
        core.option_sources.store(
            HOST_GIFTS_SOURCE,
            vec![json!({"value": "Rose", "label": "Rose"})],
            None,
        );
        // Normal read serves the cached catalog snapshot.
        let (options, _, error) = core
            .resolve_action_options(HOST_GIFTS_SOURCE, false, None)
            .await;
        assert!(error.is_none());
        assert_eq!(options.len(), 1);
        // Forced read re-reads the live catalog instead of serving the
        // snapshot, then caches the fresh result.
        let (options, _, error) = core
            .resolve_action_options(HOST_GIFTS_SOURCE, true, None)
            .await;
        assert!(error.is_none());
        assert_eq!(options, map_host_gift_options(&core.gift_catalog()));
        let (cached, _) = core.option_sources.cached(HOST_GIFTS_SOURCE).unwrap();
        assert_eq!(cached, options);
    }
    #[test]
    fn literal_disabled_metadata_survives_mapping_without_other_server_fields() {
        let options = map_option_items(&json!([
            {"id":"pocket-tts", "name":"Pocket TTS (loading)", "disabled":true, "token":"never map"},
            {"id":"supertonic", "name":"Supertonic", "disabled":false},
            {"id":"legacy", "disabled":"true"}
        ]), None, None, None).unwrap();
        assert_eq!(
            options[0],
            json!({"value":"pocket-tts", "label":"Pocket TTS (loading)", "disabled":true})
        );
        assert_eq!(options[1]["disabled"], false);
        assert!(options[2].get("disabled").is_none());
    }
    #[test]
    fn option_context_keeps_only_flat_bounded_scalars() {
        let context = json!({
            "provider": "supertonic",
            "count": 3,
            "flag": true,
            "nested": {"a": 1},
            "list": [1, 2],
            "long": "x".repeat(600),
            "": "empty-key",
        });
        let clean = sanitize_option_context(Some(&context));
        let object = clean.as_object().expect("object context");
        assert_eq!(object.len(), 3);
        assert_eq!(object["provider"], json!("supertonic"));
        assert_eq!(object["count"], json!(3));
        assert_eq!(object["flag"], json!(true));
        assert!(sanitize_option_context(None)
            .as_object()
            .expect("object")
            .is_empty());
        assert!(sanitize_option_context(Some(&json!("nope")))
            .as_object()
            .expect("object")
            .is_empty());
    }

    #[test]
    fn option_params_render_templates_and_drop_empty_values() {
        let scope = json!({
            "settings": {"defaultProvider": "edge", "tts": {"providerId": "reverb-tts"}},
            "config": {"provider": "supertonic", "voice": "af heart"}
        });
        let params = json!({
            "provider": "{{ config.provider }}",
            "voice": "{{ config.voice }}",
            "fallback": "{{ settings.defaultProvider }}",
            "storedProvider": "{{ settings.tts.providerId }}",
            "missing": "{{ config.absent }}",
        });
        // Keys sort for a deterministic cache suffix; empty renders drop and
        // values are URL-encoded (`af heart` → `af+heart`).
        assert_eq!(
            render_option_params(Some(&params), &scope),
            "fallback=edge&provider=supertonic&storedProvider=reverb-tts&voice=af+heart"
        );
        assert_eq!(render_option_params(None, &scope), "");
        assert_eq!(render_option_params(Some(&json!({})), &scope), "");
    }

    #[test]
    fn settings_change_rejects_inflight_old_tenant_catalog() {
        let service = OptionSourceService::new();
        let generation = service.generation();
        service.clear();
        assert!(!service.store_if_current(
            "source",
            vec![json!({"value":"old-profile"})],
            None,
            generation
        ));
        assert!(service.cached("source").is_none());
        assert!(service.store_if_current("source", vec![], None, service.generation()));
    }
}
