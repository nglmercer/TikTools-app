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
