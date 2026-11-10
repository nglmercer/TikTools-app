//! Cloud teams for remote control (owner view). Only `teams.invitations.create`
//! returns a secret, once; every other reply is nonsecret.

use std::sync::Arc;

use serde_json::Value;
pub use tiktools_core::cloud_remote_host::RemoteHostStatus;
pub use tiktools_core::cloud_teams::{
    TeamCreateParams, TeamDecisionParams, TeamDevice, TeamDeviceList, TeamDeviceParams,
    TeamEnrollParams, TeamGrantList, TeamGrantRevokeParams, TeamGrantUpdateParams, TeamHost,
    TeamHostList, TeamHostParams, TeamInvitationCreateParams, TeamInvitationCreated,
    TeamInvitationList, TeamInvitationParams, TeamRequest, TeamRequestList, TeamSession,
    TeamSessionList, TeamSessionParams, TeamUpdateParams,
};
use tiktools_core::AppCore;

use super::Empty;
use crate::{error::ApiError, router::ControlRouter};

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, RemoteHostStatus, _, _>(
        "teams.host.status",
        "Status of this PC's outbound link as a team host",
        false,
        |core: Arc<AppCore>, _params: Empty| async move { Ok::<_, ApiError>(core.teams_host_status()) },
    );
    router.register_typed::<Empty, RemoteHostStatus, _, _>(
        "teams.host.start",
        "Starts this PC's outbound link for the team it serves; the team must be enrolled first",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.teams_host_start().await.map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, RemoteHostStatus, _, _>(
        "teams.host.stop",
        "Stops this PC's outbound link; pending commands become unknown on the relay",
        true,
        |core: Arc<AppCore>, _params: Empty| async move { Ok::<_, ApiError>(core.teams_host_stop()) },
    );
    router.register_typed::<Empty, TeamHostList, _, _>(
        "teams.hosts.list",
        "Lists the remote-control teams owned by the signed-in account",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.teams_hosts().await.map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamCreateParams, TeamHost, _, _>(
        "teams.hosts.create",
        "Creates a remote-control team owned by the signed-in account",
        true,
        |core: Arc<AppCore>, params: TeamCreateParams| async move {
            core.teams_host_create(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamHostParams, TeamHost, _, _>(
        "teams.hosts.get",
        "Reads one owned team",
        false,
        |core: Arc<AppCore>, params: TeamHostParams| async move {
            core.teams_host_get(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamUpdateParams, TeamHost, _, _>(
        "teams.hosts.update",
        "Renames, enables or disables an owned team; requires the current revision",
        true,
        |core: Arc<AppCore>, params: TeamUpdateParams| async move {
            core.teams_host_update(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<Empty, TeamDeviceList, _, _>(
        "teams.devices.list",
        "Lists the devices enrolled for the signed-in account",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.teams_devices().await.map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamEnrollParams, TeamDevice, _, _>(
        "teams.devices.enroll",
        "Enrolls this PC's device key as a controller or as a team host, after a signed challenge",
        true,
        |core: Arc<AppCore>, params: TeamEnrollParams| async move {
            core.teams_device_enroll(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamDeviceParams, Value, _, _>(
        "teams.devices.revoke",
        "Revokes an enrolled device and everything it authorizes",
        true,
        |core: Arc<AppCore>, params: TeamDeviceParams| async move {
            core.teams_device_revoke(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamHostParams, TeamInvitationList, _, _>(
        "teams.invitations.list",
        "Lists invitations of an owned team; tokens are never listed",
        false,
        |core: Arc<AppCore>, params: TeamHostParams| async move {
            core.teams_invitations(params.host_id)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamInvitationCreateParams, TeamInvitationCreated, _, _>(
        "teams.invitations.create",
        "Creates a single-use invitation; the token is returned once and not stored by the host",
        true,
        |core: Arc<AppCore>, params: TeamInvitationCreateParams| async move {
            core.teams_invitation_create(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamInvitationParams, Value, _, _>(
        "teams.invitations.revoke",
        "Revokes an unspent invitation",
        true,
        |core: Arc<AppCore>, params: TeamInvitationParams| async move {
            core.teams_invitation_revoke(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamHostParams, TeamRequestList, _, _>(
        "teams.requests.list",
        "Lists join requests of an owned team",
        false,
        |core: Arc<AppCore>, params: TeamHostParams| async move {
            core.teams_requests(params.host_id)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamDecisionParams, Value, _, _>(
        "teams.requests.decide",
        "Approves or denies a join request; approval creates a grant awaiting host confirmation",
        true,
        |core: Arc<AppCore>, params: TeamDecisionParams| async move {
            core.teams_request_decide(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamHostParams, TeamGrantList, _, _>(
        "teams.grants.list",
        "Lists grants of an owned team",
        false,
        |core: Arc<AppCore>, params: TeamHostParams| async move {
            core.teams_grants(params.host_id)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamGrantUpdateParams, Value, _, _>(
        "teams.grants.update",
        "Changes a grant's role or policy; the host must confirm the new revision",
        true,
        |core: Arc<AppCore>, params: TeamGrantUpdateParams| async move {
            core.teams_grant_update(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamGrantRevokeParams, Value, _, _>(
        "teams.grants.revoke",
        "Revokes a grant and closes its sessions",
        true,
        |core: Arc<AppCore>, params: TeamGrantRevokeParams| async move {
            core.teams_grant_revoke(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamHostParams, TeamSessionList, _, _>(
        "teams.sessions.list",
        "Lists open controller sessions of an owned team",
        false,
        |core: Arc<AppCore>, params: TeamHostParams| async move {
            core.teams_sessions(params.host_id)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<TeamSessionParams, Value, _, _>(
        "teams.sessions.revoke",
        "Closes one controller session",
        true,
        |core: Arc<AppCore>, params: TeamSessionParams| async move {
            core.teams_session_revoke(params)
                .await
                .map_err(ApiError::from)
        },
    );
}
