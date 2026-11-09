//! Local-owner administration. The default remote policy denies this entire
//! surface; network pairing uses the same service through a trusted gateway.
use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use tiktools_core::collaboration::{
    AuditEntry, AuditQuery, Capability, Configuration, ConfigureInput, Grant, GrantSet, IdParams,
    Invitation, InviteCreate, InviteSecret, ListQuery, PairingRequest, Principal, Session, Status,
};
pub use tiktools_core::collaboration::{Authenticated, ClientKind, GatewayStatus};
pub use tiktools_core::collaboration::{CommandGet, CommandOutcome};
pub use tiktools_core::collaboration::{
    ResumeParams, ResumeResult, SnapshotPage, SnapshotParams, StreamDomain, SubscribeParams,
    SubscribeResult,
};
use tiktools_core::AppCore;

use super::{Empty, OkResult};
use crate::{ApiError, ControlRouter};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HelloParams {
    pub protocol_version: u32,
    pub client_version: String,
    pub client_kind: ClientKind,
}

pub use tiktools_core::collaboration::RemoteLimits;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HelloResult {
    pub protocol_version: u32,
    pub host_id: String,
    pub host_instance_id: String,
    pub connection_id: String,
    pub principal_id: String,
    pub session_id: String,
    pub grant_revision: String,
    pub capabilities: std::collections::BTreeSet<Capability>,
    pub resource_restrictions: std::collections::BTreeMap<
        tiktools_core::collaboration::ResourceKind,
        std::collections::BTreeSet<String>,
    >,
    pub server_time: i64,
    pub limits: RemoteLimits,
    pub features: std::collections::BTreeMap<String, bool>,
    #[serde(default)]
    pub subscription_domains: Vec<StreamDomain>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvitationsResult {
    pub invitations: Vec<Invitation>,
    pub next_after: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestsResult {
    pub requests: Vec<PairingRequest>,
    pub next_after: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrincipalsResult {
    pub principals: Vec<Principal>,
    pub next_after: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionsResult {
    pub sessions: Vec<Session>,
    pub next_after: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditResult {
    pub entries: Vec<AuditEntry>,
    pub next_after: i64,
}

impl From<tiktools_core::collaboration::Error> for ApiError {
    fn from(error: tiktools_core::collaboration::Error) -> Self {
        Self::new(error.code(), error.to_string())
    }
}

pub fn register(router: &mut ControlRouter) {
    router.register_contextual::<SubscribeParams, SubscribeResult, _, _>(
        "collaboration.subscribe",
        "Creates one authorized bounded recovery stream for this connection",
        |core, context, params| async move {
            let (auth, connection, limits) = stream_context(context)?;
            super::blocking_task("collaboration.subscribe", move || {
                core.collaboration_subscribe(auth, &connection, params, &limits)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_contextual::<SnapshotParams, SnapshotPage, _, _>(
        "collaboration.snapshot",
        "Captures an immutable authorized baseline or retrieves its next page",
        |core, context, params| async move {
            let (auth, connection, limits) = stream_context(context)?;
            super::blocking_task("collaboration.snapshot", move || {
                core.collaboration_snapshot(auth, &connection, params, limits.frame_bytes)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_contextual::<ResumeParams, ResumeResult, _, _>(
        "collaboration.resume",
        "Resumes retained reliable notifications under the same session and grant",
        |core, context, params| async move {
            let (auth, connection, limits) = stream_context(context)?;
            super::blocking_task("collaboration.resume", move || {
                core.collaboration_resume(auth, &connection, params, &limits)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_contextual::<CommandGet, CommandOutcome, _, _>(
        "collaboration.commands.get",
        "Retrieves an authorized retained command outcome without executing it",
        |core, context, params| async move {
            let caller = match context {
                crate::RequestContext::LocalOwner => {
                    tiktools_core::control::CommandCaller::LocalOwner
                }
                crate::RequestContext::Remote(session) => {
                    tiktools_core::control::CommandCaller::Remote(session.authorization()?)
                }
            };
            super::blocking_task("collaboration.commands.get", move || {
                core.collaboration_command_get(caller, params)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_contextual::<HelloParams, HelloResult, _, _>(
        "collaboration.hello",
        "Negotiates the authenticated remote protocol",
        |core, context, params| async move {
            if params.protocol_version != 1 {
                return Err(ApiError::new(
                    "protocol_incompatible",
                    "remote protocol version 1 is required",
                ));
            }
            if params.client_version.is_empty() || params.client_version.len() > 80 {
                return Err(ApiError::invalid_params("invalid client version"));
            }
            let crate::RequestContext::Remote(session) = context else {
                return Err(ApiError::new(
                    "unsupported_remote",
                    "hello requires an authenticated network connection",
                ));
            };
            let identity = session.authenticated_identity()?;
            let connection_id = session
                .connection_id()
                .ok_or_else(|| {
                    ApiError::new(
                        "unsupported_remote",
                        "network connection identity is unavailable",
                    )
                })?
                .to_owned();
            let limits = session.connection_limits().cloned().ok_or_else(|| {
                ApiError::new(
                    "unsupported_remote",
                    "network connection limits are unavailable",
                )
            })?;
            let (host_id, host_instance_id) =
                super::blocking_task("collaboration.hello", move || {
                    let service = core.collaboration()?;
                    Ok::<_, tiktools_core::collaboration::Error>((
                        service.host_id().to_owned(),
                        service.instance_id().to_owned(),
                    ))
                })
                .await??;
            let reusable_automation = identity.grant.automation_delegation.is_some();
            let shared_automation_source = identity
                .grant
                .automation_delegation
                .as_ref()
                .is_some_and(|p| p.allow_owner_source && p.allow_owner_edits);
            Ok(HelloResult {
                protocol_version: 1,
                host_id,
                host_instance_id,
                connection_id,
                principal_id: identity.principal_id,
                session_id: identity.session_id,
                grant_revision: identity.grant_revision,
                capabilities: identity.grant.capabilities,
                resource_restrictions: identity.grant.resources,
                server_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
                limits,
                features: [
                    ("subscriptions".into(), cfg!(feature = "persistence")),
                    ("mutations".into(), cfg!(feature = "persistence")),
                    ("workflowDisable".into(), cfg!(feature = "persistence")),
                    ("workflowDelete".into(), cfg!(feature = "persistence")),
                    ("pointsAdjust".into(), cfg!(feature = "persistence")),
                    ("pointsProjection".into(), cfg!(feature = "persistence")),
                    ("creatorFeed".into(), true),
                    ("browserFeed".into(), true),
                    ("competitionSummary".into(), cfg!(feature = "persistence")),
                    (
                        "competitionSummaryVersions".into(),
                        cfg!(feature = "persistence"),
                    ),
                    ("automationSummary".into(), cfg!(feature = "persistence")),
                    ("automationDrafts".into(), cfg!(feature = "persistence")),
                    (
                        "automationSharedSource".into(),
                        cfg!(feature = "persistence") && shared_automation_source,
                    ),
                    (
                        "automationReusableActivation".into(),
                        cfg!(feature = "persistence") && reusable_automation,
                    ),
                    (
                        "automationReusableWrite".into(),
                        cfg!(feature = "persistence") && reusable_automation,
                    ),
                    (
                        "automationEventDeactivate".into(),
                        cfg!(feature = "persistence"),
                    ),
                    ("workflowSave".into(), false),
                    ("workflowEnable".into(), false),
                    ("browserApp".into(), false),
                ]
                .into(),
                subscription_domains: if cfg!(feature = "persistence") {
                    vec![
                        StreamDomain::Workflows,
                        StreamDomain::Points,
                        StreamDomain::Automation,
                        StreamDomain::Commands,
                    ]
                } else {
                    Vec::new()
                },
            })
        },
    );
    router.register_contextual::<Empty, Authenticated, _, _>(
        "collaboration.me",
        "Shows only the authenticated caller's effective identity and grants",
        |_, context, _| async move {
            match context {
                crate::RequestContext::Remote(session) => session.authenticated_identity(),
                _ => Err(ApiError::new(
                    "unsupported_remote",
                    "me requires an authenticated network connection",
                )),
            }
        },
    );
    macro_rules! operation {
        ($name:literal,$description:literal,$input:ty,$output:ty,$effect:literal,|$service:ident,$params:ident|$body:expr) => {
            router.register_typed::<$input, $output, _, _>(
                $name,
                $description,
                $effect,
                |core: Arc<AppCore>, $params: $input| async move {
                    super::blocking_task($name, move || {
                        let $service = core.collaboration()?;
                        $body
                    })
                    .await?
                    .map_err(ApiError::from)
                },
            );
        };
    }
    operation!(
        "collaboration.status",
        "Local-owner collaboration configuration and session counts",
        Empty,
        Status,
        false,
        |s, _p| s.status()
    );
    operation!(
        "collaboration.configure",
        "Configures or disables authenticated remote collaboration",
        ConfigureInput,
        Status,
        true,
        |s, p| {
            if p.configuration.enabled && !s.gateway_status()?.available {
                return Err(tiktools_core::collaboration::Error::GatewayUnavailable);
            }
            s.configure_checked(p.configuration, p.expected_revision.as_deref())
        }
    );
    operation!(
        "collaboration.invites.create",
        "Creates a one-use owner-approved invitation; secret returned once",
        InviteCreate,
        InviteSecret,
        true,
        |s, p| s.create_invitation(p)
    );
    operation!(
        "collaboration.invites.list",
        "Returns a redacted invitation page",
        ListQuery,
        InvitationsResult,
        false,
        |s, p| s.invitations(p).map(|page| InvitationsResult {
            invitations: page.items,
            next_after: page.next_after,
        })
    );
    operation!(
        "collaboration.invites.revoke",
        "Revokes an invitation and its pending exchange",
        IdParams,
        OkResult,
        true,
        |s, p| s.revoke_invitation(&p.id).map(|()| OkResult::ok())
    );
    operation!(
        "collaboration.requests.list",
        "Returns devices awaiting owner approval in a bounded page",
        ListQuery,
        RequestsResult,
        false,
        |s, p| s.requests(p).map(|page| RequestsResult {
            requests: page.items,
            next_after: page.next_after,
        })
    );
    operation!(
        "collaboration.requests.approve",
        "Approves a device with its invitation's maximum grant",
        IdParams,
        OkResult,
        true,
        |s, p| s.decide_request(&p.id, true).map(|()| OkResult::ok())
    );
    operation!(
        "collaboration.requests.reject",
        "Rejects a pending device",
        IdParams,
        OkResult,
        true,
        |s, p| s.decide_request(&p.id, false).map(|()| OkResult::ok())
    );
    operation!(
        "collaboration.sessions.list",
        "Returns a redacted independent device session page",
        ListQuery,
        SessionsResult,
        false,
        |s, p| s.sessions(p).map(|page| SessionsResult {
            sessions: page.items,
            next_after: page.next_after,
        })
    );
    operation!(
        "collaboration.sessions.revoke",
        "Revokes only the selected device session",
        IdParams,
        OkResult,
        true,
        |s, p| s
            .revoke_session(&p.id, "local-owner")
            .map(|()| OkResult::ok())
    );
    operation!(
        "collaboration.principals.list",
        "Returns approved identities and grants in a bounded page",
        ListQuery,
        PrincipalsResult,
        false,
        |s, p| s.principals(p).map(|page| PrincipalsResult {
            principals: page.items,
            next_after: page.next_after,
        })
    );
    operation!(
        "collaboration.principals.get",
        "Returns one owner-visible identity and its current permission revision",
        IdParams,
        Principal,
        false,
        |s, p| s.principal(&p.id)
    );
    operation!(
        "collaboration.principals.revoke",
        "Revokes an identity and all its sessions",
        IdParams,
        OkResult,
        true,
        |s, p| s.revoke_principal(&p.id).map(|()| OkResult::ok())
    );
    operation!(
        "collaboration.grants.set",
        "Updates an identity grant with an atomic revision precondition",
        GrantSet,
        OkResult,
        true,
        |s, p| s.set_grant(p).map(|()| OkResult::ok())
    );
    operation!(
        "collaboration.audit.list",
        "Returns a bounded redacted collaboration audit page",
        AuditQuery,
        AuditResult,
        false,
        |s, p| {
            let after = p.after;
            s.audit(p).map(|entries| AuditResult {
                next_after: entries.last().map(|entry| entry.cursor).unwrap_or(after),
                entries,
            })
        }
    );
}

fn stream_context(
    context: crate::RequestContext,
) -> Result<
    (
        tiktools_core::collaboration::Authorization,
        String,
        RemoteLimits,
    ),
    ApiError,
> {
    let crate::RequestContext::Remote(session) = context else {
        return Err(ApiError::new(
            "unsupported_remote",
            "an authenticated network connection is required",
        ));
    };
    let connection = session
        .connection_id()
        .ok_or_else(|| {
            ApiError::new(
                "unsupported_remote",
                "network connection identity is required",
            )
        })?
        .to_owned();
    let limits = session.connection_limits().cloned().ok_or_else(|| {
        ApiError::new(
            "unsupported_remote",
            "network connection limits are required",
        )
    })?;
    Ok((session.authorization()?, connection, limits))
}
