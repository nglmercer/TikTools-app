use super::key_storage::load_or_create_signing_key;
use super::*;

impl CloudPublisher {
    fn key_file(&self, key_id: &str) -> PathBuf {
        self.keys_dir.join(format!("{key_id}.json"))
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
pub(super) fn proof_message(
    publisher: &str,
    challenge: &str,
    nonce: &str,
    key_id: &str,
    public_key: &str,
) -> String {
    format!("{PROOF_DOMAIN}\n{publisher}\n{challenge}\n{nonce}\n{key_id}\n{public_key}")
}

pub(super) fn base64_encode(bytes: &[u8]) -> String {
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
