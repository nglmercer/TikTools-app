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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredBackup {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub size_bytes: i64,
    pub workspace_sync: usize,
    pub library: usize,
    pub media_library: usize,
    pub assets: usize,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NamedBackupBody<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<&'a serde_json::Value>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredRestoreBody<'a> {
    mode: &'a str,
    #[serde(default)]
    dry_run: bool,
}

impl crate::Session {
    pub async fn backup_list(&self) -> Result<Vec<StoredBackup>, SyncError> {
        let response = self
            .request(Method::GET, "api/v2/me/backups")?
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        let value: serde_json::Value = decode_with_limit(response, MAX_BACKUP_RESPONSE).await?;
        serde_json::from_value(value.get("items").cloned().unwrap_or_default())
            .map_err(|_| SyncError::Protocol)
    }

    /// Store a backup under `name`. With a `manifest` (a local backup file) the
    /// server stores that exact content; without one it snapshots the account.
    pub async fn backup_create_named(
        &self,
        name: &str,
        manifest: Option<&serde_json::Value>,
    ) -> Result<StoredBackup, SyncError> {
        let response = self
            .request(Method::POST, "api/v2/me/backups")?
            .json(&NamedBackupBody { name, manifest })
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        decode_with_limit(response, MAX_BACKUP_RESPONSE).await
    }

    pub async fn backup_get(&self, id: &str) -> Result<serde_json::Value, SyncError> {
        let value: serde_json::Value = self.get(&format!("api/v2/me/backups/{id}"), &[]).await?;
        Ok(value)
    }

    pub async fn backup_delete(&self, id: &str) -> Result<serde_json::Value, SyncError> {
        let response = self
            .request(Method::DELETE, &format!("api/v2/me/backups/{id}"))?
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        decode_with_limit(response, MAX_BACKUP_RESPONSE).await
    }

    pub async fn backup_restore_stored(
        &self,
        id: &str,
        mode: &str,
        dry_run: bool,
    ) -> Result<serde_json::Value, SyncError> {
        let response = self
            .request(Method::POST, &format!("api/v2/me/backups/{id}/restore"))?
            .json(&StoredRestoreBody { mode, dry_run })
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        decode_with_limit(response, MAX_BACKUP_RESPONSE).await
    }
}
