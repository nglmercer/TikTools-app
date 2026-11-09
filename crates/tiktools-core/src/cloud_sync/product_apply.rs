//! Prepare durable local-application intents for pulled Cloud selections.
//!
//! This is a native building block only. It does not change persisted plugin
//! activation, start or stop code, acknowledge coordinator state, or expose
//! a product RPC. Recovery and every local mutation must share these fences
//! before the product can use this path.
use crate::{
    db::{DatabaseManager, PluginApplyIntent, PluginApplyPhase, PluginApplyReason},
    AppCore,
};
use tiktools_cloud_sync::{SyncCoordError, SyncCoordinator, V1ProductOutbox, V1RemoteApplyReview};

fn local_error() -> SyncCoordError {
    SyncCoordError::Local("plugin application requires a fresh local review".into())
}

/// The caller holds the real plugin invocation gate throughout this call.
/// The coordinator then acquires protocol admission and the exact live
/// session before the SQLite source and intent transaction is attempted.
pub(super) fn prepare_pulled(
    sync: &SyncCoordinator,
    db: &DatabaseManager,
    review: &V1RemoteApplyReview,
    outbox: &mut dyn V1ProductOutbox,
    installed_version: &str,
    activation_installed: bool,
    activation_enabled: bool,
) -> Result<Option<PluginApplyIntent>, SyncCoordError> {
    if !activation_installed {
        return Ok(None);
    }
    sync.with_reviewed_product_remote(review, outbox, |context, record| {
        let Some((generation, expected_enabled)) = db
            .pulled_plugin_apply_source(&record.key)
            .map_err(|_| local_error())?
        else {
            return Ok(None);
        };
        if activation_enabled != expected_enabled {
            return Err(local_error());
        }
        let enabled = match &record.value {
            Some(selection) if selection.version == installed_version => selection.enabled,
            Some(_) => return Ok(None),
            None => false,
        };
        if expected_enabled == enabled {
            return Ok(None);
        }
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|_| local_error())?;
        let intent = PluginApplyIntent {
            intent_id: random.iter().map(|byte| format!("{byte:02x}")).collect(),
            origin: context.origin.clone(),
            account_id: context.account_id.clone(),
            config_generation: context.config_generation,
            plugin_id: record.key.clone(),
            source_generation: generation,
            installed_version: installed_version.to_owned(),
            expected_enabled,
            reviewed_pending: None,
            base_record: None,
            remote: Some(record.clone()),
            reason: PluginApplyReason::Pulled,
        };
        if !db
            .prepare_plugin_apply(context, &intent)
            .map_err(|_| local_error())?
        {
            return Err(local_error());
        }
        // A crash before this step leaves the prepared journal discoverable;
        // recovery must reacquire a review before attempting any transition.
        if !db
            .mark_plugin_apply_ready_after_native_reconciliation(context, &intent)
            .map_err(|_| local_error())?
        {
            return Err(local_error());
        }
        Ok(Some(intent))
    })
}

/// Resume only the pre-runtime phases of a retained intent. The fresh native
/// review binds the same live session and durable Cloud record; the SQLite
/// phase/source check ensures restart or a later local change cannot turn a
/// saved intent into permission to repeat a runtime effect.
pub(super) fn resume_pulled(
    sync: &SyncCoordinator,
    db: &DatabaseManager,
    review: &V1RemoteApplyReview,
    outbox: &mut dyn V1ProductOutbox,
    intent_id: &str,
    installed_version: &str,
    activation_installed: bool,
    activation_enabled: bool,
) -> Result<Option<PluginApplyIntent>, SyncCoordError> {
    if !activation_installed {
        return Ok(None);
    }
    sync.with_reviewed_product_remote(review, outbox, |context, record| {
        let Some(stored) = db
            .plugin_apply_intent(intent_id)
            .map_err(|_| local_error())?
        else {
            return Ok(None);
        };
        let intent = stored.intent;
        if intent.origin != context.origin
            || intent.account_id != context.account_id
            || intent.config_generation != context.config_generation
            || intent.plugin_id != record.key
            || intent.reason != PluginApplyReason::Pulled
            || intent.remote.as_ref() != Some(record)
            || intent.installed_version != installed_version
            || intent.expected_enabled != activation_enabled
        {
            return Err(local_error());
        }
        let current = match stored.phase {
            PluginApplyPhase::Prepared => {
                db.mark_plugin_apply_ready_after_native_reconciliation(context, &intent)
            }
            PluginApplyPhase::Ready => db.revalidate_ready_plugin_apply(context, &intent),
            // Once persisted state might have changed, a different recovery
            // flow must observe that state and runtime before any decision.
            PluginApplyPhase::RuntimePending
            | PluginApplyPhase::Applied
            | PluginApplyPhase::Finalized => {
                return Err(local_error());
            }
        }
        .map_err(|_| local_error())?;
        if !current {
            return Err(local_error());
        }
        Ok(Some(intent))
    })
}

