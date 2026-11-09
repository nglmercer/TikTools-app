//! Headless control plane for TikTools.
//!
//! Every client speaks the same JSON-RPC-style protocol and every method
//! funnels into one [`AppCore`](tiktools_core::AppCore) operation:
//!
//! ```text
//! client -> ControlApi -> AppCore service
//! ```
//!
//! Clients: CLI, JSON stdio (`host --stdio`), local IPC (Unix socket /
//! Windows named pipe), WebView bridge, tests, AI agents.

pub mod client;
pub mod context;
pub mod error;
pub mod metadata;
pub mod modules;
pub mod ownership;
pub mod policy;
pub mod registry;
pub mod request;
pub mod response;
pub mod router;
pub mod transport;

pub use client::{ClientError, ControlClient, ControlEvent};
pub use context::{RemoteCapability, RemoteSession, RequestContext};
pub use error::ApiError;
pub use metadata::{CallResult, RequestMetadata, ResponseMetadata};
pub use registry::MethodMeta;
pub use request::{RpcId, RpcRequest};
pub use response::RpcResponse;
pub use router::ControlRouter;
pub use transport::{
    owner_params_bytes, owner_request_bytes, run_ipc, run_ipc_shared, run_ipc_shared_with_ready,
    run_stdio, run_stdio_shared, IPC_NAME, MAX_OWNER_LIBRARY_PARAMS_BYTES,
    MAX_OWNER_LIBRARY_REQUEST_BYTES, MAX_OWNER_RESPONSE_BYTES, MAX_PARAMS_BYTES, MAX_REQUEST_BYTES,
};

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tiktools_core::AppCore;

use crate::registry::MethodRegistry;

/// Per-request execution budget. Plugin actions can legitimately take up to
/// two minutes (manifest `timeoutMs`), so the RPC budget sits above that.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(150);

/// Login completion waits up to 180 seconds for the browser callback before
/// redeeming it; other handlers keep the standard budget.
pub fn request_timeout_for(method: &str) -> Duration {
    if method == "cloud.login.complete" {
        Duration::from_secs(240)
    } else {
        REQUEST_TIMEOUT
    }
}

/// The local IPC client must wait past the server's login deadline so the
/// authoritative timeout response can arrive. Other methods are unchanged.
pub fn client_timeout_for(method: &str) -> Duration {
    if method == "cloud.login.complete" {
        Duration::from_secs(270)
    } else {
        REQUEST_TIMEOUT
    }
}

/// Thin adapter over [`AppCore`]. Business logic stays in core services;
/// handlers only validate typed params and map results/errors.
pub struct ControlApi {
    core: Arc<AppCore>,
    router: ControlRouter,
}

impl ControlApi {
    pub fn new(core: Arc<AppCore>) -> Self {
        core.start_competition_timer();
        core.start_coordination_worker();
        #[cfg(feature = "cloud-auth")]
        core.start_cloud_sync_worker();
        let mut router = ControlRouter::new();
        Self::register_all(&mut router);
        Self { core, router }
    }

