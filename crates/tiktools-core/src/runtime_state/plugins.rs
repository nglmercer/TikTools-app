//! Plugin runtime-state operations: install/enable activation, readiness
//! checks, retry bookkeeping, and poll-task lifecycle.

use crate::plugin_runtime::PluginActivation;
use crate::*;

impl AppCore {
    /// Pending remote application may have persisted a desired value before
    /// its runtime outcome was known. Never infer permission to run from it.
    pub(crate) fn plugin_apply_quarantined(&self, id: &str) -> bool {
        recover_rwlock_read(
            &self.plugin_state.apply_quarantine,
            "plugin apply quarantine",
        )
        .as_ref()
        .is_none_or(|blocked| blocked.contains(id))
    }

    /// Future remote-apply integration must call this while holding the
    /// invocation gate *before* persisting a runtime-pending transition.
    /// The durable journal takes over on restart. This does not fence other
    /// processes; product activation also requires shared admission.
    #[cfg(all(feature = "persistence", feature = "cloud-auth"))]
    #[allow(dead_code)]
    pub(crate) fn quarantine_plugin_apply_before_commit(&self, id: &str) -> bool {
        let mut guard = recover_rwlock_write(
            &self.plugin_state.apply_quarantine,
            "plugin apply quarantine",
        );
        let Some(blocked) = guard.as_mut() else {
            return false;
        };
        blocked.insert(id.to_owned());
        drop(guard);
        self.plugin_state
            .activation_revision
            .fetch_add(1, Ordering::AcqRel);
        self.rebuild_processor_index();
        true
    }

    /// Re-read durable recovery state under the invocation gate after native
    /// reconciliation. Failed reads block all plugin execution instead of
    /// clearing evidence. Never call concurrently with an in-flight apply.
    #[cfg(feature = "persistence")]
    #[allow(dead_code)]
    pub(crate) fn refresh_plugin_apply_quarantine_locked(
        &self,
    ) -> Result<(), crate::db::DatabaseError> {
        let loaded = self.db.plugin_apply_quarantine();
        *recover_rwlock_write(
            &self.plugin_state.apply_quarantine,
            "plugin apply quarantine",
        ) = loaded.as_ref().ok().cloned();
        self.plugin_state
            .activation_revision
            .fetch_add(1, Ordering::AcqRel);
        self.rebuild_processor_index();
        loaded.map(|_| ())
    }

    pub(crate) fn plugin_ready(&self, id: &str) -> bool {
        if self.plugin_apply_quarantined(id) {
            return false;
        }
        let Some(plugin) = self.plugins.get(id) else {
            return false;
        };
        if !plugin.available {
            return false;
        }
        // Activation is an in-memory snapshot refreshed on every state write,
        // so readiness checks never hit SQLite on hot paths. Plugins without
        // a persisted row default to installed and enabled.
        recover_rwlock_read(&self.plugin_state.activation, "plugin activation")
            .get(id)
            .is_none_or(|state| state.installed && state.enabled)
    }

    /// Records an install/enable write in the activation snapshot. Callers
    /// must persist first; this only moves the snapshot the hot path reads.
    pub(crate) fn set_plugin_activation(&self, id: &str, installed: bool, enabled: bool) {
        recover_rwlock_write(&self.plugin_state.activation, "plugin activation")
            .insert(id.to_owned(), PluginActivation { installed, enabled });
        self.plugin_state
            .activation_revision
            .fetch_add(1, Ordering::AcqRel);
    }

    #[cfg(feature = "plugin-install")]
    pub(crate) fn clear_plugin_activation(&self, id: &str) {
        recover_rwlock_write(&self.plugin_state.activation, "plugin activation").remove(id);
        self.plugin_state
            .activation_revision
            .fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn plugin_retry_allowed(&self, id: &str) -> bool {
        recover_mutex(&self.plugin_state.health, "plugin health")
            .get(id)
            .and_then(|health| health.next_retry_at)
            .is_none_or(|next_retry_at| std::time::Instant::now() >= next_retry_at)
    }

    pub(crate) fn record_plugin_failure(&self, id: &str, error: String) {
        let mut health = recover_mutex(&self.plugin_state.health, "plugin health");
        let entry = health.entry(id.to_owned()).or_insert(PluginHealth {
            consecutive_failures: 0,
            next_retry_at: None,
        });
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        let delay_seconds = plugin_backoff_seconds(entry.consecutive_failures);
        entry.next_retry_at =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(delay_seconds));
        if entry.consecutive_failures <= 5 {
            tracing::warn!(
                plugin = %id,
                failures = entry.consecutive_failures,
                retry_in_seconds = delay_seconds,
                %error,
                "plugin entered retry backoff"
            );
        }
    }

    pub(crate) fn record_plugin_success(&self, id: &str) {
        let was_unhealthy = recover_mutex(&self.plugin_state.health, "plugin health")
            .remove(id)
            .is_some_and(|health| health.consecutive_failures > 0);
        if was_unhealthy {
            tracing::info!(plugin = %id, "plugin recovered");
        }
    }

