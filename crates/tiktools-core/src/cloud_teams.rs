//! Cloud teams for remote control (v3): the owner's view of hosts, devices,
//! invitations, join requests, grants and sessions.
//!
//! Enrollment proves possession of a device key. The Ed25519 key is created on
//! this PC, stored owner-only and never leaves it. Invitation tokens are returned
//! once to the owner and are not stored by the host.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use reqwest::{header::AUTHORIZATION, Client, Method, StatusCode};
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tiktools_cloud_auth::{CloudAuth, NativeToken};

use crate::{control::OperationError, paths::AppPaths};

const PROTOCOL_VERSION: i32 = 1;
const PROOF_DOMAIN: &str = "tiktools.remote.v1";
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_NAME_CHARS: usize = 80;
const MAX_LABEL_CHARS: usize = 120;
const KEY_FILE: &str = "remote-device-ed25519.pk8";
const HOST_BINDING_FILE: &str = "remote-host-binding.json";

pub(crate) fn http() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
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

// ---- Canonical JSON and proofs ----------------------------------------------

/// Same canonical form the server digests: sorted keys, no insignificant whitespace.
pub fn canonical_json(value: &Value) -> String {
    fn write(value: &Value, out: &mut String) {
        match value {
            Value::Object(map) => {
                out.push('{');
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for (index, key) in keys.into_iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&serde_json::to_string(key).unwrap_or_default());
                    out.push(':');
                    write(&map[key], out);
                }
                out.push('}');
            }
            Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write(item, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }
    let mut out = String::new();
    write(value, &mut out);
    out
}

fn binding_digest(binding: &Value) -> String {
    Sha256::digest(canonical_json(binding).as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Message signed for a challenge. Mirrors the server's `proof_message`.
pub fn proof_message(purpose: &str, nonce: &[u8], binding_sha256: &str) -> Vec<u8> {
    format!(
        "{PROOF_DOMAIN}\n{purpose}\n{}\n{binding_sha256}",
        URL_SAFE_NO_PAD.encode(nonce)
    )
    .into_bytes()
}

// ---- Device key --------------------------------------------------------------

/// Owns the on-disk Ed25519 device identity. The file is created owner-only.
pub struct CloudTeams {
    key_path: PathBuf,
    binding_path: PathBuf,
}

impl CloudTeams {
    pub(crate) fn new(paths: &AppPaths) -> Self {
        Self {
            key_path: paths.data.join("remote").join(KEY_FILE),
            binding_path: paths.data.join("remote").join(HOST_BINDING_FILE),
        }
    }

    fn device_key(&self) -> Result<Ed25519KeyPair, OperationError> {
        let pkcs8 = match fs::read(&self.key_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let generated = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
                    .map_err(|_| OperationError::internal("could not create a device key"))?;
                write_owner_only(&self.key_path, generated.as_ref())?;
                generated.as_ref().to_vec()
            }
            Err(_) => return Err(OperationError::internal("device key is unreadable")),
        };
        Ed25519KeyPair::from_pkcs8(&pkcs8)
            .map_err(|_| OperationError::internal("device key is invalid"))
    }

    pub async fn enroll(
        &self,
        auth: &CloudAuth,
        params: TeamEnrollParams,
    ) -> Result<TeamDevice, OperationError> {
        one_of(&params.kind, &["controller", "host"], "device kind")?;
        let label = bounded_text(&params.label, MAX_LABEL_CHARS, "label")?;
        let key = self.device_key()?;
        let public_key = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
        let mut binding = json!({"kind": params.kind, "label": label, "publicKey": public_key});
        if params.kind == "host" {
            let host = params
                .host_id
                .as_deref()
                .ok_or_else(|| OperationError::invalid("a host device needs a team"))?;
            valid_uuid(host)?;
            binding["hostId"] = Value::String(host.to_owned());
        } else if params.host_id.is_some() {
            return Err(OperationError::invalid(
                "a controller device does not take a team",
            ));
        }

        let (origin, token) = credentials(auth).await?;
        let challenge = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/challenges",
            Some(json!({"purpose": "enroll", "binding": binding})),
        )
        .await?;
        let challenge_id = str_field(&challenge, "id")?;
        let nonce = URL_SAFE_NO_PAD
            .decode(str_field(&challenge, "nonce")?)
            .map_err(|_| OperationError::internal("cloud sent an unexpected challenge"))?;
        let digest = str_field(&challenge, "bindingSha256")?;
        if digest != binding_digest(&binding) {
            return Err(OperationError::internal(
                "cloud changed the enrollment binding",
            ));
        }
        let signature = key.sign(&proof_message("enroll", &nonce, &digest));
        let device = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/devices",
            Some(json!({
                "challengeId": challenge_id,
                "binding": binding,
                "signature": URL_SAFE_NO_PAD.encode(signature.as_ref()),
            })),
        )
        .await?;
        let device: TeamDevice = decode(device)?;
        if params.kind == "host" {
            let host = params.host_id.clone().unwrap_or_default();
            self.save_host_binding(&host, &device.id)?;
        }
        Ok(device)
    }

    /// The team and device this PC serves as, recorded when the host device was enrolled.
    pub fn host_binding(&self) -> Result<Option<(String, String)>, OperationError> {
        let bytes = match fs::read(&self.binding_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(OperationError::internal("host binding is unreadable")),
        };
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| OperationError::internal("host binding is invalid"))?;
        let host = str_field(&value, "hostId")?;
        let device = str_field(&value, "deviceId")?;
        valid_uuid(&host)?;
        valid_uuid(&device)?;
        Ok(Some((host, device)))
    }

    fn save_host_binding(&self, host: &str, device: &str) -> Result<(), OperationError> {
        let body = json!({"hostId": host, "deviceId": device}).to_string();
        write_owner_only(&self.binding_path, body.as_bytes())
    }

    /// Obtains a connection ticket for this device by proving possession of its key.
    pub async fn issue_ticket(
        &self,
        auth: &CloudAuth,
        host_id: &str,
        device_id: &str,
    ) -> Result<String, OperationError> {
        valid_uuid(host_id)?;
        valid_uuid(device_id)?;
        let binding = json!({"deviceId": device_id, "hostId": host_id, "role": "host"});
        let key = self.device_key()?;
        let (origin, token) = credentials(auth).await?;
        let challenge = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/challenges",
            Some(json!({"purpose": "ticket", "binding": binding})),
        )
        .await?;
        let challenge_id = str_field(&challenge, "id")?;
        let nonce = URL_SAFE_NO_PAD
            .decode(str_field(&challenge, "nonce")?)
            .map_err(|_| OperationError::internal("cloud sent an unexpected challenge"))?;
        let digest = str_field(&challenge, "bindingSha256")?;
        if digest != binding_digest(&binding) {
            return Err(OperationError::internal("cloud changed the ticket binding"));
        }
        let signature = key.sign(&proof_message("ticket", &nonce, &digest));
        let issued = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/tickets",
            Some(json!({
                "challengeId": challenge_id,
                "binding": binding,
                "signature": URL_SAFE_NO_PAD.encode(signature.as_ref()),
            })),
        )
        .await?;
        let ticket = str_field(&issued, "ticket")?;
        if ticket.len() < 32 {
            return Err(OperationError::internal("cloud sent an unexpected ticket"));
        }
        Ok(ticket)
    }
}

