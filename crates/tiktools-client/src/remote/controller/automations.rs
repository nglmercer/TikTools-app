//! Closed behavior status projection. Never stores definitions or local drafts.
use super::*;
use tiktools_collaboration::{AutomationFamily, AutomationSummary, Grant};

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct AutomationRow {
    pub summary: AutomationSummary,
    pub version: ResourceVersion,
}
#[derive(Default)]
pub(super) struct State {
    rows: BTreeMap<(AutomationFamily, String), AutomationRow>,
}
pub(super) fn readable(remote: &RemoteClient) -> bool {
    let h = remote.hello();
    h.features.get("automationSummary") == Some(&true)
        && h.subscription_domains.contains(&StreamDomain::Automation)
        && h.capabilities.contains(&Capability::AutomationRead)
        && !h.resource_restrictions.contains_key(&ResourceKind::Live)
}
fn permitted(remote: &RemoteClient, family: AutomationFamily, id: &str) -> bool {
    readable(remote)
        && Grant::default().allows_automation(family, id)
        && remote
            .hello()
            .resource_restrictions
            .get(&ResourceKind::Automation)
            .is_none_or(|ids| ids.contains(&family.grant_id(id)))
}
fn row(
    remote: &RemoteClient,
    value: Value,
    version: ResourceVersion,
) -> Result<AutomationRow, ClientError> {
    let summary: AutomationSummary = serde_json::from_value(value).map_err(|_| protocol())?;
    if version.kind != ResourceKind::Automation
        || summary.kind.scope() != version.scope
        || summary.id != version.id
        || !summary.definition_redacted
        || summary.name.chars().count() > 160
        || tiktools_collaboration::stream_sequence(&version.revision).map_or(true, |r| r == 0)
        || !permitted(remote, summary.kind, &summary.id)
    {
        return Err(protocol());
    }
    Ok(AutomationRow { summary, version })
}
fn notify(v: &mut View) {
    v.status.automation_revision = v
        .status
        .automation_revision
        .parse::<u64>()
        .unwrap_or(0)
        .saturating_add(1)
        .to_string();
}
pub(super) fn clear(v: &mut View) {
    v.automation.rows.clear();
    notify(v);
}
pub(super) fn baseline(
    v: &mut View,
    remote: &RemoteClient,
    page: &SnapshotPage,
) -> Result<(), ClientError> {
    let mut rows = BTreeMap::new();
    let mut versions: BTreeMap<_, _> = page
        .resource_versions
        .iter()
        .filter(|v| v.kind == ResourceKind::Automation)
        .map(|v| ((v.scope.as_str(), v.id.as_str()), v))
        .collect();
    for value in &page.automation {
        let family = value
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(protocol)?;
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(protocol)?;
        let version = versions.remove(&(family, id)).ok_or_else(protocol)?.clone();
        let r = row(remote, value.clone(), version)?;
        if rows
            .insert((r.summary.kind, r.summary.id.clone()), r)
            .is_some()
        {
            return Err(protocol());
        }
    }
    if !versions.is_empty() {
        return Err(protocol());
    }
    if projection_bytes(&rows) > SNAPSHOT_BYTES {
        return Err(fail(
            "too_large",
            "automation projection exceeds its byte budget",
        ));
    }
    v.automation.rows = rows;
    notify(v);
    Ok(())
}
pub(super) async fn apply(
    shared: &Shared,
    epoch: &str,
    remote: &RemoteClient,
    event: &ReliableEvent,
) -> Result<(), ClientError> {
    let family = AutomationFamily::from_scope(&event.resource.scope).ok_or_else(protocol)?;
    if event.topic != "automation.changed"
        || event.resource.kind != ResourceKind::Automation
        || !permitted(remote, family, &event.resource.id)
    {
        return Err(protocol());
    }
    {
        let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch {
            return Ok(());
        }
        if v.automation
            .rows
            .get(&(family, event.resource.id.clone()))
            .is_some_and(|r| {
                r.version.revision.parse::<u64>().unwrap_or(0)
                    >= event.resource.revision.parse::<u64>().unwrap_or(u64::MAX)
            })
        {
            return Ok(());
        }
    }
    let reply = remote
        .call_value_with_meta(
            "automation.get",
            json!({"kind":family,"id":event.resource.id}),
            None,
        )
        .await;
    let (value, versions) = match reply {
        Ok(reply) => (
            Some(reply.result),
            reply.meta.ok_or_else(protocol)?.resource_versions,
        ),
        Err(e) if e.code == "automation_not_found" => (
            None,
            serde_json::from_value(
                e.data
                    .and_then(|mut v| v.get_mut("resourceVersions").map(Value::take))
                    .ok_or_else(protocol)?,
            )
            .map_err(|_| protocol())?,
        ),
        Err(e) => return Err(e),
    };
    if versions.len() != 1 {
        return Err(protocol());
    }
    let version = versions.into_iter().next().ok_or_else(protocol)?;
    let revision =
        tiktools_collaboration::stream_sequence(&version.revision).map_err(|_| protocol())?;
    if version.kind != ResourceKind::Automation
        || version.scope != family.scope()
        || version.id != event.resource.id
        || revision
            < tiktools_collaboration::stream_sequence(&event.resource.revision)
                .map_err(|_| protocol())?
    {
        return Err(protocol());
    }
    let row = value.map(|value| row(remote, value, version)).transpose()?;
    let key = (family, event.resource.id.clone());
    let mut v = shared.view.write().unwrap_or_else(|p| p.into_inner());
    if v.status.epoch != epoch
        || v.remote
            .as_ref()
            .is_none_or(|r| r.hello().connection_id != remote.hello().connection_id)
    {
        return Ok(());
    }
    if v.automation
        .rows
        .get(&key)
        .is_some_and(|r| r.version.revision.parse::<u64>().unwrap_or(0) > revision)
    {
        return Ok(());
    }
    if v.automation.rows.get(&key) == row.as_ref() {
        return Ok(());
    }
    if let Some(row) = row {
        v.automation.rows.insert(key, row);
    } else {
        v.automation.rows.remove(&key);
    }
    // Count the retained projection, not arbitrary owner definition payloads.
    if projection_bytes(&v.automation.rows) > SNAPSHOT_BYTES {
        clear(&mut v);
        shared.changed.send_modify(|n| *n = n.wrapping_add(1));
        return Err(fail(
            "too_large",
            "automation projection exceeds its byte budget",
        ));
    }
    notify(&mut v);
    shared.changed.send_modify(|n| *n = n.wrapping_add(1));
    Ok(())
}
impl Controller {
    pub fn automations(
        &self,
        epoch: &str,
        revision: &str,
        offset: usize,
    ) -> Result<Value, ClientError> {
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch || v.status.automation_revision != revision {
            return Err(fail("stale_view", "automation view changed"));
        }
        if v.status.phase != "ready"
            || v.remote
                .as_ref()
                .is_none_or(|r| r.is_closed() || r.requires_resync() || !readable(r))
        {
            return Err(fail("not_ready", "automation view is recovering"));
        }
        if offset > v.automation.rows.len() {
            return Err(fail("invalid_params", "invalid automation page"));
        }
        let rows: Vec<_> = v.automation.rows.values().skip(offset).take(32).collect();
        let next = offset + rows.len();
        Ok(
            json!({"epoch":epoch,"automationRevision":revision,"rows":rows,"next":if next<v.automation.rows.len(){Some(next)}else{None}}),
        )
    }
}
pub(super) fn event_row<'a>(v: &'a View, id: &str) -> Option<&'a AutomationRow> {
    v.automation.rows.get(&(AutomationFamily::Event, id.into()))
}

fn projection_bytes(rows: &BTreeMap<(AutomationFamily, String), AutomationRow>) -> usize {
    serde_json::to_vec(&rows.values().collect::<Vec<_>>()).map_or(usize::MAX, |b| b.len())
}
