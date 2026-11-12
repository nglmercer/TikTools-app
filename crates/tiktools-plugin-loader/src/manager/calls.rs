//! Bounded call admission, deadline enforcement, isolated preparation and generation-bound
//! result delivery.

use super::{lock_poisoned, PluginManager, PROCESS_FIRST_CALL_GRACE};
use crate::{
    quickjs::{native_addons_allowed, untrusted_native_addons},
    worker::{QueuedCall, WorkerMsg},
    PluginLoaderError,
};
use serde_json::Value;
use std::{
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};
use tiktools_plugin_api::{sync::recover_rwlock_read, PluginRuntimeKind};
use tokio::sync::oneshot;

impl PluginManager {
    pub async fn call(&self, id: &str, request: &Value) -> Result<Value, PluginLoaderError> {
        self.call_with_timeout(id, request, Duration::from_secs(30))
            .await
    }

    /// Claims the cold-start grace for one process instance, if still
    /// unclaimed, and returns the deadline the caller must enforce.
    ///
    /// Tokio-side callers race a deadline against the worker-side waiter, so
    /// both must cover the same budget: without this, a fresh child still
    /// building its engine (seconds) is failed by the steady-state descriptor
    /// deadline (milliseconds) before the worker's own grace can warm it up.
    /// Exactly one first call per generation wins the swap; the worker's swap
    /// in [`Self::call_with_deadline`] then finds the grace already consumed,
    /// so concurrent first calls never double-extend it.
    pub fn claim_cold_start_grace(&self, id: &str, timeout: Duration) -> Duration {
        let cold_process = recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .filter(|instance| instance.kind == PluginRuntimeKind::Process)
            .is_some_and(|instance| instance.cold.swap(false, Ordering::AcqRel));
        if cold_process {
            timeout.max(PROCESS_FIRST_CALL_GRACE)
        } else {
            timeout
        }
    }

