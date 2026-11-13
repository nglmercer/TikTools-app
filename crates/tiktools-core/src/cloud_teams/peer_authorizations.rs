//! Complete native-host authority snapshot. This is not an address directory.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerAuthorizationSnapshot {
    pub host_id: String,
    pub host_status: String,
    pub items: Vec<PeerAuthorization>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeerAuthorization {
    pub id: String,
    pub device_id: String,
    pub device_label: String,
    pub device_status: String,
    pub role: String,
    pub policy: Value,
    pub status: String,
    pub revision: i64,
    pub confirmed_revision: Option<i64>,
    pub endpoint_id: Option<String>,
}
impl CloudTeams {
    pub async fn peer_authorizations(
        &self,
        auth: &CloudAuth,
        host: &str,
    ) -> Result<PeerAuthorizationSnapshot, OperationError> {
        valid_uuid(host)?;
        let (origin, token) = credentials(auth).await?;
        let snapshot: PeerAuthorizationSnapshot = decode(
            send(
                Method::GET,
                &origin,
                &token,
                &format!("/api/v3/remote/hosts/{host}/peer-authorizations"),
                None,
            )
            .await?,
        )?;
        if snapshot.host_id != host
            || snapshot.items.len() > 256
            || !["active", "disabled"].contains(&snapshot.host_status.as_str())
        {
            return Err(OperationError::internal(
                "Cloud sent invalid peer authorization snapshot",
            ));
        }
        let mut grants = std::collections::BTreeSet::new();
        for item in &snapshot.items {
            valid_uuid(&item.id)?;
            valid_uuid(&item.device_id)?;
            if !grants.insert(&item.id)
                || item.revision <= 0
                || item.revision == i64::MAX
                || item.device_label.chars().count() > 120
                || item.device_label.chars().any(char::is_control)
                || !["active", "revoked"].contains(&item.device_status.as_str())
                || !["active", "pendingHost", "revoked"].contains(&item.status.as_str())
                || serde_json::to_vec(&item.policy).map_or(true, |v| v.len() > 16384)
                || item.endpoint_id.as_ref().is_some_and(|id| {
                    id.len() != 64
                        || !id
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
            {
                return Err(OperationError::internal(
                    "Cloud sent invalid peer authorization",
                ));
            }
        }
        Ok(snapshot)
    }
}
