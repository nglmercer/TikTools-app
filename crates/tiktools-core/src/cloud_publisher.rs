//! Optional Cloud publisher flow: namespaces, signing keys, artifact uploads
//! and release submissions against the Cloud backend.
//!
//! Everything here is opt-in and requires a signed-in Cloud account; without
//! one every operation fails closed with `unavailable`. The session Bearer [REDACTED]
//! is borrowed transiently from [`CloudAuth`] to authorize same-process HTTPS
//! requests and is never persisted, logged, or sent anywhere except the
//! configured Cloud origin. Private signing keys live in files under the
//! application data directory with owner-only permissions.

mod enrollment;
mod key_storage;
mod namespaces;
mod releases;
mod transport;
mod uploads;

#[cfg(test)]
use enrollment::{base64_encode, proof_message};
#[cfg(test)]
use key_storage::load_or_create_signing_key;
#[cfg(test)]
use namespaces::{is_uuid, valid_namespace};
#[cfg(test)]
use transport::{backend_failure, decode_reply};
#[cfg(test)]
use uploads::safe_upload_url;

use std::{path::PathBuf, time::Duration};

use reqwest::{Method, StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_package::{KeyId, Sha256Digest, SigningKey};

use tiktools_cloud_auth::{CloudAuth, NativeToken};

use crate::{
    cloud_http::{self, Limits, NoContent, Reply},
    paths::AppPaths,
};

/// HTTP budget for every publisher call to the Cloud API: 60 s, 2 MiB of reply
/// (namespaces, key records and release records are small JSON documents).
const HTTP_LIMITS: Limits = Limits::new(Duration::from_secs(60), 2 * 1024 * 1024);
/// The artifact PUT goes to object storage: 60 s. Its reply body is never read
/// (only the status matters), so there is no byte limit to declare.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(60);
/// Longest backend `error` text kept for the caller.
const MAX_BACKEND_MESSAGE_CHARS: usize = 200;

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

/// Result of deleting a publisher namespace. `publisher_id` is the resolved
/// id, which can differ from the namespace the caller typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublisherDeleteResult {
    pub publisher_id: String,
    pub deleted: bool,
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

#[derive(Debug, Clone)]
pub struct CloudPublisher {
    keys_dir: PathBuf,
}

impl CloudPublisher {
    pub(crate) fn new(paths: &AppPaths) -> Self {
        Self {
            keys_dir: paths.data.join("publisher-keys"),
        }
    }
}

#[cfg(test)]
mod tests;
