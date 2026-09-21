//! Restricted broker between one plugin WebView and the host.
//!
//! This is the native counterpart of the plugin-side client
//! (`plugins/sonicboom/ui/src/broker.ts`) and the web host shim
//! (`src/web/plugin-ui/plugin-webview-host.ts`). The three sides share the
//! versioned envelope `{apiVersion: 1, id, ...}` by convention, pinned by
//! interop tests — never by shared imports, since the plugin package must
//! stay buildable on its own.
//!
//! Ownership: the broker is constructed with the window's bound plugin id
//! and injects it into every downstream call. Client-supplied `pluginId`
//! values are rejected when they disagree, so a plugin page can only ever
//! read its own settings/options and execute its own actions. The method
//! surface is a fixed allowlist; anything else fails closed.

use std::{collections::BTreeMap, sync::Arc};

use serde_json::Value;
use tiktools_control_api::{ControlApi, MAX_REQUEST_BYTES};

pub const BROKER_API_VERSION: u32 = 1;
const MAX_ID_LEN: usize = 128;
const MAX_TOPIC_LEN: usize = 128;
const MAX_ACTION_LEN: usize = 256;
const MAX_SOURCE_LEN: usize = 512;

/// Out-of-band effect of an `events.subscribe` / `events.unsubscribe`
/// call, applied by the window manager to the calling window's topic set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionEffect {
    Subscribe(Vec<String>),
    Unsubscribe(Vec<String>),
}

pub struct PluginUiBroker {
    plugin_id: String,
    control: Arc<ControlApi>,
}

impl PluginUiBroker {
    pub fn new(plugin_id: String, control: Arc<ControlApi>) -> Self {
        Self { plugin_id, control }
    }

