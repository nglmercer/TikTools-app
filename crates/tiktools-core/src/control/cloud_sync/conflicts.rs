//! Current-account workspace lists and one-time explicit conflict decisions.
use super::OperationError;
#[cfg(feature = "persistence")]
use super::{
    projection::{v2_value_from_edit, v2_value_from_record},
    v2_change_entry, V2ConflictHandle,
};
use super::{
    V2ChangeEntry, V2ConflictChoice, V2ConflictOutcome, V2ConflictReview, V2ConflictTarget,
};
use crate::AppCore;

impl AppCore {
    /// Local-only list of v2 conflicts for the current account.
    pub async fn sync_v2_conflict_list(
        self: &std::sync::Arc<Self>,
    ) -> Result<Vec<V2ConflictTarget>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let keys = self
                .v2_workspace_read(|coordinator, session| {
                    coordinator.v2_product_conflict_keys(session)
                })
                .await?;
            Ok(keys
                .into_iter()
                .map(|target| V2ConflictTarget {
                    collection: target.collection,
                    key: target.key,
                })
                .collect())
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "Cloud v2 conflicts require persistence",
            ))
        }
    }

    /// Local-only read of the durable v2 workspace for the current account:
    /// cached Cloud values and pending edits, so a UI can show what differs
    /// without contacting Cloud or entering a conflict review.
    pub async fn sync_v2_changes_list(
        self: &std::sync::Arc<Self>,
    ) -> Result<Vec<V2ChangeEntry>, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let core = self.clone();
            self.v2_workspace_read(move |coordinator, session| {
                let entries = coordinator.v2_product_entries(session)?;
                let secrets = if entries
                    .iter()
                    .any(|entry| entry.target.collection == "variables")
                {
                    core.runtime_variable_secret_names()
                        .map_err(|_| tiktools_cloud_sync::WorkspaceCoordError::Ineligible)?
                } else {
                    std::collections::BTreeSet::new()
                };
                Ok(entries
                    .iter()
                    .map(|entry| {
                        let mut projected = v2_change_entry(entry);
                        if projected.collection == "variables" && secrets.contains(&projected.key) {
                            projected.remote.value = None;
                            if let Some(pending) = &mut projected.pending {
                                pending.base.value = None;
                                pending.local.value = None;
                            }
                        }
                        projected
                    })
                    .collect())
            })
            .await
        }
        #[cfg(not(feature = "persistence"))]
        {
            Err(OperationError::unavailable(
                "Cloud v2 changes require persistence",
            ))
        }
    }

    /// Capture one v2 conflict comparison. The handle is one-time; resolution
    /// re-reads Cloud and requires the same comparison.
    pub async fn sync_v2_review_conflict(
        self: &std::sync::Arc<Self>,
        collection: &str,
        key: &str,
    ) -> Result<V2ConflictReview, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let (session, _) = self.v2_identity().await?;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let target = cloud_client::WorkspaceKey::new(collection.to_owned(), key.to_owned())
                .map_err(|_| OperationError::not_found("unknown Cloud conflict"))?;
            let snapshot = coordinator
                .review_v2_product(session.clone(), target)
                .await
                .map_err(|_| OperationError::conflict("Cloud conflict changed; review again"))?;
            let redact = if collection == "variables" {
                let core = self.clone();
                let name = key.to_owned();
                tokio::task::spawn_blocking(move || {
                    core.runtime_variable_secret_names()
                        .map(|names| names.contains(&name))
                })
                .await
                .map_err(|_| {
                    OperationError::unavailable("variable review worker is unavailable")
                })??
            } else {
                false
            };
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            let entry = &snapshot.entry;
            let mut base = match entry.pending.as_ref() {
                Some(pending) => v2_value_from_record(pending.base_record.as_ref()),
                None => v2_value_from_record(entry.remote.as_ref()),
            };
            let mut local = match entry.pending.as_ref() {
                Some(pending) => v2_value_from_edit(&pending.edit),
                None => v2_value_from_record(entry.remote.as_ref()),
            };
            let mut cloud = v2_value_from_record(snapshot.cloud_record.as_ref());
            if redact {
                base.value = None;
                local.value = None;
                cloud.value = None;
            }
            let has_newer = entry.next.is_some();
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes)
                .map_err(|_| OperationError::internal("could not create review handle"))?;
            let token = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let review = V2ConflictReview {
                handle: token.clone(),
                collection: snapshot.target.collection.clone(),
                key: snapshot.target.key.clone(),
                base,
                local,
                cloud,
                has_newer,
            };
            {
                let mut slot = self.sync.v2_conflict.lock().await;
                *slot = Some(V2ConflictHandle {
                    token,
                    session,
                    snapshot,
                });
            }
            Ok(review)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (collection, key);
            Err(OperationError::unavailable(
                "Cloud v2 conflicts require persistence",
            ))
        }
    }

    /// Resolve one reviewed v2 conflict. Keep-local rebases onto the fresh
    /// Cloud base (still pending); take-cloud discards the pending edit.
    /// Neither choice applies Cloud state locally.
    pub async fn sync_v2_resolve_conflict(
        self: &std::sync::Arc<Self>,
        handle: &str,
        choice: V2ConflictChoice,
    ) -> Result<V2ConflictOutcome, OperationError> {
        #[cfg(feature = "persistence")]
        {
            let mut slot = self.sync.v2_conflict.lock().await;
            let item = slot
                .take()
                .ok_or_else(|| OperationError::conflict("review expired; review again"))?;
            if item.token != handle {
                *slot = Some(item);
                return Err(OperationError::conflict("review expired; review again"));
            }
            let session = item.session.clone();
            let target = item.snapshot.target.clone();
            let expected = item.snapshot.clone();
            let keep_local = choice == V2ConflictChoice::KeepLocal;
            let coordinator = self.sync_coordinator()?.clone();
            let epoch = self.v2_epoch(&coordinator, &session)?;
            let source = std::sync::Arc::new(
                crate::cloud_sync::product_v2_outbox::OwnedSqliteV2UploadSource::new(
                    self.clone(),
                    session.instance_origin(),
                    session.account_id().to_owned(),
                ),
            );
            let result = coordinator
                .resolve_v2_product(session.clone(), source, target, expected, keep_local)
                .await;
            match result {
                Ok(()) => {}
                Err(_) => {
                    *slot = Some(item);
                    return Err(OperationError::conflict(
                        "Cloud conflict changed; review again",
                    ));
                }
            }
            self.cloud
                .auth()
                .with_current_session(&session, || {
                    coordinator.is_v2_config_epoch_current(&epoch, &session)
                })
                .map_err(|_| OperationError::conflict("Cloud account changed; try again"))?
                .then_some(())
                .ok_or_else(|| OperationError::conflict("Cloud account changed; try again"))?;
            Ok(V2ConflictOutcome::Resolved)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (handle, choice);
            Err(OperationError::unavailable(
                "Cloud v2 conflicts require persistence",
            ))
        }
    }
}
