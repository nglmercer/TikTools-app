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

#[test]
fn declarative_plugins_start_without_an_entry_and_reject_calls() {
    let root = temp_root();
    fs::create_dir_all(root.join("decl")).unwrap();
    fs::write(
        root.join("decl/plugin.json"),
        r#"{"schemaVersion":3,"id":"decl","name":"Decl","version":"1.0.0","runtime":"declarative"}"#,
    )
    .unwrap();

    let manager = PluginManager::with_runtimes(
        plugin_roots(temp_root(), temp_root(), Some(root.clone())),
        RuntimeRegistry::new(),
    );
    let plugins = manager.scan().unwrap();
    assert_eq!(plugins.len(), 1);
    assert!(plugins[0].available);

    manager.start("decl").unwrap();
    assert!(manager.get("decl").unwrap().running);
    let error = manager
        .call("decl", &serde_json::json!({"jsonrpc": "2.0"}))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("interpreted by the host"),
        "unexpected error: {error}"
    );
    manager.stop("decl").unwrap();
    assert!(!manager.get("decl").unwrap().running);

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

/// Runtime whose instances panic during shutdown, so the worker thread
/// dies and the manager's join reports a runtime error.
struct PanicShutdownRuntime;

struct PanicShutdownInstance {
    id: String,
}

impl PluginRuntime for PanicShutdownRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Process
    }

    fn load(
        &self,
        manifest: &PluginManifest,
        _directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        Ok(Box::new(PanicShutdownInstance {
            id: manifest.id.clone(),
        }))
    }
}

impl PluginInstance for PanicShutdownInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, _request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        Ok(b"null".to_vec())
    }

    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        panic!("test shutdown boom");
    }
}

/// Runtime whose instances fail shutdown with a typed error (no panic),
/// covering the worker-error branch of the shutdown path.
struct ErrShutdownRuntime;

struct ErrShutdownInstance {
    id: String,
}

impl PluginRuntime for ErrShutdownRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Native
    }

    fn load(
        &self,
        manifest: &PluginManifest,
        _directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        Ok(Box::new(ErrShutdownInstance {
            id: manifest.id.clone(),
        }))
    }
}

