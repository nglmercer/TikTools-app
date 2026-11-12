//! Package transaction fences and exact-generation event permits. Admission is synchronous;
//! permits own delivery counts across awaits.

use super::{lock_poisoned, PluginManager};
use crate::{worker::WorkerMsg, PluginLoaderError};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tiktools_plugin_api::{
    sync::{recover_mutex, recover_rwlock_read},
    PluginManifest,
};

/// Exclusive admission for one package transaction. Dropping without a
/// successful finish deliberately leaves the package fenced for this manager.
pub struct PluginPackageLease {
    manager: Arc<PluginManager>,
    id: String,
    token: u64,
    failed: AtomicBool,
    finished: AtomicBool,
    started: Mutex<Option<u64>>,
}

/// A host event admitted for a specific live plugin generation. Holding this
/// permit prevents a package transaction from beginning until delivery has
/// completed or the future is dropped. It does not fence non-package stops or
/// effects already dispatched by the event.
pub struct PluginEventDeliveryPermit {
    deliveries: Arc<Mutex<BTreeMap<String, usize>>>,
    id: String,
    manifest: Arc<PluginManifest>,
}

impl PluginEventDeliveryPermit {
    /// The immutable manifest with which this exact worker was started.
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
}

impl Drop for PluginEventDeliveryPermit {
    fn drop(&mut self) {
        let mut deliveries = recover_mutex(&self.deliveries, "plugin event deliveries");
        if let Some(count) = deliveries.get_mut(&self.id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                deliveries.remove(&self.id);
            }
        }
    }
}

impl PluginPackageLease {
    pub fn stop_if_present(&self) -> Result<bool, PluginLoaderError> {
        let result = self.manager.stop_with_package(&self.id, Some(self.token));
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }

    pub fn start(&self) -> Result<(), PluginLoaderError> {
        let result = self
            .manager
            .start_generation_with_package(&self.id, || {}, Some(self.token))
            .and_then(|generation| {
                *self
                    .started
                    .lock()
                    .map_err(|_| lock_poisoned("plugin package lease generation"))? =
                    Some(generation);
                Ok(())
            });
        if result.is_err() {
            self.failed.store(true, Ordering::Release);
        }
        result
    }

    pub fn finish(self) -> Result<(), PluginLoaderError> {
        if self.failed.load(Ordering::Acquire) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin package lease requires recovery".into(),
            ));
        }
        let _lifecycle = self.manager.lifecycle.try_lock().map_err(|_| {
            PluginLoaderError::RuntimeUnavailable("plugin lifecycle is busy or unavailable".into())
        })?;
        self.manager.reap_retired();
        self.manager.ensure_startup_known(&self.id)?;
        self.manager.ensure_no_retiring_worker(&self.id)?;
        let mut fences = self
            .manager
            .package_fences
            .lock()
            .map_err(|_| lock_poisoned("plugin package fences"))?;
        if fences.get(&self.id) != Some(&self.token) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin package lease is invalid".into(),
            ));
        }
        let started = *self
            .started
            .lock()
            .map_err(|_| lock_poisoned("plugin package lease generation"))?;
        if let Some(generation) = started {
            let instances = self
                .manager
                .instances
                .read()
                .map_err(|_| lock_poisoned("plugin instances"))?;
            let instance = instances
                .get(&self.id)
                .filter(|instance| instance.token == generation)
                .ok_or_else(|| {
                    PluginLoaderError::RuntimeUnavailable(
                        "package worker generation changed".into(),
                    )
                })?;
            if !instance
                .worker
                .lock()
                .map_err(|_| lock_poisoned("plugin worker"))?
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
            {
                return Err(PluginLoaderError::RuntimeUnavailable(
                    "package worker exited before completion".into(),
                ));
            }
            instance
                .events
                .activate_with(|| {
                    fences.remove(&self.id);
                })
                .map_err(PluginLoaderError::RuntimeUnavailable)?;
        } else {
            fences.remove(&self.id);
        }
        self.finished.store(true, Ordering::Release);
        Ok(())
    }
}