impl AppCore {
    /// Complete one prepared v2 application under a fresh post-barrier
    /// observation. Run on a bounded blocking worker: plugin lifecycle calls
    /// and durable storage can block. Failure after commit leaves quarantine
    /// and the durable journal for observation-based recovery.
    #[cfg(feature = "plugin-install")]
    #[allow(dead_code)] // Product scheduling and interrupted-outcome recovery remain separate.
    pub(crate) async fn execute_v2_pulled_plugin_apply(
        &self,
        barrier: &tiktools_cloud_sync::V2ReconciliationBarrier,
        intent: &PluginApplyIntent,
    ) -> Result<(), tiktools_cloud_sync::WorkspaceCoordError> {
        use tiktools_cloud_sync::WorkspaceCoordError;
        use tiktools_plugin_loader::PluginWorkerObservation;
        let unavailable = || WorkspaceCoordError::Ineligible;
        if self.is_shutdown() {
            return Err(unavailable());
        }
        let sync = self.sync.coordinator().map_err(|_| unavailable())?;
        let target = sync
            .with_cached_v2_reconciliation_barrier(barrier, |admission| {
                if self.is_shutdown()
                    || !admission
                        .matches_source_dir(&self.db.paths().data.join("plugin-sync-emission"))
                {
                    return Err(unavailable());
                }
                let remote = admission.remote().ok_or_else(unavailable)?;
                if intent.reason != PluginApplyReason::Pulled
                    || intent.origin != admission.context().origin
                    || intent.account_id != admission.context().account_id
                    || intent.config_generation != admission.context().config_generation
                    || intent.plugin_id != remote.key
                    || serde_json::to_value(intent.remote.as_ref().ok_or_else(unavailable)?)
                        .map_err(|_| unavailable())?
                        != serde_json::to_value(remote).map_err(|_| unavailable())?
                {
                    return Err(unavailable());
                }
                let runtime = self
                    .try_plugin_sync_runtime_guard()
                    .map_err(|_| unavailable())?;
                if self.is_shutdown() {
                    return Err(unavailable());
                }
                let packages = self.plugin_packages().map_err(|_| unavailable())?;
                let store = packages
                    .installer
                    .try_lock_store()
                    .map_err(|_| unavailable())?;
                if !store
                    .matches_roots(&self.db.paths().plugins, &self.db.paths().plugin_store())
                    .map_err(|_| unavailable())?
                    || !runtime.matches_selection(
                        &intent.plugin_id,
                        &intent.installed_version,
                        intent.expected_enabled,
                    )
                    || !self
                        .db
                        .revalidate_ready_plugin_apply(admission.context(), intent)
                        .map_err(|_| unavailable())?
                {
                    return Err(unavailable());
                }
                let enabled = intent
                    .remote
                    .as_ref()
                    .and_then(|record| record.value.as_ref())
                    .map(|selection| selection.enabled)
                    .unwrap_or(false);
                let epoch = self.sync.runtime_epoch().map_err(|_| unavailable())?;
                if !self.quarantine_plugin_apply_before_commit(&intent.plugin_id) {
                    return Err(unavailable());
                }
                let generation = match self.db.commit_plugin_apply_state(
                    admission.context(),
                    intent,
                    &intent.installed_version,
                ) {
                    Ok(Some(generation)) => generation,
                    _ => {
                        // Re-read the durable phase even on a storage error:
                        // an unknown commit must remain quarantined.
                        let _ = self.refresh_plugin_apply_quarantine_locked();
                        return Err(unavailable());
                    }
                };
                if enabled {
                    self.plugins
                        .start_generation_before_events(&intent.plugin_id, || {
                            self.set_plugin_activation(&intent.plugin_id, true, true);
                        })
                        .map_err(|_| unavailable())?;
                } else {
                    self.plugins
                        .stop(&intent.plugin_id)
                        .map_err(|_| unavailable())?;
                    self.set_plugin_activation(&intent.plugin_id, true, false);
                }
                let expected = if enabled {
                    PluginWorkerObservation::WorkerPresent
                } else {
                    PluginWorkerObservation::Stopped
                };
                if self
                    .plugins
                    .observe_worker(&intent.plugin_id)
                    .map_err(|_| unavailable())?
                    != expected
                    || !self
                        .db
                        .confirm_plugin_apply_runtime(
                            admission.context(),
                            intent,
                            generation,
                            epoch,
                            &intent.installed_version,
                            enabled,
                        )
                        .map_err(|_| unavailable())?
                    || !self
                        .db
                        .finalize_plugin_apply_after_native_ack(
                            admission.context(),
                            intent,
                            generation,
                            epoch,
                        )
                        .map_err(|_| unavailable())?
                {
                    return Err(unavailable());
                }
                self.refresh_plugin_apply_quarantine_locked()
                    .map_err(|_| unavailable())?;
                Ok(enabled)
            })
            .await;
        // The journal and quarantine remain authoritative on every failure.
        self.refresh_automation_snapshot();
        let enabled = target?;
        self.events.publish_domain(if enabled {
            crate::events::DomainEvent::PluginStarted {
                plugin_id: intent.plugin_id.clone(),
            }
        } else {
            crate::events::DomainEvent::PluginStopped {
                plugin_id: intent.plugin_id.clone(),
            }
        });
        Ok(())
    }

