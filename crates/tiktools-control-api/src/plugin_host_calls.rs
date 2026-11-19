//! Capability-gated plugin → host calls.
//!
//! A process plugin can ask the host to execute one allowlisted RPC on its
//! behalf through the `host.call` reverse channel. This is what lets any user
//! write a replacement remote-control plugin: the plugin owns its listener
//! and protocol, while the host keeps authorization and never exposes a
//! bearer token, a `ControlApi` handle or raw settings.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use serde_json::{json, Value};
use tiktools_plugin_api::capabilities::HOST_CALL;
use tiktools_plugin_loader::HostCallHandler;
use tokio::runtime::Handle;

use crate::{ControlApi, RemoteCapability, RemoteSession, RequestContext};

const SESSION_LIFETIME: Duration = Duration::from_secs(60);

/// Methods a plugin may ask the host to execute, with the capabilities the
/// synthetic session needs. Some methods need more than the policy capability
/// because `authorize_remote` adds domain checks (e.g. `points.adjust` also
/// requires `points.read`, `workflows.disable` also requires `workflows.read`).
/// Intentionally narrow: no introspection (`rpc.discover`, `rpc.schema`), no
/// `collaboration.*`, no cloud/login, and nothing that needs durable metadata.
const ALLOWED: &[(&str, &[RemoteCapability])] = &[
    ("live.status", &[RemoteCapability::LiveRead]),
    ("live.monitor.status", &[RemoteCapability::LiveRead]),
    ("points.info", &[RemoteCapability::PointsRead]),
    ("points.leaderboard", &[RemoteCapability::PointsRead]),
    ("points.view", &[RemoteCapability::PointsRead]),
    ("workflows.list", &[RemoteCapability::WorkflowsRead]),
    ("workflows.get", &[RemoteCapability::WorkflowsRead]),
    ("automation.list", &[RemoteCapability::AutomationRead]),
    ("automation.get", &[RemoteCapability::AutomationRead]),
    (
        "competitions.summary",
        &[RemoteCapability::CompetitionsRead],
    ),
    (
        "workflows.disable",
        &[
            RemoteCapability::WorkflowsRead,
            RemoteCapability::WorkflowsToggle,
        ],
    ),
    (
        "automation.enable",
        &[
            RemoteCapability::AutomationRead,
            RemoteCapability::AutomationToggle,
        ],
    ),
    (
        "automation.disable",
        &[
            RemoteCapability::AutomationRead,
            RemoteCapability::AutomationToggle,
        ],
    ),
    (
        "points.adjust",
        &[RemoteCapability::PointsRead, RemoteCapability::PointsAdjust],
    ),
];

/// Capabilities required for one host call, or `None` when the method is not
/// in the plugin allowlist.
pub(crate) fn capabilities_for(method: &str) -> Option<&'static [RemoteCapability]> {
    ALLOWED
        .iter()
        .find(|(allowed, _)| *allowed == method)
        .map(|(_, capabilities)| *capabilities)
}

/// Host-provided policy for the reverse channel. Developers pass an explicit
/// allowlist of first-party plugin ids; a plugin can never grant itself these
/// operations by declaring `trust: "trusted"` in its own manifest.
pub struct HostCallPolicy {
    pub first_party_plugin_ids: Vec<String>,
    pub max_params_bytes: usize,
    pub max_result_bytes: usize,
}

impl Default for HostCallPolicy {
    fn default() -> Self {
        Self {
            first_party_plugin_ids: vec!["tiktools.remote-control".to_owned()],
            max_params_bytes: 16 * 1024,
            max_result_bytes: 64 * 1024,
        }
    }
}

struct PluginHostCalls {
    api: Arc<ControlApi>,
    handle: Handle,
    next: AtomicU64,
    policy: HostCallPolicy,
}

impl PluginHostCalls {
    /// Only plugins explicitly allowlisted by the host, published as `tiktools`,
    /// trusted and running as a process may use the privileged host calls.
    fn is_first_party(
        &self,
        plugin_id: &str,
        manifest: &tiktools_plugin_api::PluginManifest,
    ) -> bool {
        use tiktools_plugin_api::{manifest::PluginTrust, PluginRuntimeKind};
        self.policy
            .first_party_plugin_ids
            .iter()
            .any(|id| id == plugin_id)
            && manifest.metadata.publisher.as_deref() == Some("tiktools")
            && manifest.trust == PluginTrust::Trusted
            && manifest.runtime == PluginRuntimeKind::Process
    }
}

