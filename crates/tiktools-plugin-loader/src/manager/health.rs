//! Worker observations, failed-generation retirement and opportunistic reaping. Unknown
//! outcomes remain sticky.

use super::{
    lock_poisoned, PluginManager, PluginWorkerObservation, RetiredWorker, MAX_RETIRED_WORKERS,
};
use crate::{
    worker::{RunningInstance, WorkerMsg},
    PluginLoaderError,
};
use std::sync::{atomic::Ordering, Arc};
use tiktools_plugin_api::sync::{recover_mutex, recover_rwlock_write};

impl PluginManager {
    /// Reject activation or package changes while a prior startup outcome is
    /// unknown in this manager, including when there is no mapped worker.
    pub fn ensure_startup_known(&self, id: &str) -> Result<(), PluginLoaderError> {
        let uncertain = self
            .uncertain_startups
            .lock()
            .map_err(|_| lock_poisoned("plugin uncertain startups"))?;
        if uncertain.contains_key(id) {
            return Err(PluginLoaderError::StartupUncertain(format!(
                "plugin `{id}` requires startup recovery"
            )));
        }
        Ok(())
    }

    /// A conservative instantaneous observation of host-owned workers.
    /// It does not prove the health of a child process, fence other hosts,
    /// or prevent a worker from exiting after the observation.
    pub fn observe_worker(&self, id: &str) -> Result<PluginWorkerObservation, PluginLoaderError> {
        let _lifecycle = match self.lifecycle.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => {
                return Ok(PluginWorkerObservation::Uncertain)
            }
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(lock_poisoned("plugin lifecycle"))
            }
        };
        if self
            .package_fences
            .lock()
            .map_err(|_| lock_poisoned("plugin package fences"))?
            .contains_key(id)
        {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        if self
            .uncertain_startups
            .lock()
            .map_err(|_| lock_poisoned("plugin uncertain startups"))?
            .contains_key(id)
        {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        let stopping = self
            .stopping
            .lock()
            .map_err(|_| lock_poisoned("plugin stopping"))?;
        if stopping.contains_key(id) {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        let retired = self
            .retired
            .lock()
            .map_err(|_| lock_poisoned("plugin retired workers"))?;
        // Overflow retirement increments the abandoned count while holding
        // this mutex. Check after taking it so a removed handle cannot become
        // invisible between our count check and the retired-list observation.
        if self.abandoned_workers.load(Ordering::Acquire) != 0 {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        if retired.iter().any(|worker| worker.id == id) {
            return Ok(PluginWorkerObservation::Uncertain);
        }
        let instances = self
            .instances
            .read()
            .map_err(|_| lock_poisoned("plugin instances"))?;
        let Some(instance) = instances.get(id) else {
            return Ok(PluginWorkerObservation::Stopped);
        };
        let worker = instance
            .worker
            .lock()
            .map_err(|_| lock_poisoned("plugin worker"))?;
        Ok(
            if worker.as_ref().is_some_and(|handle| !handle.is_finished()) {
                PluginWorkerObservation::WorkerPresent
            } else {
                PluginWorkerObservation::Uncertain
            },
        )
    }

    pub(super) fn remove_instance(&self, id: &str) -> Option<Arc<RunningInstance>> {
        recover_rwlock_write(&self.instances, "plugin instances").remove(id)
    }

    /// Removes the instance generation `token` (only if it is still the
    /// current one), closes its admission and asks it to stop. The caller
    /// must already hold the lifecycle lock, so a concurrent start of the
    /// same id can never be mislabelled or left running alongside the
    /// generation being retired.
    pub(super) fn retire_generation_locked(
        &self,
        id: &str,
        token: u64,
    ) -> Option<Arc<RunningInstance>> {
        let removed = {
            let mut instances = recover_rwlock_write(&self.instances, "plugin instances");
            if instances
                .get(id)
                .is_some_and(|current| current.token == token)
            {
                instances.remove(id)
            } else {
                None
            }
        };
        if let Some(instance) = removed.as_ref() {
            instance.events.close();
            instance.admission.close();
            let _ = instance.tx.send(WorkerMsg::Shutdown);
        }
        if removed.is_some() {
            self.set_running(id, false);
        }
        removed
    }

    /// Retires the instance generation identified by `token`; a newer start
    /// is never disturbed. The hot path that observed the failure cannot
    /// block on a join, so the worker's handle keeps its owner here and is
    /// reaped (bounded) later instead of being dropped unobserved.
    pub(super) fn remove_failed_worker(&self, id: &str, token: u64) {
        let (removed, stopping) = {
            let _lifecycle = recover_mutex(&self.lifecycle, "plugin lifecycle");
            let stopping = self.note_stopping(id);
            (self.retire_generation_locked(id, token), stopping)
        };
        if let Some(instance) = removed {
            self.retire_instance(id, token, instance);
        }
        drop(stopping);
    }

    /// Hands a retired worker's join handle to the manager for bounded
    /// reaping. Never blocks: the caller may be serving a call. The list is
    /// bounded, and overflow abandons the oldest threads explicitly with
    /// the same report as a missed stop deadline.
    pub(super) fn retire_instance(&self, id: &str, token: u64, instance: Arc<RunningInstance>) {
        let worker = recover_mutex(&instance.worker, "plugin worker").take();
        let mut retired = recover_mutex(&self.retired, "plugin retired workers");
        while retired.len() >= MAX_RETIRED_WORKERS {
            let oldest = retired.remove(0);
            self.abandoned_workers.fetch_add(1, Ordering::Relaxed);
            tracing::error!(
                id = %oldest.id,
                token = oldest.token,
                "retired plugin worker backlog is full; abandoning the oldest thread (outcome unknown)"
            );
            drop(oldest.worker);
        }
        retired.push(RetiredWorker {
            id: id.to_owned(),
            token,
            worker,
        });
    }

    /// Joins every retired worker that has already finished and logs its
    /// outcome; still-running ones stay for a later pass. Bounded: it never
    /// waits on a live thread, so it is safe on the hot path.
    pub(super) fn reap_retired(&self) {
        let finished: Vec<RetiredWorker> = {
            let mut retired = recover_mutex(&self.retired, "plugin retired workers");
            let mut finished = Vec::new();
            let mut kept = Vec::new();
            for entry in retired.drain(..) {
                match &entry.worker {
                    Some(worker) if !worker.is_finished() => kept.push(entry),
                    _ => finished.push(entry),
                }
            }
            *retired = kept;
            finished
        };
        for entry in finished {
            let Some(worker) = entry.worker else { continue };
            match worker.join() {
                Ok(Ok(())) => tracing::info!(
                    id = %entry.id,
                    token = entry.token,
                    "retired plugin worker stopped"
                ),
                Ok(Err(error)) => tracing::warn!(
                    id = %entry.id,
                    token = entry.token,
                    %error,
                    "retired plugin worker exited with an error"
                ),
                Err(_) => tracing::warn!(
                    id = %entry.id,
                    token = entry.token,
                    "retired plugin worker panicked"
                ),
            }
        }
    }

    /// Join completed unknown startup threads without clearing their sticky
    /// outcome: a thread exit alone cannot undo or classify external effects.
    pub(super) fn reap_uncertain_startups(&self) {
        let finished = {
            let Ok(mut startups) = self.uncertain_startups.lock() else {
                return;
            };
            let mut finished = Vec::new();
            for (id, workers) in startups.iter_mut() {
                let mut pending = Vec::new();
                for worker in workers.drain(..) {
                    if worker.is_finished() {
                        finished.push((id.clone(), worker));
                    } else {
                        pending.push(worker);
                    }
                }
                *workers = pending;
            }
            finished
        };
        for (id, worker) in finished {
            match worker.join() {
                Ok(Ok(())) => {
                    tracing::info!(plugin = %id, "uncertain startup worker exited; recovery remains required")
                }
                Ok(Err(error)) => {
                    tracing::warn!(plugin = %id, %error, "uncertain startup worker exited with an error")
                }
                Err(_) => tracing::warn!(plugin = %id, "uncertain startup worker panicked"),
            }
        }
    }

    // Called with lifecycle held. A failure retirement may leave the instance
    // map before handing its worker to the retired list; `stopping` bridges it.
    pub(super) fn ensure_no_retiring_worker(&self, id: &str) -> Result<(), PluginLoaderError> {
        if self
            .stopping
            .lock()
            .map_err(|_| lock_poisoned("plugin stopping"))?
            .contains_key(id)
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin worker is stopping".into(),
            ));
        }
        let retired = self
            .retired
            .lock()
            .map_err(|_| lock_poisoned("plugin retired workers"))?;
        if self.abandoned_workers.load(Ordering::Acquire) != 0
            || retired.iter().any(|worker| worker.id == id)
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin worker outcome is uncertain".into(),
            ));
        }
        Ok(())
    }
}
