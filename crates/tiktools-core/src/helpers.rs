use super::*;

pub(crate) fn empty_behavior_snapshot() -> serde_json::Value {
    json!({
        "actions": [],
        "events": [],
        "plugins": [],
        "actionTypes": builtin_action_types(),
        "eventTypes": [],
        "translations": builtin_translations()
    })
}

pub(crate) fn localized(default: &str, key: &str) -> Value {
    json!({"default": default, "i18key": key})
}

pub(crate) fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some('a'..='z' | 'A'..='Z' | '_'))
        && value.len() <= 128
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

pub(crate) fn normalize_emit_type(value: &str) -> Result<String, String> {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
        .take(64)
        .collect::<String>();
    if normalized.is_empty() {
        Err("Internal event needs a name.".to_owned())
    } else {
        Ok(normalized)
    }
}

pub(crate) fn number_value(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(value)) => value.as_f64(),
        Some(Value::String(value)) => value.trim().parse().ok(),
        _ => None,
    }
}

pub(crate) fn hosts_in_source(source: &str) -> Vec<String> {
    let mut hosts = Vec::new();
    let mut rest = source;
    while let Some(offset) = rest.find("http://").or_else(|| rest.find("https://")) {
        let candidate = &rest[offset..];
        let end = candidate
            .find(|character: char| {
                character.is_whitespace()
                    || matches!(character, '"' | '\'' | '`' | ')' | '}' | ']' | ',')
            })
            .unwrap_or(candidate.len());
        let url = &candidate[..end];
        let host_start = url.find("://").map(|index| index + 3).unwrap_or(0);
        let host = url[host_start..]
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .trim_matches(['[', ']'])
            .split('@')
            .next_back()
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !host.is_empty() && !host.contains('{') && !hosts.contains(&host) {
            hosts.push(host);
        }
        rest = &candidate[end..];
        if rest == candidate {
            break;
        }
    }
    hosts
}

#[cfg(feature = "http")]
pub(crate) async fn validate_http_url(
    url: &reqwest::Url,
    configured_host: &str,
    allowed_hosts: Option<&[String]>,
    allow_private_network: bool,
) -> Result<(), String> {
    validate_http_url_shape(url, configured_host, allowed_hosts, allow_private_network)?;
    let host = url
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "HTTP URL has no host.".to_owned())?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "HTTP URL has no valid port.".to_owned())?;
    let addresses = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| format!("HTTP host could not be resolved: {host}"))?;
    if !allow_private_network && addresses.map(|address| address.ip()).any(is_private_ip) {
        return Err(format!("HTTP host resolves to a private address: {host}"));
    }
    Ok(())
}

#[cfg(feature = "http")]
pub(crate) fn validate_http_url_shape(
    url: &reqwest::Url,
    configured_host: &str,
    allowed_hosts: Option<&[String]>,
    allow_private_network: bool,
) -> Result<(), String> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Only http:// and https:// URLs are allowed.".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("HTTP URLs cannot contain embedded credentials.".to_owned());
    }
    let host = url
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "HTTP URL has no host.".to_owned())?;
    if host != configured_host {
        return Err("The rendered HTTP URL changed its configured host.".to_owned());
    }
    if let Some(allowed_hosts) = allowed_hosts {
        if !allowed_hosts.iter().any(|allowed| allowed == &host) {
            return Err(format!("HTTP host is not allowed by the script: {host}"));
        }
    }
    if is_private_host(&host) && !allow_private_network {
        return Err(format!("HTTP request to a private host is blocked: {host}"));
    }
    Ok(())
}

/// Loopback-only trust boundary for declarative integrations: local servers
/// (a SonicBoom instance on this machine) work without a token or a network
/// permission, while anything beyond loopback requires both. LAN addresses
/// are private but not loopback.
pub(crate) fn is_loopback_host(host: &str) -> bool {
    let normalized = host.trim_matches(['[', ']']).to_ascii_lowercase();
    if normalized == "localhost" || normalized.ends_with(".localhost") {
        return true;
    }
    normalized
        .parse::<std::net::IpAddr>()
        .is_ok_and(|address| match address {
            std::net::IpAddr::V4(address) => address.octets()[0] == 127,
            std::net::IpAddr::V6(address) => address.is_loopback(),
        })
}