fn write_owner_only(path: &Path, bytes: &[u8]) -> Result<(), OperationError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| OperationError::internal("could not create the device key folder"))?;
    }
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|_| OperationError::internal("could not store the device key"))?;
        file.write_all(bytes)
            .map_err(|_| OperationError::internal("could not store the device key"))
    }
    #[cfg(not(unix))]
    {
        fs::write(path, bytes)
            .map_err(|_| OperationError::internal("could not store the device key"))
    }
}

// ---- HTTP --------------------------------------------------------------------

pub(crate) async fn credentials(auth: &CloudAuth) -> Result<(String, NativeToken), OperationError> {
    let origin = auth
        .last_identity()
        .map(|(origin, _)| origin)
        .filter(|origin| !origin.is_empty())
        .ok_or_else(|| {
            OperationError::unavailable("this needs a signed-in Cloud account; sign in first")
        })?;
    let token = auth
        .native_token()
        .await
        .ok_or_else(|| OperationError::unavailable("this needs a signed-in Cloud account"))?;
    Ok((origin, token))
}

pub(crate) fn map_failure(status: StatusCode, body: &[u8]) -> OperationError {
    let code = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| value.get("code").and_then(Value::as_str).map(str::to_owned));
    match code.as_deref() {
        Some("notFound") => OperationError::not_found("team item not found"),
        Some("conflict") => OperationError::conflict("the item changed; reload and try again"),
        Some("expired") => {
            OperationError::invalid("the challenge or invitation expired; try again")
        }
        Some("invalidRequest") => OperationError::invalid("the Cloud rejected the request"),
        Some("unauthorized" | "forbidden") => {
            OperationError::forbidden("sign in to Cloud again to manage teams")
        }
        Some("offline") => OperationError::unavailable("the host is offline"),
        Some("rateLimited") => OperationError::unavailable("too many requests; try again later"),
        Some("misconfigured" | "unsupported") => {
            OperationError::unavailable("remote control is not available on this Cloud instance")
        }
        _ => match status.as_u16() {
            401 | 403 => OperationError::forbidden("sign in to Cloud again to manage teams"),
            404 => OperationError::not_found("team item not found"),
            409 => OperationError::conflict("the item changed; reload and try again"),
            _ => OperationError::unavailable("team management is unavailable"),
        },
    }
}

