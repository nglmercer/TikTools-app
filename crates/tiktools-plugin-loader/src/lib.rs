//! Runtime plugin discovery and lifecycle.
//!
//! The loader owns no GUI objects and has no compile-time plugin registry.
//! Every plugin is found through a package directory and a manifest at
//! runtime. Native plugins are trusted in-process code; process plugins are
//! isolated executables with a crash boundary, not an OS sandbox.

#[cfg(feature = "plugin-install")]
mod installer;
mod native;
mod process;
mod wasm;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use thiserror::Error;
use tiktools_plugin_api::{
    manifest::{is_safe_relative_path, ManifestError},
    PluginManifest, PluginRuntimeKind,
};

#[cfg(feature = "plugin-install")]
pub use installer::{InstalledPluginPackage, PluginInstaller};
pub use native::NativePluginRuntime;
pub use process::ProcessPluginRuntime;
pub use wasm::WasmPluginRuntime;

const MANIFEST_FILE: &str = "plugin.json";
const MAX_DIRECTORY_ENTRIES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginSource {
    Builtin,
    User,
    Development,
}

#[derive(Debug, Clone)]
pub struct PluginRoot {
    pub path: PathBuf,
    pub source: PluginSource,
}

#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub manifest: PluginManifest,
    pub directory: PathBuf,
    pub source: PluginSource,
    pub available: bool,
    pub reason: Option<String>,
    pub running: bool,
}

#[derive(Debug, Error)]
pub enum PluginLoaderError {
    #[error("plugin manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("plugin directory is not valid: {0}")]
    InvalidDirectory(String),
    #[error("plugin runtime is unavailable: {0}")]
    RuntimeUnavailable(String),
    #[error("plugin runtime failed: {0}")]
    Runtime(String),
    #[error("plugin `{0}` was not discovered")]
    NotFound(String),
    #[error("plugin call timed out: {0}")]
    Timeout(String),
}

pub trait PluginInstance: Send {
    fn id(&self) -> &str;
    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError>;
    fn handle_message_with_timeout(
        &mut self,
        request: &[u8],
        _timeout: std::time::Duration,
    ) -> Result<Vec<u8>, PluginLoaderError> {
        self.handle_message(request)
    }
    fn shutdown(&mut self) -> Result<(), PluginLoaderError>;
}

pub trait PluginRuntime: Send + Sync {
    fn kind(&self) -> PluginRuntimeKind;
    fn load(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError>;
}

/// One queued request with the absolute deadline the worker enforces.
/// The deadline covers queueing plus execution; the worker discards expired
/// requests before executing them so a timed-out caller never causes stale
/// plugin work to accumulate.
struct QueuedCall {
    request: Vec<u8>,
    timeout: Duration,
    deadline: Instant,
    respond: mpsc::Sender<Result<Vec<u8>, PluginLoaderError>>,
}

enum WorkerMsg {
    Call(QueuedCall),
    Shutdown,
}

/// A running plugin: the worker thread is the sole owner of the instance,
/// so plugin `&mut self` code stays single-threaded without a mutex. The
/// token identifies this start generation so failure cleanup never removes
/// a newer instance.
struct RunningInstance {
    token: u64,
    tx: mpsc::Sender<WorkerMsg>,
    worker: Mutex<Option<thread::JoinHandle<Result<(), PluginLoaderError>>>>,
}

fn run_instance_worker(
    id: String,
    mut instance: Box<dyn PluginInstance>,
    rx: mpsc::Receiver<WorkerMsg>,
) -> Result<(), PluginLoaderError> {
    while let Ok(msg) = rx.recv() {
        let WorkerMsg::Call(call) = msg else {
            break;
        };
        if Instant::now() >= call.deadline {
            let _ = call.respond.send(Err(PluginLoaderError::Timeout(format!(
                "plugin `{id}` call expired while queued"
            ))));
            continue;
        }
        let result = instance.handle_message_with_timeout(&call.request, call.timeout);
        let _ = call.respond.send(result);
    }
    // Fail waiters queued behind the shutdown instead of leaving them on
    // their deadlines.
    for queued in rx.try_iter() {
        if let WorkerMsg::Call(call) = queued {
            let _ = call.respond.send(Err(PluginLoaderError::Runtime(format!(
                "plugin `{id}` stopped"
            ))));
        }
    }
    instance.shutdown()
}

#[derive(Default)]
pub struct RuntimeRegistry {
    runtimes: BTreeMap<PluginRuntimeKind, Arc<dyn PluginRuntime>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        #[cfg(feature = "native-plugins")]
        registry.register(Arc::new(NativePluginRuntime));
        registry.register(Arc::new(ProcessPluginRuntime));
        registry.register(Arc::new(WasmPluginRuntime));
        registry
    }