impl HostCallHandler for PluginHostCalls {
    fn call(&self, plugin_id: &str, method: &str, params: Value) -> Result<Value, String> {
        // Trust the host's catalog, never the plugin's own declarations.
        let discovered = self
            .api
            .core()
            .plugins
            .get(plugin_id)
            .ok_or_else(|| "plugin is not in the host catalog".to_owned())?;
        self.api
            .core()
            .capabilities
            .require_capability(&discovered.manifest, HOST_CALL)
            .map_err(|error| error.to_string())?;

        // Privileged operations are first-party only. Declaring `host.call`
        // (or even `trust: trusted`) in a plugin's own manifest is NOT enough:
        // the host's explicit allowlist and publisher identity decide.
        let privileged = method.starts_with("remote.")
            || method.starts_with("cloud.login.")
            || method == "cloud.status";
        if privileged && !self.is_first_party(plugin_id, &discovered.manifest) {
            return Err(format!(
                "host call `{method}` is restricted to first-party plugins"
            ));
        }

        let params_bytes = serde_json::to_vec(&params)
            .map(|value| value.len())
            .unwrap_or(usize::MAX);
        if params_bytes > self.policy.max_params_bytes {
            return Err("host call params are too large".to_owned());
        }

        // Login-code operations are intentionally NOT exposed through the
        // generic remote-policy path (all `cloud.login.*` are RemotePolicy::
        // Deny). A plugin that displays a relayed code needs to hand the code
        // back to the host; it calls this narrow, direct branch instead. The
        // host keeps the Cloud session and the code is never stored by the
        // plugin. No bearer token or ControlApi crosses the boundary.
        #[cfg(feature = "cloud-auth")]
        match method {
            "cloud.login.code.submit" => {
                let code = params
                    .get("code")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| "code is required".to_owned())?;
                if code.is_empty() || code.len() > 16 || !code.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("invalid login code".to_owned());
                }
                let operation = params
                    .get("operationId")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty() && value.len() <= 256)
                    .map(str::to_owned);
                return self
                    .handle
                    .block_on(
                        self.api
                            .core()
                            .cloud_redeem_login_code(operation.as_deref(), code),
                    )
                    .map(|status| serde_json::to_value(status).unwrap_or(Value::Null))
                    .map_err(|error| error.to_string());
            }
            "cloud.login.code.resend" => {
                let operation = params
                    .get("operationId")
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty() && value.len() <= 256)
                    .map(str::to_owned);
                return self
                    .handle
                    .block_on(self.api.core().cloud_relay_login_code(operation.as_deref()))
                    .map(|status| serde_json::to_value(status).unwrap_or(Value::Null))
                    .map_err(|error| error.to_string());
            }
            "cloud.status" => {
                return self
                    .handle
                    .block_on(self.api.core().cloud_status())
                    .map(|status| serde_json::to_value(status).unwrap_or(Value::Null))
                    .map_err(|error| error.to_string());
            }
            "remote.command.dispatch" => {
                // The plugin supplies only a command id, operation and args.
                // Expiry, policy revision and identity are host-owned, so a
                // plugin can never forge a controller's authorization.
                let command_id = params
                    .get("commandId")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| "commandId is required".to_owned())?;
                if command_id.is_empty() || command_id.len() > 128 {
                    return Err("invalid commandId".to_owned());
                }
                let operation = params
                    .get("operation")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| "operation is required".to_owned())?;
                if !matches!(
                    operation,
                    "live.status"
                        | "workflow.list"
                        | "workflow.get"
                        | "automation.list"
                        | "automation.get"
                        | "competition.list"
                        | "points.info"
                        | "points.leaderboard"
                ) {
                    return Err(format!("remote operation `{operation}` is not allowed"));
                }
                let args = params.get("args").cloned().unwrap_or(serde_json::json!({}));
                if !args.is_object() {
                    return Err("args must be an object".to_owned());
                }
                let raw = json!({
                    "jsonrpc": "2.0",
                    "id": command_id,
                    "method": operation,
                    "params": args,
                });
                let capability = if operation.starts_with("points.") {
                    RemoteCapability::PointsRead
                } else if operation.starts_with("workflow.") {
                    RemoteCapability::WorkflowsRead
                } else if operation.starts_with("automation.") {
                    RemoteCapability::AutomationRead
                } else if operation.starts_with("competition.") {
                    RemoteCapability::CompetitionsRead
                } else {
                    RemoteCapability::LiveRead
                };
                let session = RemoteSession::new(
                    plugin_id.to_owned(),
                    format!("plugin:{plugin_id}:remote:{command_id}"),
                    [capability],
                    SESSION_LIFETIME,
                )
                .map_err(|error| error.to_string())?;
                let response = self.handle.block_on(
                    self.api
                        .execute_value_with_context(&RequestContext::Remote(session), &raw),
                );
                return match response.error {
                    Some(error) => Err(format!("{}: {}", error.code, error.message)),
                    None => Ok(serde_json::json!({
                        "commandId": command_id,
                        "status": "ok",
                        "result": response.result.unwrap_or(Value::Null),
                    })),
                };
            }
            _ => {}
        }

        let capabilities = capabilities_for(method)
            .ok_or_else(|| format!("host call `{method}` is not allowed"))?;

        // A fresh, narrow, time-limited session per call. `new` deliberately
        // carries no Authorization and no user grant: a plugin must never
        // inherit a controller's session or be able to mint durable commands.
        let sequence = self.next.fetch_add(1, Ordering::Relaxed);
        let session = RemoteSession::new(
            plugin_id.to_owned(),
            format!("plugin:{plugin_id}:{sequence}"),
            capabilities.iter().copied(),
            SESSION_LIFETIME,
        )
        .map_err(|error| error.to_string())?;
        let context = RequestContext::Remote(session);
        // Plugin-supplied `meta` is dropped: identity comes only from the
        // session above, so a plugin cannot forge command provenance.
        let raw = json!({
            "jsonrpc": "2.0",
            "id": sequence,
            "method": method,
            "params": params,
        });
        let response = self
            .handle
            .block_on(self.api.execute_value_with_context(&context, &raw));
        match response.error {
            Some(error) => Err(format!("{}: {}", error.code, error.message)),
            None => Ok(response.result.unwrap_or(Value::Null)),
        }
    }
}

