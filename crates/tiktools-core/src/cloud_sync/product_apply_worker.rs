//! Bounded background application of already-consented v2 selections.
use cloud_client::{Session, WorkspaceRecord};
use std::{
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};
use tiktools_cloud_sync::{PluginSelection, V2LocalEditContext};

use super::{product_apply::V2PulledApplyRecovery, CloudSync};
use crate::{
    db::{PluginApplyPhase, PluginApplyReason},
    AppCore,
};
use tiktools_plugin_loader::PluginWorkerObservation;

const MAX_APPLIES: usize = 8;
const PASS_BUDGET: Duration = Duration::from_secs(30);

struct ApplyLease(Arc<CloudSync>);
impl Drop for ApplyLease {
    fn drop(&mut self) {
        self.0.apply_running.store(false, Ordering::Release);
        self.0.apply_done.notify_waiters();
    }
}

enum Candidate {
    Recovery { id: String, plugin: String },
    Fresh(String),
}

impl AppCore {
    /// The result list is a hint only: each mutation rechecks the session,
    /// server barrier, durable cache, local source, package and runtime.
    pub(crate) async fn apply_cached_v2_selections(
        self: &Arc<Self>,
        session: Arc<Session>,
        context: V2LocalEditContext,
        records: Vec<WorkspaceRecord>,
    ) {
        if self.is_shutdown()
            || self
                .sync
                .apply_running
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return;
        }
        let lease = ApplyLease(self.sync.clone());
        let core = self.clone();
        let runtime = tokio::runtime::Handle::current();
        let _ = tokio::task::spawn_blocking(move || {
            let _lease = lease;
            runtime.block_on(core.apply_cached_v2_inner(session, context, records));
        })
        .await;
    }

    async fn apply_cached_v2_inner(
        &self,
        session: Arc<Session>,
        context: V2LocalEditContext,
        records: Vec<WorkspaceRecord>,
    ) {
        let deadline = Instant::now() + PASS_BUDGET;
        let Ok(sync) = self.sync.coordinator() else {
            return;
        };
        if self.is_shutdown()
            || !self.cloud.auth().is_current_session(&session)
            || sync.capture_v2_config_epoch(&session).is_none()
            || session.instance_origin() != context.origin
            || session.account_id() != context.account_id
        {
            return;
        }
        // Discovery is advisory. Every candidate is re-read under the actual
        // source and account gates before any journal or runtime transition.
        let mut candidates = Vec::new();
        let mut cursor = None;
        for _ in 0..64 {
            let Ok(page) = self
                .db
                .plugin_apply_recovery_page(&context, cursor.as_deref(), 64)
            else {
                return;
            };
            for saved in page.intents {
                if saved.intent.reason == PluginApplyReason::Pulled {
                    if matches!(
                        saved.phase,
                        PluginApplyPhase::RuntimePending | PluginApplyPhase::Applied
                    ) {
                        let enabled = saved
                            .intent
                            .remote
                            .as_ref()
                            .and_then(|record| record.value.as_ref())
                            .is_some_and(|selection| selection.enabled);
                        let state_matches =
                            self.plugin_state.activation.read().is_ok_and(|activation| {
                                activation
                                    .get(&saved.intent.plugin_id)
                                    .is_some_and(|state| {
                                        state.installed && state.enabled == enabled
                                    })
                            });
                        let expected = if enabled {
                            PluginWorkerObservation::WorkerPresent
                        } else {
                            PluginWorkerObservation::Stopped
                        };
                        if !state_matches
                            || self.plugins.observe_worker(&saved.intent.plugin_id).ok()
                                != Some(expected)
                        {
                            continue;
                        }
                    }
                    candidates.push(Candidate::Recovery {
                        id: saved.intent.intent_id,
                        plugin: saved.intent.plugin_id,
                    });
                }
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        if cursor.is_some() {
            return;
        }
        for record in records {
            if record.collection != "plugin-selections" {
                continue;
            }
            let Some(plugin) = self.plugins.get(&record.key) else {
                continue;
            };
            let Ok(Some((_, enabled))) = self.db.pulled_plugin_apply_source(&record.key) else {
                continue;
            };
            let target = if record.deleted {
                false
            } else {
                let Some(selection) = record
                    .value
                    .and_then(|v| serde_json::from_value::<PluginSelection>(v).ok())
                else {
                    continue;
                };
                if selection.plugin_id != record.key || selection.version != plugin.manifest.version
                {
                    continue;
                }
                selection.enabled
            };
            if target != enabled {
                let expected = if enabled {
                    PluginWorkerObservation::WorkerPresent
                } else {
                    PluginWorkerObservation::Stopped
                };
                if self.plugins.observe_worker(&record.key).ok() != Some(expected) {
                    continue;
                }
                candidates.push(Candidate::Fresh(record.key));
            }
        }
        if candidates.is_empty() {
            return;
        }
        let start = self
            .sync
            .apply_cursor
            .fetch_add(MAX_APPLIES, Ordering::Relaxed)
            % candidates.len();
        candidates.rotate_left(start);
        for candidate in candidates.into_iter().take(MAX_APPLIES) {
            if self.is_shutdown()
                || Instant::now() >= deadline
                || !self.cloud.auth().is_current_session(&session)
            {
                break;
            }
            let plugin = match &candidate {
                Candidate::Recovery { plugin, .. } => plugin,
                Candidate::Fresh(plugin) => plugin,
            };
            let proof = match tokio::time::timeout_at(
                deadline.into(),
                sync.prepare_v2_reconciliation_barrier_for_context(
                    session.clone(),
                    plugin.clone(),
                    context.clone(),
                ),
            )
            .await
            {
                Ok(Ok(proof)) => proof,
                _ => continue,
            };
            if !proof.matches_context(&context) {
                continue;
            }
            let intent = match candidate {
                Candidate::Recovery { id, .. } => {
                    match tokio::time::timeout_at(
                        deadline.into(),
                        self.recover_v2_pulled_plugin_apply(&proof, &id),
                    )
                    .await
                    {
                        Ok(Ok(V2PulledApplyRecovery::Ready(intent))) => Some(intent),
                        _ => None,
                    }
                }
                Candidate::Fresh(_) => tokio::time::timeout_at(
                    deadline.into(),
                    self.prepare_v2_pulled_plugin_apply(&proof),
                )
                .await
                .ok()
                .and_then(Result::ok)
                .flatten(),
            };
            let Some(intent) = intent else { continue };
            if self.is_shutdown()
                || Instant::now() >= deadline
                || !self.cloud.auth().is_current_session(&session)
            {
                break;
            }
            let proof = match tokio::time::timeout_at(
                deadline.into(),
                sync.prepare_v2_reconciliation_barrier_for_context(
                    session.clone(),
                    intent.plugin_id.clone(),
                    context.clone(),
                ),
            )
            .await
            {
                Ok(Ok(proof)) => proof,
                _ => continue,
            };
            if !proof.matches_context(&context) {
                continue;
            }
            let _ = tokio::time::timeout_at(
                deadline.into(),
                self.execute_v2_pulled_plugin_apply(&proof, &intent),
            )
            .await;
        }
    }
}
