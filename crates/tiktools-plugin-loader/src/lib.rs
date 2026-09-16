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
    sync::{Arc, Mutex, RwLock},
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

type PluginInstanceHandle = Arc<Mutex<Box<dyn PluginInstance>>>;

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
    instances: RwLock<BTreeMap<String, PluginInstanceHandle>>,
    lifecycle: Mutex<()>,
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
        self.instances
            .write()
            .expect("plugin instances poisoned")
            .insert(id.to_owned(), Arc::new(Mutex::new(instance)));
        self.set_running(id, true);
        Ok(())
    }

    pub fn stop(&self, id: &str) -> Result<(), PluginLoaderError> {
        let instance = {
            let _lifecycle = self.lifecycle.lock().expect("plugin lifecycle poisoned");
            self.remove_instance(id)
        };
        if let Some(instance) = instance {
            let result = instance
                .lock()
                .expect("plugin instance poisoned")
                .shutdown();
            self.set_running(id, false);
            return result;
        }
        self.set_running(id, false);
        Ok(())
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
        self.call_with_timeout(id, request, std::time::Duration::from_secs(30))
    }

    /// Calls one plugin instance. Instance calls stay serialized behind the
    /// per-instance mutex, so a slow call blocks later calls to the same
    /// plugin (including fast pre-filter processors that share the process).
    /// Keep processor plugins fast and never combine slow model preparation
    /// with enrichment in one process.
    ///
    /// TODO(protocol-v2): lift this without breaking plugins that assume
    /// single-threaded `&mut self` state. Candidates: (a) multiplexed process
    /// requests with request/response ids, (b) separate service instances per
    /// QoS class (realtime processor vs background action vs poll), or (c)
    /// manifest-declared per-class concurrency.
    pub fn call_with_timeout(
        &self,
        id: &str,
        request: &Value,
        timeout: std::time::Duration,
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
        let response = instance
            .lock()
            .expect("plugin instance poisoned")
            .handle_message_with_timeout(&bytes, timeout);
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                self.remove_failed_instance(id, &instance);
                return Err(error);
            }
        };
        match serde_json::from_slice(&response) {
            Ok(response) => Ok(response),
            Err(error) => {
                self.remove_failed_instance(id, &instance);
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

    fn remove_instance(&self, id: &str) -> Option<PluginInstanceHandle> {
        self.instances
            .write()
            .expect("plugin instances poisoned")
            .remove(id)
    }

    fn remove_failed_instance(&self, id: &str, failed: &PluginInstanceHandle) {
        let removed = {
            let _lifecycle = self.lifecycle.lock().expect("plugin lifecycle poisoned");
            let mut instances = self.instances.write().expect("plugin instances poisoned");
            if instances
                .get(id)
                .is_some_and(|current| Arc::ptr_eq(current, failed))
            {
                instances.remove(id)
            } else {
                None
            }
        };
        if let Some(instance) = removed {
            if let Err(error) = instance
                .lock()
                .expect("plugin instance poisoned")
                .shutdown()
            {
                tracing::warn!(id = %id, %error, "failed plugin shutdown after an unhealthy call");
            }
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
}
