use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};

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
    kind: PluginRuntimeKind,
    handler: Handler,
}

struct ScriptedInstance {
    id: String,
    handler: Handler,
}

impl PluginRuntime for ScriptedRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        self.kind
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

fn write_plugin_with_runtime(root: &std::path::Path, id: &str, runtime: &str) {
    fs::create_dir_all(root.join(id)).unwrap();
    fs::write(
        root.join(format!("{id}/plugin.json")),
        format!(
            r#"{{"schemaVersion":2,"id":"{id}","name":"{id}","version":"1.0.0","runtime":"{runtime}","entry":"entry.bin"}}"#
        ),
    )
    .unwrap();
    fs::write(root.join(format!("{id}/entry.bin")), b"fake").unwrap();
}

fn scripted_manager(ids: &[&str], handler: Handler) -> (Arc<PluginManager>, PathBuf) {
    scripted_manager_with_kind(ids, PluginRuntimeKind::Process, handler)
}

fn scripted_manager_with_kind(
    ids: &[&str],
    kind: PluginRuntimeKind,
    handler: Handler,
) -> (Arc<PluginManager>, PathBuf) {
    let root = temp_root();
    let runtime_name = match kind {
        PluginRuntimeKind::Native => "native",
        PluginRuntimeKind::Wasm => "wasm",
        PluginRuntimeKind::Process => "process",
    };
    for id in ids {
        write_plugin_with_runtime(&root, id, runtime_name);
    }
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(ScriptedRuntime { kind, handler }) as Arc<dyn PluginRuntime>);
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
    // Native workers run plugin code synchronously and never
    // self-terminate on a manager timeout, so the instance stays usable.
    // (Process workers kill their child on the same deadline instead;
    // see process_timeout_retires_the_instance_so_next_start_is_fresh.)
    let (manager, root) = scripted_manager_with_kind(
        &["slow"],
        PluginRuntimeKind::Native,
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
fn process_timeout_retires_the_instance_so_next_start_is_fresh() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let calls = Arc::new(AtomicU64::new(0));
    let calls_for_handler = Arc::clone(&calls);
    let (manager, root) = scripted_manager(
        &["slow"],
        Arc::new(move |_| {
            // Fast first call consumes the cold-start grace; the second
            // hangs past the deadline like a child that never answers.
            if calls_for_handler.fetch_add(1, Ordering::AcqRel) == 1 {
                std::thread::sleep(Duration::from_millis(300));
            }
            Ok(b"null".to_vec())
        }),
    );
    manager.start("slow").unwrap();
    let warmed = manager.call_with_deadline(
        "slow",
        &serde_json::json!({"type": "poll"}),
        Instant::now() + Duration::from_secs(5),
    );
    assert!(warmed.is_ok());
    let timed_out = manager.call_with_deadline(
        "slow",
        &serde_json::json!({"type": "poll"}),
        Instant::now() + Duration::from_millis(25),
    );
    assert!(
        matches!(timed_out, Err(PluginLoaderError::Timeout(_))),
        "hung process call should time out, got {timed_out:?}"
    );
    // A timed-out process worker kills its child, so the instance is
    // dead: keeping it would fail the next call instantly with
    // "plugin process stdin is unavailable".
    assert!(!manager.is_running("slow"));
    manager.start("slow").unwrap();
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
fn process_first_call_gets_cold_start_grace_once() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let calls = Arc::new(AtomicU64::new(0));
    let calls_for_handler = Arc::clone(&calls);
    let (manager, root) = scripted_manager(
        &["cold"],
        Arc::new(move |_| {
            // Slow engine build on the first two invocations: only the
            // first call of the generation gets grace.
            if calls_for_handler.fetch_add(1, Ordering::AcqRel) < 2 {
                std::thread::sleep(Duration::from_millis(400));
            }
            Ok(b"null".to_vec())
        }),
    );
    manager.start("cold").unwrap();
    let warmed = manager.call_with_deadline(
        "cold",
        &serde_json::json!({"type": "poll"}),
        Instant::now() + Duration::from_millis(100),
    );
    assert!(
        warmed.is_ok(),
        "first process call should get cold-start grace, got {warmed:?}"
    );
    let steady = manager.call_with_deadline(
        "cold",
        &serde_json::json!({"type": "poll"}),
        Instant::now() + Duration::from_millis(100),
    );
    assert!(
        matches!(steady, Err(PluginLoaderError::Timeout(_))),
        "second slow call should time out without grace, got {steady:?}"
    );
    assert!(!manager.is_running("cold"));
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
