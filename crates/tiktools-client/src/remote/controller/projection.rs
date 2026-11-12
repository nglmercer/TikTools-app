//! Epoch-bound shared state, permission-filtered journal views and change notifications.
use super::*;

impl Shared {
    pub(super) fn change(&self, epoch: &str, apply: impl FnOnce(&mut View)) {
        let mut v = self.view.write().unwrap_or_else(|p| p.into_inner());
        if v.status.epoch != epoch {
            return;
        }
        apply(&mut v);
        if v.competition_observation.as_ref().is_some_and(|o| {
            v.status.phase != "ready"
                || v.status.epoch != o.epoch
                || v.status.hello.as_ref().is_none_or(|h| {
                    h.connection_id != o.connection
                        || h.grant_revision != o.grant_revision
                        || h.host_instance_id != o.host_instance
                })
        }) {
            v.competition_observation = None;
        }
        let revision = v
            .status
            .view_revision
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_add(1);
        v.status.view_revision = revision.to_string();
        self.changed.send_modify(|n| *n = n.wrapping_add(1));
    }

    pub(super) fn phase(&self, epoch: &str, phase: &str, error: Option<&ClientError>) {
        self.change(epoch, |v| {
            v.status.phase = phase.into();
            v.status.error_code = error.map(|e| e.code.clone());
        });
    }

    pub(super) fn ready(
        &self,
        epoch: &str,
        remote: &RemoteClient,
        baseline: Option<&SnapshotPage>,
    ) -> Result<(), ClientError> {
        let mut result = Ok(());
        self.change(epoch, |v| {
            if !automations::readable(remote) {
                automations::clear(v);
            }
            if let Some(page) = baseline {
                if let Err(error) = automations::baseline(v, remote, page) {
                    result = Err(error);
                    return;
                }
                let versions: BTreeMap<_, _> = page
                    .resource_versions
                    .iter()
                    .filter(|v| v.kind == ResourceKind::Workflow)
                    .map(|v| (v.id.as_str(), v))
                    .collect();
                v.rows = page
                    .workflows
                    .iter()
                    .filter_map(|graph| {
                        let id = graph.get("id")?.as_str()?;
                        let version = versions.get(id)?;
                        Some((
                            id.to_owned(),
                            WorkflowRow {
                                graph: graph.clone(),
                                version: (*version).clone(),
                            },
                        ))
                    })
                    .collect();
            }
            v.remote = Some(remote.clone());
            v.status.hello = Some(remote.hello().clone());
            v.status.phase = "ready".into();
            v.status.pairing_request_id = None;
            v.status.error_code = None;
        });
        result
    }
}

impl Controller {
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.shared.changed.subscribe()
    }

    pub fn status(&self) -> ControllerStatus {
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        let mut status = v.status.clone();
        status.command_journal = journal_projection(&v);
        if status.phase == "ready"
            && v.remote
                .as_ref()
                .is_some_and(|r| r.is_closed() || r.requires_resync())
        {
            status.phase = "reconnecting".into();
        }
        status
    }
}

pub(super) fn journal_projection(v: &View) -> Option<JournalView> {
    let mut journal = v.journal.as_ref()?.view();
    let hello = v.status.hello.as_ref().filter(|h| {
        v.status.phase == "ready"
            && v.remote
                .as_ref()
                .is_some_and(|r| !r.is_closed() && !r.requires_resync())
            && journal.binding.host_id == h.host_id
            && journal.binding.principal_id == h.principal_id
            && journal.binding.session_id == h.session_id
            && Some(journal.binding.origin.as_str()) == v.status.origin.as_deref()
    });
    for record in &mut journal.recent {
        let allowed = hello.is_some_and(|h| {
            h.capabilities.contains(&Capability::PointsRead)
                && !h.resource_restrictions.contains_key(&ResourceKind::Live)
                && h.resource_restrictions
                    .get(&ResourceKind::Viewer)
                    .is_none_or(|ids| ids.contains(&record.intent.resource_id))
        });
        if !allowed {
            record.points_result = None;
        }
    }
    journal.recent.retain(|record| {
        !record.intent.operation.is_competition()
            || hello.is_some_and(|h| {
                h.capabilities.contains(&Capability::CompetitionsRead)
                    && !h.resource_restrictions.contains_key(&ResourceKind::Live)
                    && h.resource_restrictions
                        .get(&ResourceKind::Competition)
                        .is_none_or(|ids| ids.contains(&record.intent.resource_id))
            })
    });
    Some(journal)
}
