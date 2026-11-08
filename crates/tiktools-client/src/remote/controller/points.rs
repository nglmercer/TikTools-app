//! Lossy bounded display projection. Reliable invalidations are staged as a
//! dirty bit before acknowledgement; reconnect always refreshes absolute state.
use super::*;
use std::time::Instant;
use tiktools_control_api::modules::points::PointsViewResult;

#[derive(Clone, PartialEq, Serialize)]
pub struct PointRow {
    pub viewer: Value,
    pub version: ResourceVersion,
}
#[derive(Default)]
pub(super) struct State {
    pub query: Option<String>,
    pub generation: u64,
    pub dirty: bool,
    data: Option<Data>,
    last: Option<Instant>,
}
#[derive(PartialEq)]
struct Data {
    currency: String,
    rows: Vec<PointRow>,
    selected: Option<PointRow>,
    selected_version: Option<ResourceVersion>,
}
pub(super) fn readable(remote: &RemoteClient) -> bool {
    let h = remote.hello();
    h.capabilities.contains(&Capability::PointsRead)
        && !h.resource_restrictions.contains_key(&ResourceKind::Live)
        && h.features.get("pointsProjection") == Some(&true)
}
pub(super) fn limit(remote: &RemoteClient) -> u16 {
    ((remote.hello().limits.frame_bytes / 2).saturating_sub(768) / 1024).clamp(1, 20) as u16
}
pub(super) fn period(remote: &RemoteClient) -> Duration {
    Duration::from_millis(
        250.max(2000 / u64::from(remote.hello().limits.requests_per_second.max(1))),
    )
}
pub(super) fn domains(remote: &RemoteClient) -> std::collections::BTreeSet<StreamDomain> {
    let mut result = std::collections::BTreeSet::new();
    if remote
        .hello()
        .capabilities
        .contains(&Capability::WorkflowsRead)
    {
        result.insert(StreamDomain::Workflows);
    }
    if readable(remote) {
        result.insert(StreamDomain::Points);
    }
    if automations::readable(remote) {
        result.insert(StreamDomain::Automation);
    }
    result
}
pub(super) async fn stream(remote: &RemoteClient) -> Result<Option<RecoveryStream>, ClientError> {
    let domains = domains(remote);
    if domains.is_empty() {
        return Ok(None);
    }
    let bounded = domains.contains(&StreamDomain::Points).then_some(0);
    Ok(Some(
        RecoveryStream::start_projected(remote.clone(), domains, bounded).await?,
    ))
}
fn notify(shared: &Shared, v: &mut View) {
    v.status.points_revision = v
        .status
        .points_revision
        .parse::<u64>()
        .unwrap_or(0)
        .saturating_add(1)
        .to_string();
    shared.changed.send_modify(|n| *n = n.wrapping_add(1));
}
pub(super) fn dirty(shared: &Shared, epoch: &str) {
    let mut v = shared.view.write().unwrap_or_else(|p| p.into_inner());
    if v.status.epoch == epoch {
        v.points.dirty = true;
    }
}
pub(super) fn clear(v: &mut View) {
    v.points.data = None;
    v.points.dirty = true;
    v.status.points_loading = true;
}
pub(super) fn reset_scope(shared: &Shared, epoch: &str, remote: &RemoteClient) {
    let mut v = shared.view.write().unwrap_or_else(|p| p.into_inner());
    if v.status.epoch != epoch {
        return;
    }
    if !readable(remote)
        || v.points.query.as_ref().is_some_and(|id| {
            remote
                .hello()
                .resource_restrictions
                .get(&ResourceKind::Viewer)
                .is_some_and(|ids| !ids.contains(id))
        })
    {
        v.points.query = None;
        v.points.generation = v.points.generation.saturating_add(1);
    }
    clear(&mut v);
    notify(shared, &mut v);
}
pub(super) fn due(shared: &Shared, epoch: &str) -> bool {
    let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
    v.status.epoch == epoch
        && (v.points.dirty
            || v.points
                .last
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(2)))
}
pub(super) fn failure(shared: &Shared, epoch: &str, error: &ClientError) {
    let mut v = shared.view.write().unwrap_or_else(|p| p.into_inner());
    if v.status.epoch != epoch {
        return;
    }
    v.points.data = None;
    v.points.last = Some(Instant::now());
    v.points.dirty = false;
    v.status.points_loading = false;
    v.status.points_error_code = Some(error.code.clone());
    notify(shared, &mut v);
}
pub(super) async fn refresh(
    shared: &Shared,
    epoch: &str,
    remote: &RemoteClient,
) -> Result<(), ClientError> {
    if !readable(remote) {
        return Ok(());
    }
    let (query, generation) = {
        let mut v = shared.view.write().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch {
            return Err(fail("stale_view", "controller selection changed"));
        }
        v.points.dirty = false;
        (v.points.query.clone(), v.points.generation)
    };
    let take = if query.is_some() {
        0
    } else {
        i64::from(limit(remote))
    };
    let reply = remote
        .call_value_with_meta("points.view", json!({"limit":take,"uniqueId":query}), None)
        .await?;
    let data: PointsViewResult = serde_json::from_value(reply.result).map_err(|_| protocol())?;
    if data.selected_id != query
        || data.viewers.len() > take as usize
        || !valid_currency(&data.currency_name)
    {
        return Err(protocol());
    }
    let mut versions = BTreeMap::new();
    for version in reply.meta.ok_or_else(protocol)?.resource_versions {
        if remote
            .hello()
            .resource_restrictions
            .get(&ResourceKind::Viewer)
            .is_some_and(|ids| !ids.contains(&version.id))
            || version.kind != ResourceKind::Viewer
            || version.scope != "host"
            || tiktools_collaboration::stream_sequence(&version.revision).is_err()
            || versions.insert(version.id.clone(), version).is_some()
        {
            return Err(protocol());
        }
    }
    let row = |viewer: Value, version: ResourceVersion| -> Result<PointRow, ClientError> {
        if viewer.get("uniqueId").and_then(Value::as_str) != Some(version.id.as_str())
            || version.revision == "0"
            || !viewer
                .get("level")
                .and_then(Value::as_u64)
                .is_some_and(|n| n > 0 && n <= u64::from(u32::MAX))
            || !viewer
                .get("points")
                .and_then(Value::as_f64)
                .is_some_and(|n| n.is_finite() && n >= 0.0)
        {
            return Err(protocol());
        }
        Ok(PointRow { viewer, version })
    };
    let selected_version = query
        .as_ref()
        .map(|id| versions.get(id).cloned().ok_or_else(protocol))
        .transpose()?;
    let selected = match data.selected {
        Some(viewer) => Some(row(viewer, selected_version.clone().ok_or_else(protocol)?)?),
        None => {
            if selected_version.as_ref().is_some_and(|v| v.revision != "0") {
                return Err(protocol());
            }
            None
        }
    };
    let mut rows = Vec::new();
    let mut used = std::collections::BTreeSet::new();
    for viewer in data.viewers {
        let id = viewer
            .get("uniqueId")
            .and_then(Value::as_str)
            .ok_or_else(protocol)?
            .to_owned();
        if !used.insert(id.clone()) {
            return Err(protocol());
        }
        rows.push(row(
            viewer,
            versions.get(&id).cloned().ok_or_else(protocol)?,
        )?);
    }
    if let Some(id) = &query {
        used.insert(id.clone());
    }
    if selected.as_ref().is_some_and(|s| {
        rows.iter()
            .any(|r| r.version.id == s.version.id && r.viewer != s.viewer)
    }) {
        return Err(protocol());
    }
    if used.len() != versions.len() {
        return Err(protocol());
    }
    let mut v = shared.view.write().unwrap_or_else(|p| p.into_inner());
    if v.status.epoch != epoch
        || v.points.generation != generation
        || v.remote
            .as_ref()
            .is_none_or(|r| r.hello().connection_id != remote.hello().connection_id)
    {
        return Ok(());
    }
    let data = Data {
        currency: data.currency_name,
        rows,
        selected,
        selected_version,
    };
    let changed = v.points.data.as_ref() != Some(&data)
        || v.status.points_loading
        || v.status.points_error_code.is_some();
    v.points.data = Some(data);
    v.points.last = Some(Instant::now());
    v.status.points_loading = false;
    v.status.points_error_code = None;
    if changed {
        notify(shared, &mut v);
    }
    Ok(())
}
fn valid_currency(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && name.chars().count() <= 64
        && !name.chars().any(char::is_control)
}
impl Controller {
    pub fn points(&self, epoch: &str, revision: &str) -> Result<Value, ClientError> {
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch || v.status.points_revision != revision {
            return Err(fail("stale_view", "points view changed"));
        }
        if v.status.phase != "ready"
            || v.remote
                .as_ref()
                .is_none_or(|r| r.is_closed() || r.requires_resync() || !readable(r))
        {
            return Err(fail("not_ready", "points view is recovering"));
        }
        let data = v.points.data.as_ref();
        Ok(
            json!({"epoch":epoch,"pointsRevision":revision,"loading":v.status.points_loading,"errorCode":v.status.points_error_code,
            "currencyName":data.map(|d|&d.currency),"rows":data.map(|d|&d.rows),"selectedId":v.points.query,
            "selected":data.and_then(|d|d.selected.as_ref()),"selectedVersion":data.and_then(|d|d.selected_version.as_ref())}),
        )
    }
    pub fn select_points(&self, epoch: &str, id: Option<String>) -> Result<Value, ClientError> {
        let mut v = self.shared.view.write().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch || v.status.phase != "ready" {
            return Err(fail("not_ready", "selected controller is not ready"));
        }
        let r = v.remote.as_ref().ok_or_else(network)?;
        if !readable(r) || r.is_closed() || r.requires_resync() {
            return Err(fail("forbidden", "points view is unavailable"));
        }
        if let Some(id) = &id {
            if id.is_empty()
                || id.len() > 128
                || id.trim() != id
                || id.starts_with('@')
                || id.contains('*')
                || id.chars().any(char::is_control)
            {
                return Err(fail("invalid_params", "invalid viewer ID"));
            }
            if r.hello()
                .resource_restrictions
                .get(&ResourceKind::Viewer)
                .is_some_and(|ids| !ids.contains(id))
            {
                return Err(fail("forbidden", "viewer is not permitted"));
            }
        }
        v.points.query = id;
        v.points.generation = v.points.generation.saturating_add(1);
        clear(&mut v);
        notify(&self.shared, &mut v);
        Ok(json!({"ok":true}))
    }
}

#[cfg(test)]
mod tests;
