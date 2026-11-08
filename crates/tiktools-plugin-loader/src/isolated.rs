//! One-shot plugin invocation with frozen package bytes and no Owner push bus.
//! Native and process plugins remain trusted OS code. Isolation here prevents
//! host-protocol authority from being inherited from a shared Owner worker.
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use tokio::sync::{oneshot, Semaphore};

use crate::{DiscoveredPlugin, PluginLoaderError, PluginRuntime};

/// Prepared bytes only. Construction performs no plugin load, initialization,
/// background callback or invocation. This value is linear and never parsed
/// from a remote request. Dropping it before dispatch removes the snapshot.
pub struct PreparedIsolatedPlugin {
    plugin: DiscoveredPlugin,
    runtime: Arc<dyn PluginRuntime>,
    snapshot: Option<PathBuf>,
    settings: Value,
    admission: Arc<Semaphore>,
}

impl PreparedIsolatedPlugin {
    pub(crate) fn new(
        plugin: DiscoveredPlugin,
        runtime: Arc<dyn PluginRuntime>,
        snapshot_directory: PathBuf,
        artifact_digest: &str,
        settings: Value,
        admission: Arc<Semaphore>,
    ) -> Result<Self, PluginLoaderError> {
        let settings_bytes = serde_json::to_vec(&settings).map_err(|_| {
            PluginLoaderError::RuntimeUnavailable("isolated plugin settings are invalid".into())
        })?;
        if !settings.is_object() || settings_bytes.len() > 64 * 1024 {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "isolated plugin settings exceed the admission budget".into(),
            ));
        }
        crate::artifact::freeze_plugin_artifact(&plugin, &snapshot_directory, artifact_digest)?;
        let mut prepared = Self {
            plugin,
            runtime,
            snapshot: Some(snapshot_directory.clone()),
            settings,
            admission,
        };
        prepared.plugin.directory = snapshot_directory;
        let data = prepared.plugin.directory.join(".data");
        fs::create_dir(&data).map_err(|_| {
            PluginLoaderError::RuntimeUnavailable(
                "isolated plugin data directory could not be created".into(),
            )
        })?;
        let per_plugin = data.join(&prepared.plugin.manifest.id);
        fs::create_dir(&per_plugin).map_err(|_| {
            PluginLoaderError::RuntimeUnavailable(
                "isolated plugin settings directory could not be created".into(),
            )
        })?;
        // Both existing process contracts (root and root/plugin-id) receive
        // exactly the filtered settings; neither points at Owner storage.
        for directory in [&data, &per_plugin] {
            fs::write(directory.join("settings.json"), &settings_bytes).map_err(|_| {
                PluginLoaderError::RuntimeUnavailable(
                    "isolated plugin settings could not be written".into(),
                )
            })?;
        }
        Ok(prepared)
    }

    /// Actual irreversible dispatch boundary. The host must commit its durable
    /// acceptance and revalidate the exact binding/current subjects before this
    /// synchronous call. No plugin code runs while merely preparing a call.
    /// The supplied request must already contain schema-checked arguments and
    /// a public/scoped event projection; host settings overwrite JSON input.
    pub fn dispatch(
        mut self,
        mut request: Value,
        timeout: Duration,
    ) -> Result<IsolatedPluginCall, PluginLoaderError> {
        let request_object = request.as_object_mut().ok_or_else(|| {
            PluginLoaderError::RuntimeUnavailable(
                "isolated plugin request must be an object".into(),
            )
        })?;
        request_object.insert("settings".into(), self.settings.clone());
        let request = serde_json::to_vec(&request).map_err(|_| {
            PluginLoaderError::RuntimeUnavailable(
                "isolated plugin request could not be encoded".into(),
            )
        })?;
        if request.len() > tiktools_plugin_api::MAX_FRAME_BYTES {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "isolated plugin request exceeds the frame budget".into(),
            ));
        }
        if timeout.is_zero() || timeout > Duration::from_secs(180) {
            return Err(PluginLoaderError::RuntimeUnavailable(
                "isolated plugin timeout is outside the admission budget".into(),
            ));
        }
        let admission = Arc::clone(&self.admission)
            .try_acquire_owned()
            .map_err(|_| PluginLoaderError::Overloaded("isolated plugin invocation".into()))?;
        let plugin = self.plugin.clone();
        let runtime = Arc::clone(&self.runtime);
        let snapshot = self
            .snapshot
            .take()
            .expect("prepared invocation owns its snapshot");
        let failed_snapshot = snapshot.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let deadline = Instant::now() + timeout;
        let (respond, answer) = oneshot::channel();
        let worker = thread::Builder::new()
            .name("tiktools-plugin-scoped".into())
            .spawn(move || {
                // A timed-out native call retains admission and the snapshot until
                // its thread actually exits. Repeated failures cannot create an
                // unbounded population of detached native workers.
                let _admission = admission;
                let result = (|| {
                    if worker_cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
                        return Err(PluginLoaderError::Timeout(
                            "isolated plugin expired before initialization".into(),
                        ));
                    }
                    // Direct VM loads use an orphan/closed push bus. This instance
                    // is never registered with the manager's Owner poll/observer
                    // workers and is never reused by a later Owner invocation.
                    let mut instance = runtime.load_isolated(
                        &plugin.manifest,
                        &plugin.directory,
                        &plugin.directory.join(".data"),
                    )?;
                    let result = if worker_cancelled.load(Ordering::Acquire)
                        || Instant::now() >= deadline
                    {
                        Err(PluginLoaderError::Timeout(
                            "isolated plugin expired during initialization; outcome unknown".into(),
                        ))
                    } else {
                        instance.handle_message_with_timeout(
                            &request,
                            deadline.saturating_duration_since(Instant::now()),
                        )
                    };
                    let shutdown = instance.shutdown();
                    match (result, shutdown) {
                        (Ok(bytes), Ok(()))
                            if bytes.len() <= tiktools_plugin_api::MAX_FRAME_BYTES =>
                        {
                            serde_json::from_slice(&bytes).map_err(|_| {
                                PluginLoaderError::Runtime(
                                    "isolated plugin returned invalid JSON".into(),
                                )
                            })
                        }
                        (Ok(_), Ok(())) => Err(PluginLoaderError::Runtime(
                            "isolated plugin returned an oversized response".into(),
                        )),
                        (Err(error), _) | (_, Err(error)) => Err(error),
                    }
                })();
                let _ = fs::remove_dir_all(snapshot);
                let _ = respond.send(result);
            });
        match worker {
            Ok(worker) => Ok(IsolatedPluginCall {
                answer,
                worker: Some(worker),
                deadline,
                cancelled,
            }),
            Err(_) => {
                let _ = fs::remove_dir_all(failed_snapshot);
                Err(PluginLoaderError::Runtime(
                    "isolated plugin dispatch thread could not be started".into(),
                ))
            }
        }
    }
}