    pub fn register(&mut self, runtime: Arc<dyn PluginRuntime>) {
        self.runtimes.insert(runtime.kind(), runtime);
    }

    fn get(&self, kind: PluginRuntimeKind) -> Option<Arc<dyn PluginRuntime>> {
        self.runtimes.get(&kind).cloned()
    }
}

#[derive(Default)]
struct PluginRegistry {
    entries: BTreeMap<String, DiscoveredPlugin>,
}

/// Runtime plugin manager. Discovery is deterministic: later roots override
/// earlier roots by plugin id, so development overrides can replace built-ins
/// without a compiled registration list.
pub struct PluginManager {
    roots: Vec<PluginRoot>,
    registry: RwLock<PluginRegistry>,
    runtimes: RuntimeRegistry,
    instances: RwLock<BTreeMap<String, Arc<RunningInstance>>>,
    lifecycle: Mutex<()>,
    next_token: AtomicU64,
}

impl PluginManager {
    pub fn new(roots: Vec<PluginRoot>) -> Self {
        Self::with_runtimes(roots, RuntimeRegistry::new())
    }

    pub fn with_runtimes(roots: Vec<PluginRoot>, runtimes: RuntimeRegistry) -> Self {
        Self {
            roots,
            registry: RwLock::new(PluginRegistry::default()),
            runtimes,
            instances: RwLock::new(BTreeMap::new()),
            lifecycle: Mutex::new(()),
            next_token: AtomicU64::new(1),
        }
    }

    pub fn roots(&self) -> &[PluginRoot] {
        &self.roots
    }

    pub fn scan(&self) -> Result<Vec<DiscoveredPlugin>, PluginLoaderError> {
        let mut discovered = BTreeMap::new();
        for root in &self.roots {
            if !root.path.exists() {
                continue;
            }
            let entries = fs::read_dir(&root.path).map_err(|error| {
                PluginLoaderError::InvalidDirectory(format!("{}: {error}", root.path.display()))
            })?;
            for (index, entry) in entries.enumerate() {
                if index >= MAX_DIRECTORY_ENTRIES {
                    tracing::warn!(path = %root.path.display(), "plugin directory entry limit reached");
                    break;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        tracing::warn!(path = %root.path.display(), %error, "could not inspect plugin entry");
                        continue;
                    }
                };
                let directory = entry.path();
                if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                    continue;
                }
                match read_discovered_plugin(&directory, root.source) {
                    Ok(plugin) => {
                        if let Some(previous) =
                            discovered.insert(plugin.manifest.id.clone(), plugin)
                        {
                            tracing::debug!(
                                id = %previous.manifest.id,
                                "plugin overridden by a later runtime root"
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(path = %directory.display(), %error, "plugin skipped")
                    }
                }
            }
        }

        let running_ids = self
            .instances
            .read()
            .expect("plugin instances poisoned")
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let result: Vec<_> = discovered
            .into_values()
            .map(|mut plugin| {
                // The active instance registry is authoritative. A scan must
                // never report a live runtime as stopped merely because it
                // rebuilt the discovered package record.
                plugin.running = running_ids.contains(&plugin.manifest.id);
                plugin
            })
            .collect();
        let mut registry = self.registry.write().expect("plugin registry poisoned");
        registry.entries = result
            .iter()
            .cloned()
            .map(|plugin| (plugin.manifest.id.clone(), plugin))
            .collect();
        Ok(result)
    }

