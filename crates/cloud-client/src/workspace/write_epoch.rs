//! Optional v2 server barrier transport. This module does not persist requests,
//! decide recovery, or enable product synchronization.
use super::*;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

const EPOCH_HEADER: &str = "x-workspace-write-epoch";
const MAX_EPOCH_RESPONSE: usize = 1024;

fn canonical_epoch(value: &str) -> Option<i64> {
    if value.is_empty()
        || value.len() > 19
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    value.parse::<i64>().ok().filter(|value| *value >= 0)
}

mod epoch_string {
    use super::*;

    pub fn serialize<S: Serializer>(value: &i64, serializer: S) -> Result<S::Ok, S::Error> {
        if *value < 0 {
            return Err(serde::ser::Error::custom("invalid workspace write epoch"));
        }
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
        let value = String::deserialize(deserializer)?;
        canonical_epoch(&value).ok_or_else(|| D::Error::custom("invalid workspace write epoch"))
    }
}

/// Account- and origin-bound epoch. It must be durably stored together with
/// the exact pending write before using a fenced PUT or DELETE. Serialization
/// always stores the full signed-64-bit range as canonical decimal text.
/// Restoring this object does not establish a current session or consent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceWriteEpoch {
    origin: String,
    account_id: String,
    #[serde(with = "epoch_string")]
    write_epoch: i64,
    /// Server observation at the time this epoch was read; absent means the
    /// server did not report the enforcement policy. It can change later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    epoch_required: Option<bool>,
}

impl WorkspaceWriteEpoch {
    pub fn decimal(&self) -> String {
        self.write_epoch.to_string()
    }

    pub fn epoch_required(&self) -> Option<bool> {
        self.epoch_required
    }

    pub(crate) fn matches_identity(&self, origin: &str, account_id: &str) -> bool {
        self.origin == origin && self.account_id == account_id
    }

    /// Compare two durable epochs without converting through JSON numbers.
    /// This alone does not prove that a request used the older epoch.
    pub fn is_after(&self, older: &Self) -> Result<bool, SyncError> {
        if self.origin != older.origin || self.account_id != older.account_id {
            return Err(SyncError::WrongAccount);
        }
        Ok(self.write_epoch > older.write_epoch)
    }

    fn verify_session(&self, session: &Session) -> Result<(), SyncError> {
        if self.origin != session.instance_origin() || self.account_id != session.account_id() {
            return Err(SyncError::WrongAccount);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EpochResponse {
    #[serde(with = "epoch_string")]
    write_epoch: i64,
    #[serde(default)]
    epoch_required: Option<bool>,
}

impl Session {
    async fn request_write_epoch(
        &self,
        method: Method,
        path: &str,
    ) -> Result<WorkspaceWriteEpoch, SyncError> {
        let response = self
            .request(method, path)?
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        if response.status().is_success() && response.status() != StatusCode::OK {
            return Err(SyncError::Protocol);
        }
        let result: EpochResponse = decode_with_limit(response, MAX_EPOCH_RESPONSE).await?;
        Ok(WorkspaceWriteEpoch {
            origin: self.instance_origin(),
            account_id: self.account_id().to_owned(),
            write_epoch: result.write_epoch,
            epoch_required: result.epoch_required,
        })
    }

    /// Obtain the account's current epoch. A GET is not a quiescence barrier.
    pub async fn workspace_write_epoch(&self) -> Result<WorkspaceWriteEpoch, SyncError> {
        self.request_write_epoch(Method::GET, "api/v2/sync/write-epoch")
            .await
    }

    /// Advance the account epoch. A successful response fences cooperating
    /// older writes according to the server contract. An ambiguous result must
    /// not be treated as successful; an explicit retry advances it again.
    pub async fn workspace_write_barrier(&self) -> Result<WorkspaceWriteEpoch, SyncError> {
        self.request_write_epoch(Method::POST, "api/v2/sync/write-barrier")
            .await
    }

    /// Irreversibly require epoch headers for this account's v2 writes on a
    /// compatible server. This affects every device, including older clients;
    /// callers must coordinate upgrades and explicitly review the change.
    /// An ambiguous response must not be interpreted as non-activation.
    pub async fn workspace_require_write_epoch(&self) -> Result<WorkspaceWriteEpoch, SyncError> {
        let epoch = self
            .request_write_epoch(Method::POST, "api/v2/sync/require-write-epoch")
            .await?;
        if epoch.epoch_required != Some(true) {
            return Err(SyncError::Protocol);
        }
        Ok(epoch)
    }

    /// Conditionally put using a previously persisted, identity-bound epoch.
    /// The caller owns persistence, retry, conflict resolution and local
    /// application; the session's existing HTTP timeout still applies.
    pub async fn workspace_put_fenced(
        &self,
        collection: &str,
        key: &str,
        value: Value,
        base_revision: Option<i64>,
        epoch: &WorkspaceWriteEpoch,
    ) -> Result<WorkspaceRecord, SyncError> {
        epoch.verify_session(self)?;
        if !portable::validate(collection, key, &value) || base_revision.is_some_and(|n| n < 1) {
            return Err(SyncError::Configuration);
        }
        let mut request = self
            .request(
                Method::PUT,
                &format!("api/v2/sync/collections/{collection}/{key}"),
            )?
            .header(EPOCH_HEADER, epoch.decimal())
            .json(&value);
        request = if let Some(n) = base_revision {
            request.header(IF_MATCH, format!("\"{n}\""))
        } else {
            request.header(IF_NONE_MATCH, "*")
        };
        let record: WorkspaceRecord = decode_with_limit(
            request.send().await.map_err(|_| SyncError::Network)?,
            MAX_V2_RESPONSE,
        )
        .await?;
        if record.collection != collection
            || record.key != key
            || record.deleted
            || !record
                .value
                .as_ref()
                .is_some_and(|returned| portable::values_equivalent(returned, &value))
            || base_revision.is_some_and(|base| record.revision < base)
            || !record.valid(record.revision)
        {
            return Err(SyncError::Protocol);
        }
        Ok(record)
    }

    /// Conditionally delete using a previously persisted, identity-bound epoch.
    pub async fn workspace_delete_fenced(
        &self,
        collection: &str,
        key: &str,
        base_revision: i64,
        epoch: &WorkspaceWriteEpoch,
    ) -> Result<WorkspaceRecord, SyncError> {
        epoch.verify_session(self)?;
        if !portable::valid_key(collection, key) || base_revision < 1 {
            return Err(SyncError::Configuration);
        }
        let response = self
            .request(
                Method::DELETE,
                &format!("api/v2/sync/collections/{collection}/{key}"),
            )?
            .header(EPOCH_HEADER, epoch.decimal())
            .header(IF_MATCH, format!("\"{base_revision}\""))
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        let record: WorkspaceRecord = decode_with_limit(response, MAX_V2_RESPONSE).await?;
        if record.collection != collection
            || record.key != key
            || !record.deleted
            || record.revision < base_revision
            || !record.valid(record.revision)
        {
            return Err(SyncError::Protocol);
        }
        Ok(record)
    }
}