pub(crate) fn is_private_host(host: &str) -> bool {
    let normalized = host.trim_matches(['[', ']']).to_ascii_lowercase();
    normalized == "localhost"
        || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
        || normalized == "::"
        || normalized == "::1"
        || normalized
            .parse::<std::net::IpAddr>()
            .is_ok_and(is_private_ip)
}

pub(crate) fn is_private_ip(address: std::net::IpAddr) -> bool {
    match address {
        std::net::IpAddr::V4(address) => {
            let octets = address.octets();
            octets[0] == 0
                || octets[0] == 10
                || octets[0] == 127
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                || (octets[0] == 169 && octets[1] == 254)
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
        }
        std::net::IpAddr::V6(address) => {
            address.is_loopback()
                || address.is_unspecified()
                || (address.segments()[0] & 0xfe00) == 0xfc00
                || (address.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

pub(crate) fn render_json_map(
    value: &Value,
    event: &Value,
    globals: &std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, Value> {
    value
        .as_object()
        .map(|object| {
            object
                .iter()
                .map(|(key, value)| {
                    let value = value
                        .as_str()
                        .map(|value| Value::String(render_template(value, event, globals)))
                        .unwrap_or_else(|| value.clone());
                    (key.clone(), value)
                })
                .collect()
        })
        .unwrap_or_default()
}

enum SpanAction {
    Render(String),
    Drop,
    Keep,
}

fn substitute_spans(source: &str, resolve: impl Fn(&str) -> SpanAction) -> String {
    let mut rendered = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        rendered.push_str(&rest[..start]);
        let expression = &rest[start + 2..];
        let Some(end) = expression.find("}}") else {
            rendered.push_str(&rest[start..]);
            break;
        };
        match resolve(expression[..end].trim()) {
            SpanAction::Render(text) => rendered.push_str(&text),
            SpanAction::Drop => {}
            SpanAction::Keep => rendered.push_str(&rest[start..start + 2 + end + 2]),
        }
        rest = &expression[end + 2..];
    }
    rendered.push_str(rest);
    rendered
}

/// `scheme:///path` parses with the first segment as host; reject it so
/// a missing global in host position fails closed instead of redirecting.
pub(crate) fn has_empty_authority(url: &str) -> bool {
    match url.split_once("://") {
        Some((_, rest)) => rest.starts_with('/'),
        None => false,
    }
}

/// Full runtime render: `{{ globals.* }}` resolves from the globals
/// snapshot, everything else from the event (unchanged legacy behavior).
/// Unknown spans of either kind render as empty.
pub(crate) fn render_template(
    source: &str,
    event: &Value,
    globals: &std::collections::BTreeMap<String, String>,
) -> String {
    substitute_spans(source, |path| match read_global_path(globals, path) {
        Some(text) => SpanAction::Render(text),
        None if is_globals_path(path) => SpanAction::Drop,
        None => match read_event_path(event, path) {
            Some(value) => SpanAction::Render(value_to_string(value)),
            None => SpanAction::Drop,
        },
    })
}

/// Phase-one render for URLs: only `{{ globals.* }}` resolves
/// (operator-trusted); every other span stays literal for phase two.
pub(crate) fn render_globals_only(
    source: &str,
    globals: &std::collections::BTreeMap<String, String>,
) -> String {
    substitute_spans(source, |path| match read_global_path(globals, path) {
        Some(text) => SpanAction::Render(text),
        None if is_globals_path(path) => SpanAction::Drop,
        None => SpanAction::Keep,
    })
}

fn is_globals_path(path: &str) -> bool {
    path == "globals" || path.starts_with("globals.")
}

/// Flat lookup: dots in keys are literal (`{{ globals.a.b }}` reads key
/// `a.b`, never a nested walk). Bare `{{ globals }}` renders the snapshot.
fn read_global_path(
    globals: &std::collections::BTreeMap<String, String>,
    path: &str,
) -> Option<String> {
    if path == "globals" {
        let snapshot = Value::Object(
            globals
                .iter()
                .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                .collect(),
        );
        return Some(value_to_string(&snapshot));
    }
    let key = path.strip_prefix("globals.")?;
    globals.get(key).cloned()
}

pub(crate) fn read_event_path<'a>(event: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path.trim();
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

pub(crate) fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

pub(crate) fn result_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub(crate) fn as_values(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(values) => values.iter().collect(),
        value => vec![value],
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LiveContext {
    pub(crate) unique_id: String,
    pub(crate) room_id: String,
    pub(crate) connection_id: String,
}

#[cfg(feature = "native-tiktok")]
pub(crate) fn clean_unique_id(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('@');
    (!value.is_empty()).then_some(value.to_owned())
}

#[cfg(feature = "native-tiktok")]
pub(crate) fn native_user(event: &NativeLiveEvent) -> Option<&tiktools_tiktok::events::EventUser> {
    event.user()
}

#[cfg(feature = "native-tiktok")]
pub(crate) fn client_event_kind(event: &ClientEvent) -> &'static str {
    match event {
        ClientEvent::Connected(_) => "connected",
        ClientEvent::Event(_) => "live-event",
        ClientEvent::Reconnecting { .. } => "reconnecting",
        ClientEvent::Disconnected { .. } => "disconnected",
        ClientEvent::Error { .. } => "error",
    }
}

/// UI-ready `room.stats` contributor row (`TopViewerPayload`): rank, score,
/// identity, and the display-only avatar URL. Renderers fall back to
/// initials when the avatar is absent.
#[cfg(feature = "native-tiktok")]
pub(crate) fn top_viewer_value(viewer: &tiktools_tiktok::events::TopViewer) -> serde_json::Value {
    let mut value = json!({
        "rank": viewer.rank,
        "score": viewer.score,
        "delta": viewer.delta,
        "uniqueId": clean_unique_id(&viewer.user.unique_id).unwrap_or_else(|| "viewer".to_owned()),
        "nickname": viewer.user.nickname,
        "userId": viewer.user.id.to_string(),
    });
    if let Some(avatar_url) = viewer.user.avatar_url.as_ref() {
        value["avatarUrl"] = json!(avatar_url);
    }
    value
}

#[cfg(feature = "native-tiktok")]
pub(crate) fn user_value(user: &tiktools_tiktok::events::EventUser) -> serde_json::Value {
    serde_json::to_value(crate::contracts::AutomationUser {
        user_id: (user.id != 0).then(|| user.id.to_string()),
        unique_id: clean_unique_id(&user.unique_id).unwrap_or_else(|| "viewer".to_owned()),
        nickname: user.nickname.clone(),
        sec_uid: user.sec_uid.clone(),
        avatar_url: user.avatar_url.clone(),
    })
    .expect("automation user must serialize")
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

/// How often running plugins with declared event types are asked for fresh
/// spontaneous events (global hotkeys, timers, watchers).
pub(crate) const PLUGIN_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
pub(crate) const PLUGIN_POLL_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);
/// Safety cap for one plugin poll response. Well-behaved plugins send at
/// most [`tiktools_plugin_sdk::POLL_MAX_EVENTS_PER_RESPONSE`] events per
/// poll and retain the rest for the next tick; this host-side bound only
/// constrains misbehaving plugins, and every event dropped here is counted
/// and logged instead of silently lost.
pub(crate) const MAX_POLLED_EVENTS_PER_RESPONSE: usize = 64;
pub(crate) const MAX_PLUGIN_EVENT_BYTES: usize = 64 * 1024;
/// Host capability for plugins that need to report long-running preparation
/// work without publishing an automation event.
pub(crate) const PLUGIN_PROGRESS_CAPABILITY: &str = "ui.progress";
/// Reserved poll event consumed by the host UI instead of the automation bus.
pub(crate) const PLUGIN_PROGRESS_EVENT_TYPE: &str = "plugin.progress";

/// Shared retry backoff for plugin call health (polling and processors):
/// 1s, 2s, 5s, 10s, then 30s. Kept in one place so both call classes
/// recover identically.
pub(crate) fn plugin_backoff_seconds(consecutive_failures: u32) -> u64 {
    match consecutive_failures {
        1 => 1,
        2 => 2,
        3 => 5,
        4 => 10,
        _ => 30,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PluginProgressUpdate {
    pub(crate) state: crate::ipc::messages::PluginProgressState,
    pub(crate) progress: Option<f32>,
    pub(crate) message: String,
}

/// Parses the reserved progress event used by plugins that download or load
/// a model. Invalid or oversized updates are ignored at this boundary.
pub(crate) fn parse_plugin_progress(
    response: &tiktools_plugin_sdk::PluginCallResult,
) -> Option<PluginProgressUpdate> {
    response.events.iter().find_map(|event| {
        if event.event_type != PLUGIN_PROGRESS_EVENT_TYPE {
            return None;
        }
        let object = event.data.as_object()?;
        let state = match object.get("status").and_then(Value::as_str)? {
            "downloading" => crate::ipc::messages::PluginProgressState::Downloading,
            "loading" => crate::ipc::messages::PluginProgressState::Loading,
            "ready" => crate::ipc::messages::PluginProgressState::Ready,
            "failed" => crate::ipc::messages::PluginProgressState::Failed,
            _ => return None,
        };
        let progress = match object.get("progress") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let value = value.as_f64()?;
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return None;
                }
                Some(value as f32)
            }
        };
        let message = object
            .get("message")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|message| !message.is_empty())
            .map(|message| message.chars().take(240).collect::<String>())?;
        Some(PluginProgressUpdate {
            state,
            progress,
            message,
        })
    })
}

pub(crate) use tiktools_plugin_api::manifest::declared_event_types;

/// Host-stamped owner of a plugin event (`source.kind == "plugin"`).
/// Only the host writes this stamp (`make_plugin_event`); plugin payloads
/// live under `data` and can never forge it.
pub(crate) fn plugin_owner(event: &Value) -> Option<String> {
    let source = event.get("source")?.as_object()?;
    if source.get("kind").and_then(Value::as_str) != Some("plugin") {
        return None;
    }
    source
        .get("pluginId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
}

/// Context for spontaneous polled events. A polled keypress starts a new
/// chain, it never continues the previous one: connection/user identity is
/// borrowed from the last event, but any inherited emit depth is dropped.
/// Without this, sequential presses poison each other (`depth + 1` per
/// press) until every new event exceeds the depth limit and is dropped
/// forever, since even dropped events are remembered as the last event.
pub(crate) fn fresh_poll_context(source: &Value) -> Value {
    let mut context = source.clone();
    if let Some(data) = context.get_mut("data").and_then(Value::as_object_mut) {
        data.remove("depth");
    }
    context
}

/// Validated `poll` response: publishable `(type, data)` pairs plus the
/// per-reason drop counts the caller reports through diagnostics and logs.
/// Drops are observable (`plugin_events_dropped_total`-style counters and
/// structured warnings); they are never silent.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct ParsedPolledEvents {
    pub(crate) events: Vec<(String, Value)>,
    /// Response carried more than `MAX_POLLED_EVENTS_PER_RESPONSE` events.
    pub(crate) truncated: u64,
    /// Event type was not declared in the plugin manifest (or unparseable).
    pub(crate) undeclared: u64,
    /// Payload was not an object or exceeded `MAX_PLUGIN_EVENT_BYTES`.
    pub(crate) invalid: u64,
}

impl ParsedPolledEvents {
    pub(crate) fn dropped(&self) -> u64 {
        self.truncated + self.undeclared + self.invalid
    }
}

/// Parses a plugin `poll` response into publishable `(type, data)` pairs.
/// Unknown types, non-object payloads, and oversized payloads are dropped so
/// one misbehaving plugin cannot poison the automation pipeline. The
/// complete bounded response is accepted: conforming plugins already batch
/// to the protocol size, so the host cap is only a backstop.
pub(crate) fn parse_polled_events(
    declared: &[String],
    response: &tiktools_plugin_sdk::PluginCallResult,
) -> ParsedPolledEvents {
    let mut parsed = ParsedPolledEvents::default();
    for event in response.events.iter() {
        if parsed.events.len() + (parsed.undeclared as usize + parsed.invalid as usize)
            >= MAX_POLLED_EVENTS_PER_RESPONSE
        {
            parsed.truncated += 1;
            continue;
        }
        let event_type = event.event_type.as_str();
        let data = &event.data;
        if data.as_object().is_none() {
            parsed.invalid += 1;
            continue;
        }
        let Ok(event_type) = normalize_emit_type(event_type) else {
            parsed.undeclared += 1;
            continue;
        };
        if !declared.contains(&event_type) {
            parsed.undeclared += 1;
            continue;
        }
        if serde_json::to_vec(&data)
            .map(|bytes| bytes.len() > MAX_PLUGIN_EVENT_BYTES)
            .unwrap_or(true)
        {
            parsed.invalid += 1;
            continue;
        }
        parsed.events.push((event_type, data.clone()));
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plugin_progress_without_treating_it_as_automation() {
        let response = tiktools_plugin_sdk::PluginCallResult {
            events: vec![tiktools_plugin_sdk::PluginEvent {
                event_type: PLUGIN_PROGRESS_EVENT_TYPE.to_owned(),
                data: json!({
                    "status": "downloading",
                    "progress": 0.5,
                    "message": "Downloading model"
                }),
            }],
            ..Default::default()
        };
        let update = parse_plugin_progress(&response).expect("progress event should parse");
        assert_eq!(
            update.state,
            crate::ipc::messages::PluginProgressState::Downloading
        );
        assert_eq!(update.progress, Some(0.5));
        assert_eq!(update.message, "Downloading model");
        let parsed = parse_polled_events(&[], &response);
        assert!(parsed.events.is_empty());
        assert_eq!(parsed.undeclared, 1);
    }

    #[test]
    fn rejects_invalid_plugin_progress() {
        let response = tiktools_plugin_sdk::PluginCallResult {
            events: vec![tiktools_plugin_sdk::PluginEvent {
                event_type: PLUGIN_PROGRESS_EVENT_TYPE.to_owned(),
                data: json!({
                    "status": "downloading",
                    "progress": 2.0,
                    "message": "bad"
                }),
            }],
            ..Default::default()
        };
        assert!(parse_plugin_progress(&response).is_none());
    }

    fn globals_fixture() -> std::collections::BTreeMap<String, String> {
        [
            ("commandHost".to_owned(), "127.0.0.1".to_owned()),
            ("commandPort".to_owned(), "46665".to_owned()),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn render_resolves_globals_beside_event_spans() {
        let event = json!({"data": {"repeatCount": 3}});
        let rendered = render_template(
            "http://{{ globals.commandHost }}:{{ globals.commandPort }}/x {{ event.data.repeatCount }}",
            &event,
            &globals_fixture(),
        );
        assert_eq!(rendered, "http://127.0.0.1:46665/x 3");
    }

    #[test]
    fn render_drops_unknown_globals_and_keeps_event_behavior() {
        let event = json!({"data": {"giftName": "Rose"}});
        let rendered = render_template(
            "{{ globals.missing }}|{{ event.data.giftName }}|{{ event.nope }}",
            &event,
            &globals_fixture(),
        );
        assert_eq!(rendered, "|Rose|");
    }

    #[test]
    fn empty_authority_detection_covers_globals_gaps() {
        assert!(has_empty_authority("http:///api/chat"));
        assert!(has_empty_authority("https:///x"));
        assert!(!has_empty_authority("http://127.0.0.1:46665/api/chat"));
        assert!(!has_empty_authority("http://host/api/chat"));
        assert!(!has_empty_authority("not a url"));
    }

    #[test]
    fn render_globals_only_leaves_event_spans_literal() {
        let rendered = render_globals_only(
            "http://{{ globals.commandHost }}:{{ globals.commandPort }}/{{ event.data.path }}",
            &globals_fixture(),
        );
        assert_eq!(rendered, "http://127.0.0.1:46665/{{ event.data.path }}");
    }
}