    pub fn list(&self) -> Vec<DiscoveredPlugin> {
        self.registry
            .read()
            .expect("plugin registry poisoned")
            .entries
            .values()
            .cloned()
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<DiscoveredPlugin> {
        self.registry
            .read()
            .expect("plugin registry poisoned")
            .entries
            .get(id)
            .cloned()
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.instances
            .read()
            .expect("plugin instances poisoned")
            .contains_key(id)
    }

    pub fn start(&self, id: &str) -> Result<(), PluginLoaderError> {
        let _lifecycle = self.lifecycle.lock().expect("plugin lifecycle poisoned");
        if self
            .instances
            .read()
            .expect("plugin instances poisoned")
            .contains_key(id)
        {
            return Ok(());
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
        let runtime = self.runtimes.get(plugin.manifest.runtime).ok_or_else(|| {
            PluginLoaderError::RuntimeUnavailable(plugin.manifest.runtime.to_string())
        })?;
        let instance = runtime.load(&plugin.manifest, &plugin.directory)?;
        let token = self.next_token.fetch_add(1, Ordering::AcqRel);
        let (tx, rx) = mpsc::channel();
        let worker_id = id.to_owned();
        let worker = thread::Builder::new()
            .name("tiktools-plugin".to_owned())
            .spawn(move || run_instance_worker(worker_id, instance, rx))
            .map_err(|error| {
                PluginLoaderError::Runtime(format!("could not start plugin worker: {error}"))
            })?;
        self.instances
            .write()
            .expect("plugin instances poisoned")
            .insert(
                id.to_owned(),
                Arc::new(RunningInstance {
                    token,
                    tx,
                    worker: Mutex::new(Some(worker)),
                }),
            );
        self.set_running(id, true);
        Ok(())
    }

    pub fn stop(&self, id: &str) -> Result<(), PluginLoaderError> {
        let instance = {
            let _lifecycle = self.lifecycle.lock().expect("plugin lifecycle poisoned");
            self.remove_instance(id)
        };
        let Some(instance) = instance else {
            self.set_running(id, false);
            return Ok(());
        };
        let _ = instance.tx.send(WorkerMsg::Shutdown);
        let worker = instance
            .worker
            .lock()
            .expect("plugin worker poisoned")
            .take();
        let result = match worker {
            Some(worker) => worker
                .join()
                .map_err(|_| PluginLoaderError::Runtime(format!("plugin `{id}` worker panicked")))
                .and_then(|inner| inner),
            None => Ok(()),
        };
        self.set_running(id, false);
        result
    }

    pub fn stop_all(&self) {
        let ids: Vec<String> = self
            .instances
            .read()
            .expect("plugin instances poisoned")
            .keys()
            .cloned()
            .collect();
        for id in ids {
            if let Err(error) = self.stop(&id) {
                tracing::warn!(id = %id, %error, "plugin shutdown failed");
            }
        }
    }

    pub fn call(&self, id: &str, request: &Value) -> Result<Value, PluginLoaderError> {
        self.call_with_timeout(id, request, Duration::from_secs(30))
    }

    /// Calls one plugin instance. Calls stay serialized per instance through
    /// its worker queue, so a slow call delays (but never starves) later
    /// calls to the same plugin. Keep processor plugins fast and never
    /// combine slow model preparation with enrichment in one process.
    ///
    /// TODO(protocol-v2): per-QoS-class instances or multiplexed process
    /// requests would let realtime processors skip ahead of background
    /// actions sharing one process.
    pub fn call_with_timeout(
        &self,
        id: &str,
        request: &Value,
        timeout: Duration,
    ) -> Result<Value, PluginLoaderError> {
        self.call_with_deadline(id, request, Instant::now() + timeout)
    }

    /// Calls one plugin instance with an absolute deadline covering queueing
    /// plus execution. The worker discards requests that expire while queued
    /// instead of executing stale work; the waiter gives up at the same
    /// deadline. A timeout never removes the instance, while transport and
    /// protocol errors retire it so the next start loads fresh state.
    pub fn call_with_deadline(
        &self,
        id: &str,
        request: &Value,
        deadline: Instant,
    ) -> Result<Value, PluginLoaderError> {
        let bytes = serde_json::to_vec(request)
            .map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
        let instance = self
            .instances
            .read()
            .expect("plugin instances poisoned")
            .get(id)
            .cloned()
            .ok_or_else(|| PluginLoaderError::NotFound(id.to_owned()))?;
        let now = Instant::now();
        if now >= deadline {
            return Err(PluginLoaderError::Timeout(format!(
                "plugin `{id}` call already expired"
            )));
        }
        let remaining = deadline - now;
        let (respond, answer) = mpsc::channel();
        instance
            .tx
            .send(WorkerMsg::Call(QueuedCall {
                request: bytes,
                timeout: remaining,
                deadline,
                respond,
            }))
            .map_err(|_| {
                self.remove_failed_worker(id, instance.token);
                PluginLoaderError::Runtime(format!("plugin `{id}` worker is gone"))
            })?;
        let response = answer
            .recv_timeout(remaining)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => {
                    PluginLoaderError::Timeout(format!("plugin `{id}` call timed out"))
                }
                mpsc::RecvTimeoutError::Disconnected => {
                    self.remove_failed_worker(id, instance.token);
                    PluginLoaderError::Runtime(format!("plugin `{id}` worker died"))
                }
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

    fn set_running(&self, id: &str, running: bool) {
        if let Some(plugin) = self
            .registry
            .write()
            .expect("plugin registry poisoned")
            .entries
            .get_mut(id)
        {
            plugin.running = running;
        }
    }

    fn remove_instance(&self, id: &str) -> Option<Arc<RunningInstance>> {
        self.instances
            .write()
            .expect("plugin instances poisoned")
            .remove(id)
    }

    /// Retires the instance generation identified by `token`; a newer start
    /// is never disturbed. The worker is asked to stop but not joined, so a
    /// stuck plugin cannot block the hot path that observed the failure.
    fn remove_failed_worker(&self, id: &str, token: u64) {
        let removed = {
            let _lifecycle = self.lifecycle.lock().expect("plugin lifecycle poisoned");
            let mut instances = self.instances.write().expect("plugin instances poisoned");
            if instances
                .get(id)
                .is_some_and(|current| current.token == token)
            {
                instances.remove(id)
            } else {
                None
            }
        };
        if let Some(instance) = removed {
            let _ = instance.tx.send(WorkerMsg::Shutdown);
            self.set_running(id, false);
        }
    }
}

fn read_discovered_plugin(
    directory: &Path,
    source: PluginSource,
) -> Result<DiscoveredPlugin, PluginLoaderError> {
    let manifest_path = directory.join(MANIFEST_FILE);
    let bytes = fs::read(&manifest_path).map_err(|error| {
        PluginLoaderError::InvalidDirectory(format!("{}: {error}", manifest_path.display()))
    })?;
    if bytes.len() > 256 * 1024 {
        return Err(PluginLoaderError::Manifest(ManifestError::TooLarge));
    }
    let manifest =
        PluginManifest::from_json_str(std::str::from_utf8(&bytes).map_err(|_| {
            PluginLoaderError::InvalidDirectory("manifest is not UTF-8".to_owned())
        })?)?;
    let mut available = true;
    let mut reason = None;
    if let Err(error) = manifest.validate_compatibility() {
        available = false;
        reason = Some(error.to_string());
    } else if !manifest.target_matches_current_platform() {
        available = false;
        reason = Some("plugin has no build for this platform".to_owned());
    } else {
        let entry = manifest.entry.as_str();
        if !is_safe_relative_path(entry) {
            return Err(PluginLoaderError::Manifest(ManifestError::UnsafeEntry));
        }
        let package_root = fs::canonicalize(directory).map_err(|error| {
            PluginLoaderError::InvalidDirectory(format!("{}: {error}", directory.display()))
        })?;
        match fs::canonicalize(directory.join(entry)) {
            Ok(path) if path.starts_with(&package_root) && path.is_file() => {}
            Ok(_) => {
                available = false;
                reason = Some(format!("entry escapes the plugin directory: {entry}"));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                available = false;
                reason = Some(format!("entry does not exist: {entry}"));
            }
            Err(error) => {
                available = false;
                reason = Some(format!("entry could not be inspected: {error}"));
            }
        }
        if available && manifest.runtime == PluginRuntimeKind::Process && is_javascript_entry(entry)
        {
            available = false;
            reason = Some(
                "JavaScript plugin entries are not run by the desktop host; use a Rust ABI or standalone process plugin".to_owned(),
            );
        }
    }

    Ok(DiscoveredPlugin {
        manifest,
        directory: directory.to_owned(),
        source,
        available,
        reason,
        running: false,
    })
}

fn is_javascript_entry(entry: &str) -> bool {
    matches!(
        Path::new(entry)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "js" | "mjs" | "cjs" | "ts"
    )
}

pub fn plugin_roots(
    builtin: impl Into<PathBuf>,
    user: impl Into<PathBuf>,
    development: Option<PathBuf>,
) -> Vec<PluginRoot> {
    let mut roots = vec![
        PluginRoot {
            path: builtin.into(),
            source: PluginSource::Builtin,
        },
        PluginRoot {
            path: user.into(),
            source: PluginSource::User,
        },
    ];
    if let Some(path) = development {
        roots.push(PluginRoot {
            path,
            source: PluginSource::Development,
        });
    }
    roots
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    struct TestRuntime;

    struct TestInstance {
        id: String,
    }

    impl PluginRuntime for TestRuntime {
        fn kind(&self) -> PluginRuntimeKind {
            PluginRuntimeKind::Process
        }

        fn load(
            &self,
            manifest: &PluginManifest,
            _directory: &Path,
        ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
            Ok(Box::new(TestInstance {
                id: manifest.id.clone(),
            }))
        }
    }

    impl PluginInstance for TestInstance {
        fn id(&self) -> &str {
            &self.id
        }

        fn handle_message(&mut self, _request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
            Ok(b"null".to_vec())
        }

        fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
            Ok(())
        }
    }

    fn temp_root() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("tiktools-plugin-loader-{suffix}"))
    }

