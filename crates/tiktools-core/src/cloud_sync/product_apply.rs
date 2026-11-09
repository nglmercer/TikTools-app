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
