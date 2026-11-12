//! Bounded, account-bound sync and account exports.
use super::*;

impl SyncExport {
    pub(super) fn validate(&self) -> Result<(), SyncError> {
        if self.schema_version != 1 || self.cursor < 0 || self.records.len() > 1000 {
            return Err(SyncError::Protocol);
        }
        let mut keys = BTreeSet::new();
        for record in &self.records {
            validate_record(record)?;
            if record.revision > self.cursor || !keys.insert(&record.key) {
                return Err(SyncError::Protocol);
            }
        }
        Ok(())
    }
}

impl AccountExport {
    pub(super) fn validate(&self, account_id: &str) -> Result<(), SyncError> {
        if self.schema_version != 1 || self.profile.id != account_id {
            return Err(SyncError::WrongAccount);
        }
        self.sync.validate()?;
        for section in [
            &self.publisher_memberships,
            &self.publisher_keys_requested,
            &self.releases_submitted,
            &self.uploads_requested,
            &self.reviews_recorded,
            &self.publication_requests,
            &self.ownership_transfers,
            &self.audit_events_by_actor,
            &self.yanks_recorded,
            &self.advisories_recorded,
        ] {
            if section.len() > 1000 || section.iter().any(|item| !item.is_object()) {
                return Err(SyncError::Protocol);
            }
            if section.iter().any(contains_sensitive_field) {
                return Err(SyncError::Protocol);
            }
        }
        if self.cloud_roles.len() > 1000
            || self
                .cloud_roles
                .iter()
                .any(|role| !matches!(role.as_str(), "admin" | "moderator"))
        {
            return Err(SyncError::Protocol);
        }
        Ok(())
    }
}

impl Session {
    /// Retrieve a point-in-time export of server-side sync records without changing local state.
    pub async fn export_sync(&self) -> Result<SyncExport, SyncError> {
        let export: SyncExport = self.get("api/v1/native/sync/export", &[]).await?;
        export.validate()?;
        Ok(export)
    }

    /// Retrieve and validate a bounded snapshot of this account's Cloud-owned data.
    pub async fn export_account(&self) -> Result<AccountExport, SyncError> {
        let response = self
            .request(reqwest::Method::GET, "api/v1/native/me/export")?
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        let export: AccountExport = decode_with_limit(response, MAX_ACCOUNT_EXPORT).await?;
        export.validate(&self.account_id)?;
        Ok(export)
    }
}

fn contains_sensitive_field(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(fields) => fields.iter().any(|(name, child)| {
            matches!(
                name.as_str(),
                "token"
                    | "sessionToken"
                    | "accessToken"
                    | "refreshToken"
                    | "password"
                    | "passwordHash"
                    | "privateKey"
                    | "objectKey"
                    | "versionId"
                    | "signedUrl"
            ) || contains_sensitive_field(child)
        }),
        serde_json::Value::Array(items) => items.iter().any(contains_sensitive_field),
        _ => false,
    }
}