    #[test]
    fn later_root_overrides_by_id_without_compile_time_registration() {
        let first = temp_root();
        let second = temp_root();
        fs::create_dir_all(first.join("demo")).unwrap();
        fs::create_dir_all(second.join("demo")).unwrap();
        let manifest = |name: &str| {
            format!(
                r#"{{"schemaVersion":2,"id":"demo","name":"{name}","version":"1.0.0","runtime":"process","entry":"index.js"}}"#
            )
        };
        fs::write(first.join("demo/plugin.json"), manifest("builtin")).unwrap();
        fs::write(first.join("demo/index.js"), "").unwrap();
        fs::write(second.join("demo/plugin.json"), manifest("development")).unwrap();
        fs::write(second.join("demo/index.js"), "").unwrap();

        let manager = PluginManager::with_runtimes(
            plugin_roots(&first, temp_root(), Some(second.clone())),
            RuntimeRegistry::default(),
        );
        let plugins = manager.scan().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.name, "development");

        let _ = fs::remove_dir_all(first);
        let _ = fs::remove_dir_all(second);
    }

    #[test]
    fn scan_preserves_the_authoritative_running_instance_state() {
        let root = temp_root();
        fs::create_dir_all(root.join("demo")).unwrap();
        fs::write(
            root.join("demo/plugin.json"),
            r#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"index.bin"}"#,
        )
        .unwrap();
        fs::write(root.join("demo/index.bin"), b"test").unwrap();

        let mut runtimes = RuntimeRegistry::default();
        runtimes.register(Arc::new(TestRuntime));
        let manager =
            PluginManager::with_runtimes(plugin_roots(root.clone(), temp_root(), None), runtimes);
        manager.scan().unwrap();
        manager.start("demo").unwrap();
        assert!(manager.get("demo").unwrap().running);
        manager.scan().unwrap();
        assert!(manager.get("demo").unwrap().running);
        manager.stop("demo").unwrap();
        manager.scan().unwrap();
        assert!(!manager.get("demo").unwrap().running);

        let _ = fs::remove_dir_all(root);
    }

    type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, PluginLoaderError> + Send + Sync>;

    struct ScriptedRuntime {
        handler: Handler,
    }

    struct ScriptedInstance {
        id: String,
        handler: Handler,
    }

    impl PluginRuntime for ScriptedRuntime {
        fn kind(&self) -> PluginRuntimeKind {
            PluginRuntimeKind::Process
        }

        fn load(
            &self,
            manifest: &PluginManifest,
            _directory: &Path,
        ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
            Ok(Box::new(ScriptedInstance {
                id: manifest.id.clone(),
                handler: Arc::clone(&self.handler),
            }))
        }
    }

    impl PluginInstance for ScriptedInstance {
        fn id(&self) -> &str {
            &self.id
        }

        fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
            (self.handler)(request)
        }

        fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
            Ok(())
        }
    }

    fn write_plugin(root: &std::path::Path, id: &str) {
        fs::create_dir_all(root.join(id)).unwrap();
        fs::write(
            root.join(format!("{id}/plugin.json")),
            format!(
                r#"{{"schemaVersion":2,"id":"{id}","name":"{id}","version":"1.0.0","runtime":"process","entry":"entry.bin"}}"#
            ),
        )
        .unwrap();
        fs::write(root.join(format!("{id}/entry.bin")), b"fake").unwrap();
    }

    fn scripted_manager(ids: &[&str], handler: Handler) -> (Arc<PluginManager>, PathBuf) {
        let root = temp_root();
        for id in ids {
            write_plugin(&root, id);
        }
        let mut runtimes = RuntimeRegistry::default();
        runtimes.register(Arc::new(ScriptedRuntime { handler }) as Arc<dyn PluginRuntime>);
        let manager = Arc::new(PluginManager::with_runtimes(
            vec![PluginRoot {
                path: root.clone(),
                source: PluginSource::Development,
            }],
            runtimes,
        ));
        manager.scan().unwrap();
        (manager, root)
    }

    #[test]
    fn worker_discards_calls_that_expire_while_queued() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let executions = Arc::new(AtomicU64::new(0));
        let executions_for_handler = Arc::clone(&executions);
        let (manager, root) = scripted_manager(
            &["slow"],
            Arc::new(move |_| {
                executions_for_handler.fetch_add(1, Ordering::AcqRel);
                std::thread::sleep(std::time::Duration::from_millis(200));
                Ok(b"null".to_vec())
            }),
        );
        manager.start("slow").unwrap();
        let worker = Arc::clone(&manager);
        let first = std::thread::spawn(move || {
            worker.call_with_deadline(
                "slow",
                &serde_json::json!({"type": "poll"}),
                Instant::now() + Duration::from_secs(5),
            )
        });
        // Let the first call reach the worker before queueing an expired one.
        std::thread::sleep(Duration::from_millis(50));
        let expired = manager.call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_millis(25),
        );
        assert!(
            matches!(expired, Err(PluginLoaderError::Timeout(_))),
            "expired queued call should time out, got {expired:?}"
        );
        assert!(first.join().unwrap().is_ok());
        assert_eq!(executions.load(Ordering::Acquire), 1);
        manager.stop_all();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn timeout_keeps_a_healthy_instance_running() {
        let (manager, root) = scripted_manager(
            &["slow"],
            Arc::new(|_| {
                std::thread::sleep(Duration::from_millis(100));
                Ok(b"null".to_vec())
            }),
        );
        manager.start("slow").unwrap();
        let expired = manager.call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_millis(10),
        );
        assert!(matches!(expired, Err(PluginLoaderError::Timeout(_))));
        assert!(manager.is_running("slow"));
        let recovered = manager.call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_secs(5),
        );
        assert!(recovered.is_ok());
        manager.stop_all();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn transport_errors_retire_the_instance_and_restart_recovers() {
        use std::sync::atomic::{AtomicU64, Ordering};
        let calls = Arc::new(AtomicU64::new(0));
        let calls_for_handler = Arc::clone(&calls);
        let (manager, root) = scripted_manager(
            &["flaky"],
            Arc::new(move |_| {
                if calls_for_handler.fetch_add(1, Ordering::AcqRel) == 0 {
                    return Err(PluginLoaderError::Runtime("boom".to_owned()));
                }
                Ok(b"null".to_vec())
            }),
        );
        manager.start("flaky").unwrap();
        assert!(manager
            .call_with_deadline(
                "flaky",
                &serde_json::json!({"type": "poll"}),
                Instant::now() + Duration::from_secs(5),
            )
            .is_err());
        assert!(!manager.is_running("flaky"));
        manager.start("flaky").unwrap();
        assert!(manager
            .call_with_deadline(
                "flaky",
                &serde_json::json!({"type": "poll"}),
                Instant::now() + Duration::from_secs(5),
            )
            .is_ok());
        manager.stop_all();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn worker_panic_isolates_the_plugin_without_hanging_siblings() {
        let (manager, root) = scripted_manager(
            &["doomed", "healthy"],
            Arc::new(|request| {
                if request.windows(6).any(|window| window == b"doomed") {
                    panic!("native plugin bug");
                }
                Ok(b"null".to_vec())
            }),
        );
        manager.start("doomed").unwrap();
        manager.start("healthy").unwrap();
        let failed = manager.call_with_deadline(
            "doomed",
            &serde_json::json!({"type": "doomed"}),
            Instant::now() + Duration::from_secs(5),
        );
        assert!(
            matches!(failed, Err(PluginLoaderError::Runtime(_))),
            "panicking worker should surface a runtime error, got {failed:?}"
        );
        assert!(!manager.is_running("doomed"));
        let sibling = manager.call_with_deadline(
            "healthy",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_secs(5),
        );
        assert!(sibling.is_ok());
        manager.stop_all();
        let _ = fs::remove_dir_all(root);
    }
}