    /// Prepare a v2 application only after the cached selection matches a
    /// fresh post-barrier read. This records intent but does not run code.
    #[cfg(feature = "plugin-install")]
    #[allow(dead_code)] // Runtime execution and recovery are wired separately.
    pub(crate) async fn prepare_v2_pulled_plugin_apply(
        &self,
        barrier: &tiktools_cloud_sync::V2ReconciliationBarrier,
    ) -> Result<Option<PluginApplyIntent>, tiktools_cloud_sync::WorkspaceCoordError> {
        use tiktools_cloud_sync::WorkspaceCoordError;
        let unavailable = || WorkspaceCoordError::Ineligible;
        if self.is_shutdown() {
            return Err(unavailable());
        }
        let sync = self.sync.coordinator().map_err(|_| unavailable())?;
        sync.with_cached_v2_reconciliation_barrier(barrier, |admission| {
            if !admission.matches_source_dir(&self.db.paths().data.join("plugin-sync-emission")) {
                return Err(unavailable());
            }
            let runtime = self
                .try_plugin_sync_runtime_guard()
                .map_err(|_| unavailable())?;
            if self.is_shutdown() {
                return Err(unavailable());
            }
            let packages = self.plugin_packages().map_err(|_| unavailable())?;
            let store = packages
                .installer
                .try_lock_store()
                .map_err(|_| unavailable())?;
            if !store
                .matches_roots(&self.db.paths().plugins, &self.db.paths().plugin_store())
                .map_err(|_| unavailable())?
            {
                return Err(unavailable());
            }
            let remote = admission.remote().ok_or_else(unavailable)?;
            let Some(plugin) = self
                .plugins
                .get_for_admission(&remote.key)
                .map_err(|_| unavailable())?
            else {
                return Ok(None);
            };
            let Some((generation, enabled)) = self
                .db
                .pulled_plugin_apply_source(&remote.key)
                .map_err(|_| unavailable())?
            else {
                return Ok(None);
            };
            if !runtime.matches_selection(&remote.key, &plugin.manifest.version, enabled) {
                return Err(unavailable());
            }
            let selection = remote
                .value
                .as_ref()
                .map(|value| {
                    serde_json::from_value::<tiktools_cloud_sync::PluginSelection>(value.clone())
                })
                .transpose()
                .map_err(|_| unavailable())?;
            let target = match &selection {
                Some(value)
                    if value.plugin_id == remote.key
                        && value.version == plugin.manifest.version =>
                {
                    value.enabled
                }
                Some(_) => return Ok(None),
                None if remote.deleted => false,
                None => return Err(unavailable()),
            };
            if target == enabled {
                return Ok(None);
            }
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes).map_err(|_| unavailable())?;
            let context = admission.context();
            let intent = PluginApplyIntent {
                intent_id: bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
                origin: context.origin.clone(),
                account_id: context.account_id.clone(),
                config_generation: context.config_generation,
                plugin_id: remote.key.clone(),
                source_generation: generation,
                installed_version: plugin.manifest.version,
                expected_enabled: enabled,
                reviewed_pending: None,
                base_record: None,
                remote: Some(tiktools_cloud_sync::V1Record {
                    collection: remote.collection.clone(),
                    key: remote.key.clone(),
                    revision: remote.revision,
                    deleted: remote.deleted,
                    value: selection,
                }),
                reason: PluginApplyReason::Pulled,
            };
            if !self
                .db
                .prepare_plugin_apply(context, &intent)
                .map_err(|_| unavailable())?
            {
                return Err(unavailable());
            }
            if !self
                .db
                .mark_plugin_apply_ready_after_native_reconciliation(context, &intent)
                .map_err(|_| unavailable())?
            {
                return Err(unavailable());
            }
            Ok(Some(intent))
        })
        .await
    }

    /// Prepare only while the installed artifact and activation are stable.
    /// This private method is intentionally not called by product sync yet.
    #[allow(dead_code)]
    pub(crate) fn prepare_pulled_plugin_apply(
        &self,
        review: &V1RemoteApplyReview,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<Option<PluginApplyIntent>, SyncCoordError> {
        let _gate = self
            .plugin_state
            .invocation_gate
            .lock()
            .map_err(|_| local_error())?;
        let Ok(plugin) = self.require_discovered(&review.record().key) else {
            return Ok(None);
        };
        let activation = self
            .plugin_state
            .activation
            .read()
            .map_err(|_| local_error())?;
        let state = activation.get(&review.record().key).copied();
        drop(activation);
        let (installed, enabled) =
            state.map_or((true, true), |state| (state.installed, state.enabled));
        let sync = self.sync.coordinator().map_err(|_| local_error())?;
        prepare_pulled(
            sync,
            &self.db,
            review,
            outbox,
            &plugin.manifest.version,
            installed,
            enabled,
        )
    }

    /// A saved pre-runtime intent is never resumed solely by ID: it must be
    /// matched to a fresh account-bound review and current local artifact.
    #[allow(dead_code)]
    pub(crate) fn resume_pulled_plugin_apply(
        &self,
        review: &V1RemoteApplyReview,
        outbox: &mut dyn V1ProductOutbox,
        intent_id: &str,
    ) -> Result<Option<PluginApplyIntent>, SyncCoordError> {
        let _gate = self
            .plugin_state
            .invocation_gate
            .lock()
            .map_err(|_| local_error())?;
        let Ok(plugin) = self.require_discovered(&review.record().key) else {
            return Ok(None);
        };
        let activation = self
            .plugin_state
            .activation
            .read()
            .map_err(|_| local_error())?;
        let state = activation.get(&review.record().key).copied();
        drop(activation);
        let (installed, enabled) =
            state.map_or((true, true), |state| (state.installed, state.enabled));
        let sync = self.sync.coordinator().map_err(|_| local_error())?;
        resume_pulled(
            sync,
            &self.db,
            review,
            outbox,
            intent_id,
            &plugin.manifest.version,
            installed,
            enabled,
        )
    }
}