    /// Starts the background poll that lets plugins publish spontaneous
    /// events (hotkeys, timers, watchers).
    pub fn spawn_plugin_event_poll(self: &Arc<Self>, runtime: &tokio::runtime::Handle) {
        if self
            .shutdown_started
            .load(std::sync::atomic::Ordering::Acquire)
            || self
                .plugin_state
                .poll_started
                .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return;
        }
        self.start_enabled_event_subscribers();
        self.spawn_plugin_event_observer(runtime);
        #[cfg(feature = "plugin-install")]
        {
            self.adopt_user_plugin_packages();
            self.spawn_plugin_update_checker(runtime);
        }
        let core = Arc::clone(self);
        let shutdown = Arc::clone(&self.plugin_state.poll_shutdown);
        tracing::info!("plugin event poll started");
        let task = runtime.spawn(async move {
            let mut ticker = tokio::time::interval(PLUGIN_POLL_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    biased;
                    _ = shutdown.notified() => break,
                    _ = ticker.tick() => core.poll_plugin_events().await,
                }
            }
        });
        *recover_mutex(&self.plugin_state.poll_task, "plugin poll task") = Some(task);
        // Subscribe before spawning so the first push cannot race startup.
        // One independently owned receiver; no polling future can starve it.
        let mut emit_rx = self.plugins.subscribe_emitted_events();
        let core = Arc::clone(self);
        let shutdown = self.plugin_state.observer_shutdown.clone();
        let task = runtime.spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    () = shutdown.cancelled() => break,
                    emitted = emit_rx.recv() => match emitted {
                        Ok(emitted) => core.handle_pushed_event(emitted).await,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            core.record_event_gap();
                            tracing::warn!(skipped, "push bus lagged; event gap recorded");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    },
                }
            }
        });
        *recover_mutex(&self.plugin_state.push_task, "plugin push task") = Some(task);
    }
}

#[cfg(test)]
mod isolation_tests {
    use super::*;
    use std::{path::Path, sync::atomic::AtomicBool, time::Duration};
    use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};
    use tiktools_plugin_loader::{
        ManagedWorker, PluginInstance, PluginLoaderError, PluginRoot, PluginRuntime, PluginSource,
        RuntimeRegistry, WorkerContext,
    };

    struct SlowRuntime {
        polling: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
        sender: Arc<
            Mutex<Option<tokio::sync::broadcast::Sender<tiktools_plugin_loader::EmittedEvent>>>,
        >,
    }
    struct SlowInstance {
        polling: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
    }
    impl PluginInstance for SlowInstance {
        fn id(&self) -> &str {
            "isolation"
        }
        fn handle_message(&mut self, _: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
            self.polling.store(true, Ordering::Release);
            while !self.release.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(br#"{"events":[]}"#.to_vec())
        }
        fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
            Ok(())
        }
    }
    impl PluginRuntime for SlowRuntime {
        fn kind(&self) -> PluginRuntimeKind {
            PluginRuntimeKind::Process
        }
        fn load(
            &self,
            _: &PluginManifest,
            _: &Path,
        ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
            Ok(Box::new(SlowInstance {
                polling: Arc::clone(&self.polling),
                release: Arc::clone(&self.release),
            }))
        }
        fn spawn_worker(
            &self,
            _: &PluginManifest,
            _: &Path,
            ctx: &WorkerContext,
        ) -> Option<Result<ManagedWorker, PluginLoaderError>> {
            *self.sender.lock().unwrap() = Some(ctx.emit_sender());
            None
        }
    }
    struct NoopEmitter;
    impl HostEmitter for NoopEmitter {
        fn emit(&self, _: HostMessage) {}
    }

    #[tokio::test]
    async fn slow_poll_does_not_starve_push_and_both_owners_join() {
        let root =
            std::env::temp_dir().join(format!("tiktools-poll-isolation-{}", fastrand::u128(..)));
        let directory = root.join("isolation");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("entry.bin"), b"fixture").unwrap();
        std::fs::write(
            directory.join("plugin.json"),
            serde_json::to_vec(&json!({
                "schemaVersion": 3, "id": "isolation", "name": "Isolation", "version": "1.0.0",
                "runtime": "process", "entry": "entry.bin", "capabilities": ["events.publish"],
                "eventTypes": [{"type": "isolation.tick", "title": {"default": "Tick"}}]
            }))
            .unwrap(),
        )
        .unwrap();
        let polling = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let sender = Arc::new(Mutex::new(None));
        let mut runtimes = RuntimeRegistry::default();
        runtimes.register(Arc::new(SlowRuntime {
            polling: Arc::clone(&polling),
            release: Arc::clone(&release),
            sender: Arc::clone(&sender),
        }));
        let manager = Arc::new(PluginManager::with_runtimes(
            vec![PluginRoot {
                path: root.clone(),
                source: PluginSource::Development,
            }],
            runtimes,
        ));
        manager.scan().unwrap();
        manager.start("isolation").unwrap();
        let mut app = AppCore::new(Arc::new(NoopEmitter));
        app.plugins = manager;
        let core = Arc::new(app);
        let mut events = core.events.subscribe_domain();
        core.spawn_plugin_event_poll(&tokio::runtime::Handle::current());
        tokio::time::timeout(Duration::from_secs(2), async {
            while !polling.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        sender
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(tiktools_plugin_loader::EmittedEvent {
                plugin_id: "isolation".into(),
                event_type: "isolation.tick".into(),
                data: json!({"value": 1}),
            })
            .unwrap();
        let outcome = tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                if matches!(
                    events.recv().await.unwrap(),
                    crate::events::DomainEvent::PluginEvent { .. }
                ) {
                    break;
                }
            }
        })
        .await;
        // Release before asserting so a failed regression cannot retain a
        // blocked fixture thread or make cleanup hang.
        release.store(true, Ordering::Release);
        core.shutdown().await;
        assert!(outcome.is_ok(), "push was starved by the outstanding poll");
        assert!(recover_mutex(&core.plugin_state.poll_task, "poll").is_none());
        assert!(recover_mutex(&core.plugin_state.push_task, "push").is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
