//! Approved teams visible to one enrolled native controller.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerMembership {
    pub id: String,
    pub name: String,
    pub role: String,
    pub client_kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerMemberships {
    pub items: Vec<PeerMembership>,
}
impl CloudTeams {
    pub async fn peer_memberships(
        &self,
        auth: &CloudAuth,
        device: &str,
    ) -> Result<PeerMemberships, OperationError> {
        valid_uuid(device)?;
        let (origin, token) = credentials(auth).await?;
        let result: PeerMemberships = decode(
            send(
                Method::GET,
                &origin,
                &token,
                &format!("/api/v3/remote/devices/{device}/peer-hosts"),
                None,
            )
            .await?,
        )?;
        if result.items.len() > 64 {
            return Err(OperationError::internal("Cloud sent too many peer teams"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for item in &result.items {
            valid_uuid(&item.id)?;
            if !seen.insert(&item.id)
                || item.name.chars().count() > 80
                || item.name.chars().any(char::is_control)
                || !["desktop", "agent"].contains(&item.client_kind.as_str())
                || !PRESET_ROLES.contains(&item.role.as_str())
            {
                return Err(OperationError::internal(
                    "Cloud sent invalid peer membership",
                ));
            }
        }
        Ok(result)
    }
}
