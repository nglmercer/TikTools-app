//! Local-only draft bridge; scope is checked again after blocking file I/O.
use super::*;
use profiles::drafts::Draft;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Read {
    pub epoch: String,
    pub view_revision: String,
    #[serde(default)]
    pub offset: usize,
    pub id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Change {
    pub epoch: String,
    pub view_revision: String,
    pub id: String,
    pub expected_store_revision: String,
    pub base_revision: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub acknowledge_storage: bool,
}
fn selected(
    shared: &Shared,
    epoch: &str,
    revision: &str,
    id: Option<&str>,
) -> Result<Arc<Journal>, ClientError> {
    let v = shared.view.read().unwrap_or_else(|p| p.into_inner());
    selected_view(&v, epoch, revision, id)
}
fn selected_view(
    v: &View,
    epoch: &str,
    revision: &str,
    id: Option<&str>,
) -> Result<Arc<Journal>, ClientError> {
    if v.status.epoch != epoch || v.status.view_revision != revision {
        return Err(fail("stale_view", "controller view changed"));
    }
    let r = v
        .remote
        .as_ref()
        .filter(|r| v.status.phase == "ready" && !r.is_closed() && !r.requires_resync())
        .ok_or_else(|| {
            fail(
                "not_ready",
                "recover the controller before accessing drafts",
            )
        })?;
    let h = r.hello();
    if !h.capabilities.contains(&Capability::WorkflowsRead)
        || id.is_some_and(|id| {
            !profiles::drafts::valid_id(id)
                || h.resource_restrictions
                    .get(&ResourceKind::Workflow)
                    .is_some_and(|ids| !ids.contains(id))
        })
    {
        return Err(fail(
            "forbidden",
            "workflow draft is unavailable for this access",
        ));
    }
    let j = v
        .journal
        .clone()
        .ok_or_else(|| fail("not_ready", "draft profile is unavailable"))?;
    let b = j.view().binding;
    if b.host_id != h.host_id
        || b.principal_id != h.principal_id
        || b.session_id != h.session_id
        || Some(b.origin.as_str()) != v.status.origin.as_deref()
    {
        return Err(fail("stale_view", "draft profile changed"));
    }
    Ok(j)
}
impl Controller {
    pub(super) async fn read_drafts(&self, p: Read) -> Result<Value, ClientError> {
        let j = selected(&self.shared, &p.epoch, &p.view_revision, p.id.as_deref())?;
        let (revision, records) = tokio::task::spawn_blocking(move || j.drafts())
            .await
            .map_err(|_| profiles::drafts::invalid())??;
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        selected_view(&v, &p.epoch, &p.view_revision, p.id.as_deref())?;
        if let Some(id) = p.id {
            return Ok(
                json!({"epoch":p.epoch,"viewRevision":p.view_revision,"storeRevision":revision,"draft":records.into_iter().find(|d|d.id==id)}),
            );
        }
        let h = v.remote.as_ref().ok_or_else(network)?.hello();
        let rows:Vec<_>=records.iter().filter(|d|h.resource_restrictions.get(&ResourceKind::Workflow).is_none_or(|ids|ids.contains(&d.id)))
            .map(|d|json!({"id":d.id,"baseRevision":d.base_revision,"updatedAt":d.updated_at,"textBytes":d.text.len()})).collect();
        if p.offset > rows.len() {
            return Err(fail("invalid_params", "invalid draft page"));
        }
        let end = rows.len().min(p.offset.saturating_add(32));
        Ok(
            json!({"epoch":p.epoch,"viewRevision":p.view_revision,"storeRevision":revision,"rows":rows[p.offset..end],"next":if end<rows.len(){Some(end)}else{None}}),
        )
    }
    pub(super) async fn change_draft(&self, p: Change, delete: bool) -> Result<Value, ClientError> {
        let j = selected(&self.shared, &p.epoch, &p.view_revision, Some(&p.id))?;
        let draft = if delete {
            if p.text.is_some() || p.base_revision.is_some() {
                return Err(fail(
                    "invalid_params",
                    "draft deletion does not accept text",
                ));
            }
            None
        } else {
            if !p.acknowledge_storage {
                return Err(fail(
                    "confirmation_required",
                    "confirm saving this text on this PC, including any sensitive values",
                ));
            }
            Some(Draft {
                id: p.id.clone(),
                base_revision: p
                    .base_revision
                    .clone()
                    .ok_or_else(profiles::drafts::invalid)?,
                text: p.text.clone().ok_or_else(profiles::drafts::invalid)?,
                updated_at: profiles::commands::timestamp(),
            })
        };
        let expected = p.expected_store_revision.clone();
        let id = p.id.clone();
        let revision = tokio::task::spawn_blocking(move || j.change_draft(&expected, &id, draft))
            .await
            .map_err(|_| profiles::drafts::invalid())??;
        // A delayed save may finish after profile/permission changes. It remains
        // a local saved draft, never a host operation or a newly exposed body.
        selected(&self.shared, &p.epoch, &p.view_revision, Some(&p.id))?;
        Ok(json!({"epoch":p.epoch,"viewRevision":p.view_revision,"storeRevision":revision}))
    }
}
#[cfg(test)]
mod tests;