impl Drop for PreparedIsolatedPlugin {
    fn drop(&mut self) {
        if let Some(snapshot) = self.snapshot.take() {
            let _ = fs::remove_dir_all(snapshot);
        }
    }
}

/// Response of one admitted invocation. Timeout/cancellation is an unknown
/// external outcome, never permission to retry or start a second instance.
pub struct IsolatedPluginCall {
    answer: oneshot::Receiver<Result<Value, PluginLoaderError>>,
    worker: Option<thread::JoinHandle<()>>,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}

impl IsolatedPluginCall {
    pub async fn wait(mut self) -> Result<Value, PluginLoaderError> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        let result = tokio::time::timeout(remaining, &mut self.answer).await;
        match result {
            Ok(Ok(result)) => {
                if let Some(worker) = self.worker.take() {
                    let _ = worker.join();
                }
                result
            }
            Ok(Err(_)) => Err(PluginLoaderError::Runtime(
                "isolated plugin worker died; outcome unknown".into(),
            )),
            Err(_) => Err(PluginLoaderError::Timeout(
                "isolated plugin call timed out; outcome unknown; do not replay".into(),
            )),
        }
    }
}

impl Drop for IsolatedPluginCall {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if self.worker.take().is_some() {
            tracing::warn!(
                "isolated plugin call abandoned; external outcome is unknown; no automatic replay"
            );
        }
    }
}

#[cfg(test)]
mod tests;
