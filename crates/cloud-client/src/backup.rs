//! Portable backup manifest and explicit restore of Cloud-owned data.

use reqwest::Method;
use serde::Serialize;
use serde_json::Value;

use crate::{transport::decode_with_limit, SyncError};

const MAX_BACKUP_RESPONSE: usize = 64 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreBody<'a> {
    manifest: &'a Value,
    mode: &'a str,
    dry_run: bool,
}

impl crate::Session {
    /// Fetch the full, versioned backup manifest for the signed-in account.
    pub async fn backup(&self) -> Result<Value, SyncError> {
        let response = self
            .request(Method::GET, "api/v2/me/backup")?
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        let manifest: Value = decode_with_limit(response, MAX_BACKUP_RESPONSE).await?;
        if manifest.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
            return Err(SyncError::Protocol);
        }
        Ok(manifest)
    }

    /// Report the restore summary (`dry_run`) or apply the manifest.
    pub async fn restore(
        &self,
        manifest: &Value,
        mode: &str,
        dry_run: bool,
    ) -> Result<Value, SyncError> {
        let response = self
            .request(Method::POST, "api/v2/me/restore")?
            .json(&RestoreBody {
                manifest,
                mode,
                dry_run,
            })
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        decode_with_limit(response, MAX_BACKUP_RESPONSE).await
    }
}
