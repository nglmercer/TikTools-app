//! Account-bound, local-only comparisons and explicit cached-variable adoption.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{OperationError, RuntimeVariable};
use crate::AppCore;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VariableSyncChange {
    /// Native comparison reference, never a value or a credential.
    pub review: String,
    pub name: String,
    pub local: Option<RuntimeVariable>,
    pub cloud: Option<RuntimeVariable>,
    pub cloud_deleted: bool,
    pub local_secret: bool,
    pub blocked_local: bool,
    pub pending_status: Option<String>,
    pub can_send: bool,
    pub can_apply: bool,
    pub can_withdraw: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VariableSyncReview {
    pub changes: Vec<VariableSyncChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariableSyncApply {
    pub review: String,
}

#[cfg(feature = "persistence")]
mod native {
    use super::*;
    use crate::control::{RuntimeVariableKind, RuntimeVariablesReplace};
    use cloud_client::{
        Session, WorkspaceEdit, WorkspaceEntry, WorkspaceKey, WorkspaceRecord, WorkspaceStore,
    };
    use std::{
        collections::{BTreeMap, BTreeSet, VecDeque},
        sync::Arc,
        time::{Duration, Instant},
    };
    use tiktools_cloud_sync::V2LocalEditContext;

    const MAX_REVIEW_BYTES: usize = 16 * 1024 * 1024;
    const MAX_REVIEWS: usize = 4096;
    const REVIEW_TTL: Duration = Duration::from_secs(300);

    #[derive(Clone)]
    pub(crate) struct Review {
        token: String,
        until: Instant,
        bytes: usize,
        session: Arc<Session>,
        context: V2LocalEditContext,
        local_revision: String,
        name: String,
        entry: Option<WorkspaceEntry>,
        can_apply: bool,
    }

    #[derive(Default)]
    pub(crate) struct VariableReviewCache(VecDeque<Review>);

    fn stale() -> OperationError {
        OperationError::conflict("variable comparison changed or expired; reload changes")
    }

    fn with_reviewed_local<R>(
        core: &AppCore,
        review: &Review,
        operation: impl FnOnce(
            &super::super::RuntimeVariablesSnapshot,
            &tiktools_cloud_sync::SyncCoordinator,
        ) -> Result<R, OperationError>,
    ) -> Result<R, OperationError> {
        let _emission = tiktools_cloud_sync::ProductEmissionFence::open(
            core.db.paths().data.join("plugin-sync-emission"),
        )
        .map_err(|_| stale())?
        .try_emission()
        .map_err(|_| stale())?;
        if core
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(stale());
        }
        let coordinator = core.sync.coordinator().map_err(|_| stale())?;
        let lease = coordinator.product_read().map_err(|_| stale())?;
        if lease.v2_context(&review.session).map_err(|_| stale())? != review.context {
            return Err(stale());
        }
        let local = core.runtime_variables_list()?;
        if local.revision != review.local_revision {
            return Err(stale());
        }
        lease.check_current().map_err(|_| stale())?;
        operation(&local, coordinator)
    }

    /// Only authorizes cancelling the compared queue entry. It has no upload
    /// implementation, including when that entry now belongs to a local secret.
    struct ReviewedDiscardSource {
        core: Arc<AppCore>,
        review: Review,
    }
    impl tiktools_cloud_sync::V2ProductUploadSource for ReviewedDiscardSource {
        fn with_fenced_owned_discard(
            &self,
            target: &WorkspaceKey,
            _: &WorkspaceEdit,
            admit: &mut dyn FnMut() -> Result<(), tiktools_cloud_sync::WorkspaceCoordError>,
        ) -> Result<(), tiktools_cloud_sync::WorkspaceCoordError> {
            if target.collection != "variables" || target.key != self.review.name {
                return Err(tiktools_cloud_sync::WorkspaceCoordError::Ineligible);
            }
            with_reviewed_local(&self.core, &self.review, |_, _| {
                admit().map_err(|_| stale())
            })
            .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)
        }
        fn with_fenced_upload(
            &self,
            _: &tiktools_cloud_sync::V2ProductDecision,
            _: &mut dyn FnMut() -> Result<(), tiktools_cloud_sync::WorkspaceCoordError>,
        ) -> Result<(), tiktools_cloud_sync::WorkspaceCoordError> {
            Err(tiktools_cloud_sync::WorkspaceCoordError::Ineligible)
        }
    }

    impl VariableReviewCache {
        fn render(
            &mut self,
            rows: Vec<(VariableSyncChange, Review)>,
        ) -> Result<VariableSyncReview, OperationError> {
            self.0.retain(|review| review.until > Instant::now());
            let needed: usize = rows.iter().map(|(_, review)| review.bytes).sum();
            if needed > MAX_REVIEW_BYTES || rows.len() > MAX_REVIEWS {
                return Err(OperationError::unavailable(
                    "variable comparisons exceed the review budget",
                ));
            }
            let mut retained = BTreeSet::new();
            let mut ready = Vec::new();
            let mut output = Vec::new();
            for (mut change, mut review) in rows {
                if let Some(existing) = self.0.iter_mut().find(|existing| {
                    Arc::ptr_eq(&existing.session, &review.session)
                        && existing.context == review.context
                        && existing.local_revision == review.local_revision
                        && existing.name == review.name
                        && existing.entry == review.entry
                }) {
                    existing.until = review.until;
                    retained.insert(existing.token.clone());
                    change.review = existing.token.clone();
                } else {
                    let mut bytes = [0u8; 32];
                    getrandom::fill(&mut bytes).map_err(|_| {
                        OperationError::unavailable("variable review identity is unavailable")
                    })?;
                    review.token = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                    change.review = review.token.clone();
                    ready.push(review);
                }
                retained.insert(change.review.clone());
                output.push(change);
            }
            let mut bytes: usize = self.0.iter().map(|review| review.bytes).sum::<usize>()
                + ready.iter().map(|review| review.bytes).sum::<usize>();
            while bytes > MAX_REVIEW_BYTES || self.0.len() + ready.len() > MAX_REVIEWS {
                let index = self
                    .0
                    .iter()
                    .position(|review| !retained.contains(&review.token))
                    .ok_or_else(|| {
                        OperationError::unavailable("variable comparisons exceed the review budget")
                    })?;
                bytes = bytes.saturating_sub(self.0.remove(index).ok_or_else(stale)?.bytes);
            }
            self.0.extend(ready);
            Ok(VariableSyncReview { changes: output })
        }

        fn take(&mut self, token: &str) -> Result<Review, OperationError> {
            self.0.retain(|review| review.until > Instant::now());
            let index = self
                .0
                .iter()
                .position(|review| review.token == token)
                .ok_or_else(stale)?;
            self.0.remove(index).ok_or_else(stale)
        }
    }

    pub(crate) fn portable(name: &str, variable: &RuntimeVariable) -> Option<WorkspaceEdit> {
        if variable.kind == RuntimeVariableKind::Secret {
            return None;
        }
        let key = WorkspaceKey::new("variables", name).ok()?;
        let edit = WorkspaceEdit::Put(
            serde_json::json!({"schemaVersion":1,"name":name,"type":variable.kind,"value":variable.value}),
        );
        edit.valid(&key).then_some(edit)
    }

    fn cloud_variable(
        name: &str,
        record: Option<&WorkspaceRecord>,
    ) -> Result<Option<RuntimeVariable>, OperationError> {
        let Some(record) = record.filter(|record| !record.deleted) else {
            return Ok(None);
        };
        let doc = record.value.as_ref().ok_or_else(stale)?;
        let key = WorkspaceKey::new("variables", name).map_err(|_| stale())?;
        if !WorkspaceEdit::Put(doc.clone()).valid(&key) {
            return Err(stale());
        }
        // The validated native cache has not crossed JavaScript. Preserve its
        // exact number directly; the local RPC codec adds browser metadata.
        let kind = serde_json::from_value(doc["type"].clone()).map_err(|_| stale())?;
        Ok(Some(RuntimeVariable {
            kind,
            value: doc["value"].clone(),
        }))
    }

    impl AppCore {
        fn take_variable_review(
            &self,
            params: VariableSyncApply,
        ) -> Result<Review, OperationError> {
            self.sync
                .variable_reviews
                .lock()
                .map_err(|_| stale())?
                .take(&params.review)
        }

        async fn clear_reviewed_variable_pending(
            self: &Arc<Self>,
            review: &mut Review,
            clear_queued: bool,
        ) -> Result<(), OperationError> {
            let core = self.clone();
            let checked = review.clone();
            tokio::task::spawn_blocking(move || {
                with_reviewed_local(&core, &checked, |_, _| {
                    core.cloud
                        .auth()
                        .with_current_session_permit(&checked.session, || ())
                        .map_err(|_| stale())
                })
            })
            .await
            .map_err(|_| stale())??;
            let Some(pending) = review
                .entry
                .as_ref()
                .and_then(|entry| entry.pending.as_ref())
            else {
                return Ok(());
            };
            let target = WorkspaceKey::new("variables", &review.name).map_err(|_| stale())?;
            if pending.status == cloud_client::WorkspacePendingStatus::Queued {
                if !clear_queued {
                    return Ok(());
                }
                let core = self.clone();
                let checked = review.clone();
                let entry = tokio::task::spawn_blocking(move || {
                    with_reviewed_local(&core, &checked, |_, _| {
                        let store = WorkspaceStore::open(
                            core.db.paths().data.join("cloud-sync"),
                            &checked.context.origin,
                            &checked.context.account_id,
                        )
                        .map_err(|_| stale())?;
                        let _operation = store
                            .try_operation_lock()
                            .map_err(|_| stale())?
                            .ok_or_else(stale)?;
                        let mut snapshot = store.load().map_err(|_| stale())?.ok_or_else(stale)?;
                        if !snapshot.state.bootstrap_complete
                            || snapshot.state.needs_rebuild
                            || snapshot.state.entry(&target) != checked.entry.as_ref()
                        {
                            return Err(stale());
                        }
                        let proof = snapshot
                            .state
                            .product_queued_proof(&target)
                            .ok_or_else(stale)?;
                        snapshot
                            .state
                            .discard_product_queued(&proof)
                            .map_err(|_| stale())?;
                        core.cloud
                            .auth()
                            .with_current_session_permit(&checked.session, || {
                                store
                                    .save(Some(snapshot.generation), &snapshot.state)
                                    .map_err(|_| stale())?;
                                Ok(snapshot.state.entry(&target).cloned())
                            })
                            .map_err(|_| stale())?
                    })
                })
                .await
                .map_err(|_| stale())??;
                review.entry = entry;
            } else {
                crate::control::cloud_variables::require_variables(&review.context.origin).await?;
                let coordinator = self.sync_coordinator()?.clone();
                let snapshot = coordinator
                    .review_v2_product(review.session.clone(), target.clone())
                    .await
                    .map_err(|_| stale())?;
                if Some(&snapshot.entry) != review.entry.as_ref()
                    || snapshot.cloud_record.as_ref()
                        != review
                            .entry
                            .as_ref()
                            .and_then(|entry| entry.remote.as_ref())
                {
                    return Err(stale());
                }
                let remote = snapshot.cloud_record.clone();
                coordinator
                    .resolve_v2_product(
                        review.session.clone(),
                        Arc::new(ReviewedDiscardSource {
                            core: self.clone(),
                            review: review.clone(),
                        }),
                        target.clone(),
                        snapshot,
                        false,
                    )
                    .await
                    .map_err(|_| stale())?;
                review.entry = remote.map(|remote| WorkspaceEntry {
                    target,
                    remote: Some(remote),
                    pending: None,
                    next: None,
                });
            }
            Ok(())
        }

        /// The native variable is re-read, never accepted from a page's value.
        pub async fn sync_v2_variables_send(
            self: &Arc<Self>,
            params: VariableSyncApply,
        ) -> Result<super::super::WorkspaceV2SyncResult, OperationError> {
            self.publish_reviewed_variable(params, false).await
        }

        /// Withdraws only the Cloud copy; the local value and secret type stay.
        pub async fn sync_v2_variables_withdraw(
            self: &Arc<Self>,
            params: VariableSyncApply,
        ) -> Result<super::super::WorkspaceV2SyncResult, OperationError> {
            self.publish_reviewed_variable(params, true).await
        }

        async fn publish_reviewed_variable(
            self: &Arc<Self>,
            params: VariableSyncApply,
            withdraw: bool,
        ) -> Result<super::super::WorkspaceV2SyncResult, OperationError> {
            let mut review = self.take_variable_review(params)?;
            // Reject stale identity/local state before capability requests or recovery barriers.
            let core = self.clone();
            let checked = review.clone();
            tokio::task::spawn_blocking(move || {
                with_reviewed_local(&core, &checked, |local, _| {
                    if !withdraw
                        && local
                            .variables
                            .get(&checked.name)
                            .and_then(|value| portable(&checked.name, value))
                            .is_none()
                    {
                        return Err(OperationError::invalid(
                            "local variable cannot be published",
                        ));
                    }
                    core.cloud
                        .auth()
                        .with_current_session_permit(&checked.session, || ())
                        .map_err(|_| stale())
                })
            })
            .await
            .map_err(|_| stale())??;
            if withdraw
                && !review.entry.as_ref().is_some_and(|entry| {
                    entry.pending.is_some()
                        || entry.remote.as_ref().is_some_and(|record| !record.deleted)
                })
            {
                return Err(stale());
            }
            crate::control::cloud_variables::require_variables(&review.context.origin).await?;
            self.clear_reviewed_variable_pending(&mut review, false)
                .await?;
            let target = WorkspaceKey::new("variables", &review.name).map_err(|_| stale())?;
            let scope = tiktools_cloud_sync::WorkspaceSyncScope::Records([target.clone()].into());
            let binding = (review.session.clone(), review.context.clone());
            let core = self.clone();
            tokio::task::spawn_blocking(move || {
                with_reviewed_local(&core, &review, |local, coordinator| {
                    let edit = if withdraw {
                        WorkspaceEdit::Delete
                    } else {
                        local
                            .variables
                            .get(&review.name)
                            .and_then(|value| portable(&review.name, value))
                            .ok_or_else(stale)?
                    };
                    coordinator
                        .stage_v2_product_edit_reviewed(
                            review.session.clone(),
                            &review.context,
                            target,
                            edit,
                            review.entry.as_ref(),
                        )
                        .map_err(|_| stale())?;
                    Ok(())
                })
            })
            .await
            .map_err(|_| stale())??;
            self.sync_v2_scoped_bound(false, scope, Some(binding)).await
        }

        pub async fn sync_v2_variables_review(
            self: &Arc<Self>,
        ) -> Result<VariableSyncReview, OperationError> {
            let core = self.clone();
            self.v2_workspace_read(move |coordinator, session| {
                let read = coordinator
                    .product_read()
                    .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?;
                let context = read
                    .v2_context(session)
                    .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?;
                let local = core
                    .runtime_variables_list()
                    .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?;
                let snapshot = WorkspaceStore::open(
                    core.db.paths().data.join("cloud-sync"),
                    context.origin.clone(),
                    context.account_id.clone(),
                )?
                .load()?
                .filter(|snapshot| {
                    snapshot.state.bootstrap_complete
                        && snapshot.state.is_product_guarded()
                        && !snapshot.state.needs_rebuild
                })
                .ok_or(tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?;
                let entries = snapshot.state.entries;
                let entries: BTreeMap<_, _> = entries
                    .into_iter()
                    .filter(|entry| entry.target.collection == "variables")
                    .map(|entry| (entry.target.key.clone(), entry))
                    .collect();
                let names: BTreeSet<_> = local
                    .variables
                    .keys()
                    .chain(entries.keys())
                    .cloned()
                    .collect();
                let mut rows = Vec::new();
                for name in names {
                    let entry = entries.get(&name);
                    let variable = local.variables.get(&name);
                    let secret = variable
                        .is_some_and(|variable| variable.kind == RuntimeVariableKind::Secret);
                    let edit = variable.and_then(|variable| portable(&name, variable));
                    let blocked = variable.is_some() && edit.is_none() && !secret;
                    let remote = entry.and_then(|entry| entry.remote.as_ref());
                    let pending = entry.and_then(|entry| entry.pending.as_ref());
                    if pending.is_none()
                        && ((variable.is_none() && remote.is_none_or(|record| record.deleted))
                            || (secret && remote.is_none_or(|record| record.deleted))
                            || (edit.as_ref().is_some_and(|edit| {
                                remote.is_some_and(|record| edit.matches(record))
                            })))
                    {
                        continue;
                    }
                    let cloud = if secret || blocked {
                        None
                    } else {
                        cloud_variable(&name, remote)
                            .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?
                    };
                    let local_value = if edit.is_some() {
                        variable.cloned()
                    } else {
                        None
                    };
                    let can_apply = remote.is_some() && !secret && !blocked;
                    let change = VariableSyncChange {
                        review: String::new(),
                        name: name.clone(),
                        local: local_value,
                        cloud,
                        cloud_deleted: remote.is_some_and(|record| record.deleted),
                        local_secret: secret,
                        blocked_local: blocked,
                        pending_status: pending.map(|pending| {
                            match pending.status {
                                cloud_client::WorkspacePendingStatus::Queued => "queued",
                                cloud_client::WorkspacePendingStatus::Conflict => "conflict",
                                cloud_client::WorkspacePendingStatus::Unconfirmed => "unconfirmed",
                            }
                            .to_owned()
                        }),
                        can_send: edit.is_some(),
                        can_apply,
                        can_withdraw: remote.is_some_and(|record| !record.deleted)
                            || pending.is_some(),
                    };
                    let bytes = serde_json::to_vec(&entry)
                        .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?
                        .len()
                        + serde_json::to_vec(&change)
                            .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?
                            .len()
                        + 4096;
                    let review = Review {
                        token: String::new(),
                        until: Instant::now() + REVIEW_TTL,
                        bytes,
                        session: session.clone(),
                        context: context.clone(),
                        local_revision: local.revision.clone(),
                        name,
                        entry: entry.cloned(),
                        can_apply,
                    };
                    rows.push((change, review));
                }
                core.sync
                    .variable_reviews
                    .lock()
                    .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?
                    .render(rows)
                    .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)
            })
            .await
        }

        pub async fn sync_v2_variables_apply(
            self: &Arc<Self>,
            params: VariableSyncApply,
        ) -> Result<(), OperationError> {
            let mut review = self.take_variable_review(params)?;
            if !review.can_apply {
                return Err(OperationError::invalid(
                    "this comparison cannot be applied; review its pending change or local secret",
                ));
            }
            self.clear_reviewed_variable_pending(&mut review, true)
                .await?;
            let core = self.clone();
            tokio::task::spawn_blocking(move || {
                let coordinator = core.sync.coordinator().map_err(|_| stale())?;
                let lease = coordinator.product_read().map_err(|_| stale())?;
                if lease.v2_context(&review.session).map_err(|_| stale())? != review.context {
                    return Err(stale());
                }
                let store = WorkspaceStore::open(
                    core.db.paths().data.join("cloud-sync"),
                    review.context.origin.clone(),
                    review.context.account_id.clone(),
                )
                .map_err(|_| stale())?;
                store
                    .try_with_locked_snapshot::<_, cloud_client::StoreError>(|snapshot| {
                        Ok((|| -> Result<(), OperationError> {
                            let snapshot = snapshot.ok_or_else(stale)?;
                            let key = WorkspaceKey::new("variables", review.name.clone())
                                .map_err(|_| stale())?;
                            if !snapshot.state.bootstrap_complete
                                || !snapshot.state.is_product_guarded()
                                || snapshot.state.needs_rebuild
                                || snapshot.state.entry(&key) != review.entry.as_ref()
                            {
                                return Err(stale());
                            }
                            let remote = review
                                .entry
                                .as_ref()
                                .and_then(|entry| entry.remote.as_ref())
                                .ok_or_else(stale)?;
                            let variable = cloud_variable(&review.name, Some(remote))?;
                            let auth = core.cloud.auth();
                            auth.with_current_session_permit(&review.session, || {
                                let mut local = core.runtime_variables_list()?;
                                if local.revision != review.local_revision
                                    || local.variables.get(&review.name).is_some_and(|variable| {
                                        variable.kind == RuntimeVariableKind::Secret
                                    })
                                {
                                    return Err(stale());
                                }
                                match variable {
                                    Some(variable) => {
                                        local.variables.insert(review.name, variable);
                                    }
                                    None => {
                                        local.variables.remove(&review.name);
                                    }
                                }
                                core.runtime_variables_replace(RuntimeVariablesReplace {
                                    expected_revision: review.local_revision,
                                    variables: local.variables,
                                })?;
                                Ok(())
                            })
                            .map_err(|_| stale())?
                        })())
                    })
                    .map_err(|_| stale())?
                    .ok_or_else(stale)?
            })
            .await
            .map_err(|_| {
                OperationError::unavailable("variable application worker is unavailable")
            })?
        }
    }
}

#[cfg(feature = "persistence")]
pub(crate) use native::VariableReviewCache;

#[cfg(not(feature = "persistence"))]
impl AppCore {
    pub async fn sync_v2_variables_send(
        self: &std::sync::Arc<Self>,
        _: VariableSyncApply,
    ) -> Result<super::WorkspaceV2SyncResult, OperationError> {
        Err(OperationError::unavailable(
            "variable sync requires persistence",
        ))
    }
    pub async fn sync_v2_variables_withdraw(
        self: &std::sync::Arc<Self>,
        _: VariableSyncApply,
    ) -> Result<super::WorkspaceV2SyncResult, OperationError> {
        Err(OperationError::unavailable(
            "variable sync requires persistence",
        ))
    }
    pub async fn sync_v2_variables_review(
        self: &std::sync::Arc<Self>,
    ) -> Result<VariableSyncReview, OperationError> {
        Err(OperationError::unavailable(
            "variable sync requires persistence",
        ))
    }
    pub async fn sync_v2_variables_apply(
        self: &std::sync::Arc<Self>,
        _: VariableSyncApply,
    ) -> Result<(), OperationError> {
        Err(OperationError::unavailable(
            "variable sync requires persistence",
        ))
    }
}