    /// Handles one raw broker request, returning the serialized response
    /// plus an optional subscription effect for the window manager.
    pub async fn handle(&self, raw: &str) -> (String, Option<SubscriptionEffect>) {
        if raw.len() > MAX_REQUEST_BYTES {
            return (
                error_response(&extract_id(raw), "request exceeds the size limit"),
                None,
            );
        }
        let value: Value = match serde_json::from_str(raw) {
            Ok(value) => value,
            Err(_) => {
                return (
                    error_response(&extract_id(raw), "malformed broker request"),
                    None,
                );
            }
        };
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= MAX_ID_LEN)
            .unwrap_or("unknown")
            .to_owned();
        if value.get("apiVersion").and_then(Value::as_u64) != Some(u64::from(BROKER_API_VERSION)) {
            return (error_response(&id, "unsupported broker apiVersion"), None);
        }
        let method = value
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = value.get("params").cloned().unwrap_or(Value::Null);
        if params.is_null() {
            // Methods below treat missing params as `{}` where sensible.
        } else if !params.is_object() {
            return (error_response(&id, "broker params must be an object"), None);
        }
        // Fail closed on confused-deputy identities: the page never needs
        // to name its plugin, and a mismatched name is always rejected.
        if let Some(claimed) = params.get("pluginId").and_then(Value::as_str) {
            if claimed != self.plugin_id {
                return (
                    error_response(&id, "pluginId does not match the owning plugin"),
                    None,
                );
            }
        }
        match method {
            "settings.get" => (self.settings_get(&id).await, None),
            "settings.set" => (self.settings_set(&id, &params).await, None),
            "actions.execute" => (self.action_execute(&id, &params).await, None),
            "options.get" => (self.options_get(&id, &params).await, None),
            "events.subscribe" => self.subscribe(&id, &params, true),
            "events.unsubscribe" => self.subscribe(&id, &params, false),
            "host.locale" => (ok_response(&id, Value::String(host_locale())), None),
            "host.theme" => (ok_response(&id, Value::String(host_theme())), None),
            _ => (
                error_response(&id, &format!("unknown broker method `{method}`")),
                None,
            ),
        }
    }

    async fn settings_get(&self, id: &str) -> String {
        let response = self
            .control
            .execute_value(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "plugins.settings.get",
                "params": { "pluginId": self.plugin_id },
            }))
            .await;
        project_response(id, &response, |result| {
            result.get("values").cloned().unwrap_or(Value::Null)
        })
    }

    async fn settings_set(&self, id: &str, params: &Value) -> String {
        let Some(values) = params.get("values").and_then(Value::as_object) else {
            return error_response(id, "settings.set needs a values object");
        };
        let response = self
            .control
            .execute_value(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "plugins.settings.set",
                "params": { "pluginId": self.plugin_id, "values": values },
            }))
            .await;
        project_response(id, &response, |result| {
            result.get("values").cloned().unwrap_or(Value::Null)
        })
    }

    async fn action_execute(&self, id: &str, params: &Value) -> String {
        let Some(action) = params
            .get("action")
            .and_then(Value::as_str)
            .filter(|action| !action.is_empty() && action.len() <= MAX_ACTION_LEN)
        else {
            return error_response(id, "actions.execute needs an action name");
        };
        let config: BTreeMap<String, Value> = params
            .get("config")
            .and_then(Value::as_object)
            .map(|object| {
                object
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default();
        // Broker execution is always live: the plugin UI's execute buttons
        // (voice tests, output switches) must take real effect. Ownership
        // scoping still confines the call to the window's own plugin.
        let response = self
            .control
            .execute_value(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "plugins.action.execute",
                "params": {
                    "actionType": action,
                    "config": config,
                    "live": true,
                    "pluginId": self.plugin_id,
                },
            }))
            .await;
        project_response(id, &response, |result| result.clone())
    }

    async fn options_get(&self, id: &str, params: &Value) -> String {
        let Some(source) = params
            .get("source")
            .and_then(Value::as_str)
            .filter(|source| !source.is_empty() && source.len() <= MAX_SOURCE_LEN)
        else {
            return error_response(id, "options.get needs an option source");
        };
        let refresh = params
            .get("refresh")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let response = self
            .control
            .execute_value(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "plugins.options",
                "params": {
                    "source": source,
                    "refresh": refresh,
                    "pluginId": self.plugin_id,
                },
            }))
            .await;
        project_response(id, &response, |result| {
            serde_json::json!({
                "options": result.get("options").cloned().unwrap_or(Value::Null),
                "selected": result.get("selected").cloned().unwrap_or(Value::Null),
            })
        })
    }

    fn subscribe(
        &self,
        id: &str,
        params: &Value,
        subscribe: bool,
    ) -> (String, Option<SubscriptionEffect>) {
        let Some(topics) = params.get("topics").and_then(Value::as_array) else {
            return (
                error_response(id, "events subscription needs a topics array"),
                None,
            );
        };
        if topics.is_empty() || topics.len() > 32 {
            return (
                error_response(id, "events subscription needs 1..=32 topics"),
                None,
            );
        }
        let mut names = Vec::with_capacity(topics.len());
        for topic in topics {
            let Some(name) = topic.as_str().filter(|name| {
                !name.is_empty() && name.len() <= MAX_TOPIC_LEN && name.starts_with("plugin.")
            }) else {
                return (error_response(id, "event topics are plugin.* names"), None);
            };
            names.push(name.to_owned());
        }
        let effect = if subscribe {
            SubscriptionEffect::Subscribe(names)
        } else {
            SubscriptionEffect::Unsubscribe(names)
        };
        (ok_response(id, Value::Bool(true)), Some(effect))
    }
}

fn ok_response(id: &str, result: Value) -> String {
    serde_json::json!({
        "apiVersion": BROKER_API_VERSION,
        "id": id,
        "ok": true,
        "result": result,
    })
    .to_string()
}

fn error_response(id: &str, message: &str) -> String {
    serde_json::json!({
        "apiVersion": BROKER_API_VERSION,
        "id": id,
        "ok": false,
        "error": message,
    })
    .to_string()
}

