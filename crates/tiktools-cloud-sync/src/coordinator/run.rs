//! Bounded v1 pull/upload runs and cancellation-safe runtime accounting.
use crate::{
    product_outbox, protocol_fence, RemoteSelection, SyncConfig, SyncOutcome, V1ProductOutbox,
};
use crate::{SyncCoordError, SyncCoordinator};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

mod pull;
mod upload;

/// Clears the `syncing` flag when dropped, including on cancellation.
struct RunningGuard<'a>(&'a AtomicBool);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl SyncCoordinator {
    /// Pull remote changes, push staged edits, persist, and report what the
    /// caller must apply to already-installed plugins. Expired cursors
    /// rebuild from a snapshot automatically; conflicts are collected (not
    /// fatal) for explicit resolution via [`SyncCoordinator::keep_local`]
    /// or [`SyncCoordinator::take_remote`].
    pub async fn sync_now(&self) -> Result<SyncOutcome, SyncCoordError> {
        // Compatibility entry point for applications without a product
        // outbox. A product that records candidates in a separate durable
        // store must use `sync_now_with_outbox` instead.
        self.sync_now_inner(&mut product_outbox::NoProductOutbox, false)
            .await
    }

    /// Sync with a product-owned durable outbox. Every cooperating local
    /// mutation must use `with_v1_local_edit`; direct callers of `sync_now`
    /// cannot provide the product outbox's cross-store safety guarantee.
    /// This low-level compatibility entry point does not require a migration
    /// marker or a runtime guard and rejects migrated accounts; product integrations must use
    /// `sync_now_with_marked_outbox` after explicit source migration instead.
    pub async fn sync_now_with_outbox(
        &self,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<SyncOutcome, SyncCoordError> {
        self.sync_now_inner(outbox, false).await
    }

    /// Product drain for a previously reviewed v1 migration. The durable
    /// per-account marker must already fence cooperating legacy writers, and
    /// the native outbox must supply a runtime gate. This method never creates
    /// the marker or authorizes migration on its own. Older binaries and
    /// direct filesystem writers remain outside the cooperative protocol.
    pub async fn sync_now_with_marked_outbox(
        &self,
        outbox: &mut dyn V1ProductOutbox,
    ) -> Result<SyncOutcome, SyncCoordError> {
        self.sync_now_inner(outbox, true).await
    }

    async fn sync_now_inner(
        &self,
        outbox: &mut dyn V1ProductOutbox,
        require_product_marker: bool,
    ) -> Result<SyncOutcome, SyncCoordError> {
        let lease = self.protocol.try_v1()?;
        let config = self.read_config();
        if !config.enabled {
            return Err(SyncCoordError::Disabled);
        }
        if !self
            .syncing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return Err(SyncCoordError::AlreadyRunning);
        }
        // Release the run flag even when the caller drops this future
        // mid-sync (the control plane enforces a per-request timeout);
        // otherwise every later run reports `AlreadyRunning` until restart.
        let running = RunningGuard(&self.syncing);
        let _network = lease.try_network()?;
        if self.read_config() != config {
            return Err(SyncCoordError::Disabled);
        }
        let outcome = self
            .sync_inner(&lease, config, outbox, require_product_marker)
            .await;
        drop(running);
        {
            let mut runtime = self.runtime.lock().await;
            match &outcome {
                Ok(_) => {
                    runtime.last_error = None;
                    runtime.last_synced_at = Some(now_secs());
                }
                Err(error) => {
                    runtime.last_error = Some(error.to_string());
                }
            }
        }
        outcome
    }

    async fn sync_inner(
        &self,
        lease: &protocol_fence::V1Lease,
        config: SyncConfig,
        outbox: &mut dyn V1ProductOutbox,
        require_product_marker: bool,
    ) -> Result<SyncOutcome, SyncCoordError> {
        // A fresh process holds no in-memory session; reconnect from the
        // store first so one-shot callers (CLI, startup) sync without a
        // separate restore step. Failures fall through to NotSignedIn.
        if self.auth.session().await.is_none() {
            let _ = self.auth.restore().await;
        }
        let session = self
            .auth
            .session()
            .await
            .ok_or(SyncCoordError::NotSignedIn)?;
        let _marker_admission = lease.try_admission()?;
        self.with_live_session(&session, config, || {
            lease.ensure_v1(&session.instance_origin(), session.account_id())?;
            lease.ensure_outbox_mode(
                &session.instance_origin(),
                session.account_id(),
                require_product_marker,
            )?;
            lease.ensure_no_uncertain(&session.instance_origin(), session.account_id())?;
            Ok(())
        })?;
        drop(_marker_admission);
        let snapshot = self.load_or_reset(&session, lease)?;
        let (snapshot, pulled) = self
            .pull_v1_remote_pages(
                lease,
                &session,
                config,
                snapshot,
                outbox,
                require_product_marker,
            )
            .await?;
        let (snapshot, pushed) = self
            .upload_v1_pending(
                lease,
                &session,
                config,
                snapshot,
                outbox,
                require_product_marker,
            )
            .await?;
        let outcome = SyncOutcome {
            pulled,
            pushed,
            conflicts: snapshot
                .state
                .entries
                .iter()
                .filter(|(_, e)| e.pending.as_ref().is_some_and(|p| p.conflict))
                .map(|(k, _)| k.clone())
                .collect::<Vec<_>>(),
            remote: snapshot
                .state
                .entries
                .iter()
                // A remote value is not a local decision while an edit is
                // pending. In particular, conflicted values must wait for
                // keep_local/take_remote instead of changing a running plugin.
                .filter(|(_, entry)| entry.pending.is_none())
                .filter_map(|(key, entry)| {
                    entry.remote.as_ref().map(|record| RemoteSelection {
                        key: key.clone(),
                        selection: record.value.clone(),
                    })
                })
                .collect(),
        };
        self.save_product_checkpoint(
            lease,
            &session,
            config,
            snapshot,
            outbox,
            None,
            require_product_marker,
        )?;
        Ok(outcome)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
