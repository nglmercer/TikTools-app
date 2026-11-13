//! Cloud teams for remote control (v3): the owner's view of hosts, devices,
//! invitations, join requests, grants and sessions.
//!
//! Enrollment proves possession of a device key. The Ed25519 key is created on
//! this PC, stored owner-only and never leaves it. Invitation tokens are returned
//! once to the owner and are not stored by the host.

mod access;
mod devices;
mod hosts;
mod invitations;
mod peer_authorizations;
mod peer_memberships;
mod peers;
mod rtc;
pub use peer_authorizations::{PeerAuthorization, PeerAuthorizationSnapshot};
pub use peer_memberships::{PeerMembership, PeerMemberships};
pub use rtc::RtcTransportConfig;
mod proofs;
pub use peers::{
    PeerDirectory, PeerEndpointRecord, PeerEndpointSigner, PeerPublication, PeerPublished,
    PeerTransportConfig,
};
mod storage;
mod tickets;
mod transport;

pub use access::{grants, revoke_grant, revoke_session, sessions, update_grant};
pub use devices::{devices, revoke_device};
pub use hosts::{create_host, get_host, hosts, update_host};
pub use invitations::{
    create_invitation, decide_request, invitations, redeem_invitation, requests, revoke_invitation,
};
use proofs::binding_digest;
pub use proofs::{canonical_json, proof_message};
#[cfg(test)]
use transport::map_failure;
pub(crate) use transport::{credentials, send, str_field};

use std::{fs, io, path::PathBuf, time::Duration};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use reqwest::{Method, StatusCode};
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tiktools_cloud_auth::{CloudAuth, NativeToken};

use crate::{
    cloud_http::{self, decode, FailureEnvelope, Limits, NoContent},
    control::OperationError,
    paths::AppPaths,
    private_file::{self, ReplaceError},
};

const PROTOCOL_VERSION: i32 = 1;
const PROOF_DOMAIN: &str = "tiktools.remote.v1";
/// HTTP budget for every teams operation: 15 s, 4 MiB of reply.
const HTTP_LIMITS: Limits = Limits::new(Duration::from_secs(15), 4 * 1024 * 1024);
const MAX_NAME_CHARS: usize = 80;
const MAX_LABEL_CHARS: usize = 120;
const KEY_FILE: &str = "remote-device-ed25519.pk8";
const HOST_BINDING_FILE: &str = "remote-host-binding.json";
const CONTROLLER_FILE: &str = "remote-controller.json";

/// Backend flags are never trusted to identify the local host. The owner-scoped
/// device list and this PC's binding must agree on both host and device.
pub(crate) fn mark_local_host_devices(
    list: &mut TeamDeviceList,
    binding: Option<&(String, String)>,
) {
    for device in &mut list.items {
        device.is_this_device = binding.is_some_and(|(host, id)| {
            device.kind == "host" && device.host_id.as_ref() == Some(host) && device.id == *id
        });
    }
}

// ---- Views (nonsecret) -------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamHost {
    pub id: String,
    pub name: String,
    pub status: String,
    pub protocol_version: i32,
    pub revision: i64,
    #[serde(default)]
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamHostList {
    pub items: Vec<TeamHost>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamDevice {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub host_id: Option<String>,
    pub status: String,
    /// Set only by the local core after matching this PC's persisted host binding.
    #[serde(default)]
    pub is_this_device: bool,
    /// Absent from the enrollment reply; present in the list.
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamDeviceList {
    pub items: Vec<TeamDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamInvitation {
    pub id: String,
    pub requested_role: String,
    pub status: String,
    pub expires_at: String,
    pub redeemed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamInvitationList {
    pub items: Vec<TeamInvitation>,
}

/// The only reply that carries an invitation token. The owner shares it once.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamInvitationCreated {
    pub id: String,
    pub requested_role: String,
    pub status: String,
    pub expires_at: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamRequest {
    pub id: String,
    pub requested_role: String,
    pub status: String,
    pub device_label: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamRequestList {
    pub items: Vec<TeamRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamGrant {
    pub id: String,
    pub device_id: String,
    pub device_label: String,
    pub role: String,
    pub policy: Value,
    pub status: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamGrantList {
    pub items: Vec<TeamGrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamSession {
    pub id: String,
    pub grant_id: String,
    pub device_id: String,
    pub grant_revision: i64,
    pub opened_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamSessionList {
    pub items: Vec<TeamSession>,
}

// ---- Parameters --------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamCreateParams {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamHostParams {
    pub host_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamUpdateParams {
    pub host_id: String,
    pub name: Option<String>,
    pub status: Option<String>,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamEnrollParams {
    /// `controller` for this PC as a viewer, or `host` for this PC as a served team host.
    pub kind: String,
    pub label: String,
    /// Required for `host`; the team must belong to the signed-in account.
    pub host_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamDeviceParams {
    pub device_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamInvitationCreateParams {
    pub host_id: String,
    pub requested_role: String,
    pub ttl_seconds: Option<i64>,
    pub recipient_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamPeopleSearchParams {
    pub query: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamPerson {
    pub id: String,
    pub nickname: String,
    pub ambiguous: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamPeople {
    pub items: Vec<TeamPerson>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamInvitationParams {
    pub host_id: String,
    pub invitation_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamDecisionParams {
    pub host_id: String,
    pub request_id: String,
    /// `approve` or `deny`.
    pub decision: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamGrantUpdateParams {
    pub host_id: String,
    pub grant_id: String,
    pub role: Option<String>,
    pub policy: Option<Value>,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamGrantRevokeParams {
    pub host_id: String,
    pub grant_id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamSessionParams {
    pub host_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamRedeemParams {
    /// The invitation token shown once by the owner.
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamRedeemResult {
    pub request_id: String,
    pub host_id: String,
    pub status: String,
    pub requested_role: String,
}

// ---- Validation --------------------------------------------------------------

pub(crate) fn valid_uuid(value: &str) -> Result<(), OperationError> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| OperationError::invalid("identifier is not valid"))
}

fn bounded_text(value: &str, max: usize, what: &str) -> Result<String, OperationError> {
    let text = value.trim();
    if text.is_empty() || text.chars().count() > max || text.chars().any(char::is_control) {
        return Err(OperationError::invalid(format!(
            "{what} must be 1 to {max} characters without control characters"
        )));
    }
    Ok(text.to_owned())
}

fn one_of(value: &str, allowed: &[&str], what: &str) -> Result<(), OperationError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(OperationError::invalid(format!("{what} is not supported")))
    }
}

const PRESET_ROLES: &[&str] = &["observer", "moderator", "automationEditor", "custom"];

// ---- Device key --------------------------------------------------------------

/// Owns the on-disk Ed25519 device identity. The file is created owner-only.
pub struct CloudTeams {
    key_path: PathBuf,
    binding_path: PathBuf,
    controller_path: PathBuf,
}

impl CloudTeams {
    pub(crate) fn new(paths: &AppPaths) -> Self {
        Self::from_data_directory(&paths.data)
    }
    /// Native controller metadata only; constructs no host runtime.
    pub fn from_data_directory(data: &std::path::Path) -> Self {
        Self {
            key_path: data.join("remote").join(KEY_FILE),
            binding_path: data.join("remote").join(HOST_BINDING_FILE),
            controller_path: data.join("remote").join(CONTROLLER_FILE),
        }
    }
}

#[cfg(test)]
mod tests;