fn project_response(
    id: &str,
    response: &tiktools_control_api::RpcResponse,
    project: impl FnOnce(&Value) -> Value,
) -> String {
    if let Some(error) = response.error.as_ref() {
        return error_response(id, &error.message);
    }
    match response.result.as_ref() {
        Some(result) => ok_response(id, project(result)),
        None => error_response(id, "host returned no result"),
    }
}

/// Best-effort id recovery for malformed input so the page can correlate
/// the rejection instead of hanging the call.
fn extract_id(raw: &str) -> String {
    let prefix: String = raw.chars().take(512).collect();
    let mut rest = prefix.as_str();
    while let Some(start) = rest.find("\"id\"") {
        // The match is ASCII, so slicing after it is always a boundary.
        let mut candidate = &rest[start + 4..];
        candidate = candidate.trim_start_matches([' ', '\t', '\r', '\n', ':']);
        candidate = candidate.trim_start_matches([' ', '\t', '\r', '\n']);
        if let Some(quoted) = candidate.strip_prefix('"') {
            let end = quoted.find('"').unwrap_or(quoted.len());
            let id = &quoted[..end];
            if !id.is_empty() && id.len() <= MAX_ID_LEN {
                return id.to_owned();
            }
        }
        rest = rest.get(start + 5..).unwrap_or_default();
    }
    "unknown".to_owned()
}

/// Host locale from the process environment (`language-region`); the core
/// owns no locale setting yet, so the desktop host reports its own.
fn host_locale() -> String {
    for variable in ["LANGUAGE", "LC_ALL", "LANG"] {
        if let Ok(raw) = std::env::var(variable) {
            let first = raw.split(':').next().unwrap_or_default();
            let base = first.split('.').next().unwrap_or_default();
            let tag = base.replace('_', "-");
            if !tag.is_empty() && tag != "C" && tag != "POSIX" {
                return tag;
            }
        }
    }
    "en".to_owned()
}

