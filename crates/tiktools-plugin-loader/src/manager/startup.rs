//! Worker startup and activation. One lifecycle critical section protects generation reuse,
//! publication and startup event release.

use super::{lock_poisoned, PluginManager, StartingGuard, MAX_UNCERTAIN_STARTUPS};
use crate::{
    quickjs::{native_addons_allowed, untrusted_native_addons},
    worker::{run_instance_worker, RunningInstance, WorkerMsg},
    PluginLoaderError, WorkerContext,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tiktools_plugin_api::sync::{recover_mutex, recover_rwlock_read, recover_rwlock_write};
use tokio::sync::mpsc;

impl PluginManager {
    pub fn start(&self, id: &str) -> Result<(), PluginLoaderError> {
        self.start_generation(id).map(|_| ())
    }

    /// Starts or reuses a worker and returns its identity while lifecycle is
    /// still locked. A subsequent call can bind to that exact worker instead
    /// of accidentally crossing a concurrent stop/restart.
    pub fn start_generation(&self, id: &str) -> Result<u64, PluginLoaderError> {
        self.start_generation_before_events(id, || {})
    }

    /// Runs a short host activation callback after a successful start and
    /// before startup events are released. It also runs when an existing
    /// worker is reused. The callback must not reenter the manager or wait.
    /// Host invocation admission must be acquired before calling this method.
    pub fn start_generation_before_events(
        &self,
        id: &str,
        before_events: impl FnOnce(),
    ) -> Result<u64, PluginLoaderError> {
        self.start_generation_with_package(id, before_events, None)
    }

    pub(super) fn start_generation_with_package(
        &self,
        id: &str,
        before_events: impl FnOnce(),
        package_token: Option<u64>,
    ) -> Result<u64, PluginLoaderError> {
        if self.shutdown_admission_closed.load(Ordering::Acquire) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin manager is shutting down".into(),
            ));
        }
        let _lifecycle = self
            .lifecycle
            .lock()
            .map_err(|_| lock_poisoned("plugin lifecycle"))?;
        if self.shutdown_admission_closed.load(Ordering::Acquire) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin manager is shutting down".into(),
            ));
        }
        {
            let fences = self
                .package_fences
                .lock()
                .map_err(|_| lock_poisoned("plugin package fences"))?;
            if fences.get(id).copied() != package_token {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "plugin package mutation is active or requires recovery".into(),
                ));
            }
        }
        if package_token.is_some() {
            self.ensure_no_retiring_worker(id)?;
        }
        self.reap_uncertain_startups();
        {
            let uncertain = self
                .uncertain_startups
                .lock()
                .map_err(|_| lock_poisoned("plugin uncertain startups"))?;
            if uncertain.contains_key(id) {
                return Err(PluginLoaderError::StartupUncertain(format!(
                    "plugin `{id}` requires startup recovery"
                )));
            }
        }
        // A worker that died without a caller observing it still holds its
        // map entry; `contains_key` alone would report it running forever
        // and refuse to restart it. Only a live generation short-circuits
        // here — a finished one is retired (ownership retained) and this
        // start spawns a fresh generation. The lookup guard is dropped
        // before retiring: retirement takes the instances write lock, and
        // an edition-2021 `if let` would otherwise hold the read guard
        // across that call.
        let existing = recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .cloned();
        if let Some(existing) = existing {
            if !existing.worker_finished() {
                before_events();
                return Ok(existing.token);
            }
            tracing::warn!(
                id = %id,
                token = existing.token,
                "plugin worker exited without being observed; retiring the dead generation"
            );
            if let Some(instance) = self.retire_generation_locked(id, existing.token) {
                self.retire_instance(id, existing.token, instance);
            }
        }
        if self
            .uncertain_startups
            .lock()
            .map_err(|_| lock_poisoned("plugin uncertain startups"))?
            .len()
            >= MAX_UNCERTAIN_STARTUPS
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "too many unresolved plugin startups".into(),
            ));
        }
        let plugin = self
            .get(id)
            .ok_or_else(|| PluginLoaderError::NotFound(id.to_owned()))?;
        if !plugin.available {
            return Err(PluginLoaderError::RuntimeUnavailable(
                plugin
                    .reason
                    .unwrap_or_else(|| "plugin is unavailable".to_owned()),
            ));
        }
        if !plugin.manifest.native_addons.is_empty()
            && !native_addons_allowed(&plugin.manifest, plugin.source)
        {
            return Err(untrusted_native_addons(&plugin.manifest.id));
        }
        let runtime = self.runtimes.get(plugin.manifest.runtime).ok_or_else(|| {
            PluginLoaderError::RuntimeUnavailable(plugin.manifest.runtime.to_string())
        })?;
        // Lifecycle entry point: a load that never finishes (a guest stuck
        // in onLoad, a native addon hanging) shows here as `starting` with
        // no matching `started`, which is the whole diagnostic.
        let start_started = std::time::Instant::now();
        tracing::info!(
            id = %id,
            runtime = %plugin.manifest.runtime,
            "plugin starting"
        );
        let token = self.next_token.fetch_add(1, Ordering::AcqRel);
        self.starts_inflight.fetch_add(1, Ordering::AcqRel);
        let _starting = StartingGuard(&self.starts_inflight);
        let artifact_fingerprint = crate::plugin_artifact_fingerprint(&plugin).ok();
        // Runtimes with an owned worker (napi-vm) serve the call queue on
        // their own thread, bypassing the generic worker: one thread per
        // plugin instead of two, with identical queue/deadline semantics.
        let uncertain = Arc::clone(&self.uncertain_startups);
        let uncertain_id = id.to_owned();
        let retain = Arc::new(move |worker| {
            recover_mutex(&uncertain, "plugin uncertain startups")
                .entry(uncertain_id.clone())
                .or_default()
                .push(worker);
        });
        let mut worker_ctx = WorkerContext::new(
            self.emit_bus.clone(),
            self.deadlines().start,
            id.to_owned(),
            token,
            Some(retain),
        );
        let (tx, worker) = if let Some(owned) =
            runtime.spawn_worker(&plugin.manifest, &plugin.directory, &worker_ctx)
        {
            match owned {
                Ok(owned) => owned.into_parts(),
                Err(error) => {
                    if matches!(error, PluginLoaderError::StartupUncertain(_)) {
                        recover_mutex(&self.uncertain_startups, "plugin uncertain startups")
                            .entry(id.to_owned())
                            .or_default();
                    }
                    tracing::warn!(
                        id = %id,
                        runtime = %plugin.manifest.runtime,
                        elapsed_ms = start_started.elapsed().as_millis() as u64,
                        %error,
                        "plugin failed to start"
                    );
                    return Err(error);
                }
            }
        } else {
            let instance = match runtime.load(&plugin.manifest, &plugin.directory) {
                Ok(instance) => instance,
                Err(error) => {
                    if matches!(error, PluginLoaderError::StartupUncertain(_)) {
                        recover_mutex(&self.uncertain_startups, "plugin uncertain startups")
                            .entry(id.to_owned())
                            .or_default();
                    }
                    tracing::warn!(
                        id = %id,
                        runtime = %plugin.manifest.runtime,
                        elapsed_ms = start_started.elapsed().as_millis() as u64,
                        %error,
                        "plugin failed to start"
                    );
                    return Err(error);
                }
            };
            let (tx, rx) = mpsc::unbounded_channel();
            let worker_id = id.to_owned();
            let worker = thread::Builder::new()
                .name("tiktools-plugin".to_owned())
                .spawn(move || run_instance_worker(worker_id, instance, rx))
                .map_err(|error| {
                    PluginLoaderError::Runtime(format!("could not start plugin worker: {error}"))
                })?;
            (tx, worker)
        };
        let artifact_fingerprint = artifact_fingerprint.filter(|before| {
            crate::plugin_artifact_fingerprint(&plugin).ok().as_ref() == Some(before)
        });
        // The worker thread is already spawned here, so a poisoned map
        // recovers instead of failing midway through the mutation.
        let instance = Arc::new(RunningInstance {
            token,
            manifest: Arc::new(plugin.manifest.clone()),
            events: worker_ctx.emit_sender(),
            artifact_fingerprint,
            kind: plugin.manifest.runtime,
            cold: AtomicBool::new(true),
            tx,
            admission: Arc::new(tokio::sync::Semaphore::new(
                crate::worker::PLUGIN_CALL_CAPACITY,
            )),
            worker: Mutex::new(Some(worker)),
        });
        if self.shutdown_admission_closed.load(Ordering::Acquire) {
            instance.events.close();
            instance.admission.close();
            let _ = instance.tx.send(WorkerMsg::Shutdown);
            self.retire_instance(id, token, instance);
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin manager is shutting down".into(),
            ));
        }
        recover_rwlock_write(&self.instances, "plugin instances").insert(id.to_owned(), instance);
        self.set_running(id, true);
        if package_token.is_none() {
            if let Err(error) = worker_ctx.activate_events(before_events) {
                if let Some(instance) = self.retire_generation_locked(id, token) {
                    self.retire_instance(id, token, instance);
                }
                return Err(PluginLoaderError::RuntimeUnavailable(error));
            }
        } else {
            worker_ctx.defer_events_to_package();
        }
        tracing::info!(
            id = %id,
            elapsed_ms = start_started.elapsed().as_millis() as u64,
            "plugin started"
        );
        Ok(token)
    }

    pub fn start_all(&self) -> Vec<(String, Result<(), PluginLoaderError>)> {
        self.list()
            .into_iter()
            .map(|plugin| {
                let id = plugin.manifest.id;
                let result = self.start(&id);
                (id, result)
            })
            .collect()
    }
}
