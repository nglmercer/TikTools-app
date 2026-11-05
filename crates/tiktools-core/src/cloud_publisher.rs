//! Optional Cloud publisher flow: namespaces, signing keys, artifact uploads
//! and release submissions against the Cloud backend.
//!
//! Everything here is opt-in and requires a signed-in Cloud account; without
//! one every operation fails closed with `unavailable`. The session Bearer [REDACTED]
//! is borrowed transiently from [`CloudAuth`] to authorize same-process HTTPS
//! requests and is never persisted, logged, or sent anywhere except the
//! configured Cloud origin. Private signing keys live in files under the
//! application data directory with owner-only permissions.

use std::path::PathBuf;

use reqwest::{Client, StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_package::{KeyId, Sha256Digest, SigningKey};

use tiktools_cloud_auth::{CloudAuth, NativeToken};

use crate::paths::AppPaths;

const STEP_UP_ENROLL_ACTION: &str = "publisher.key.enroll";
const PROOF_DOMAIN: &str = "tiktools-cloud-publisher-key-v1";
const MAX_ARTIFACT_SIZE: i64 = 128 * 1024 * 1024;

/// A publisher namespace the signed-in account belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherSummary {
    pub id: String,
    pub namespace: String,
    pub status: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherListResult {
    pub publishers: Vec<PublisherSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherCreateParams {
    pub namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherIdParams {
    pub publisher_id: String,
}

/// A signing-key record as the backend reports it. `status` is `pending`
/// until an independent Cloud admin approves the key; pending keys sign
/// nothing the registry trusts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherKeyRecord {
    pub id: String,
    pub key_id: String,
    pub public_key: String,
    pub fingerprint: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherKeysResult {
    pub keys: Vec<PublisherKeyRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherEnrollParams {
    pub publisher_id: String,
    /// Backend account password for the session-bound step-up grant. Held in
    /// process memory only for this call; never stored.
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherEnrollResult {
    pub record: PublisherKeyRecord,
    pub fingerprint: String,
    /// Local private-key file. Never send this file to Cloud.
    pub key_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherUploadParams {
    pub publisher_id: String,
    /// Local `.plugin` artifact. The host reads and uploads it; the path
    /// must exist on the host filesystem.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherUploadResult {
    pub upload_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherSubmitParams {
    pub upload_id: String,
    pub publisher_key_id: String,
    pub plugin_id: String,
    pub version: String,
    #[serde(default = "default_target")]
    pub target: String,
}

fn default_target() -> String {
    "any".to_owned()
}

/// A plugin release as reported by Cloud. Unknown backend fields are
/// ignored so newer servers stay compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherReleaseView {
    pub id: String,
    pub publisher_id: String,
    pub upload_id: String,
    pub publisher_key_id: String,
    pub plugin_id: String,
    pub version: String,
    pub target: String,
    pub status: String,
    #[serde(default)]
    pub yanked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yank_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherReleaseResult {
    pub release: PublisherReleaseView,
}

/// Publication review request as the backend reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherPublicationResult {
    pub job_id: String,
    pub status: String,
    pub processing_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherReleaseParams {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StepUpGrantRequest {
    action: String,
    publisher_id: String,
    password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StepUpGrant {
    grant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyChallengeRequest {
    step_up_grant: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyChallenge {
    challenge_id: String,
    nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyEnrollRequest {
    challenge_id: String,
    key_id: String,
    public_key: String,
    signature: String,
    step_up_grant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadCreateRequest {
    publisher_id: String,
    size: i64,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadCreateResponse {
    id: String,
    url: String,
    headers: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseSubmitRequest {
    upload_id: String,
    publisher_key_id: String,
    plugin_id: String,
    version: String,
    target: String,
}

#[derive(Debug, Clone, Deserialize)]
struct BackendError {
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CloudPublisher {
    http: Client,
    keys_dir: PathBuf,
}

impl CloudPublisher {
    pub(crate) fn new(paths: &AppPaths) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("failed to initialize cloud HTTP client"),
            keys_dir: paths.data.join("publisher-keys"),
        }
    }

    fn key_file(&self, key_id: &str) -> PathBuf {
        self.keys_dir.join(format!("{key_id}.json"))
    }

    /// Accepts a publisher UUID or a namespace and returns the UUID. The CLI
    /// works in human namespaces; the backend keys everything by id.
    async fn resolve_publisher_id(
        &self,
        origin: &str,
        token: &NativeToken,
        input: &str,
    ) -> Result<String, crate::control::OperationError> {
        use crate::control::OperationError;
        let trimmed = input.trim();
        if is_uuid(trimmed) {
            return Ok(trimmed.to_owned());
        }
        let publishers: Vec<PublisherSummary> =
            self.get_json(origin, token, "/api/v1/publishers").await?;
        publishers
            .into_iter()
            .find(|publisher| publisher.namespace == trimmed)
            .map(|publisher| publisher.id)
            .ok_or_else(|| {
                OperationError::not_found(format!("no publisher named `{trimmed}` in your account"))
            })
    }

    async fn credentials(
        &self,
        auth: &CloudAuth,
    ) -> Result<(String, NativeToken), crate::control::OperationError> {
        use crate::control::OperationError;
        let origin = auth
            .last_identity()
            .map(|(origin, _)| origin)
            .filter(|origin| !origin.is_empty())
            .ok_or_else(|| {
                OperationError::unavailable(
                    "cloud publisher needs a configured origin; set TIKTOOLS_CLOUD_ORIGIN and sign in",
                )
            })?;
        let token = auth.native_token().await.ok_or_else(|| {
            OperationError::unavailable("cloud publisher needs a signed-in account")
        })?;
        Ok((origin, token))
    }

    async fn post_json<Req: Serialize, Res: for<'de> Deserialize<'de>>(
        &self,
        origin: &str,
        token: &NativeToken,
        path: &str,
        idempotency_key: Option<&str>,
        body: &Req,
    ) -> Result<Res, crate::control::OperationError> {
        use crate::control::OperationError;
        let url = format!("{origin}{path}");
        let mut request = self
            .http
            .post(url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token.expose()),
            )
            .json(body);
        if let Some(key) = idempotency_key {
            request = request.header("idempotency-key", key);
        }
        let response = request.send().await.map_err(|error| {
            OperationError::unavailable(format!("cloud request failed: {error}"))
        })?;
        self.decode(response).await
    }

    async fn get_json<Res: for<'de> Deserialize<'de>>(
        &self,
        origin: &str,
        token: &NativeToken,
        path: &str,
    ) -> Result<Res, crate::control::OperationError> {
        use crate::control::OperationError;
        let url = format!("{origin}{path}");
        let response = self
            .http
            .get(url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token.expose()),
            )
            .send()
            .await
            .map_err(|error| {
                OperationError::unavailable(format!("cloud request failed: {error}"))
            })?;
        self.decode(response).await
    }

    async fn decode<Res: for<'de> Deserialize<'de>>(
        &self,
        response: reqwest::Response,
    ) -> Result<Res, crate::control::OperationError> {
        use crate::control::OperationError;
        let status = response.status();
        if status.is_success() {
            return response.json().await.map_err(|error| {
                OperationError::internal(format!("cloud sent an unreadable reply: {error}"))
            });
        }
        let message = response
            .json::<BackendError>()
            .await
            .ok()
            .and_then(|body| body.error)
            .unwrap_or_else(|| format!("cloud rejected the request ({status})"));
        Err(match status {
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
                OperationError::invalid(message)
            }
            StatusCode::UNAUTHORIZED => OperationError::unavailable(format!(
                "{message}; the stored session may have expired, sign in again"
            )),
            StatusCode::FORBIDDEN => OperationError::forbidden(message),
            StatusCode::NOT_FOUND => OperationError::not_found(message),
            StatusCode::CONFLICT => OperationError::conflict(message),
            StatusCode::TOO_MANY_REQUESTS => {
                OperationError::unavailable(format!("cloud rate limit: {message}"))
            }
            _ => OperationError::internal(message),
        })
    }

    /// Namespaces the signed-in account belongs to.
    pub(crate) async fn list(
        &self,
        auth: &CloudAuth,
    ) -> Result<PublisherListResult, crate::control::OperationError> {
        let (origin, token) = self.credentials(auth).await?;
        let publishers: Vec<PublisherSummary> =
            self.get_json(&origin, &token, "/api/v1/publishers").await?;
        Ok(PublisherListResult { publishers })
    }

    /// Requests a publisher namespace. Starts `pending`; a Cloud admin
    /// approves it before anything can be published under it.
    pub(crate) async fn create(
        &self,
        auth: &CloudAuth,
        params: PublisherCreateParams,
    ) -> Result<PublisherSummary, crate::control::OperationError> {
        use crate::control::OperationError;
        let namespace = params.namespace.trim().to_owned();
        if !valid_namespace(&namespace) {
            return Err(OperationError::invalid(
                "namespace must be 2-63 lowercase letters, digits or hyphens",
            ));
        }
        let (origin, token) = self.credentials(auth).await?;
        self.post_json(
            &origin,
            &token,
            "/api/v1/publishers",
            None,
            &serde_json::json!({ "namespace": namespace }),
        )
        .await
    }

    /// Signing keys of one publisher (pending and approved).
    pub(crate) async fn keys(
        &self,
        auth: &CloudAuth,
        params: PublisherIdParams,
    ) -> Result<PublisherKeysResult, crate::control::OperationError> {
        let (origin, token) = self.credentials(auth).await?;
        let publisher_id = self
            .resolve_publisher_id(&origin, &token, &params.publisher_id)
            .await?;
        let keys: Vec<PublisherKeyRecord> = self
            .get_json(
                &origin,
                &token,
                &format!("/api/v1/publishers/{publisher_id}/keys"),
            )
            .await?;
        Ok(PublisherKeysResult { keys })
    }

    /// Full key enrollment in one call: step-up grant, challenge, local
    /// key generation, proof signature, registration. Returns the pending
    /// record; an independent Cloud admin approves it before use. The
    /// private key stays in an owner-only file on this host.
    pub(crate) async fn enroll(
        &self,
        auth: &CloudAuth,
        params: PublisherEnrollParams,
    ) -> Result<PublisherEnrollResult, crate::control::OperationError> {
        use crate::control::OperationError;
        let (origin, token) = self.credentials(auth).await?;
        let publishers: Vec<PublisherSummary> =
            self.get_json(&origin, &token, "/api/v1/publishers").await?;
        let publisher = publishers
            .into_iter()
            .find(|p| {
                p.id == params.publisher_id.trim() || p.namespace == params.publisher_id.trim()
            })
            .ok_or_else(|| {
                OperationError::not_found("publisher is not available in this account")
            })?;
        let publisher_id = publisher.id;
        let grant: StepUpGrant = self
            .post_json(
                &origin,
                &token,
                "/api/v1/step-up/password",
                None,
                &StepUpGrantRequest {
                    action: STEP_UP_ENROLL_ACTION.to_owned(),
                    publisher_id: publisher_id.clone(),
                    password: params.password.clone(),
                },
            )
            .await
            .map_err(|error| {
                if error.code == "unavailable" {
                    OperationError::unavailable(format!(
                        "step-up failed; check the backend password ({})",
                        error.message
                    ))
                } else {
                    error
                }
            })?;
        let challenge: KeyChallenge = self
            .post_json(
                &origin,
                &token,
                &format!("/api/v1/publishers/{publisher_id}/keys/challenges"),
                None,
                &KeyChallengeRequest {
                    step_up_grant: grant.grant_id.clone(),
                },
            )
            .await?;
        let key_id = match params.key_id {
            Some(id) if !id.trim().is_empty() => id.trim().to_owned(),
            _ => default_key_id(&params.publisher_id),
        };
        let parsed =
            KeyId::parse(&key_id).map_err(|error| OperationError::invalid(error.to_string()))?;
        // Persist the publisher-bound key before registration. If a network
        // reply is lost, retrying this key id reuses the same private key.
        let path = self.key_file(&key_id);
        let signing =
            load_or_create_signing_key(&path, parsed, &publisher.namespace).map_err(|error| {
                OperationError::internal(format!("could not prepare the private key: {error}"))
            })?;
        let public_key = signing.public_key();
        let raw = public_key
            .raw()
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let proof = proof_message(
            &publisher_id,
            &challenge.challenge_id,
            &challenge.nonce,
            &key_id,
            &base64_encode(&raw),
        );
        let signature = base64_encode(&signing.sign(proof.as_bytes()));
        let record: PublisherKeyRecord = self
            .post_json(
                &origin,
                &token,
                &format!("/api/v1/publishers/{publisher_id}/keys"),
                None,
                &KeyEnrollRequest {
                    challenge_id: challenge.challenge_id,
                    key_id: key_id.clone(),
                    public_key: base64_encode(&raw),
                    signature,
                    step_up_grant: grant.grant_id.clone(),
                },
            )
            .await?;
        let fingerprint = record.fingerprint.clone();
        Ok(PublisherEnrollResult {
            record,
            fingerprint,
            key_file: path.to_string_lossy().into_owned(),
        })
    }

    /// Uploads a local `.plugin` artifact: reserves the upload, PUTs the
    /// bytes to object storage, then completes the session. Returns the
    /// upload id for `releases.submit`.
    pub(crate) async fn upload(
        &self,
        auth: &CloudAuth,
        params: PublisherUploadParams,
    ) -> Result<PublisherUploadResult, crate::control::OperationError> {
        use crate::control::OperationError;
        let (origin, token) = self.credentials(auth).await?;
        let publisher_id = self
            .resolve_publisher_id(&origin, &token, &params.publisher_id)
            .await?;
        use std::io::Read;
        let file = std::fs::File::open(&params.path).map_err(|error| {
            OperationError::not_found(format!("could not read {}: {error}", params.path))
        })?;
        // Bound the read itself, not just the validation after allocation.
        let mut bytes = Vec::new();
        file.take(MAX_ARTIFACT_SIZE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| OperationError::unavailable("could not read artifact"))?;
        if bytes.is_empty() || bytes.len() as i64 > MAX_ARTIFACT_SIZE {
            return Err(OperationError::invalid(
                "artifact must be non-empty and at most 128 MiB",
            ));
        }
        let sha256 = Sha256Digest::of_bytes(&bytes).to_string();
        // Deterministic per distinct upload so a retry reuses the reservation
        // instead of creating a duplicate. Lowercase hex fits the backend's
        // idempotency-key charset.
        let idempotency =
            Sha256Digest::of_bytes(format!("{}:{}:{sha256}", publisher_id, bytes.len()).as_bytes())
                .to_string();
        let created: UploadCreateResponse = self
            .post_json(
                &origin,
                &token,
                "/api/v1/plugins/uploads",
                Some(&idempotency),
                &UploadCreateRequest {
                    publisher_id,
                    size: bytes.len() as i64,
                    sha256,
                },
            )
            .await?;
        let upload_url = safe_upload_url(&created.url)
            .ok_or_else(|| OperationError::invalid("cloud returned an unsafe upload URL"))?;
        let mut put = self.http.put(upload_url);
        for (name, value) in &created.headers {
            put = put.header(name.as_str(), value.as_str());
        }
        let put_status = put
            .body(bytes)
            .send()
            .await
            // reqwest errors can contain the signed URL, which is a bearer capability.
            .map_err(|_| OperationError::unavailable("artifact upload failed"))?
            .status();
        if !put_status.is_success() {
            return Err(OperationError::unavailable(format!(
                "object storage refused the artifact ({put_status})"
            )));
        }
        #[derive(Deserialize)]
        struct CompleteResponse {
            #[allow(dead_code)]
            id: String,
            status: String,
        }
        let completed: CompleteResponse = self
            .post_json(
                &origin,
                &token,
                &format!("/api/v1/plugins/uploads/{}/complete", created.id),
                None,
                &serde_json::json!({}),
            )
            .await?;
        Ok(PublisherUploadResult {
            upload_id: created.id,
            status: completed.status,
        })
    }

    /// Submits an uploaded artifact as a plugin release for review.
    pub(crate) async fn submit(
        &self,
        auth: &CloudAuth,
        params: PublisherSubmitParams,
    ) -> Result<PublisherReleaseResult, crate::control::OperationError> {
        use crate::control::OperationError;
        if params.plugin_id.trim().is_empty() || params.plugin_id.len() > 256 {
            return Err(OperationError::invalid(
                "plugin id must be 1-256 characters",
            ));
        }
        if semver::Version::parse(&params.version).is_err() {
            return Err(OperationError::invalid(
                "version must be semantic (e.g. 1.2.3)",
            ));
        }
        let (origin, token) = self.credentials(auth).await?;
        let release: PublisherReleaseView = self
            .post_json(
                &origin,
                &token,
                "/api/v1/plugins/releases",
                None,
                &ReleaseSubmitRequest {
                    upload_id: params.upload_id,
                    publisher_key_id: params.publisher_key_id,
                    plugin_id: params.plugin_id,
                    version: params.version,
                    target: if params.target.trim().is_empty() {
                        default_target()
                    } else {
                        params.target
                    },
                },
            )
            .await?;
        Ok(PublisherReleaseResult { release })
    }

    /// Fetches one release for status review.
    pub(crate) async fn release(
        &self,
        auth: &CloudAuth,
        params: PublisherReleaseParams,
    ) -> Result<PublisherReleaseResult, crate::control::OperationError> {
        let (origin, token) = self.credentials(auth).await?;
        let release: PublisherReleaseView = self
            .get_json(
                &origin,
                &token,
                &format!("/api/v1/plugins/releases/{}", params.id),
            )
            .await?;
        Ok(PublisherReleaseResult { release })
    }

    /// Requests publication review for a release. Requires a reviewer role;
    /// the backend enforces it and reports promotion progress.
    pub(crate) async fn publish(
        &self,
        auth: &CloudAuth,
        params: PublisherReleaseParams,
    ) -> Result<PublisherPublicationResult, crate::control::OperationError> {
        let (origin, token) = self.credentials(auth).await?;
        let result: PublisherPublicationResult = self
            .post_json(
                &origin,
                &token,
                &format!("/api/v1/plugins/releases/{}/publish", params.id),
                None,
                &serde_json::json!({}),
            )
            .await?;
        Ok(result)
    }
}

fn is_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes[8] == b'-'
        && bytes[13] == b'-'
        && bytes[18] == b'-'
        && bytes[23] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 8 | 13 | 18 | 23) || byte.is_ascii_hexdigit())
}

fn safe_upload_url(raw: &str) -> Option<reqwest::Url> {
    let url = reqwest::Url::parse(raw).ok()?;
    let loopback = url.host_str().is_some_and(|host| {
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    });
    if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url)
}

fn valid_namespace(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=63).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[bytes.len() - 1] != b'-'
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

/// Backend key ids are globally unique; default to the namespace so two
/// publishers rarely collide, truncated to the 64-character limit.
fn default_key_id(publisher_id: &str) -> String {
    let base = format!("{publisher_id}-1");
    if base.len() <= 64 {
        return base;
    }
    format!("{}-1", &publisher_id[..62.min(publisher_id.len())])
}

/// Canonical proof bytes the backend verifies against the challenge.
fn proof_message(
    publisher: &str,
    challenge: &str,
    nonce: &str,
    key_id: &str,
    public_key: &str,
) -> String {
    format!("{PROOF_DOMAIN}\n{publisher}\n{challenge}\n{nonce}\n{key_id}\n{public_key}")
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut block: u32 = 0;
        for (index, byte) in chunk.iter().enumerate() {
            block |= (*byte as u32) << (16 - 8 * index);
        }
        let padding = 3 - chunk.len();
        for index in 0..4 - padding {
            out.push(ALPHABET[((block >> (18 - 6 * index)) & 63) as usize] as char);
        }
        for _ in 0..padding {
            out.push('=');
        }
    }
    out
}

fn load_or_create_signing_key(
    path: &std::path::Path,
    id: KeyId,
    publisher: &str,
) -> Result<SigningKey, String> {
    use std::io::Read;
    let parent = path.parent().ok_or("missing key directory")?;
    std::fs::create_dir_all(parent).map_err(|_| "could not create key directory")?;
    let metadata =
        std::fs::symlink_metadata(parent).map_err(|_| "could not inspect key directory")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("key directory must be a regular directory".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "could not protect key directory")?;
    }
    let load = || -> Result<SigningKey, String> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let file = options
            .open(path)
            .map_err(|_| "could not safely open existing key")?;
        let metadata = file
            .metadata()
            .map_err(|_| "could not inspect existing key")?;
        if !metadata.is_file() || metadata.len() > 16384 {
            return Err("existing key is not a small regular file".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("existing private key must have mode 0600 or stricter".into());
            }
        }
        let mut bytes = Vec::new();
        file.take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| "could not read existing key")?;
        let key = SigningKey::from_json(&bytes).map_err(|_| "existing key is invalid")?;
        if key.key_id() != &id || key.publisher() != Some(publisher) {
            return Err(
                "existing key id or publisher does not match; choose a different key id".into(),
            );
        }
        Ok(key)
    };
    match std::fs::symlink_metadata(path) {
        Ok(_) => return load(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("could not inspect existing key".into()),
    }
    let key =
        SigningKey::generate(id.clone(), Some(publisher.to_owned())).map_err(|e| e.to_string())?;
    match write_owner_only(path, &key.to_json()) {
        Ok(()) => Ok(key),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => load(),
        Err(_) => Err("could not persist private key".into()),
    }
}

fn write_owner_only(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = std::fs::OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW);
        let mut file = options.open(path)?;
        use std::io::Write;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (path, bytes);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "private key ACL verification is not available on this platform",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_url_requires_tls_or_loopback_and_has_no_embedded_credentials() {
        for good in [
            "https://store.example/object?token=secret",
            "http://127.0.0.1:5000/x",
            "http://[::1]:5000/x",
        ] {
            assert!(safe_upload_url(good).is_some());
        }
        for bad in [
            "http://example.com/x",
            "http://localhost/x",
            "https://user:pass@example.com/x",
            "https://example.com/x#frag",
            "file:///tmp/file",
        ] {
            assert!(safe_upload_url(bad).is_none());
        }
    }

    #[cfg(unix)]
    #[test]
    fn enrollment_reuses_publisher_bound_key_without_overwriting() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = std::env::temp_dir().join(format!("publisher-key-test-{}", uuid_for_test()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("keys").join("key.json");
        let first =
            load_or_create_signing_key(&path, KeyId::parse("key-1").unwrap(), "tiktools").unwrap();
        let before = std::fs::read(&path).unwrap();
        let second =
            load_or_create_signing_key(&path, KeyId::parse("key-1").unwrap(), "tiktools").unwrap();
        assert_eq!(first.public_key(), second.public_key());
        assert_eq!(second.publisher(), Some("tiktools"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(
            load_or_create_signing_key(&path, KeyId::parse("key-1").unwrap(), "other").is_err()
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            load_or_create_signing_key(&path, KeyId::parse("key-1").unwrap(), "tiktools").is_err()
        );
        let link = dir.join("keys/link.json");
        symlink(&path, &link).unwrap();
        assert!(
            load_or_create_signing_key(&link, KeyId::parse("key-1").unwrap(), "tiktools").is_err()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    fn uuid_for_test() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    #[test]
    fn valid_namespace_mirrors_backend_rules() {
        assert!(valid_namespace("acme"));
        assert!(valid_namespace("tiktools-2"));
        assert!(!valid_namespace("A"));
        assert!(!valid_namespace("a"));
        assert!(!valid_namespace("-acme"));
        assert!(!valid_namespace("acme-"));
        assert!(!valid_namespace("acme_plugin"));
        assert!(!valid_namespace("acme.plugin"));
    }

    #[test]
    fn proof_message_matches_backend_domain() {
        assert_eq!(
            proof_message("pub", "chal", "nonce", "key-1", "cGsta2V5"),
            "tiktools-cloud-publisher-key-v1\npub\nchal\nnonce\nkey-1\ncGsta2V5"
        );
    }

    #[test]
    fn base64_matches_standard_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(
            base64_encode(&[0xff; 32]),
            "//////////////////////////////////////////8="
        );
    }

    #[tokio::test]
    async fn full_publisher_flow_against_mock_backend() {
        use std::collections::HashMap;
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::{Arc, Mutex};

        use secrecy::SecretString;
        use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore, TokenStore};

        type RequestLog = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;
        #[derive(Clone)]
        struct Log(RequestLog);

        fn json(status: u16, body: &str) -> Vec<u8> {
            let reason = match status {
                200 => "OK",
                201 => "Created",
                400 => "Bad Request",
                _ => "Error",
            };
            format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_bytes()
        }

        fn route(
            method: &str,
            path: &str,
            headers: &HashMap<String, String>,
            body: &[u8],
            log: &Log,
            origin: &str,
        ) -> Vec<u8> {
            let body_json: serde_json::Value =
                serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
            log.0
                .lock()
                .unwrap()
                .push((method.to_owned(), path.to_owned(), body.to_vec()));
            if path != "/store/object"
                && headers.get("authorization").map(String::as_str) != Some("Bearer test-token")
            {
                return json(401, r#"{"error":"unauthorized"}"#);
            }
            let release = |status: &str| {
                format!(
                    r#"{{"id":"rel-1","publisherId":"pub-1","uploadId":"upload-1","publisherKeyId":"rec-1","pluginId":"acme.tts","version":"1.2.3","target":"any","status":"{status}","yanked":false}}"#
                )
            };
            match (method, path) {
                ("GET", "/api/v1/native/me") => json(200, r#"{"id":"user-1"}"#),
                ("GET", "/api/v1/native/me/export") => json(
                    200,
                    r#"{"schemaVersion":1,"profile":{"id":"user-1","email":"user@example.com","name":"Test User"},"cloudRoles":[],"publisherMemberships":[],"publisherKeysRequested":[],"releasesSubmitted":[],"uploadsRequested":[],"reviewsRecorded":[],"publicationRequests":[],"ownershipTransfers":[],"auditEventsByActor":[],"yanksRecorded":[],"advisoriesRecorded":[],"sync":{"schemaVersion":1,"cursor":0,"records":[]}}"#,
                ),
                ("GET", "/api/v1/publishers") => json(
                    200,
                    r#"[{"id":"pub-1","namespace":"acme","status":"approved","role":"owner"}]"#,
                ),
                ("POST", "/api/v1/publishers") => {
                    let namespace = body_json
                        .get("namespace")
                        .and_then(|value| value.as_str())
                        .unwrap_or("?");
                    json(
                        201,
                        &format!(
                            r#"{{"id":"pub-2","namespace":"{namespace}","status":"pending","role":"owner"}}"#
                        ),
                    )
                }
                ("GET", "/api/v1/publishers/pub-1/keys") => json(200, "[]"),
                ("POST", "/api/v1/step-up/password") => {
                    if body_json.get("password").and_then(|value| value.as_str())
                        != Some("correct-horse")
                    {
                        return json(401, r#"{"error":"bad password"}"#);
                    }
                    json(200, r#"{"grantId":"grant-1","expiresIn":300}"#)
                }
                ("POST", "/api/v1/publishers/pub-1/keys/challenges") => json(
                    200,
                    r#"{"challengeId":"chal-1","nonce":"n0nce","expiresIn":300}"#,
                ),
                ("POST", "/api/v1/publishers/pub-1/keys") => {
                    let key_id = body_json
                        .get("keyId")
                        .and_then(|value| value.as_str())
                        .unwrap_or("?");
                    let public_key = body_json
                        .get("publicKey")
                        .and_then(|value| value.as_str())
                        .unwrap_or("?");
                    json(
                        200,
                        &format!(
                            r#"{{"id":"rec-1","keyId":"{key_id}","publicKey":"{public_key}","fingerprint":"fp-1","status":"pending"}}"#
                        ),
                    )
                }
                ("POST", "/api/v1/plugins/uploads") => {
                    if body_json.get("size").and_then(|value| value.as_i64()) != Some(4)
                        || body_json.get("sha256").and_then(|value| value.as_str())
                            != Some(
                                "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
                            )
                    {
                        return json(400, r#"{"error":"bad reservation"}"#);
                    }
                    json(
                        200,
                        &format!(
                            r#"{{"id":"upload-1","url":"{origin}/store/object","headers":{{}},"expiresIn":299}}"#
                        ),
                    )
                }
                ("PUT", "/store/object") => json(200, "{}"),
                ("POST", "/api/v1/plugins/uploads/upload-1/complete") => {
                    json(200, r#"{"id":"upload-1","status":"complete"}"#)
                }
                ("POST", "/api/v1/plugins/releases") => json(200, &release("submitted")),
                ("GET", "/api/v1/plugins/releases/rel-1") => json(200, &release("submitted")),
                ("POST", "/api/v1/plugins/releases/rel-1/publish") => json(
                    200,
                    r#"{"jobId":"job-1","status":"requested","processingAvailable":true}"#,
                ),
                _ => json(400, r#"{"error":"unexpected"}"#),
            }
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let log = Log(Arc::new(Mutex::new(Vec::new())));
        let routes = log.clone();
        let serve_origin = origin.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut head = Vec::new();
                let mut byte = [0_u8; 1];
                while head.len() < 65536 {
                    if stream.read_exact(&mut byte).is_err() {
                        break;
                    }
                    head.push(byte[0]);
                    if head.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let head = String::from_utf8_lossy(&head).into_owned();
                let mut lines = head.lines();
                let mut parts = lines.next().unwrap_or("").split_whitespace();
                let method = parts.next().unwrap_or("").to_owned();
                let path = parts.next().unwrap_or("/").to_owned();
                let mut headers = HashMap::new();
                for line in lines {
                    if line.is_empty() {
                        break;
                    }
                    if let Some((key, value)) = line.split_once(':') {
                        headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
                    }
                }
                let length: usize = headers
                    .get("content-length")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
                let mut body = vec![0_u8; length.min(8 * 1024 * 1024)];
                if length > 0 {
                    let _ = stream.read_exact(&mut body);
                }
                let reply = route(&method, &path, &headers, &body, &routes, &serve_origin);
                let _ = stream.write_all(&reply);
            }
        });

        let dir = std::env::temp_dir().join(format!(
            "tt-publisher-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let sidecar = dir.join("cloud-account.json");
        std::fs::write(
            &sidecar,
            format!(r#"{{"origin":"{origin}","account_id":"user-1"}}"#),
        )
        .unwrap();
        let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
        let session = cloud_client::Session::connect_secret(
            &origin,
            SecretString::from("test-token".to_owned()),
        )
        .await
        .unwrap();
        store.store(&origin, "user-1", &session).unwrap();
        let auth = CloudAuth::new(Some(origin.clone()), sidecar, store);
        let status = auth.restore().await.unwrap();
        assert_eq!(status.email.as_deref(), Some("user@example.com"));

        let data = dir.join("data");
        let paths = crate::paths::AppPaths {
            root: dir.clone(),
            data: data.clone(),
            plugins: dir.join("plugins"),
            plugin_data: dir.join("plugin-data"),
            builtin_plugins: dir.join("builtin"),
            development_plugins: None,
            logs: dir.join("logs"),
            temp: dir.join("temp"),
        };
        let publisher = CloudPublisher::new(&paths);

        let list = publisher.list(&auth).await.unwrap();
        assert_eq!(list.publishers.len(), 1);
        assert_eq!(list.publishers[0].namespace, "acme");

        let created = publisher
            .create(
                &auth,
                PublisherCreateParams {
                    namespace: "newns".to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(created.status, "pending");

        let keys = publisher
            .keys(
                &auth,
                PublisherIdParams {
                    publisher_id: "acme".to_owned(),
                },
            )
            .await
            .unwrap();
        assert!(keys.keys.is_empty());

        let enrolled = publisher
            .enroll(
                &auth,
                PublisherEnrollParams {
                    publisher_id: "acme".to_owned(),
                    password: "correct-horse".to_owned(),
                    key_id: Some("acme-1".to_owned()),
                },
            )
            .await
            .unwrap();
        assert_eq!(enrolled.record.status, "pending");
        assert_eq!(enrolled.record.key_id, "acme-1");
        let key_bytes = std::fs::read(&enrolled.key_file).unwrap();
        let loaded = SigningKey::from_json(&key_bytes).unwrap();
        assert_eq!(loaded.key_id().as_str(), "acme-1");
        assert_eq!(loaded.publisher(), Some("acme"));
        let retried = publisher
            .enroll(
                &auth,
                PublisherEnrollParams {
                    publisher_id: "acme".into(),
                    password: "correct-horse".into(),
                    key_id: Some("acme-1".into()),
                },
            )
            .await
            .unwrap();
        assert_eq!(std::fs::read(&retried.key_file).unwrap(), key_bytes);
        assert_eq!(retried.record.public_key, enrolled.record.public_key);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&enrolled.key_file)
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }

        let artifact = dir.join("acme.tts.plugin");
        std::fs::write(&artifact, b"test").unwrap();
        let uploaded = publisher
            .upload(
                &auth,
                PublisherUploadParams {
                    publisher_id: "acme".to_owned(),
                    path: artifact.to_string_lossy().into_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(uploaded.upload_id, "upload-1");
        assert_eq!(uploaded.status, "complete");

        let submitted = publisher
            .submit(
                &auth,
                PublisherSubmitParams {
                    upload_id: "upload-1".to_owned(),
                    publisher_key_id: "rec-1".to_owned(),
                    plugin_id: "acme.tts".to_owned(),
                    version: "1.2.3".to_owned(),
                    target: String::new(),
                },
            )
            .await
            .unwrap();
        assert_eq!(submitted.release.target, "any");
        assert!(!submitted.release.yanked);

        let fetched = publisher
            .release(
                &auth,
                PublisherReleaseParams {
                    id: "rel-1".to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(fetched.release.id, "rel-1");

        let publication = publisher
            .publish(
                &auth,
                PublisherReleaseParams {
                    id: "rel-1".to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(publication.job_id, "job-1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn uuid_detection_accepts_only_canonical_ids() {
        assert!(is_uuid("38af3e2c-e851-4d56-9f71-75c5437d5c34"));
        assert!(!is_uuid("acme"));
        assert!(!is_uuid(""));
        assert!(!is_uuid("38af3e2c-e851-4d56-9f71-75c5437d5c3"));
        assert!(!is_uuid("38af3e2c_e851_4d56_9f71_75c5437d5c34"));
    }

    #[test]
    fn signing_round_trip_matches_wire_format() {
        let key = SigningKey::generate(KeyId::parse("key-1").unwrap(), None).unwrap();
        let raw = key.public_key().raw().unwrap();
        assert_eq!(raw.len(), 32);
        assert_eq!(base64_encode(&raw).len(), 44);
        let proof = proof_message("pub", "chal", "nonce", "key-1", &base64_encode(&raw));
        let signature = key.sign(proof.as_bytes());
        assert_eq!(signature.len(), 64);
        key.public_key()
            .verify(proof.as_bytes(), &signature)
            .unwrap();
    }
}
