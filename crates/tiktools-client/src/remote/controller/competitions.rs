//! Read-only, bounded competition status for an explicitly scoped controller.
use super::*;
use tiktools_control_api::modules::competitions::{CompetitionSummary, CompetitionSummaryPage};

pub(super) struct Observation {
    pub epoch: String,
    pub connection: String,
    pub grant_revision: String,
    pub host_instance: String,
    pub rows: BTreeMap<String, CompetitionSummary>,
}

fn bound(
    view: &View,
    epoch: &str,
    connection: &str,
    expected: Option<&RemoteClient>,
) -> Result<RemoteClient, ClientError> {
    let remote = view.remote.as_ref().ok_or_else(network)?;
    let host = view.status.hello.as_ref().ok_or_else(network)?;
    let actual = remote.hello();
    if view.status.phase != "ready"
        || view.status.epoch != epoch
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
        || !actual.capabilities.contains(&Capability::CompetitionsRead)
        || actual
            .resource_restrictions
            .contains_key(&ResourceKind::Live)
        || actual.features.get("competitionSummary") != Some(&true)
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
            "competition status belongs to a previous controller session",
        ));
    }
    Ok(remote.clone())
}

fn valid_decimal(value: &str) -> bool {
    value
        .parse::<u64>()
        .is_ok_and(|number| number.to_string() == value)
}

impl Controller {
    pub(super) async fn competitions(
        &self,
        epoch: &str,
        connection: &str,
        after: Option<&str>,
    ) -> Result<Value, ClientError> {
        if after
            .is_some_and(|id| id.is_empty() || id.len() > 128 || id.chars().any(char::is_control))
        {
            return Err(fail("invalid_params", "invalid competition cursor"));
        }
        let remote = {
            let view = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            bound(&view, epoch, connection, None)?
        };
        let versioned = remote.hello().features.get("competitionSummaryVersions") == Some(&true);
        let mut request = json!({"after":after,"limit":20});
        if versioned {
            request["includeRevision"] = json!(true);
        }
        let response = remote
            .call_value_with_meta("competitions.summary", request, None)
            .await?;
        let page: CompetitionSummaryPage =
            serde_json::from_value(response.result).map_err(|_| protocol())?;
        if page.competitions.len() > 20 {
            return Err(protocol());
        }
        let mut previous = after;
        for row in &page.competitions {
            if row.id.is_empty()
                || row.id.len() > 128
                || row.id.chars().any(char::is_control)
                || row.name.trim().is_empty()
                || row.name.len() > 256
                || row.name.chars().any(char::is_control)
                || previous.is_some_and(|old| row.id.as_str() <= old)
                || !valid_decimal(&row.generation)
                || (versioned && row.revision.is_none())
                || row
                    .revision
                    .as_deref()
                    .is_some_and(|n| !n.parse::<i64>().is_ok_and(|v| v > 0 && v.to_string() == n))
                || row
                    .deadline_ms
                    .as_deref()
                    .is_some_and(|n| !valid_decimal(n))
                || row
                    .remaining_ms
                    .as_deref()
                    .is_some_and(|n| !valid_decimal(n))
                || remote
                    .hello()
                    .resource_restrictions
                    .get(&ResourceKind::Competition)
                    .is_some_and(|ids| !ids.contains(&row.id))
            {
                return Err(protocol());
            }
            previous = Some(&row.id);
        }
        if page
            .next_after
            .as_deref()
            .is_some_and(|next| Some(next) != page.competitions.last().map(|row| row.id.as_str()))
        {
            return Err(protocol());
        }
        {
            let mut view = self.shared.view.write().unwrap_or_else(|p| p.into_inner());
            bound(&view, epoch, connection, Some(&remote))?;
            view.competition_observation = Some(Observation {
                epoch: epoch.into(),
                connection: connection.into(),
                grant_revision: remote.hello().grant_revision.clone(),
                host_instance: remote.hello().host_instance_id.clone(),
                rows: page
                    .competitions
                    .iter()
                    .cloned()
                    .map(|row| (row.id.clone(), row))
                    .collect(),
            });
        }
        let mut result = serde_json::to_value(page).map_err(|_| protocol())?;
        let result = result.as_object_mut().ok_or_else(protocol)?;
        result.insert("epoch".into(), json!(epoch));
        result.insert("connectionId".into(), json!(connection));
        Ok(Value::Object(result.clone()))
    }
}