pub(crate) async fn send(
    method: Method,
    origin: &str,
    token: &NativeToken,
    path: &str,
    body: Option<Value>,
) -> Result<Value, OperationError> {
    let mut request = http()
        .request(method, format!("{origin}{path}"))
        .header(AUTHORIZATION, format!("Bearer {}", token.expose()));
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .await
        .map_err(|_| OperationError::unavailable("cloud request failed"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|_| OperationError::unavailable("cloud reply could not be read"))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(OperationError::unavailable("cloud reply is too large"));
    }
    if status == StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    if !status.is_success() {
        return Err(map_failure(status, &bytes));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| OperationError::internal("cloud sent an unreadable reply"))
}

pub(crate) fn str_field(value: &Value, key: &str) -> Result<String, OperationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| OperationError::internal("cloud sent an unexpected reply"))
}

fn decode<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, OperationError> {
    serde_json::from_value(value)
        .map_err(|_| OperationError::internal("cloud sent an unexpected reply"))
}

// ---- Operations --------------------------------------------------------------

pub async fn hosts(auth: &CloudAuth) -> Result<TeamHostList, OperationError> {
    let (origin, token) = credentials(auth).await?;
    decode(send(Method::GET, &origin, &token, "/api/v3/remote/hosts", None).await?)
}

pub async fn create_host(
    auth: &CloudAuth,
    params: TeamCreateParams,
) -> Result<TeamHost, OperationError> {
    let name = bounded_text(&params.name, MAX_NAME_CHARS, "team name")?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/hosts",
            Some(json!({"name": name, "protocolVersion": PROTOCOL_VERSION})),
        )
        .await?,
    )
}

pub async fn get_host(
    auth: &CloudAuth,
    params: TeamHostParams,
) -> Result<TeamHost, OperationError> {
    valid_uuid(&params.host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{}", params.host_id),
            None,
        )
        .await?,
    )
}

pub async fn update_host(
    auth: &CloudAuth,
    params: TeamUpdateParams,
) -> Result<TeamHost, OperationError> {
    valid_uuid(&params.host_id)?;
    let mut body = json!({"expectedRevision": params.expected_revision});
    if let Some(name) = &params.name {
        body["name"] = Value::String(bounded_text(name, MAX_NAME_CHARS, "team name")?);
    }
    if let Some(status) = &params.status {
        one_of(status, &["active", "disabled"], "team status")?;
        body["status"] = Value::String(status.clone());
    }
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::PATCH,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{}", params.host_id),
            Some(body),
        )
        .await?,
    )
}

pub async fn devices(auth: &CloudAuth) -> Result<TeamDeviceList, OperationError> {
    let (origin, token) = credentials(auth).await?;
    decode(send(Method::GET, &origin, &token, "/api/v3/remote/devices", None).await?)
}

pub async fn revoke_device(
    auth: &CloudAuth,
    params: TeamDeviceParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.device_id)?;
    let (origin, token) = credentials(auth).await?;
    send(
        Method::DELETE,
        &origin,
        &token,
        &format!("/api/v3/remote/devices/{}", params.device_id),
        None,
    )
    .await
}

pub async fn invitations(
    auth: &CloudAuth,
    host_id: &str,
) -> Result<TeamInvitationList, OperationError> {
    valid_uuid(host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{host_id}/invitations"),
            None,
        )
        .await?,
    )
}

pub async fn create_invitation(
    auth: &CloudAuth,
    params: TeamInvitationCreateParams,
) -> Result<TeamInvitationCreated, OperationError> {
    valid_uuid(&params.host_id)?;
    one_of(&params.requested_role, PRESET_ROLES, "role")?;
    let mut body = json!({"requestedRole": params.requested_role});
    if let Some(ttl) = params.ttl_seconds {
        if !(60..=900).contains(&ttl) {
            return Err(OperationError::invalid(
                "invitations last between 1 and 15 minutes",
            ));
        }
        body["ttlSeconds"] = json!(ttl);
    }
    let (origin, token) = credentials(auth).await?;
    let created: TeamInvitationCreated = decode(
        send(
            Method::POST,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{}/invitations", params.host_id),
            Some(body),
        )
        .await?,
    )?;
    if created.token.len() < 32 {
        return Err(OperationError::internal(
            "cloud sent an unexpected invitation",
        ));
    }
    Ok(created)
}

pub async fn revoke_invitation(
    auth: &CloudAuth,
    params: TeamInvitationParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.invitation_id)?;
    let (origin, token) = credentials(auth).await?;
    send(
        Method::POST,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/invitations/{}/revoke",
            params.host_id, params.invitation_id
        ),
        None,
    )
    .await
}