    /// Registers every domain module plus `rpc.*` discovery into `router`.
    /// This is the single source of truth for the control surface: `new`
    /// builds hosts from it and out-of-tree parity tests enumerate it.
    pub fn register_all(router: &mut ControlRouter) {
        modules::system::register(router);
        modules::collaboration::register(router);
        modules::plugins::register(router);
        #[cfg(feature = "plugin-install")]
        modules::packages::register(router);
        #[cfg(feature = "plugin-install")]
        modules::registry::register(router);
        #[cfg(feature = "cloud-auth")]
        modules::cloud::register(router);
        #[cfg(feature = "cloud-auth")]
        modules::sync::register(router);
        #[cfg(all(feature = "cloud-auth", feature = "http", feature = "plugin-install"))]
        modules::publisher::register(router);
        modules::settings::register(router);
        modules::live::register(router);
        modules::points::register(router);
        modules::rankings::register(router);
        modules::competitions::register(router);
        modules::automation::register(router);
        modules::automation_adoption::register(router);
        modules::automation_approval::register(router);
        modules::automation_definition::register(router);
        modules::automation_effects::register(router);
        modules::automation_library::register(router);
        modules::processors::register(router);
        modules::media::register(router);
        modules::app::register(router);
        modules::updates::register(router);
        modules::globals::register(router);
        modules::creators::register(router);
        modules::analytics::register(router);
        modules::gifts::register(router);
        modules::ui::register(router);
        modules::widgets::register(router);
        modules::workflows::register(router);
        // Agent-facing risk metadata: destructive ops delete or reset
        // persisted state, so agents should confirm before calling them.
        for name in [
            "plugins.uninstall",
            "plugins.settings.reset",
            "automation.delete",
            "globals.delete",
            "workflows.delete",
            "points.reset",
            "competitions.delete",
            "competitions.cancel",
            "rankings.delete",
            "rankings.reset",
            "creators.history.clear",
            "system.shutdown",
        ] {
            router.set_flags(name, true, false);
        }
        let registry = std::sync::Arc::new(std::sync::RwLock::new(MethodRegistry::default()));
        modules::rpc::register(router, std::sync::Arc::clone(&registry));
        // Side-effect metadata never grants authority. Each supported method
        // needs explicit policy; unlisted methods stay denied.
        for name in [
            "live.status",
            "live.monitor.status",
            "ui.feed.backfill",
            "ui.feed.browser",
            "ui.feed.creator",
        ] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::LiveRead),
            );
        }
        router.set_remote_policy(
            "workflows.disable",
            policy::RemotePolicy::Capability(RemoteCapability::WorkflowsToggle),
        );
        router.set_remote_policy(
            "workflows.delete",
            policy::RemotePolicy::Capability(RemoteCapability::WorkflowsEdit),
        );
        for name in ["automation.list", "automation.get", "automation.source.get"] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::AutomationRead),
            );
        }
        router.set_remote_policy(
            "competitions.summary",
            policy::RemotePolicy::Capability(RemoteCapability::CompetitionsRead),
        );
        for name in [
            "competitions.start",
            "competitions.resume",
            "competitions.pause",
            "competitions.cancel",
        ] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::CompetitionsManage),
            );
        }
        for name in ["automation.disable", "automation.enable"] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::AutomationToggle),
            );
        }
        router.set_remote_policy(
            "automation.delete",
            policy::RemotePolicy::Capability(RemoteCapability::AutomationEdit),
        );
        for name in ["automation.create", "automation.update"] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::AutomationEdit),
            );
        }

        for name in [
            "points.viewer.get",
            "points.leaderboard",
            "points.info",
            "points.view",
        ] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::PointsRead),
            );
        }
        router.set_remote_policy(
            "points.adjust",
            policy::RemotePolicy::Capability(RemoteCapability::PointsAdjust),
        );
        router.set_remote_policy(
            "collaboration.commands.get",
            policy::RemotePolicy::Capability(RemoteCapability::CommandsRead),
        );
        for name in [
            "rpc.discover",
            "rpc.schema",
            "collaboration.hello",
            "collaboration.me",
            "collaboration.subscribe",
            "collaboration.snapshot",
            "collaboration.resume",
        ] {
            router.set_remote_policy(name, policy::RemotePolicy::Authenticated);
        }
        for name in ["workflows.list", "workflows.get"] {
            router.set_remote_policy(
                name,
                policy::RemotePolicy::Capability(RemoteCapability::WorkflowsRead),
            );
        }
        // Snapshot after every module (including rpc.*) registered so
        // discovery lists the discovery methods themselves. The lock is
        // local to this call and no registered handler can run while the
        // router is mutably borrowed, so poisoning is impossible here.
        *registry.write().expect("method registry poisoned") = MethodRegistry::snapshot(router);
    }

    pub fn core(&self) -> &Arc<AppCore> {
        &self.core
    }

    pub fn router(&self) -> &ControlRouter {
        &self.router
    }

    /// Reauthorize a queued remote response immediately before network send.
    pub fn authorize_remote(&self, session: &RemoteSession, method: &str) -> Result<(), ApiError> {
        self.router
            .get(method)
            .map(|entry| entry.remote_policy)
            .unwrap_or_default()
            .authorize(session)?;
        // Shared backfill is not filtered by creator. Use a scoped projection
        // before exposing history under a creator-restricted LIVE grant.
        if matches!(method, "ui.feed.backfill" | "ui.feed.browser")
            && session
                .grant()?
                .resources
                .contains_key(&tiktools_core::collaboration::ResourceKind::Live)
        {
            return Err(context::forbidden());
        }
        if matches!(method, "workflows.disable" | "workflows.delete")
            && !session
                .grant()?
                .capabilities
                .contains(&RemoteCapability::WorkflowsRead)
        {
            return Err(context::forbidden());
        }
        let grant = session.grant()?;
        if matches!(
            method,
            "competitions.summary"
                | "competitions.start"
                | "competitions.resume"
                | "competitions.pause"
                | "competitions.cancel"
        ) && grant
            .resources
            .contains_key(&tiktools_core::collaboration::ResourceKind::Live)
        {
            return Err(context::forbidden());
        }
        if matches!(
            method,
            "competitions.start"
                | "competitions.resume"
                | "competitions.pause"
                | "competitions.cancel"
        ) && !grant
            .capabilities
            .contains(&RemoteCapability::CompetitionsRead)
        {
            return Err(context::forbidden());
        }
        if method.starts_with("automation.") && !policy::automation_read_allowed(&grant) {
            return Err(context::forbidden());
        }
        let can_read_domain = grant
            .capabilities
            .contains(&RemoteCapability::WorkflowsRead)
            || grant.capabilities.contains(&RemoteCapability::PointsRead)
                && !grant
                    .resources
                    .contains_key(&tiktools_core::collaboration::ResourceKind::Live)
            || policy::automation_read_allowed(&grant)
            || grant
                .capabilities
                .contains(&RemoteCapability::CompetitionsRead)
                && !grant
                    .resources
                    .contains_key(&tiktools_core::collaboration::ResourceKind::Live);
        if matches!(
            method,
            "collaboration.subscribe" | "collaboration.snapshot" | "collaboration.resume"
        ) && !can_read_domain
        {
            return Err(context::forbidden());
        }
        if method == "collaboration.commands.get"
            && !can_read_domain
            && !(grant
                .capabilities
                .contains(&RemoteCapability::CompetitionsRead)
                && !grant
                    .resources
                    .contains_key(&tiktools_core::collaboration::ResourceKind::Live))
        {
            return Err(context::forbidden());
        }
        if matches!(
            method,
            "points.adjust"
                | "points.viewer.get"
                | "points.leaderboard"
                | "points.info"
                | "points.view"
        ) && (!grant.capabilities.contains(&RemoteCapability::PointsRead)
            || grant
                .resources
                .contains_key(&tiktools_core::collaboration::ResourceKind::Live))
        {
            return Err(context::forbidden());
        }
        Ok(())
    }

    pub fn authorize_remote_response(
        &self,
        session: &RemoteSession,
        method: &str,
        result: &Value,
    ) -> Result<(), ApiError> {
        self.authorize_remote(session, method)?;
        let grant = session.grant()?;
        if matches!(
            method,
            "competitions.start"
                | "competitions.resume"
                | "competitions.pause"
                | "competitions.cancel"
        ) && !policy::competition_stop_result_allowed(&grant, method, result)
        {
            return Err(context::forbidden());
        }
        if method == "competitions.summary" {
            if !grant
                .capabilities
                .contains(&RemoteCapability::CompetitionsRead)
                || grant
                    .resources
                    .contains_key(&tiktools_core::collaboration::ResourceKind::Live)
            {
                return Err(context::forbidden());
            }
            if serde_json::from_value::<tiktools_core::control::CompetitionSummaryPage>(
                result.clone(),
            )
            .is_err()
            {
                return Err(context::forbidden());
            }
            let allowed = |id: &str| {
                grant.allows_resource(tiktools_core::collaboration::ResourceKind::Competition, id)
            };
            let rows = result
                .get("competitions")
                .and_then(Value::as_array)
                .ok_or_else(context::forbidden)?;
            if result.as_object().is_none_or(|obj| {
                obj.len() != 2
                    || obj
                        .keys()
                        .any(|key| !matches!(key.as_str(), "competitions" | "nextAfter"))
            }) || rows.len() > 50
                || !rows.iter().all(|row| {
                    row.as_object().is_some_and(|obj| {
                        (obj.len() == 7
                            || (obj.len() == 8
                                && obj
                                    .get("revision")
                                    .and_then(Value::as_str)
                                    .is_some_and(|s| {
                                        s.parse::<i64>().is_ok_and(|n| n > 0 && n.to_string() == s)
                                    })))
                            && obj.keys().all(|key| {
                                matches!(
                                    key.as_str(),
                                    "id" | "name"
                                        | "kind"
                                        | "lifecycle"
                                        | "generation"
                                        | "revision"
                                        | "deadlineMs"
                                        | "remainingMs"
                                )
                            })
                    }) && row.get("id").and_then(Value::as_str).is_some_and(allowed)
                })
                || !result.get("nextAfter").is_some_and(|value| {
                    value.is_null()
                        || value.as_str().is_some_and(|id| {
                            allowed(id) && rows.last().and_then(|row| row.get("id")) == Some(value)
                        })
                })
            {
                return Err(context::forbidden());
            }
        }
        if method == "ui.feed.creator"
            && !result
                .get("creatorId")
                .and_then(Value::as_str)
                .is_some_and(|id| {
                    grant.allows_resource(tiktools_core::collaboration::ResourceKind::Live, id)
                })
        {
            return Err(context::forbidden());
        }
        if matches!(
            method,
            "points.adjust" | "points.viewer.get" | "points.leaderboard" | "points.view"
        ) {
            let allowed = |row: &Value| {
                row.get("uniqueId")
                    .and_then(Value::as_str)
                    .is_some_and(|id| {
                        grant
                            .allows_resource(tiktools_core::collaboration::ResourceKind::Viewer, id)
                    })
            };
            let permitted = match method {
                "points.view" => {
                    result
                        .get("viewers")
                        .and_then(Value::as_array)
                        .is_some_and(|rows| rows.iter().all(allowed))
                        && result
                            .get("selected")
                            .is_some_and(|v| v.is_null() || allowed(v))
                        && result.get("selectedId").is_some_and(|v| {
                            v.is_null()
                                || v.as_str().is_some_and(|id| {
                                    grant.allows_resource(
                                        tiktools_core::collaboration::ResourceKind::Viewer,
                                        id,
                                    )
                                })
                        })
                }
                "points.adjust" => allowed(result),
                "points.viewer.get" => result.get("viewer").is_some_and(allowed),
                _ => result
                    .get("viewers")
                    .and_then(Value::as_array)
                    .is_some_and(|rows| rows.iter().all(allowed)),
            };
            if !permitted
                || (method == "points.adjust"
                    && grant.max_points_delta.is_some_and(|max| {
                        !result
                            .get("delta")
                            .and_then(Value::as_f64)
                            .is_some_and(|n| n.is_finite() && n.abs() <= max)
                    }))
            {
                return Err(context::forbidden());
            }
        }
        if matches!(
            method,
            "collaboration.subscribe" | "collaboration.snapshot" | "collaboration.resume"
        ) {
            let id = result
                .get("subscriptionId")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            let revision = result
                .get("grantRevision")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            self.core
                .collaboration_stream_validate(
                    &session.authorization()?,
                    session.connection_id().ok_or_else(context::forbidden)?,
                    id,
                    revision,
                    result.get("snapshotId").and_then(Value::as_str),
                )
                .map_err(ApiError::from)?;
        }
        if matches!(
            method,
            "workflows.list" | "workflows.get" | "workflows.disable" | "workflows.delete"
        ) {
            let allowed = |record: &Value| {
                record.get("id").and_then(Value::as_str).is_some_and(|id| {
                    grant.allows_resource(tiktools_core::collaboration::ResourceKind::Workflow, id)
                })
            };
            let permitted = if method != "workflows.list" {
                allowed(result)
            } else {
                result
                    .get("workflows")
                    .and_then(Value::as_array)
                    .is_some_and(|rows| rows.iter().all(allowed))
            };
            if !permitted {
                return Err(context::forbidden());
            }
        }
        if method.starts_with("automation.")
            && !policy::automation_result_allowed(&grant, method, result)
        {
            return Err(context::forbidden());
        }
        if method == "automation.source.get" {
            let kind = result
                .get("kind")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<tiktools_core::control::AutomationKind>().ok())
                .ok_or_else(context::forbidden)?;
            let id = result
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            let revision = result
                .get("revision")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            if !self
                .core
                .automation_shared_source_current(kind, id, revision, &session.authorization()?)
                .unwrap_or(false)
            {
                return Err(context::forbidden());
            }
        }
        if method == "collaboration.commands.get" {
            if result.get("principalId").and_then(Value::as_str) != Some(session.principal_id()) {
                return Err(context::forbidden());
            }
            let resource = result
                .pointer("/command/resource")
                .ok_or_else(context::forbidden)?;
            let kind = match resource.get("kind").and_then(Value::as_str) {
                Some("workflow")
                    if grant
                        .capabilities
                        .contains(&RemoteCapability::WorkflowsRead) =>
                {
                    tiktools_core::collaboration::ResourceKind::Workflow
                }
                Some("viewer")
                    if grant.capabilities.contains(&RemoteCapability::PointsRead)
                        && !grant
                            .resources
                            .contains_key(&tiktools_core::collaboration::ResourceKind::Live) =>
                {
                    tiktools_core::collaboration::ResourceKind::Viewer
                }
                Some("automation") if policy::automation_read_allowed(&grant) => {
                    tiktools_core::collaboration::ResourceKind::Automation
                }
                Some("competition")
                    if grant
                        .capabilities
                        .contains(&RemoteCapability::CompetitionsRead)
                        && !grant
                            .resources
                            .contains_key(&tiktools_core::collaboration::ResourceKind::Live) =>
                {
                    tiktools_core::collaboration::ResourceKind::Competition
                }
                _ => return Err(context::forbidden()),
            };
            let scope = resource
                .get("scope")
                .and_then(Value::as_str)
                .ok_or_else(context::forbidden)?;
            if !resource
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| grant.allows_scoped_resource(kind, scope, id))
            {
                return Err(context::forbidden());
            }
            if kind == tiktools_core::collaboration::ResourceKind::Automation {
                if tiktools_core::collaboration::AutomationFamily::from_scope(scope).is_none() {
                    return Err(context::forbidden());
                }
                // A retained outcome must be just as narrow as the live
                // command response. Expired-outcome errors carry target only.
                if let Some(method) = result.get("method").and_then(Value::as_str) {
                    if !matches!(
                        method,
                        "automation.disable"
                            | "automation.delete"
                            | "automation.create"
                            | "automation.update"
                            | "automation.enable"
                    ) || (method == "automation.delete" && scope != "event")
                        || !result.get("result").is_some_and(|v| {
                            policy::automation_result_allowed(&grant, method, v)
                                && v.get("id") == resource.get("id")
                                && (!matches!(
                                    method,
                                    "automation.create" | "automation.update" | "automation.enable"
                                ) || v.get("kind").and_then(Value::as_str) == Some(scope))
                                && (method != "automation.disable"
                                    || if scope == "action" {
                                        v.get("kind").and_then(Value::as_str) == Some("action")
                                    } else {
                                        scope == "event" && v.get("kind").is_none()
                                    })
                        })
                        || !result
                            .get("resourceVersions")
                            .and_then(Value::as_array)
                            .is_some_and(|rows| {
                                rows.len() == 1
                                    && rows.iter().all(|v| {
                                        v.get("kind") == resource.get("kind")
                                            && v.get("scope") == resource.get("scope")
                                            && v.get("id") == resource.get("id")
                                    })
                            })
                    {
                        return Err(context::forbidden());
                    }
                }
            }
            if kind == tiktools_core::collaboration::ResourceKind::Competition {
                if let Some(method) = result.get("method").and_then(Value::as_str) {
                    if !result.get("result").is_some_and(|value| {
                        policy::competition_stop_result_allowed(&grant, method, value)
                            && value.get("id") == resource.get("id")
                    }) || !result
                        .get("resourceVersions")
                        .and_then(Value::as_array)
                        .is_some_and(|rows| {
                            rows.len() == 1
                                && rows[0].get("kind") == resource.get("kind")
                                && rows[0].get("scope") == resource.get("scope")
                                && rows[0].get("id") == resource.get("id")
                        })
                    {
                        return Err(context::forbidden());
                    }
                }
            }
        }
        if matches!(method, "live.status" | "live.monitor.status") {
            if let Some(ids) = grant
                .resources
                .get(&tiktools_core::collaboration::ResourceKind::Live)
            {
                if !result
                    .get("uniqueId")
                    .and_then(Value::as_str)
                    .is_some_and(|id| ids.contains(id))
                {
                    return Err(context::forbidden());
                }
                if method == "live.status" {
                    let verified = result
                        .get("uniqueId")
                        .and_then(Value::as_str)
                        .zip(result.get("roomId").and_then(Value::as_str))
                        .zip(result.get("connectionId").and_then(Value::as_str))
                        .is_some_and(|((creator, room), connection)| {
                            self.core
                                .live_status_matches_verified_scope(creator, room, connection)
                        });
                    if !verified {
                        return Err(context::forbidden());
                    }
                }
            }
        }
        Ok(())
    }

    /// Resource-bound command failures must obey the same egress policy as
    /// successful outcomes, including while queued behind other socket work.
    pub fn authorize_remote_failure(
        &self,
        session: &RemoteSession,
        method: &str,
        data: Option<&Value>,
    ) -> Result<(), ApiError> {
        self.authorize_remote(session, method)?;
        if matches!(
            method,
            "workflows.get"
                | "workflows.list"
                | "workflows.disable"
                | "workflows.delete"
                | "points.viewer.get"
                | "points.leaderboard"
                | "points.view"
                | "points.adjust"
                | "competitions.start"
                | "competitions.resume"
                | "competitions.pause"
                | "competitions.cancel"
                | "automation.list"
                | "automation.get"
                | "automation.source.get"
                | "automation.disable"
                | "automation.enable"
                | "automation.delete"
                | "automation.create"
                | "automation.update"
        ) {
            if let Some(versions) = data
                .and_then(|data| data.get("resourceVersions"))
                .and_then(Value::as_array)
            {
                let grant = session.grant()?;
                let (kind, wire) = if method.starts_with("automation.") {
                    (
                        tiktools_core::collaboration::ResourceKind::Automation,
                        "automation",
                    )
                } else if method.starts_with("competitions.") {
                    (
                        tiktools_core::collaboration::ResourceKind::Competition,
                        "competition",
                    )
                } else if method.starts_with("points.") {
                    (tiktools_core::collaboration::ResourceKind::Viewer, "viewer")
                } else {
                    (
                        tiktools_core::collaboration::ResourceKind::Workflow,
                        "workflow",
                    )
                };
                if versions.iter().any(|v| {
                    v.get("kind").and_then(Value::as_str) != Some(wire)
                        || !v.get("id").and_then(Value::as_str).is_some_and(|id| {
                            v.get("scope").and_then(Value::as_str).is_some_and(|scope| {
                                grant.allows_scoped_resource(kind, scope, id)
                                    && (method != "automation.delete" || scope == "event")
                            })
                        })
                }) {
                    return Err(context::forbidden());
                }
            }
        }
        if let Some(data) = data.filter(|data| {
            method == "collaboration.commands.get" && data.pointer("/command/resource").is_some()
        }) {
            return self.authorize_remote_response(session, method, data);
        }
        Ok(())
    }

    /// Executes one validated request.
    pub async fn execute(&self, request: RpcRequest) -> RpcResponse {
        self.execute_with_context(&RequestContext::LocalOwner, request)
            .await
    }

    /// Remote gateways MUST use this path with a transport-authenticated
    /// session. Identity and grants cannot come from request params.
    pub async fn execute_with_context(
        &self,
        context: &RequestContext,
        request: RpcRequest,
    ) -> RpcResponse {
        let mut response = self.execute_authorized(context, request).await;
        if matches!(context, RequestContext::Remote(_)) {
            if let Some(error) = &mut response.error {
                policy::redact_remote_failure(error);
            }
        }
        response
    }

    async fn execute_authorized(
        &self,
        context: &RequestContext,
        request: RpcRequest,
    ) -> RpcResponse {
        if let RequestContext::Remote(session) = context {
            let authorization = self.authorize_remote(session, &request.method);
            if let Err(error) = authorization {
                return RpcResponse::error(request.id, error);
            }
        }
        if let Some(version) = request.jsonrpc.as_deref() {
            if version != "2.0" {
                return RpcResponse::error(
                    request.id,
                    ApiError::invalid_params("jsonrpc must be \"2.0\""),
                );
            }
        }
        let Some(entry) = self.router.get(&request.method) else {
            return RpcResponse::error(request.id, ApiError::method_not_found(&request.method));
        };
        if let Some(meta) = &request.meta {
            // Never acknowledge command metadata that a method cannot enforce.
            if (meta.command_id.is_some() || meta.host_instance_id.is_some())
                && !entry.meta.durable_commands
            {
                return RpcResponse::error(
                    request.id,
                    ApiError::new(
                        "unsupported_remote",
                        "this method does not support durable commands",
                    ),
                );
            }
            if (meta.command_id.is_some() || meta.host_instance_id.is_some())
                && (meta.command_id.is_none()
                    || meta.host_instance_id.is_none()
                    || (entry.meta.concurrency
                        == Some(registry::ConcurrencyStrategy::CompareAndSet)
                        && meta.expected_revision.is_none()))
            {
                return RpcResponse::error(
                    request.id,
                    ApiError::invalid_params(
                        "commandId and hostInstanceId are required; compare-and-set also requires expectedRevision",
                    ),
                );
            }
            if meta.expected_revision.is_some()
                && entry.meta.concurrency != Some(registry::ConcurrencyStrategy::CompareAndSet)
            {
                return RpcResponse::error(
                    request.id,
                    ApiError::invalid_params("this method does not accept expectedRevision"),
                );
            }
        }
        let params_limit = if matches!(context, RequestContext::LocalOwner) {
            owner_params_bytes(&request.method)
        } else {
            MAX_PARAMS_BYTES
        };
        if request.params_size() > params_limit {
            return RpcResponse::error(request.id, ApiError::too_large());
        }
        let params = request.params_object();
        if let RequestContext::Remote(session) = context {
            if request.method == "rpc.discover" {
                if let Err(error) = serde_json::from_value::<modules::Empty>(params) {
                    return RpcResponse::error(
                        request.id,
                        ApiError::invalid_params(error.to_string()),
                    );
                }
                let methods: Vec<_> = self
                    .router
                    .metadata()
                    .into_iter()
                    .filter(|meta| {
                        self.router
                            .get(&meta.name)
                            .is_some_and(|_| self.authorize_remote(session, &meta.name).is_ok())
                    })
                    .collect();
                return match session.authorize(None) {
                    Ok(()) => {
                        RpcResponse::success(request.id, serde_json::json!({"methods": methods}))
                    }
                    Err(error) => RpcResponse::error(request.id, error),
                };
            }
            if request.method == "rpc.schema" {
                let params = match serde_json::from_value::<modules::rpc::SchemaParams>(params) {
                    Ok(params) => params,
                    Err(error) => {
                        return RpcResponse::error(
                            request.id,
                            ApiError::invalid_params(error.to_string()),
                        )
                    }
                };
                let Some(entry) = self.router.get(&params.method) else {
                    return RpcResponse::error(request.id, context::forbidden());
                };
                if let Err(error) = self.authorize_remote(session, &params.method) {
                    return RpcResponse::error(request.id, error);
                }
                return match serde_json::to_value(&entry.meta) {
                    Ok(meta) => RpcResponse::success(request.id, meta),
                    Err(_) => RpcResponse::error(
                        request.id,
                        ApiError::internal("schema serialization failed"),
                    ),
                };
            }
        }
        let outcome = tokio::time::timeout(
            request_timeout_for(&request.method),
            (entry.handler)(self.core.clone(), context.clone(), params, request.meta),
        )
        .await;
        // Do not disclose a result after an in-flight revocation/expiry.
        if let RequestContext::Remote(session) = context {
            let authorization = match &outcome {
                Ok(Ok(result)) => {
                    self.authorize_remote_response(session, &request.method, &result.value)
                }
                Ok(Err(error)) => {
                    self.authorize_remote_failure(session, &request.method, error.data.as_ref())
                }
                Err(_) => self.authorize_remote(session, &request.method),
            };
            if let Err(error) = authorization {
                return RpcResponse::error(request.id, error);
            }
        }
        match outcome {
            Ok(Ok(result)) => {
                let mut response = RpcResponse::success(
                    request.id,
                    if matches!(context, RequestContext::Remote(_)) {
                        policy::redact_remote_result(&request.method, result.value)
                    } else {
                        result.value
                    },
                );
                response.meta =
                    result
                        .resource_versions
                        .map(|resource_versions| ResponseMetadata {
                            resource_versions,
                            command: result.command,
                            host_instance_id: result.host_instance_id,
                        });
                response
            }
            Ok(Err(error)) => RpcResponse::error(request.id, error),
            Err(_) => RpcResponse::error(request.id, ApiError::timeout()),
        }
    }

    /// Parses one raw JSON value (a stdio/IPC line) and executes it.
    pub async fn execute_value(&self, raw: &Value) -> RpcResponse {
        self.execute_value_with_context(&RequestContext::LocalOwner, raw)
            .await
    }

    pub async fn execute_value_with_context(
        &self,
        context: &RequestContext,
        raw: &Value,
    ) -> RpcResponse {
        let id = RpcId::extract(raw);
        let Some(object) = raw.as_object() else {
            return RpcResponse::error(id, ApiError::invalid_params("request must be an object"));
        };
        if let Some(version) = object.get("jsonrpc").and_then(Value::as_str) {
            if version != "2.0" {
                return RpcResponse::error(id, ApiError::invalid_params("jsonrpc must be \"2.0\""));
            }
        }
        let method = object
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if method.is_empty() || method.len() > 128 {
            return RpcResponse::error(
                id,
                ApiError::invalid_params("method must be 1..=128 characters"),
            );
        }
        // Preserve policy-before-validation for hidden methods, including a
        // malformed metadata envelope supplied by a remote caller.
        if let RequestContext::Remote(session) = context {
            if let Err(error) = self.authorize_remote(session, method) {
                return RpcResponse::error(id, error);
            }
        }
        let meta = match object.get("meta").filter(|value| !value.is_null()) {
            Some(value) => match serde_json::from_value::<RequestMetadata>(value.clone()) {
                Ok(meta) => Some(meta),
                Err(_) => {
                    return RpcResponse::error(
                        id,
                        ApiError::invalid_params("invalid request metadata"),
                    )
                }
            },
            None => None,
        };
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        self.execute_with_context(
            context,
            RpcRequest {
                jsonrpc: None,
                id,
                method: method.to_owned(),
                params,
                meta,
            },
        )
        .await
    }

    /// Subscribes to the domain event bus (reliable + lossy lanes,
    /// reliable drained first). Each event maps to one JSON-RPC
    /// notification via [`event_notification`].
    pub fn subscribe(&self) -> tiktools_core::events::DomainSubscription {
        self.core.events.subscribe_domain()
    }

    /// Subscribes to the domain event bus through the shared
    /// [`ControlEvent`] broadcast shape, so direct (in-process) and
    /// connected (IPC) subscribers consume one stream type.
    pub fn subscribe_broadcast(&self) -> tokio::sync::broadcast::Receiver<ControlEvent> {
        crate::client::bridge_subscription(self.subscribe())
    }
}

