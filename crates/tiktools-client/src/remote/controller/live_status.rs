//! Dedicated read-only LIVE connection status for the controller window.
//! Neither this method nor its projection can start, stop or control a LIVE.
use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HostStatus {
    connected: bool,
    unique_id: Option<String>,
    room_id: Option<String>,
    connection_id: Option<String>,
    native: bool,
}

fn valid_optional(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|value| {
        !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
    })
}

fn bound(
    v: &View,
    epoch: &str,
    connection: &str,
    expected: Option<&RemoteClient>,
) -> Result<RemoteClient, ClientError> {
    let remote = v.remote.as_ref().ok_or_else(network)?;
    let host = v.status.hello.as_ref().ok_or_else(network)?;
    let actual = remote.hello();
    if v.status.phase != "ready"
        || v.status.epoch != epoch
        || connection.is_empty()
        || host.connection_id != connection
        || actual.connection_id != connection
        || host.host_id != actual.host_id
        || host.host_instance_id != actual.host_instance_id
        || host.principal_id != actual.principal_id
        || host.session_id != actual.session_id
        || host.grant_revision != actual.grant_revision
        || host.capabilities != actual.capabilities
        || host.resource_restrictions != actual.resource_restrictions
        || !host.capabilities.contains(&Capability::LiveRead)
        || remote.is_closed()
        || remote.requires_resync()
        || expected.is_some_and(|old| {
            old.is_closed()
                || old.requires_resync()
                || old.hello().connection_id != actual.connection_id
                || old.hello().host_id != actual.host_id
                || old.hello().host_instance_id != actual.host_instance_id
                || old.hello().principal_id != actual.principal_id
                || old.hello().session_id != actual.session_id
                || old.hello().grant_revision != actual.grant_revision
        })
    {
        return Err(fail(
            "stale_view",
            "LIVE status belongs to a previous controller session",
        ));
    }
    Ok(remote.clone())
}

impl Controller {
    pub(super) async fn live_status(
        &self,
        epoch: &str,
        connection: &str,
    ) -> Result<Value, ClientError> {
        let remote = {
            let view = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            bound(&view, epoch, connection, None)?
        };
        let result = remote
            .call_value_with_meta("live.status", json!({}), None)
            .await?;
        let result: HostStatus = serde_json::from_value(result.result).map_err(|_| protocol())?;
        if !valid_optional(&result.unique_id)
            || !valid_optional(&result.room_id)
            || !valid_optional(&result.connection_id)
        {
            return Err(protocol());
        }
        {
            let view = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            let current = bound(&view, epoch, connection, Some(&remote))?;
            if current
                .hello()
                .resource_restrictions
                .get(&ResourceKind::Live)
                .is_some_and(|allowed| {
                    !result
                        .unique_id
                        .as_ref()
                        .is_some_and(|id| allowed.contains(id))
                })
            {
                return Err(fail("forbidden", "LIVE creator is not permitted"));
            }
        }
        Ok(
            json!({"epoch":epoch,"connectionId":connection,"connected":result.connected,
            "uniqueId":result.unique_id,"native":result.native}),
        )
    }
}
