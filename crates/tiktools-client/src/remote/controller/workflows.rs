//! Bounded workflow pages and version-checked reliable event projection.
use super::*;

impl Controller {
    pub fn workflows(
        &self,
        epoch: &str,
        revision: &str,
        offset: usize,
    ) -> Result<Value, ClientError> {
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch || v.status.view_revision != revision {
            return Err(fail("stale_view", "controller view changed; read it again"));
        }
        let Some(remote) = &v.remote else {
            return Err(fail("not_ready", "remote controller is not ready"));
        };
        if v.status.phase != "ready" || remote.is_closed() || remote.requires_resync() {
            return Err(fail("not_ready", "remote controller is recovering"));
        }
        if offset > v.rows.len() {
            return Err(fail("invalid_params", "invalid controller page"));
        }
        let mut rows = Vec::new();
        let mut bytes = 0;
        for row in v.rows.values().skip(offset).take(32) {
            let size = serde_json::to_vec(row).map_err(|_| protocol())?.len();
            if bytes + size > 512 * 1024 {
                break;
            }
            bytes += size;
            rows.push(row.clone());
        }
        if rows.is_empty() && offset < v.rows.len() {
            return Err(fail("too_large", "workflow exceeds controller page budget"));
        }
        let next = offset + rows.len();
        Ok(
            json!({"epoch":epoch,"viewRevision":revision,"rows":rows,"next":if next<v.rows.len(){Some(next)}else{None}}),
        )
    }
}

pub(super) async fn apply(
    shared: &Shared,
    epoch: &str,
    remote: &RemoteClient,
    event: &ReliableEvent,
) -> Result<(), ClientError> {
    if event.resource.kind != ResourceKind::Workflow
        || event.resource.scope != "host"
        || event.topic != "workflow.changed"
    {
        return Err(protocol());
    }
    let result = remote
        .call_value_with_meta("workflows.get", json!({"id":event.resource.id}), None)
        .await;
    let (graph, versions) = match result {
        Ok(reply) => (
            Some(reply.result),
            reply.meta.ok_or_else(protocol)?.resource_versions,
        ),
        Err(e) if e.code == "workflow_not_found" => (
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
    let version = versions
        .into_iter()
        .find(|v| {
            v.kind == ResourceKind::Workflow && v.scope == "host" && v.id == event.resource.id
        })
        .ok_or_else(protocol)?;
    if graph
        .as_ref()
        .is_some_and(|g| g.get("id").and_then(Value::as_str) != Some(version.id.as_str()))
    {
        return Err(protocol());
    }
    let revision =
        tiktools_collaboration::stream_sequence(&version.revision).map_err(|_| protocol())?;
    if revision
        < tiktools_collaboration::stream_sequence(&event.resource.revision)
            .map_err(|_| protocol())?
    {
        return Err(protocol());
    }
    let version = ResourceVersion {
        kind: ResourceKind::Workflow,
        id: version.id,
        scope: version.scope,
        revision: version.revision,
    };
    let mut over = false;
    shared.change(epoch, |v| {
        if v.rows
            .get(&version.id)
            .is_some_and(|r| r.version.revision.parse::<u64>().unwrap_or(0) > revision)
        {
            return;
        }
        match graph {
            Some(graph) => {
                v.rows
                    .insert(version.id.clone(), WorkflowRow { graph, version });
            }
            None => {
                v.rows.remove(&version.id);
            }
        }
        let size = serde_json::to_vec(&v.rows).map_or(usize::MAX, |v| v.len());
        if size > SNAPSHOT_BYTES {
            v.rows.clear();
            over = true;
        }
    });
    if over {
        Err(fail(
            "too_large",
            "controller projection exceeds its byte budget",
        ))
    } else {
        Ok(())
    }
}