/// Maps one domain event onto the JSON-RPC notification envelope:
/// `{ jsonrpc, method: "event", params: { topic, data } }`.
pub fn event_notification(event: &tiktools_core::events::DomainEvent) -> Value {
    // The notification params carry the event's own `{topic, data}` serde
    // form (unit variants serialize without a `data` key); only a
    // conversion failure substitutes the explicit error marker below.
    match serde_json::to_value(event) {
        Ok(params) => serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": params,
        }),
        Err(error) => {
            tracing::error!(topic = event.topic(), %error, "domain event failed envelope conversion");
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "event",
                "params": {
                    "topic": event.topic(),
                    "data": { "error": "event_serialization_failed" },
                },
            })
        }
    }
}

/// Maps a reliable-lane lag onto the JSON-RPC gap notification envelope:
/// `{ jsonrpc, method: "event.gap", params: { lost, resync: true } }`.
/// `lost` counts the skipped authoritative events. The stream is no
/// longer complete: the client must refresh authoritative state instead
/// of reconstructing the missing events.
pub fn gap_notification(lost: u64) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event.gap",
        "params": {
            "lost": lost,
            "resync": true,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_login_deadlines_leave_time_for_callback_and_server_response() {
        assert_eq!(
            request_timeout_for("cloud.login.complete"),
            Duration::from_secs(240)
        );
        assert_eq!(
            client_timeout_for("cloud.login.complete"),
            Duration::from_secs(270)
        );
        for other in [
            "cloud.login.begin",
            "cloud.login.cancel",
            "cloud.login.complete.extra",
            "",
        ] {
            assert_eq!(request_timeout_for(other), REQUEST_TIMEOUT);
            assert_eq!(client_timeout_for(other), REQUEST_TIMEOUT);
        }
    }

    #[tokio::test]
    async fn remote_storage_failures_omit_owner_diagnostics_but_local_failures_keep_them() {
        struct Emitter;
        impl tiktools_core::HostEmitter for Emitter {
            fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
        }
        let root = std::env::temp_dir().join(format!(
            "tt-private-error-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = tiktools_core::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(tiktools_core::NoopMediaHost),
            paths,
        ));
        let mut router = ControlRouter::new();
        router.register_typed::<modules::Empty, Value, _, _>(
            "test.failure",
            "fixture",
            false,
            |_, _| async {
                let mut error =
                    ApiError::internal("SQL trigger failed at /private/owner.db: secret-marker");
                error.data = Some(serde_json::json!({"diagnostic":"secret-marker"}));
                Err(error)
            },
        );
        router.set_remote_policy("test.failure", policy::RemotePolicy::Authenticated);
        let api = ControlApi {
            core: core.clone(),
            router,
        };
        let session = RemoteSession::new("remote", "session", [], Duration::from_secs(60)).unwrap();
        let call = || RpcRequest::new(RpcId::Number(1), "test.failure", serde_json::json!({}));
        let remote = api
            .execute_with_context(&RequestContext::Remote(session), call())
            .await
            .error
            .unwrap();
        assert_eq!(remote.code, "internal");
        assert!(remote.data.is_none());
        assert!(!remote.message.contains("secret-marker"));
        assert!(!remote.message.contains("/private"));
        let local = api.execute(call()).await.error.unwrap();
        assert!(local.message.contains("secret-marker"));
        assert!(local.data.is_some());
        core.shutdown().await;
        drop(api);
        drop(core);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plugin_and_automation_topics_map_to_event_notifications() {
        // The stdio/IPC/WebView transports all send `event_notification`
        // verbatim, so this envelope is what an IPC subscriber receives.
        for event in [
            tiktools_core::events::DomainEvent::PluginEvent {
                plugin_id: "hotkeys".to_owned(),
                event_type: "hotkey.pressed".to_owned(),
                event: serde_json::json!({"type": "hotkey.pressed"}),
            },
            tiktools_core::events::DomainEvent::PluginStatus {
                plugin_id: "hotkeys".to_owned(),
                status: serde_json::json!({}),
            },
            tiktools_core::events::DomainEvent::AutomationRunsChanged { runs: vec![] },
            tiktools_core::events::DomainEvent::AutomationRunCompleted {
                run: serde_json::json!({}),
            },
        ] {
            let notification = event_notification(&event);
            assert_eq!(notification["method"], "event");
            assert_eq!(notification["params"]["topic"], event.topic());
        }
        let notification = event_notification(&tiktools_core::events::DomainEvent::PluginEvent {
            plugin_id: "hotkeys".to_owned(),
            event_type: "hotkey.pressed".to_owned(),
            event: serde_json::json!({"type": "hotkey.pressed"}),
        });
        assert_eq!(notification["params"]["data"]["pluginId"], "hotkeys");
        assert_eq!(
            notification["params"]["data"]["eventType"],
            "hotkey.pressed"
        );
    }
}