pub async fn requests(auth: &CloudAuth, host_id: &str) -> Result<TeamRequestList, OperationError> {
    valid_uuid(host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{host_id}/requests"),
            None,
        )
        .await?,
    )
}

pub async fn decide_request(
    auth: &CloudAuth,
    params: TeamDecisionParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.request_id)?;
    one_of(&params.decision, &["approve", "deny"], "decision")?;
    let mut body = json!({"decision": params.decision});
    if let Some(role) = &params.role {
        one_of(role, PRESET_ROLES, "role")?;
        body["role"] = Value::String(role.clone());
    }
    let (origin, token) = credentials(auth).await?;
    send(
        Method::POST,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/requests/{}/decision",
            params.host_id, params.request_id
        ),
        Some(body),
    )
    .await
}

pub async fn grants(auth: &CloudAuth, host_id: &str) -> Result<TeamGrantList, OperationError> {
    valid_uuid(host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{host_id}/grants"),
            None,
        )
        .await?,
    )
}

pub async fn update_grant(
    auth: &CloudAuth,
    params: TeamGrantUpdateParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.grant_id)?;
    let mut body = json!({"expectedRevision": params.expected_revision});
    if let Some(role) = &params.role {
        one_of(role, PRESET_ROLES, "role")?;
        body["role"] = Value::String(role.clone());
    }
    if let Some(policy) = params.policy {
        body["policy"] = policy;
    }
    let (origin, token) = credentials(auth).await?;
    send(
        Method::PATCH,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/grants/{}",
            params.host_id, params.grant_id
        ),
        Some(body),
    )
    .await
}

pub async fn revoke_grant(
    auth: &CloudAuth,
    params: TeamGrantRevokeParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.grant_id)?;
    let (origin, token) = credentials(auth).await?;
    send(
        Method::POST,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/grants/{}/revoke",
            params.host_id, params.grant_id
        ),
        Some(json!({"expectedRevision": params.expected_revision})),
    )
    .await
}

pub async fn sessions(auth: &CloudAuth, host_id: &str) -> Result<TeamSessionList, OperationError> {
    valid_uuid(host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{host_id}/sessions"),
            None,
        )
        .await?,
    )
}

pub async fn revoke_session(
    auth: &CloudAuth,
    params: TeamSessionParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.session_id)?;
    let (origin, token) = credentials(auth).await?;
    send(
        Method::POST,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/sessions/{}/revoke",
            params.host_id, params.session_id
        ),
        None,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{UnparsedPublicKey, ED25519};

    #[test]
    fn canonical_json_sorts_keys_recursively() {
        let value = json!({"b": 1, "a": {"d": [true, null], "c": "x"}});
        assert_eq!(
            canonical_json(&value),
            r#"{"a":{"c":"x","d":[true,null]},"b":1}"#
        );
    }

    #[test]
    fn proofs_verify_only_for_the_exact_challenge() {
        let rng = SystemRandom::new();
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let key = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let nonce = [7u8; 32];
        let message = proof_message("enroll", &nonce, "abc");
        let signature = key.sign(&message);
        let public = UnparsedPublicKey::new(&ED25519, key.public_key().as_ref());
        assert!(public.verify(&message, signature.as_ref()).is_ok());
        let other = proof_message("ticket", &nonce, "abc");
        assert!(public.verify(&other, signature.as_ref()).is_err());
    }

    #[test]
    fn identifiers_roles_and_labels_are_bounded() {
        assert!(valid_uuid("3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c").is_ok());
        assert!(valid_uuid("../hosts").is_err());
        assert!(one_of("admin", PRESET_ROLES, "role").is_err());
        assert!(bounded_text("  Team  ", MAX_NAME_CHARS, "name").is_ok());
        assert!(bounded_text("bad\u{0007}", MAX_NAME_CHARS, "name").is_err());
    }

    #[test]
    fn device_key_is_created_once_and_reused() {
        let dir = std::env::temp_dir().join(format!("teams-key-{}", std::process::id()));
        let paths = AppPaths {
            root: dir.clone(),
            data: dir.join("data"),
            plugins: dir.join("plugins"),
            plugin_data: dir.join("plugin-data"),
            builtin_plugins: dir.join("builtin"),
            development_plugins: None,
            logs: dir.join("logs"),
            temp: dir.join("temp"),
        };
        let teams = CloudTeams::new(&paths);
        let first = teams.device_key().unwrap();
        let second = teams.device_key().unwrap();
        assert_eq!(first.public_key().as_ref(), second.public_key().as_ref());
        let _ = fs::remove_dir_all(&dir);
    }
}
