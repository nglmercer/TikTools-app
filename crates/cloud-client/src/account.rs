//! Account deletion blockers and explicit session revocation.
use super::*;

impl DeletionBlockerPage {
    pub(super) fn validate(&self, after: Option<Uuid>) -> Result<(), SyncError> {
        let mut roles = BTreeSet::new();
        if self
            .cloud_roles
            .iter()
            .any(|r| !matches!(r.as_str(), "admin" | "moderator") || !roles.insert(r))
            || self.memberships.len() > 100
        {
            return Err(SyncError::Protocol);
        }
        let mut previous = after;
        for member in &self.memberships {
            let name = member.namespace.as_bytes();
            if previous.is_some_and(|id| member.publisher_id <= id)
                || !(2..=63).contains(&name.len())
                || !name[0].is_ascii_lowercase()
                || name[name.len() - 1] == b'-'
                || !name
                    .iter()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
                || !matches!(
                    member.role.as_str(),
                    "owner" | "admin" | "publisher" | "viewer"
                )
            {
                return Err(SyncError::Protocol);
            }
            previous = Some(member.publisher_id);
        }
        if self.next_after.is_some()
            && (self.memberships.len() != 100 || self.next_after != previous)
        {
            return Err(SyncError::Protocol);
        }
        Ok(())
    }
}

impl Session {
    /// List Cloud-owned deletion blockers. An empty page is not a deletion guarantee.
    pub async fn deletion_blockers(
        &self,
        after: Option<Uuid>,
    ) -> Result<DeletionBlockerPage, SyncError> {
        let params = after
            .map(|id| vec![("after", id.to_string())])
            .unwrap_or_default();
        let page: DeletionBlockerPage = self
            .get("api/v1/native/me/deletion-blockers", &params)
            .await?;
        page.validate(after)?;
        Ok(page)
    }

    /// Revoke the current bearer session on the server. Remove any stored keychain
    /// entry separately after success; an offline error means revocation is unknown.
    pub async fn logout(&self) -> Result<(), SyncError> {
        let response = self
            .request(reqwest::Method::POST, "api/v1/native/logout")?
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(SyncError::Unauthorized),
            status => Err(SyncError::Http(status.as_u16())),
        }
    }
}

/// Effective ceilings of the account's plan, as the backend enforces them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanLimits {
    pub max_records: i64,
    pub max_bytes: i64,
    pub max_assets: i64,
    pub max_assets_bytes: i64,
    pub max_overlays: i64,
    pub max_overlay_bytes: i64,
    pub max_backups: i64,
    pub max_backup_bytes: i64,
}

/// What the account stores now, counted the same way the quota checks count it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanUsage {
    pub records: i64,
    pub record_bytes: i64,
    pub assets: i64,
    pub asset_bytes: i64,
    pub overlays: i64,
    pub overlay_bytes: i64,
    pub backups: i64,
    pub backup_bytes: i64,
}

/// The account's plan, its ceilings and its current usage.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountPlan {
    pub plan: String,
    pub limits: PlanLimits,
    pub usage: PlanUsage,
}

impl Session {
    /// Read the caller's plan, limits and usage. Read-only: the plan is assigned
    /// by an operator and can never be changed from the client.
    pub async fn plan(&self) -> Result<AccountPlan, SyncError> {
        self.get("api/v1/native/plan", &[]).await
    }
}

#[cfg(test)]
mod plan_tests {
    use super::AccountPlan;

    /// The backend's `GET /api/v1/native/plan` shape. A renamed field must fail here.
    #[test]
    fn plan_response_decodes_limits_and_usage() {
        let body = r#"{
            "plan": "free",
            "limits": {"maxRecords": 1000, "maxBytes": 8388608, "maxChanges": 10000,
                       "maxChangeBytes": 16777216, "maxAssets": 200, "maxAssetsBytes": 33554432,
                       "maxOverlays": 50, "maxRevisionsPerOverlay": 100, "maxOverlayBytes": 16777216},
            "usage": {"records": 3, "recordBytes": 120, "assets": 1, "assetBytes": 5,
                      "overlays": 0, "overlayBytes": 0}
        }"#;
        let plan: AccountPlan = serde_json::from_str(body).unwrap();
        assert_eq!(plan.plan, "free");
        assert_eq!(plan.limits.max_assets_bytes, 33_554_432);
        assert_eq!(plan.usage.records, 3);
        assert_eq!(plan.usage.asset_bytes, 5);
    }
}