impl ControlApi {
    /// Install the reverse-call sink. Must be called before plugins that use
    /// `host.call` are started; plugins already running keep their previous
    /// (ungranted) handler.
    pub fn install_plugin_host_calls(self: &Arc<Self>, handle: Handle) {
        self.install_plugin_host_calls_with_policy(handle, HostCallPolicy::default());
    }

    /// Install with an explicit host-controlled policy (e.g. a different
    /// first-party plugin allowlist for a custom build).
    pub fn install_plugin_host_calls_with_policy(
        self: &Arc<Self>,
        handle: Handle,
        policy: HostCallPolicy,
    ) {
        let handler = Arc::new(PluginHostCalls {
            api: Arc::clone(self),
            handle,
            next: AtomicU64::new(1),
            policy,
        });
        self.core().plugins.set_host_call_handler(handler);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_call_allowlist_excludes_introspection_and_collaboration() {
        assert!(ALLOWED.iter().any(|(method, _)| *method == "live.status"));
        assert!(ALLOWED.iter().any(|(method, _)| *method == "points.adjust"));
        for forbidden in [
            "rpc.discover",
            "rpc.schema",
            "collaboration.hello",
            "collaboration.subscribe",
            "cloud.login.begin",
            "plugins.settings.get",
        ] {
            assert!(
                !ALLOWED.iter().any(|(method, _)| *method == forbidden),
                "{forbidden} must not be callable by a plugin"
            );
        }
    }

    #[test]
    fn domain_checks_get_the_capabilities_they_require() {
        // `authorize_remote` requires these extra capabilities in addition to
        // the method's own remote policy; without them the call is denied.
        let adjust = capabilities_for("points.adjust").unwrap();
        assert!(adjust.contains(&RemoteCapability::PointsRead));
        assert!(adjust.contains(&RemoteCapability::PointsAdjust));

        let disable = capabilities_for("workflows.disable").unwrap();
        assert!(disable.contains(&RemoteCapability::WorkflowsRead));
        assert!(disable.contains(&RemoteCapability::WorkflowsToggle));

        for method in ["automation.enable", "automation.disable"] {
            let caps = capabilities_for(method).unwrap();
            assert!(caps.contains(&RemoteCapability::AutomationRead));
            assert!(caps.contains(&RemoteCapability::AutomationToggle));
        }

        // plugins.list has no remote policy; it must stay out of the allowlist.
        assert!(capabilities_for("plugins.list").is_none());
    }

    #[test]
    fn default_policy_names_the_first_party_plugin() {
        let policy = HostCallPolicy::default();
        assert_eq!(policy.first_party_plugin_ids, ["tiktools.remote-control"]);
        assert_eq!(policy.max_params_bytes, 16 * 1024);
        assert_eq!(policy.max_result_bytes, 64 * 1024);
    }
}