/// Host theme override (`TIKTOOLS_THEME=light|dark`); defaults to
/// `system` until the core owns a theme setting.
fn host_theme() -> String {
    match std::env::var("TIKTOOLS_THEME")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "light" | "dark" => std::env::var("TIKTOOLS_THEME")
            .unwrap_or_default()
            .to_ascii_lowercase(),
        _ => "system".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiktools_core::AppCore;

    struct NullEmitter;
    impl tiktools_core::HostEmitter for NullEmitter {
        fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
    }

    fn broker_for(plugin_id: &str) -> PluginUiBroker {
        let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
        PluginUiBroker::new(plugin_id.to_owned(), Arc::new(ControlApi::new(core)))
    }

    fn request(id: &str, method: &str, params: Value) -> String {
        serde_json::json!({
            "apiVersion": BROKER_API_VERSION,
            "id": id,
            "method": method,
            "params": params,
        })
        .to_string()
    }

    #[tokio::test]
    async fn unknown_methods_versions_and_identities_fail_closed() {
        let broker = broker_for("owner");
        let (raw, effect) = broker
            .handle(&request("1", "plugins.uninstall", serde_json::json!({})))
            .await;
        assert!(effect.is_none());
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["id"], "1");

        let bad_version = serde_json::json!({
            "apiVersion": 999, "id": "2", "method": "host.locale", "params": {},
        })
        .to_string();
        let (raw, _) = broker.handle(&bad_version).await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);

        // A mismatched pluginId is rejected even for harmless methods.
        let spoofed = request(
            "3",
            "host.locale",
            serde_json::json!({ "pluginId": "someone.else" }),
        );
        let (raw, _) = broker.handle(&spoofed).await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);

        // Malformed input still correlates by the extracted id.
        let (raw, _) = broker.handle(r#"{"id": "abc", "method": broken"#).await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["id"], "abc");

        // Oversized input never reaches JSON parsing.
        let big = format!(
            r#"{{"apiVersion": 1, "id": "big", "method": "host.locale", "params": {{}}, "pad": "{}"}}"#,
            "x".repeat(MAX_REQUEST_BYTES)
        );
        let (raw, _) = broker.handle(&big).await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["id"], "big");
    }

    #[tokio::test]
    async fn host_introspection_always_answers() {
        let broker = broker_for("owner");
        let (raw, _) = broker
            .handle(&request("1", "host.locale", serde_json::json!({})))
            .await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], true);
        assert!(!response["result"].as_str().unwrap_or_default().is_empty());
        let (raw, _) = broker
            .handle(&request("2", "host.theme", serde_json::json!({})))
            .await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert!(
            ["light", "dark", "system"].contains(&response["result"].as_str().unwrap_or_default())
        );
    }

    #[tokio::test]
    async fn subscriptions_validate_topics_and_emit_effects() {
        let broker = broker_for("owner");
        let (raw, effect) = broker
            .handle(&request(
                "1",
                "events.subscribe",
                serde_json::json!({ "topics": ["plugin.event", "plugin.status"] }),
            ))
            .await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], true);
        assert_eq!(
            effect,
            Some(SubscriptionEffect::Subscribe(vec![
                "plugin.event".to_owned(),
                "plugin.status".to_owned()
            ]))
        );
        // Non-plugin topics are rejected: the broker never fans out
        // unrelated host traffic to a plugin page.
        let (raw, effect) = broker
            .handle(&request(
                "2",
                "events.subscribe",
                serde_json::json!({ "topics": ["live.event"] }),
            ))
            .await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);
        assert!(effect.is_none());
    }

    #[tokio::test]
    async fn raw_native_ipc_cannot_escape_the_broker_allowlist() {
        // The pop-out window's raw `window.ipc` transport reaches ONLY
        // this broker (see the `with_ipc_handler` wiring in `open`), so
        // arbitrary posts from page JavaScript are still confined to the
        // allowlist and the bound plugin.
        let broker = broker_for("owner");
        // Main-API method names smuggled through the raw transport are
        // unknown methods here — never routed to the control API.
        for method in [
            "plugins.settings.get",
            "plugins.settings.set",
            "plugins.action.execute",
            "plugins.uninstall",
            "app.state.get",
            "ipc",
            "eval",
            "settings.get ", // trailing space: exact match only
        ] {
            let (raw, effect) = broker
                .handle(&request("1", method, serde_json::json!({})))
                .await;
            assert!(effect.is_none(), "{method}");
            let response: Value = serde_json::from_str(&raw).unwrap();
            assert_eq!(response["ok"], false, "{method}");
            assert_eq!(response["id"], "1", "{method}");
        }
        // Allowlisted methods still validate their arguments.
        for (method, params) in [
            ("actions.execute", serde_json::json!({})),
            ("actions.execute", serde_json::json!({"action": ""})),
            ("options.get", serde_json::json!({})),
            ("settings.set", serde_json::json!({"values": [1, 2]})),
            (
                "events.subscribe",
                serde_json::json!({"topics": ["live.event"]}),
            ),
        ] {
            let (raw, effect) = broker.handle(&request("2", method, params)).await;
            assert!(effect.is_none(), "{method}");
            let response: Value = serde_json::from_str(&raw).unwrap();
            assert_eq!(response["ok"], false, "{method}");
        }
        // Non-object params never reach a method.
        let (raw, _) = broker
            .handle(&request("3", "host.locale", serde_json::json!([1])))
            .await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);
        // A matching pluginId is accepted shape-wise but the call stays
        // scoped to the bound plugin (this core has no plugins, so the
        // downstream lookup fails naming the bound id — never another).
        let (raw, _) = broker
            .handle(&request(
                "4",
                "settings.get",
                serde_json::json!({ "pluginId": "owner" }),
            ))
            .await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);
        assert!(response["error"]
            .as_str()
            .unwrap_or_default()
            .contains("owner"));
    }

    #[tokio::test]
    async fn settings_calls_scope_to_the_bound_plugin() {
        let broker = broker_for("missing.plugin");
        let (raw, _) = broker
            .handle(&request("1", "settings.get", serde_json::json!({})))
            .await;
        let response: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(response["ok"], false);
        assert!(response["error"]
            .as_str()
            .unwrap_or_default()
            .contains("missing.plugin"));
    }
}