impl PluginInstance for ErrShutdownInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, _request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        Ok(b"null".to_vec())
    }

    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        Err(PluginLoaderError::Runtime(
            "test shutdown failure".to_owned(),
        ))
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
        PluginRuntimeKind::Declarative => "declarative",
        PluginRuntimeKind::NapiVm => "napi-vm",
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
fn cold_start_claim_extends_exactly_one_caller_per_generation() {
    let (manager, root) = scripted_manager(&["cold"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.start("cold").unwrap();
    let short = Duration::from_millis(100);
    let claimed = manager.claim_cold_start_grace("cold", short);
    assert!(
        claimed >= Duration::from_secs(10),
        "first claim should win the grace, got {claimed:?}"
    );
    // The winning claim consumes the worker's swap too: later claims and
    // unknown ids keep their own budgets.
    assert_eq!(manager.claim_cold_start_grace("cold", short), short);
    assert_eq!(manager.claim_cold_start_grace("missing", short), short);
    // Larger budgets are never shrunk by a claim on a fresh generation.
    manager.stop("cold").unwrap();
    manager.start("cold").unwrap();
    let long = Duration::from_secs(30);
    assert_eq!(manager.claim_cold_start_grace("cold", long), long);
    manager.stop_all();
    let _ = fs::remove_dir_all(root);

    let (native, native_root) = scripted_manager_with_kind(
        &["inproc"],
        PluginRuntimeKind::Native,
        Arc::new(|_| Ok(b"null".to_vec())),
    );
    native.start("inproc").unwrap();
    assert_eq!(
        native.claim_cold_start_grace("inproc", short),
        short,
        "in-process instances never need cold-start grace"
    );
    native.stop_all();
    let _ = fs::remove_dir_all(native_root);
}

#[test]
fn claimed_grace_is_not_re_extended_by_the_worker() {
    let (manager, root) = scripted_manager(
        &["slow"],
        Arc::new(|_| {
            std::thread::sleep(Duration::from_millis(300));
            Ok(b"null".to_vec())
        }),
    );
    manager.start("slow").unwrap();
    // An invoker-style claim consumes the generation's grace...
    let effective = manager.claim_cold_start_grace("slow", Duration::from_millis(100));
    assert!(effective >= Duration::from_secs(10));
    // ...so a direct caller that was not told about the claim still times
    // out on its own short deadline instead of being extended twice.
    let timed_out = manager.call_with_deadline(
        "slow",
        &serde_json::json!({"type": "poll"}),
        Instant::now() + Duration::from_millis(100),
    );
    assert!(
        matches!(timed_out, Err(PluginLoaderError::Timeout(_))),
        "consumed grace must not re-extend, got {timed_out:?}"
    );
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

fn lock_poisoned(result: &Result<Vec<DiscoveredPlugin>, PluginLoaderError>) -> bool {
    matches!(result, Err(PluginLoaderError::LockPoisoned(_)))
}

#[test]
fn scan_reports_poisoned_locks_as_a_typed_error() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.poison_locks_for_test();
    let scanned = manager.scan();
    assert!(
        lock_poisoned(&scanned),
        "poisoned scan should fail typed, got {scanned:?}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn start_reports_poisoned_locks_as_a_typed_error() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.poison_locks_for_test();
    let started = manager.start("demo");
    assert!(
        matches!(started, Err(PluginLoaderError::LockPoisoned(_))),
        "poisoned start should fail typed, got {started:?}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stop_reports_a_poisoned_lifecycle_lock_as_a_typed_error() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.start("demo").unwrap();
    manager.poison_locks_for_test();
    let stopped = manager.stop("demo");
    assert!(
        matches!(stopped, Err(PluginLoaderError::LockPoisoned(_))),
        "poisoned stop should fail typed, got {stopped:?}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn call_reports_poisoned_locks_as_a_typed_error() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.start("demo").unwrap();
    manager.poison_locks_for_test();
    let called = manager.call_with_timeout(
        "demo",
        &serde_json::json!({"type": "poll"}),
        Duration::from_secs(5),
    );
    assert!(
        matches!(called, Err(PluginLoaderError::LockPoisoned(_))),
        "poisoned call should fail typed, got {called:?}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn read_paths_recover_from_poisoned_locks() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.start("demo").unwrap();
    manager.poison_locks_for_test();
    assert_eq!(manager.list().len(), 1);
    assert!(manager.get("demo").is_some());
    assert!(manager.is_running("demo"));
    let grace = manager.claim_cold_start_grace("demo", Duration::from_secs(1));
    assert!(grace >= Duration::from_secs(1));
    // stop_all is best-effort: it recovers the poisoned lifecycle lock
    // and shuts the instance down instead of failing typed.
    manager.stop_all();
    assert!(!manager.is_running("demo"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stop_recovers_from_a_poisoned_worker_lock() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.start("demo").unwrap();
    manager.poison_worker_for_test("demo");
    let stopped = manager.stop("demo");
    assert!(
        stopped.is_ok(),
        "poisoned worker lock should recover, got {stopped:?}"
    );
    assert!(!manager.is_running("demo"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stop_all_recovers_from_a_poisoned_lifecycle_lock() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.start("demo").unwrap();
    manager.poison_locks_for_test();
    // The strict single-stop still reports the poisoned lifecycle typed...
    let stopped = manager.stop("demo");
    assert!(
        matches!(stopped, Err(PluginLoaderError::LockPoisoned(_))),
        "poisoned stop should fail typed, got {stopped:?}"
    );
    assert!(manager.is_running("demo"));
    // ...while the shutdown path recovers and cleans the instance up.
    manager.stop_all();
    assert!(!manager.is_running("demo"));
    assert!(
        !manager
            .get("demo")
            .map(|plugin| plugin.running)
            .unwrap_or(true),
        "shutdown must clear the running flag"
    );
    // Shutdown stays idempotent after recovery.
    manager.stop_all();
    assert!(!manager.is_running("demo"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stop_all_recovers_from_a_poisoned_worker_lock() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.start("demo").unwrap();
    manager.poison_worker_for_test("demo");
    manager.stop_all();
    assert!(
        !manager.is_running("demo"),
        "shutdown must clean up a poisoned worker handle"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn one_broken_plugin_does_not_block_sibling_shutdown() {
    let root = temp_root();
    write_plugin_with_runtime(&root, "doomed", "process");
    write_plugin_with_runtime(&root, "flaky", "native");
    write_plugin_with_runtime(&root, "healthy", "wasm");
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(PanicShutdownRuntime) as Arc<dyn PluginRuntime>);
    runtimes.register(Arc::new(ErrShutdownRuntime) as Arc<dyn PluginRuntime>);
    runtimes.register(Arc::new(ScriptedRuntime {
        kind: PluginRuntimeKind::Wasm,
        handler: Arc::new(|_| Ok(b"null".to_vec())),
    }) as Arc<dyn PluginRuntime>);
    let manager = PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    );
    manager.scan().unwrap();
    manager.start("doomed").unwrap();
    manager.start("flaky").unwrap();
    manager.start("healthy").unwrap();
    // The panicking worker and the failing shutdown are logged, and every
    // instance — including the healthy sibling — is still removed.
    manager.stop_all();
    assert!(!manager.is_running("doomed"));
    assert!(!manager.is_running("flaky"));
    assert!(!manager.is_running("healthy"));
    // The healthy sibling restarts and answers afterwards.
    manager.start("healthy").unwrap();
    let answer = manager.call("healthy", &serde_json::json!({"type": "poll"}));
    assert!(
        answer.is_ok(),
        "healthy sibling should answer, got {answer:?}"
    );
    manager.stop_all();
    let _ = fs::remove_dir_all(root);
}
