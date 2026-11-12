//! Strict stop and best-effort shutdown, including bounded joins and permanent start-
//! admission closure.

use super::{lock_poisoned, PluginManager, JOIN_POLL_INTERVAL};
use crate::{
    worker::{RunningInstance, WorkerMsg},
    PluginLoaderError,
};
use std::{
    sync::{atomic::Ordering, Arc},
    thread,
    time::{Duration, Instant},
};
use tiktools_plugin_api::sync::{recover_mutex, recover_rwlock_read};

impl PluginManager {
    pub fn stop(&self, id: &str) -> Result<(), PluginLoaderError> {
        self.stop_if_present(id).map(|_| ())
    }

    /// Atomically determine whether a worker was running and stop it. An
    /// unresolved startup must never be reported as a successful absence.
    pub fn stop_if_present(&self, id: &str) -> Result<bool, PluginLoaderError> {
        self.stop_with_package(id, None)
    }

    pub(super) fn stop_with_package(
        &self,
        id: &str,
        package_token: Option<u64>,
    ) -> Result<bool, PluginLoaderError> {
        let (instance, stopping) = {
            let _lifecycle = self
                .lifecycle
                .lock()
                .map_err(|_| lock_poisoned("plugin lifecycle"))?;
            if self
                .package_fences
                .lock()
                .map_err(|_| lock_poisoned("plugin package fences"))?
                .get(id)
                .copied()
                != package_token
            {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin package mutation is active or requires recovery".into(),
                ));
            }
            if package_token.is_some() {
                self.ensure_no_retiring_worker(id)?;
            }
            if self
                .uncertain_startups
                .lock()
                .map_err(|_| lock_poisoned("plugin uncertain startups"))?
                .contains_key(id)
            {
                return Err(PluginLoaderError::StartupUncertain(format!(
                    "plugin `{id}` requires startup recovery"
                )));
            }
            let stopping = self.note_stopping(id);
            let removed = self.remove_instance(id);
            if let Some(instance) = removed.as_ref() {
                // Close admission and ask the worker to stop while the
                // lifecycle lock is still held: a start of the same id
                // cannot slip in between the removal and the shutdown
                // request, so two generations never coexist for one id.
                instance.events.close();
                instance.admission.close();
                let _ = instance.tx.send(WorkerMsg::Shutdown);
            }
            // The flag flips under the same lock hold that removed the
            // instance, so an old stop can never clear a newer start's
            // running state.
            self.set_running(id, false);
            (removed, stopping)
        };
        let was_running = instance.is_some();
        let result = self.finish_stop(id, instance);
        drop(stopping);
        result.map(|()| was_running)
    }

    /// Best-effort stop used only by shutdown paths. Unlike [`Self::stop`],
    /// a poisoned lifecycle lock is recovered (clearing the flag) instead
    /// of failing typed, so one torn transition can never wedge shutdown
    /// forever. Worker failures are logged and the instance is always
    /// removed with its running state cleared.
    fn stop_for_shutdown(&self, id: &str) -> bool {
        let (instance, stopping) = {
            let _lifecycle = match self.lifecycle.try_lock() {
                Ok(guard) => guard,
                Err(std::sync::TryLockError::WouldBlock) => return false,
                Err(std::sync::TryLockError::Poisoned(error)) => {
                    tracing::warn!("recovering poisoned plugin lifecycle during shutdown");
                    self.lifecycle.clear_poison();
                    error.into_inner()
                }
            };
            if recover_mutex(&self.package_fences, "plugin package fences").contains_key(id) {
                return false;
            }
            if recover_mutex(&self.uncertain_startups, "plugin uncertain startups").contains_key(id)
            {
                return false;
            }
            let stopping = self.note_stopping(id);
            let removed = self.remove_instance(id);
            if let Some(instance) = removed.as_ref() {
                instance.events.close();
                instance.admission.close();
                let _ = instance.tx.send(WorkerMsg::Shutdown);
            }
            self.set_running(id, false);
            (removed, stopping)
        };
        let result = match self.finish_stop(id, instance) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(id = %id, %error, "plugin shutdown failed");
                false
            }
        };
        drop(stopping);
        result
    }

    /// Joins an already-stopped instance within the stop deadline. The
    /// caller has already removed it from the map and asked it to stop
    /// under the lifecycle lock, so a poisoned worker lock recovers instead
    /// of failing the stop. Shared by the strict [`Self::stop`] and the
    /// best-effort shutdown path.
    fn finish_stop(
        &self,
        id: &str,
        instance: Option<Arc<RunningInstance>>,
    ) -> Result<(), PluginLoaderError> {
        let Some(instance) = instance else {
            tracing::debug!(id = %id, "plugin stop requested while not running");
            return Ok(());
        };
        // Stop entry point: a join that never returns (a guest stuck in
        // onUnload, native teardown hanging) shows here as `stopping`
        // with no matching `stopped`.
        let stop_started = std::time::Instant::now();
        tracing::info!(id = %id, "plugin stopping");
        // The instance is already removed from the map here, so a
        // poisoned worker lock recovers instead of failing the stop.
        let worker = recover_mutex(&instance.worker, "plugin worker").take();
        let result = match worker {
            Some(worker) => self.join_worker(id, worker, self.deadlines().stop),
            None => Ok(()),
        };
        match &result {
            Ok(()) => tracing::info!(
                id = %id,
                elapsed_ms = stop_started.elapsed().as_millis() as u64,
                "plugin stopped"
            ),
            Err(error) => tracing::warn!(
                id = %id,
                elapsed_ms = stop_started.elapsed().as_millis() as u64,
                %error,
                "plugin stop failed"
            ),
        }
        self.reap_retired();
        result
    }

    /// Joins one worker within `deadline`. Polling `is_finished` is the
    /// only way to bound a std join without handing the handle to another
    /// thread; on expiry the handle is dropped (the thread detaches) and
    /// the abandonment is reported explicitly, mirroring the LIVE ingress
    /// ladder: bounded, with an "outcome unknown" report instead of a hang.
    fn join_worker(
        &self,
        id: &str,
        worker: thread::JoinHandle<Result<(), PluginLoaderError>>,
        deadline: Duration,
    ) -> Result<(), PluginLoaderError> {
        let started = Instant::now();
        while !worker.is_finished() {
            if started.elapsed() >= deadline {
                self.abandoned_workers.fetch_add(1, Ordering::Relaxed);
                recover_mutex(&self.uncertain_startups, "plugin uncertain startups")
                    .entry(id.to_owned())
                    .or_default()
                    .push(worker);
                tracing::error!(
                    id = %id,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    deadline_ms = deadline.as_millis() as u64,
                    "plugin worker missed the stop deadline; abandoning the thread (outcome unknown)"
                );
                return Err(PluginLoaderError::Runtime(format!(
                    "plugin `{id}` worker did not stop within {}ms; thread abandoned with unknown outcome",
                    deadline.as_millis()
                )));
            }
            thread::sleep(JOIN_POLL_INTERVAL);
        }
        worker
            .join()
            .map_err(|_| PluginLoaderError::Runtime(format!("plugin `{id}` worker panicked")))
            .and_then(|inner| inner)
    }

    pub fn stop_all(&self) {
        let _ = self.stop_all_snapshot();
    }

    /// Permanently close start admission and report whether all workers and
    /// in-flight stops finished. A false result remains unsafe for updating.
    pub fn stop_all_checked(&self) -> bool {
        self.close_start_admission();
        self.stop_all_snapshot()
    }

    /// Close admission immediately at host shutdown, before awaiting other
    /// tasks. It remains closed for this manager's lifetime.
    pub fn close_start_admission(&self) {
        self.shutdown_admission_closed
            .store(true, Ordering::Release);
    }

    /// Nonterminal best-effort snapshot used by `stop_all` callers.
    fn stop_all_snapshot(&self) -> bool {
        // A start holds lifecycle while its outcome is unresolved. Do not
        // report a clean shutdown merely because its worker is not in the
        // instance map yet; avoid blocking indefinitely on native loading.
        let lifecycle = match self.lifecycle.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => return false,
            Err(std::sync::TryLockError::Poisoned(error)) => {
                tracing::warn!("recovering poisoned plugin lifecycle during shutdown");
                self.lifecycle.clear_poison();
                error.into_inner()
            }
        };
        if self.starts_inflight.load(Ordering::Acquire) != 0 {
            return false;
        }
        if self
            .package_fences
            .lock()
            .map_or(true, |fences| !fences.is_empty())
        {
            return false;
        }
        drop(lifecycle);
        let ids: Vec<String> = recover_rwlock_read(&self.instances, "plugin instances")
            .keys()
            .cloned()
            .collect();
        let mut stopped = true;
        for id in ids {
            stopped &= self.stop_for_shutdown(&id);
        }
        // Shutdown is the last structured chance to join retired workers
        // before the manager itself is dropped.
        self.reap_retired();
        self.reap_uncertain_startups();
        let uncertain = self
            .uncertain_startups
            .lock()
            .map_or(true, |starts| !starts.is_empty());
        let stopping = self.stopping.lock().map_or(true, |stops| !stops.is_empty());
        stopped
            && !uncertain
            && !stopping
            && self.starts_inflight.load(Ordering::Acquire) == 0
            && self.retired_worker_count() == 0
            && self.abandoned_worker_count() == 0
    }
}