    /// Calls one plugin instance. Calls stay serialized per instance through
    /// its worker queue, so a slow call delays (but never starves) later
    /// calls to the same plugin. Keep processor plugins fast and never
    /// combine slow model preparation with enrichment in one process.
    ///
    /// TODO(protocol-v2): per-QoS-class instances or multiplexed process
    /// requests would let realtime processors skip ahead of background
    /// actions sharing one process.
    pub async fn call_with_timeout(
        &self,
        id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<Value, PluginLoaderError> {
        self.call_with_deadline(id, request, Instant::now() + timeout)
            .await
    }

    /// Calls one plugin instance with an absolute deadline covering queueing
    /// plus execution. The worker discards requests that expire while queued
    /// instead of executing stale work; the waiter gives up at the same
    /// deadline. A timeout never removes a native instance, while transport
    /// and protocol errors retire it so the next start loads fresh state.
    ///
    /// Process instances differ twice: the first call of a generation gets
    /// cold-start grace (a fresh child may still be building its engine),
    /// and a timeout retires the instance because the worker enforces the
    /// same deadline on its I/O and kills the child when it fires. Keeping a
    /// timed-out process instance would fail the next call instantly with
    /// "plugin process stdin is unavailable".
    pub async fn call_with_deadline(
        &self,
        id: &str,
        request: &Value,
        deadline: Instant,
    ) -> Result<Value, PluginLoaderError> {
        self.call_generation_deadline(id, request, deadline, None)
            .await
    }

    /// Exact package identity captured before this runtime generation loaded.
    /// Missing identity keeps legacy Owner execution available, but cannot be
    /// used for delegated approval. A disk hash alone cannot identify code
    /// already loaded by a native worker.
    pub fn running_artifact_fingerprint(&self, id: &str) -> Option<String> {
        recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .and_then(|instance| instance.artifact_fingerprint.clone())
    }

    /// Prepare under the host's trusted catalog/settings lease. This reads
    /// package data and freezes it, but never runs initialization or plugin
    /// code. The resulting linear value still requires host final admission.
    pub fn prepare_isolated_call(
        &self,
        id: &str,
        artifact_digest: &str,
        snapshot_directory: std::path::PathBuf,
        settings: Value,
    ) -> Result<crate::PreparedIsolatedPlugin, PluginLoaderError> {
        if self
            .package_fences
            .lock()
            .map_err(|_| lock_poisoned("plugin package fences"))?
            .contains_key(id)
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin package mutation is active or requires recovery".into(),
            ));
        }
        let plugin = self
            .get(id)
            .ok_or_else(|| PluginLoaderError::NotFound(id.into()))?;
        if !plugin.available {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin is unavailable".into(),
            ));
        }
        if !plugin.manifest.native_addons.is_empty()
            && !native_addons_allowed(&plugin.manifest, plugin.source)
        {
            return Err(untrusted_native_addons(id));
        }
        let runtime = self.runtimes.get(plugin.manifest.runtime).ok_or_else(|| {
            PluginLoaderError::RuntimeUnavailable("plugin runtime is unavailable".into())
        })?;
        crate::PreparedIsolatedPlugin::new(
            plugin,
            runtime,
            snapshot_directory,
            artifact_digest,
            settings,
            Arc::clone(&self.isolated_admission),
        )
    }

    pub async fn call_with_generation(
        &self,
        id: &str,
        request: &Value,
        timeout: Duration,
        generation: u64,
    ) -> Result<Value, PluginLoaderError> {
        self.call_generation_deadline(id, request, Instant::now() + timeout, Some(generation))
            .await
    }

    async fn call_generation_deadline(
        &self,
        id: &str,
        request: &Value,
        deadline: Instant,
        generation: Option<u64>,
    ) -> Result<Value, PluginLoaderError> {
        let bytes = serde_json::to_vec(request)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        if Instant::now() >= deadline {
            return Err(PluginLoaderError::Timeout(format!(
                "plugin `{id}` call already expired"
            )));
        }
        let (instance, is_process, remaining, answer, sent) = {
            // Admission and enqueue are synchronous and ordered against package
            // reservation; never hold this mutex across the asynchronous reply.
            let fences = self
                .package_fences
                .lock()
                .map_err(|_| lock_poisoned("plugin package fences"))?;
            if fences.contains_key(id) {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin package mutation is active or requires recovery".into(),
                ));
            }
            let instance = self
                .instances
                .read()
                .map_err(|_| lock_poisoned("plugin instances"))?
                .get(id)
                .cloned()
                .ok_or_else(|| {
                    if generation.is_some() {
                        PluginLoaderError::RuntimeUnavailable("plugin generation changed".into())
                    } else {
                        PluginLoaderError::NotFound(id.to_owned())
                    }
                })?;
            if generation.is_some_and(|generation| generation != instance.token) {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin generation changed".into(),
                ));
            }
            let admission = Arc::clone(&instance.admission)
                .try_acquire_owned()
                .map_err(|_| PluginLoaderError::Overloaded(id.to_owned()))?;
            let now = Instant::now();
            if now >= deadline {
                return Err(PluginLoaderError::Timeout(format!(
                    "plugin `{id}` call already expired"
                )));
            }
            let is_process = instance.kind == PluginRuntimeKind::Process;
            let deadline = if is_process && instance.cold.swap(false, Ordering::AcqRel) {
                deadline.max(now + PROCESS_FIRST_CALL_GRACE)
            } else {
                deadline
            };
            let remaining = deadline - now;
            let (respond, answer) = oneshot::channel();
            let sent = instance.tx.send(WorkerMsg::Call(QueuedCall {
                admission: Some(admission),
                request: bytes,
                timeout: remaining,
                deadline,
                respond,
            }));
            (instance, is_process, remaining, answer, sent)
        };
        sent.map_err(|_| {
            self.remove_failed_worker(id, instance.token);
            PluginLoaderError::Runtime(format!("plugin `{id}` worker is gone"))
        })?;
        // Async wait: no thread is spent here, unlike the old blocking
        // `recv_timeout` inside `spawn_blocking`. Timeout and worker-death
        // semantics are unchanged: a dropped oneshot sender means the
        // worker died without answering, exactly like a disconnected
        // channel before.
        let response = tokio::time::timeout(remaining, answer)
            .await
            .map_err(|_| {
                if is_process {
                    self.remove_failed_worker(id, instance.token);
                }
                PluginLoaderError::Timeout(format!("plugin `{id}` call timed out"))
            })?
            .map_err(|_| {
                self.remove_failed_worker(id, instance.token);
                PluginLoaderError::Runtime(format!("plugin `{id}` worker died"))
            })?;
        let response = match response {
            Ok(response) => response,
            // The worker discarded this request as expired; the instance
            // itself is healthy, so it stays running.
            Err(error @ PluginLoaderError::Timeout(_)) => return Err(error),
            Err(error) => {
                self.remove_failed_worker(id, instance.token);
                return Err(error);
            }
        };
        // A stopped/replaced worker may still complete an in-flight call.
        // Do not surface its result to a generation-bound caller. This is an
        // instantaneous check, not a lease covering later host side effects.
        if let Some(expected) = generation {
            let instances = self
                .instances
                .read()
                .map_err(|_| lock_poisoned("plugin instances"))?;
            if !instances
                .get(id)
                .is_some_and(|current| current.token == expected)
            {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin generation changed".into(),
                ));
            }
        }
        match serde_json::from_slice(&response) {
            Ok(response) => Ok(response),
            Err(error) => {
                self.remove_failed_worker(id, instance.token);
                Err(PluginLoaderError::Runtime(format!(
                    "plugin returned invalid JSON: {error}"
                )))
            }
        }
    }

    /// Aggregate admission occupancy, without request payloads or plugin IDs.
    pub fn outstanding_call_count(&self) -> usize {
        recover_rwlock_read(&self.instances, "plugin instances")
            .values()
            .map(|instance| {
                crate::worker::PLUGIN_CALL_CAPACITY
                    .saturating_sub(instance.admission.available_permits())
            })
            .sum()
    }
}