impl Drop for PluginPackageLease {
    fn drop(&mut self) {
        if self.finished.load(Ordering::Acquire) {
            return;
        }
        if let Ok(started) = self.started.lock() {
            if let Some(generation) = *started {
                if let Ok(instances) = self.manager.instances.read() {
                    if let Some(instance) = instances
                        .get(&self.id)
                        .filter(|instance| instance.token == generation)
                    {
                        instance.events.close();
                        instance.admission.close();
                        let _ = instance.tx.send(WorkerMsg::Shutdown);
                    }
                }
            }
        }
    }
}

impl PluginManager {
    /// Fence all ordinary starts while a package transaction is in progress.
    /// A failed or interrupted transaction keeps the fence until restart.
    pub fn begin_package_mutation(
        self: &Arc<Self>,
        id: &str,
    ) -> Result<PluginPackageLease, PluginLoaderError> {
        let _lifecycle = self.lifecycle.try_lock().map_err(|_| {
            PluginLoaderError::RuntimeUnavailable("plugin lifecycle is busy or unavailable".into())
        })?;
        if self.shutdown_admission_closed.load(Ordering::Acquire) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin manager is shutting down".into(),
            ));
        }
        self.ensure_startup_known(id)?;
        self.reap_retired();
        self.ensure_no_retiring_worker(id)?;
        let mut fences = self
            .package_fences
            .lock()
            .map_err(|_| lock_poisoned("plugin package fences"))?;
        if fences.contains_key(id) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin package mutation is already active or requires recovery".into(),
            ));
        }
        if self
            .event_deliveries
            .lock()
            .map_err(|_| lock_poisoned("plugin event deliveries"))?
            .contains_key(id)
        {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "plugin event publication is still in progress".into(),
            ));
        }
        let token = self
            .next_package_token
            .try_update(Ordering::AcqRel, Ordering::Acquire, |token| {
                token.checked_add(1)
            })
            .map_err(|_| {
                PluginLoaderError::RuntimeUnavailable("plugin token space exhausted".into())
            })?;
        fences.insert(id.to_owned(), token);
        Ok(PluginPackageLease {
            manager: Arc::clone(self),
            id: id.to_owned(),
            token,
            failed: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            started: Mutex::new(None),
        })
    }

    pub fn running_generation(&self, id: &str) -> Option<u64> {
        if self
            .package_fences
            .lock()
            .map_or(true, |fences| fences.contains_key(id))
        {
            return None;
        }
        recover_rwlock_read(&self.instances, "plugin instances")
            .get(id)
            .map(|instance| instance.token)
    }

    /// Fail closed if the precise host-owned worker has exited or its map or
    /// worker lock is poisoned. This is a snapshot, not an effects lease.
    pub fn is_live_generation(&self, id: &str, generation: u64) -> bool {
        let Ok(fences) = self.package_fences.lock() else {
            return false;
        };
        if fences.contains_key(id) {
            return false;
        }
        let Ok(instances) = self.instances.read() else {
            return false;
        };
        let Some(instance) = instances
            .get(id)
            .filter(|instance| instance.token == generation)
        else {
            return false;
        };
        let Ok(worker) = instance.worker.lock() else {
            return false;
        };
        worker.as_ref().is_some_and(|handle| !handle.is_finished())
    }

    /// Atomically admit event delivery for the exact live worker and exclude
    /// package mutation for this plugin until the returned permit is dropped.
    /// A blocked delivery makes package mutation fail quickly; it never holds
    /// a mutex across an await or prevents unrelated plugins from mutating.
    pub fn try_reserve_event_delivery(
        &self,
        id: &str,
        generation: u64,
    ) -> Option<PluginEventDeliveryPermit> {
        let fences = self.package_fences.lock().ok()?;
        if fences.contains_key(id) {
            return None;
        }
        let instances = self.instances.read().ok()?;
        let instance = instances
            .get(id)
            .filter(|instance| instance.token == generation)?;
        let worker = instance.worker.lock().ok()?;
        if worker.as_ref().is_none_or(|handle| handle.is_finished()) {
            return None;
        }
        let mut deliveries = self.event_deliveries.lock().ok()?;
        let count = deliveries.get(id).copied().unwrap_or(0).checked_add(1)?;
        deliveries.insert(id.to_owned(), count);
        Some(PluginEventDeliveryPermit {
            deliveries: Arc::clone(&self.event_deliveries),
            id: id.to_owned(),
            manifest: Arc::clone(&instance.manifest),
        })
    }
}
