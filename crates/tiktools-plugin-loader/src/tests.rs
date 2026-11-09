use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};

use super::*;

// These lifecycle mocks execute no native library. Keep native coverage on
// supported desktops, and use the other in-process kind on Android so the
// real ABI admission guard does not prevent the mocked worker from starting.
fn in_process_test_kind() -> PluginRuntimeKind {
    if cfg!(target_os = "android") {
        PluginRuntimeKind::NapiVm
    } else {
        PluginRuntimeKind::Native
    }
}

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
fn package_lease_fences_racing_starts_and_preserves_other_plugins() {
    let root = temp_root();
    write_plugin_with_runtime(&root, "first", "process");
    write_plugin_with_runtime(&root, "second", "process");
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(TestRuntime));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    manager.start("first").unwrap();
    let lease = manager.begin_package_mutation("first").unwrap();
    assert_eq!(
        manager.observe_worker("first").unwrap(),
        PluginWorkerObservation::Uncertain
    );
    assert!(lease.stop_if_present().unwrap());
    let other = Arc::clone(&manager);
    let raced = std::thread::spawn(move || other.start("first"))
        .join()
        .unwrap();
    assert!(matches!(
        raced,
        Err(PluginLoaderError::RuntimeUnavailable(_))
    ));
    manager.start("second").unwrap();
    lease.start().unwrap();
    assert!(manager.stop("first").is_err());
    assert!(manager.start("first").is_err());
    lease.finish().unwrap();
    manager.start("first").unwrap();
    assert!(manager.stop_all_checked());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn admitted_event_delivery_excludes_package_mutation_for_only_its_plugin() {
    let root = temp_root();
    write_plugin_with_runtime(&root, "first", "process");
    write_plugin_with_runtime(&root, "second", "process");
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(TestRuntime));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    let generation = manager.start_generation("first").unwrap();
    manager.start("second").unwrap();

    assert!(manager
        .try_reserve_event_delivery("first", generation + 1)
        .is_none());
    let first = manager
        .try_reserve_event_delivery("first", generation)
        .unwrap();
    let second = manager
        .try_reserve_event_delivery("first", generation)
        .unwrap();
    assert!(manager.begin_package_mutation("first").is_err());
    manager
        .begin_package_mutation("second")
        .unwrap()
        .finish()
        .unwrap();
    drop(first);
    assert!(manager.begin_package_mutation("first").is_err());
    drop(second);

    let lease = manager.begin_package_mutation("first").unwrap();
    assert!(manager
        .try_reserve_event_delivery("first", generation)
        .is_none());
    lease.finish().unwrap();
    assert!(manager
        .try_reserve_event_delivery("first", generation)
        .is_some());
    assert!(manager.stop_all_checked());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn concurrent_delivery_and_package_admission_have_only_one_winner() {
    let root = temp_root();
    write_plugin_with_runtime(&root, "racing", "process");
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(TestRuntime));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    let generation = manager.start_generation("racing").unwrap();
    for _ in 0..16 {
        let start = Arc::new(std::sync::Barrier::new(3));
        let admitted = Arc::new(std::sync::Barrier::new(3));
        let event = {
            let (manager, start, admitted) = (
                Arc::clone(&manager),
                Arc::clone(&start),
                Arc::clone(&admitted),
            );
            std::thread::spawn(move || {
                start.wait();
                let permit = manager.try_reserve_event_delivery("racing", generation);
                admitted.wait();
                permit.is_some()
            })
        };
        let package = {
            let (manager, start, admitted) = (
                Arc::clone(&manager),
                Arc::clone(&start),
                Arc::clone(&admitted),
            );
            std::thread::spawn(move || {
                start.wait();
                let lease = manager.begin_package_mutation("racing").ok();
                admitted.wait();
                let won = lease.is_some();
                if let Some(lease) = lease {
                    lease.finish().unwrap();
                }
                won
            })
        };
        start.wait();
        admitted.wait();
        assert_ne!(event.join().unwrap(), package.join().unwrap());
    }
    assert!(manager.stop_all_checked());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn interrupted_package_and_shutdown_stay_fail_closed() {
    let manager = Arc::new(PluginManager::new(Vec::new()));
    let lease = manager.begin_package_mutation("first").unwrap();
    assert!(!manager.stop_all_checked());
    assert!(lease.start().is_err());
    assert!(manager.begin_package_mutation("second").is_err());
    drop(lease);
    assert!(!manager.stop_all_checked());

    let manager = Arc::new(PluginManager::new(Vec::new()));
    drop(manager.begin_package_mutation("first").unwrap());
    assert!(manager.start("first").is_err());
    assert!(manager.begin_package_mutation("first").is_err());
    assert!(!manager.stop_all_checked());
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
fn cutover_inventory_rejects_partial_failed_and_ambiguous_scans() {
    let root = temp_root();
    let other = temp_root();
    fs::create_dir_all(root.join("demo")).unwrap();
    fs::write(root.join("demo/plugin.json"), r#"{"schemaVersion":3,"id":"demo","name":"Demo","version":"1.0.0","runtime":"declarative"}"#).unwrap();
    let manager = PluginManager::with_runtimes(
        plugin_roots(root.clone(), other.clone(), None),
        RuntimeRegistry::default(),
    );
    assert!(manager.list_complete_for_cutover().is_err());
    manager.scan().unwrap();
    assert!(manager.list_complete_for_cutover().is_err());
    fs::create_dir_all(&other).unwrap();
    manager.scan().unwrap();
    let (revision, plugins) = manager.list_complete_for_cutover().unwrap();
    assert_eq!(plugins.len(), 1);
    fs::create_dir_all(root.join("broken")).unwrap();
    manager.scan().unwrap();
    assert!(manager.list_complete_for_cutover().is_err());
    assert_eq!(manager.list().len(), 1);
    fs::remove_dir(root.join("broken")).unwrap();
    manager.scan().unwrap();
    assert!(manager.list_complete_for_cutover().unwrap().0 > revision);
    fs::create_dir_all(other.join("demo")).unwrap();
    fs::write(other.join("demo/plugin.json"), r#"{"schemaVersion":3,"id":"demo","name":"Duplicate","version":"1.0.0","runtime":"declarative"}"#).unwrap();
    manager.scan().unwrap();
    assert!(manager.list_complete_for_cutover().is_err());
    assert_eq!(manager.list().len(), 1);
    fs::remove_dir_all(&other).unwrap();
    manager.scan().unwrap();
    assert!(manager.list_complete_for_cutover().is_err());
    let restarted = PluginManager::with_runtimes(
        plugin_roots(root.clone(), other.clone(), None),
        RuntimeRegistry::default(),
    );
    restarted.scan().unwrap();
    assert!(restarted.list_complete_for_cutover().is_err());
    fs::create_dir_all(&other).unwrap();
    manager.scan().unwrap();
    assert!(manager.list_complete_for_cutover().is_ok());
    fs::remove_dir_all(&root).unwrap();
    fs::write(&root, b"not a directory").unwrap();
    assert!(manager.scan().is_err());
    assert!(manager.list_complete_for_cutover().is_err());
    assert_eq!(manager.list().len(), 1);
    fs::remove_file(root).unwrap();
    fs::remove_dir_all(other).unwrap();
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
fn quickjs_onload_events_are_published_only_after_successful_start() {
    let root = temp_root();
    for (id, fail) in [("startup-good", false), ("startup-bad", true)] {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("plugin.json"), format!(r#"{{"schemaVersion":3,"id":"{id}","name":"test","version":"1.0.0","runtime":"napi-vm","entry":"index.js","capabilities":["events.publish"],"eventTypes":[{{"type":"push.tick","title":{{"default":"Tick"}}}}],"apiVersion":1}}"#)).unwrap();
        let failure = if fail {
            "throw new Error('onLoad failure');"
        } else {
            ""
        };
        fs::write(dir.join("index.js"), format!(r#"import {{ emit }} from 'tiktools:events';
            export default {{ onLoad() {{ emit({{type:'push.tick',data:{{id:'{id}'}}}}); {failure} }},
            call() {{ return {{logs:[],intents:[],events:[]}}; }}, onUnload() {{}} }};"#)).unwrap();
    }
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    manager.scan().unwrap();
    let mut events = manager.subscribe_emitted_events();
    let mut called = 0;
    let generation = manager
        .start_generation_before_events("startup-good", || {
            assert!(
                events.try_recv().is_err(),
                "startup events must wait for host activation"
            );
            called += 1;
        })
        .unwrap();
    let event = events.try_recv().unwrap();
    assert_eq!(event.plugin_id, "startup-good");
    assert_eq!(event.generation, generation);
    assert_eq!(event.data, serde_json::json!({"id":"startup-good"}));
    assert!(manager.is_live_generation("startup-good", generation));
    let reused = manager
        .start_generation_before_events("startup-good", || called += 1)
        .unwrap();
    assert_eq!(reused, generation);
    assert_eq!(
        called, 2,
        "reusing a live worker still applies host activation"
    );
    assert!(manager
        .start_generation_before_events("startup-bad", || panic!("failed start must not activate"))
        .is_err());
    assert!(
        events.try_recv().is_err(),
        "failed startup must discard its buffer"
    );
    assert!(!manager.is_running("startup-bad"));
    manager.stop("startup-good").unwrap();
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn package_worker_defers_startup_events_and_calls_until_commit() {
    let root = temp_root();
    for id in ["committed", "cancelled"] {
        let dir = root.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("plugin.json"), format!(r#"{{"schemaVersion":3,"id":"{id}","name":"test","version":"1.0.0","runtime":"napi-vm","entry":"index.js","capabilities":["events.publish"],"eventTypes":[{{"type":"push.tick","title":{{"default":"Tick"}}}}],"apiVersion":1}}"#)).unwrap();
        fs::write(dir.join("index.js"), format!(r#"import {{ emit }} from 'tiktools:events'; export default {{ onLoad() {{ emit({{type:'push.tick',data:{{id:'{id}'}}}}); }}, call() {{ return {{ok:true}}; }}, onUnload() {{}} }};"#)).unwrap();
    }
    let manager = Arc::new(PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]));
    manager.scan().unwrap();
    let mut events = manager.subscribe_emitted_events();
    let lease = manager.begin_package_mutation("committed").unwrap();
    assert!(!lease.stop_if_present().unwrap());
    lease.start().unwrap();
    assert!(events.try_recv().is_err());
    assert!(manager.running_generation("committed").is_none());
    assert!(manager
        .call("committed", &serde_json::Value::Null)
        .await
        .is_err());
    lease.finish().unwrap();
    assert_eq!(events.try_recv().unwrap().plugin_id, "committed");
    assert!(manager
        .call("committed", &serde_json::Value::Null)
        .await
        .is_ok());
    let lease = manager.begin_package_mutation("cancelled").unwrap();
    assert!(!lease.stop_if_present().unwrap());
    lease.start().unwrap();
    drop(lease);
    assert!(events.try_recv().is_err());
    assert!(manager
        .call("cancelled", &serde_json::Value::Null)
        .await
        .is_err());
    assert!(manager.running_generation("cancelled").is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_cannot_finish_if_restarted_worker_already_exited() {
    struct Exiting;
    impl PluginRuntime for Exiting {
        fn kind(&self) -> PluginRuntimeKind {
            PluginRuntimeKind::Process
        }
        fn load(
            &self,
            _: &PluginManifest,
            _: &Path,
        ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
            unreachable!()
        }
        fn spawn_worker(
            &self,
            _: &PluginManifest,
            _: &Path,
            _: &WorkerContext,
        ) -> Option<Result<ManagedWorker, PluginLoaderError>> {
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            Some(Ok(ManagedWorker::new(tx, std::thread::spawn(|| Ok(())))))
        }
    }
    let root = temp_root();
    write_plugin_with_runtime(&root, "exited", "process");
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(Exiting));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    let lease = manager.begin_package_mutation("exited").unwrap();
    lease.start().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !manager.worker_finished_for_test("exited") {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(lease.finish().is_err());
    assert!(manager.begin_package_mutation("exited").is_err());
    assert!(!manager.stop_all_checked());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn aborted_package_requests_worker_shutdown_without_reopening_admission() {
    struct Runtime(Arc<std::sync::atomic::AtomicBool>);
    struct Instance(Arc<std::sync::atomic::AtomicBool>);
    impl PluginRuntime for Runtime {
        fn kind(&self) -> PluginRuntimeKind {
            PluginRuntimeKind::Process
        }
        fn load(
            &self,
            _: &PluginManifest,
            _: &Path,
        ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
            Ok(Box::new(Instance(Arc::clone(&self.0))))
        }
    }
    impl PluginInstance for Instance {
        fn id(&self) -> &str {
            "aborted"
        }
        fn handle_message(&mut self, _: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
            Ok(b"null".to_vec())
        }
        fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
            self.0.store(true, std::sync::atomic::Ordering::Release);
            Ok(())
        }
    }
    let root = temp_root();
    write_plugin_with_runtime(&root, "aborted", "process");
    let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(Runtime(Arc::clone(&stopped))));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    let lease = manager.begin_package_mutation("aborted").unwrap();
    lease.start().unwrap();
    drop(lease);
    let deadline = Instant::now() + Duration::from_secs(2);
    while !stopped.load(std::sync::atomic::Ordering::Acquire) {
        assert!(Instant::now() < deadline, "worker did not receive shutdown");
        std::thread::yield_now();
    }
    assert!(manager.start("aborted").is_err());
    assert!(manager.begin_package_mutation("aborted").is_err());
    assert!(!manager.stop_all_checked());
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn declarative_plugins_start_without_an_entry_and_reject_calls() {
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
        .await
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
        in_process_test_kind()
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

#[tokio::test]
async fn worker_discards_calls_that_expire_while_queued() {
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
    let first = tokio::spawn(async move {
        worker
            .call_with_deadline(
                "slow",
                &serde_json::json!({"type": "poll"}),
                Instant::now() + Duration::from_secs(5),
            )
            .await
    });
    // Let the first call reach the worker before queueing an expired one.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let expired = manager
        .call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_millis(25),
        )
        .await;
    assert!(
        matches!(expired, Err(PluginLoaderError::Timeout(_))),
        "expired queued call should time out, got {expired:?}"
    );
    assert!(first.await.unwrap().is_ok());
    assert_eq!(executions.load(Ordering::Acquire), 1);
    manager.stop_all();
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn timeout_keeps_a_healthy_instance_running() {
    // In-process workers run plugin code synchronously and never
    // self-terminate on a manager timeout, so the instance stays usable.
    // (Process workers kill their child on the same deadline instead;
    // see process_timeout_retires_the_instance_so_next_start_is_fresh.)
    let (manager, root) = scripted_manager_with_kind(
        &["slow"],
        in_process_test_kind(),
        Arc::new(|_| {
            std::thread::sleep(Duration::from_millis(100));
            Ok(b"null".to_vec())
        }),
    );
    manager.start("slow").unwrap();
    let expired = manager
        .call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_millis(10),
        )
        .await;
    assert!(matches!(expired, Err(PluginLoaderError::Timeout(_))));
    assert!(manager.is_running("slow"));
    let recovered = manager
        .call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_secs(5),
        )
        .await;
    assert!(recovered.is_ok());
    manager.stop_all();
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn process_timeout_retires_the_instance_so_next_start_is_fresh() {
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
    let warmed = manager
        .call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_secs(5),
        )
        .await;
    assert!(warmed.is_ok());
    let timed_out = manager
        .call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_millis(25),
        )
        .await;
    assert!(
        matches!(timed_out, Err(PluginLoaderError::Timeout(_))),
        "hung process call should time out, got {timed_out:?}"
    );
    // A timed-out process worker kills its child, so the instance is
    // dead: keeping it would fail the next call instantly with
    // "plugin process stdin is unavailable".
    assert!(!manager.is_running("slow"));
    manager.start("slow").unwrap();
    let recovered = manager
        .call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_secs(5),
        )
        .await;
    assert!(recovered.is_ok());
    manager.stop_all();
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn process_first_call_gets_cold_start_grace_once() {
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
    let warmed = manager
        .call_with_deadline(
            "cold",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_millis(100),
        )
        .await;
    assert!(
        warmed.is_ok(),
        "first process call should get cold-start grace, got {warmed:?}"
    );
    let steady = manager
        .call_with_deadline(
            "cold",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_millis(100),
        )
        .await;
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
        in_process_test_kind(),
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

#[tokio::test]
async fn claimed_grace_is_not_re_extended_by_the_worker() {
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
    let timed_out = manager
        .call_with_deadline(
            "slow",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_millis(100),
        )
        .await;
    assert!(
        matches!(timed_out, Err(PluginLoaderError::Timeout(_))),
        "consumed grace must not re-extend, got {timed_out:?}"
    );
    manager.stop_all();
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn transport_errors_retire_the_instance_and_restart_recovers() {
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
        .await
        .is_err());
    assert!(!manager.is_running("flaky"));
    manager.start("flaky").unwrap();
    assert!(manager
        .call_with_deadline(
            "flaky",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_secs(5),
        )
        .await
        .is_ok());
    manager.stop_all();
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn worker_panic_isolates_the_plugin_without_hanging_siblings() {
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
    let failed = manager
        .call_with_deadline(
            "doomed",
            &serde_json::json!({"type": "doomed"}),
            Instant::now() + Duration::from_secs(5),
        )
        .await;
    assert!(
        matches!(failed, Err(PluginLoaderError::Runtime(_))),
        "panicking worker should surface a runtime error, got {failed:?}"
    );
    assert!(!manager.is_running("doomed"));
    let sibling = manager
        .call_with_deadline(
            "healthy",
            &serde_json::json!({"type": "poll"}),
            Instant::now() + Duration::from_secs(5),
        )
        .await;
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

#[tokio::test]
async fn call_reports_poisoned_locks_as_a_typed_error() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    manager.start("demo").unwrap();
    manager.poison_locks_for_test();
    let called = manager
        .call_with_timeout(
            "demo",
            &serde_json::json!({"type": "poll"}),
            Duration::from_secs(5),
        )
        .await;
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
fn admission_catalog_rejects_poisoned_registry() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    assert_eq!(manager.list_complete_for_cutover().unwrap().1.len(), 1);
    manager.poison_locks_for_test();
    assert!(matches!(
        manager.list_complete_for_cutover(),
        Err(PluginLoaderError::LockPoisoned(_))
    ));
    assert_eq!(manager.list().len(), 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn guarded_cutover_catalog_keeps_registry_stable_until_callback_returns() {
    let (manager, root) = scripted_manager(&["demo"], Arc::new(|_| Ok(b"null".to_vec())));
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (finished_tx, finished_rx) = std::sync::mpsc::channel();
    let mut worker = None;
    manager
        .with_complete_cutover_catalog(|_, plugins| {
            assert_eq!(plugins.len(), 1);
            let other = manager.clone();
            worker = Some(std::thread::spawn(move || {
                started_tx.send(()).unwrap();
                let result = other.scan();
                finished_tx.send(result.is_ok()).unwrap();
            }));
            started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
            assert!(finished_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err());
        })
        .unwrap();
    assert!(finished_rx.recv_timeout(Duration::from_secs(3)).unwrap());
    worker.unwrap().join().unwrap();
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

#[tokio::test]
async fn one_broken_plugin_does_not_block_sibling_shutdown() {
    let root = temp_root();
    write_plugin_with_runtime(&root, "doomed", "process");
    write_plugin_with_runtime(&root, "flaky", &in_process_test_kind().to_string());
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
    let answer = manager
        .call("healthy", &serde_json::json!({"type": "poll"}))
        .await;
    assert!(
        answer.is_ok(),
        "healthy sibling should answer, got {answer:?}"
    );
    // The panicking worker and the failing shutdown are logged, and every
    // instance — including the healthy sibling — is still removed.
    assert!(
        !manager.stop_all_checked(),
        "failed plugin shutdown must block an application update"
    );
    assert!(!manager.is_running("doomed"));
    assert!(!manager.is_running("flaky"));
    assert!(!manager.is_running("healthy"));
    // Checked shutdown is terminal even when another worker failed.
    assert!(matches!(
        manager.start("healthy"),
        Err(PluginLoaderError::RuntimeUnavailable(_))
    ));
    manager.stop_all();
    let _ = fs::remove_dir_all(root);
}

fn copy_fixture_dir(source: &std::path::Path, target: &std::path::Path) {
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&destination).unwrap();
            copy_fixture_dir(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), &destination).unwrap();
        }
    }
}

#[tokio::test]
async fn napi_vm_plugin_runs_on_a_single_owner_thread() {
    // Real fixture through real discovery: the napi-vm runtime must serve
    // the manager queue on its own owner thread, with no generic worker
    // thread in front of it.
    let root = temp_root();
    let staged = root.join("napi-vm-echo");
    std::fs::create_dir_all(&staged).unwrap();
    copy_fixture_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/napi-vm-echo"),
        &staged,
    );
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    manager.scan().unwrap();
    manager.start("tiktools.napi-vm-echo").unwrap();
    assert_eq!(
        manager
            .worker_thread_name_for_test("tiktools.napi-vm-echo")
            .as_deref(),
        Some("tiktools-napi-vm-tiktools.napi-vm-echo"),
        "napi-vm plugin must run on its owner thread, not the generic worker"
    );
    let result = manager
        .call(
            "tiktools.napi-vm-echo",
            &serde_json::json!({"type": "poll"}),
        )
        .await
        .unwrap();
    assert!(result.get("events").is_some(), "{result}");
    manager.stop("tiktools.napi-vm-echo").unwrap();
    assert!(!manager.is_running("tiktools.napi-vm-echo"));
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn repeated_restart_rejects_stale_generations_without_retiring_the_new_worker() {
    let root = temp_root();
    fs::create_dir_all(root.join("demo")).unwrap();
    fs::write(root.join("demo/plugin.json"), r#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"index.bin"}"#).unwrap();
    fs::write(root.join("demo/index.bin"), b"test").unwrap();
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(TestRuntime));
    let manager =
        PluginManager::with_runtimes(plugin_roots(root.clone(), temp_root(), None), runtimes);
    manager.scan().unwrap();
    let mut previous = None;
    for _ in 0..128 {
        manager.start("demo").unwrap();
        let generation = manager.running_generation("demo").unwrap();
        if let Some(stale) = previous {
            assert!(matches!(
                manager
                    .call_with_generation(
                        "demo",
                        &serde_json::Value::Null,
                        Duration::from_secs(1),
                        stale
                    )
                    .await,
                Err(PluginLoaderError::RuntimeUnavailable(_))
            ));
            assert!(manager.is_running("demo"));
        }
        manager
            .call_with_generation(
                "demo",
                &serde_json::Value::Null,
                Duration::from_secs(1),
                generation,
            )
            .await
            .unwrap();
        manager.stop("demo").unwrap();
        assert_eq!(manager.outstanding_call_count(), 0);
        assert!(manager.running_generation("demo").is_none());
        previous = Some(generation);
    }
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn stopped_generation_reports_unavailable_without_changing_generic_not_found() {
    let handler: Handler = Arc::new(|_| Ok(b"null".to_vec()));
    let (manager, root) = scripted_manager(&["demo"], handler);
    let generation = manager.start_generation("demo").unwrap();
    manager.stop("demo").unwrap();
    assert!(matches!(
        manager
            .call_with_generation(
                "demo",
                &serde_json::Value::Null,
                Duration::from_secs(1),
                generation
            )
            .await,
        Err(PluginLoaderError::RuntimeUnavailable(_))
    ));
    assert!(matches!(
        manager
            .call_with_timeout("demo", &serde_json::Value::Null, Duration::from_secs(1))
            .await,
        Err(PluginLoaderError::NotFound(_))
    ));
    assert!(!manager.is_running("demo"));
    let _ = fs::remove_dir_all(root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generation_bound_call_discards_response_after_uncertain_stop() {
    let root = temp_root();
    write_plugin_with_runtime(&root, "demo", &in_process_test_kind().to_string());
    let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let entered_tx = Arc::new(std::sync::Mutex::new(Some(entered_tx)));
    let handler: Handler = {
        let gate = Arc::clone(&gate);
        Arc::new(move |_| {
            if let Some(entered) = entered_tx.lock().unwrap().take() {
                let _ = entered.send(());
                let (lock, changed) = &*gate;
                let mut released = lock.lock().unwrap();
                while !*released {
                    released = changed.wait(released).unwrap();
                }
            }
            Ok(br#"{"old":true}"#.to_vec())
        })
    };
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(ScriptedRuntime {
        kind: in_process_test_kind(),
        handler,
    }));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    manager.set_stop_deadline_for_test(Duration::from_millis(50));
    let generation = manager.start_generation("demo").unwrap();
    assert_eq!(manager.start_generation("demo").unwrap(), generation);
    let pending = {
        let manager = Arc::clone(&manager);
        tokio::spawn(async move {
            manager
                .call_with_generation(
                    "demo",
                    &serde_json::Value::Null,
                    Duration::from_secs(5),
                    generation,
                )
                .await
        })
    };
    entered_rx.await.unwrap();
    let stopped = {
        let manager = Arc::clone(&manager);
        tokio::task::spawn_blocking(move || manager.stop("demo"))
            .await
            .unwrap()
    };
    assert!(stopped.is_err(), "the blocked old worker must time out");
    assert!(matches!(
        manager.start_generation("demo"),
        Err(PluginLoaderError::StartupUncertain(_))
    ));
    {
        let (lock, changed) = &*gate;
        *lock.lock().unwrap() = true;
        changed.notify_all();
    }
    assert!(matches!(
        pending.await.unwrap(),
        Err(PluginLoaderError::RuntimeUnavailable(_))
    ));
    assert_eq!(manager.running_generation("demo"), None);
    assert!(matches!(
        manager.start_generation("demo"),
        Err(PluginLoaderError::StartupUncertain(_))
    ));
    let _ = fs::remove_dir_all(root);
}

// ---------------------------------------------------------------------------
// Phase 3: plugin worker lifecycle & isolation.
// ---------------------------------------------------------------------------

/// Gate a wedged worker's `shutdown()` waits on, so a test can prove the
/// stop deadline abandons it and release it afterwards (so the abandoned
/// thread exits instead of leaking into later tests).
struct ShutdownGate(std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>);

impl ShutdownGate {
    fn new() -> Self {
        Self(std::sync::Arc::new((
            std::sync::Mutex::new(false),
            std::sync::Condvar::new(),
        )))
    }

    fn release(&self) {
        let (lock, condvar) = &*self.0;
        *lock.lock().unwrap() = true;
        condvar.notify_all();
    }
}

#[test]
fn shutdown_check_fails_fast_while_plugin_load_is_in_progress() {
    struct BlockingRuntime {
        gate: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
        entered: std::sync::mpsc::SyncSender<()>,
    }
    impl PluginRuntime for BlockingRuntime {
        fn kind(&self) -> PluginRuntimeKind {
            in_process_test_kind()
        }
        fn load(
            &self,
            manifest: &PluginManifest,
            _: &Path,
        ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
            self.entered.send(()).unwrap();
            let (lock, changed) = &*self.gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = changed.wait(released).unwrap();
            }
            Ok(Box::new(TestInstance {
                id: manifest.id.clone(),
            }))
        }
    }
    let root = temp_root();
    write_plugin_with_runtime(&root, "loading", &in_process_test_kind().to_string());
    let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let (entered, ready) = std::sync::mpsc::sync_channel(0);
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(BlockingRuntime {
        gate: Arc::clone(&gate),
        entered,
    }));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    let starting = {
        let manager = Arc::clone(&manager);
        std::thread::spawn(move || manager.start("loading"))
    };
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let began = Instant::now();
    assert!(!manager.stop_all_checked());
    assert!(began.elapsed() < Duration::from_millis(250));
    {
        let (lock, changed) = &*gate;
        *lock.lock().unwrap() = true;
        changed.notify_all();
    }
    assert!(matches!(
        starting.join().unwrap(),
        Err(PluginLoaderError::RuntimeUnavailable(_))
    ));
    let deadline = Instant::now() + Duration::from_secs(2);
    while !manager.stop_all_checked() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(manager.stop_all_checked());
    assert!(matches!(
        manager.start("loading"),
        Err(PluginLoaderError::RuntimeUnavailable(_))
    ));
    let _ = fs::remove_dir_all(root);
}

struct WedgedShutdownRuntime {
    gate: ShutdownGate,
}

struct WedgedShutdownInstance {
    id: String,
    gate: ShutdownGate,
}

impl PluginRuntime for WedgedShutdownRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        in_process_test_kind()
    }

    fn load(
        &self,
        manifest: &PluginManifest,
        _directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        Ok(Box::new(WedgedShutdownInstance {
            id: manifest.id.clone(),
            gate: ShutdownGate(std::sync::Arc::clone(&self.gate.0)),
        }))
    }
}

impl PluginInstance for WedgedShutdownInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, _request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        Ok(b"null".to_vec())
    }

    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        let (lock, condvar) = &*self.gate.0;
        let mut released = lock.lock().unwrap();
        while !*released {
            released = condvar.wait(released).unwrap();
        }
        Ok(())
    }
}

#[test]
fn worker_observation_does_not_treat_an_inflight_stop_as_stopped() {
    let gate = ShutdownGate::new();
    let root = temp_root();
    write_plugin_with_runtime(&root, "wedged", &in_process_test_kind().to_string());
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(WedgedShutdownRuntime {
        gate: ShutdownGate(std::sync::Arc::clone(&gate.0)),
    }));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    assert_eq!(
        manager.observe_worker("wedged").unwrap(),
        PluginWorkerObservation::Stopped
    );
    manager.start("wedged").unwrap();
    assert_eq!(
        manager.observe_worker("wedged").unwrap(),
        PluginWorkerObservation::WorkerPresent
    );
    let worker = Arc::clone(&manager);
    let stopped = std::thread::spawn(move || worker.stop("wedged"));
    let deadline = Instant::now() + Duration::from_secs(2);
    while manager.is_running("wedged") && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    let removed = !manager.is_running("wedged");
    let observation = manager.observe_worker("wedged");
    assert!(
        !manager.stop_all_checked(),
        "in-flight teardown cannot authorize an update"
    );
    gate.release();
    let result = stopped.join().unwrap();
    assert!(removed);
    assert_eq!(observation.unwrap(), PluginWorkerObservation::Uncertain);
    result.unwrap();
    assert!(manager.stop_all_checked());
    assert!(matches!(
        manager.start("wedged"),
        Err(PluginLoaderError::RuntimeUnavailable(_))
    ));
    assert_eq!(
        manager.observe_worker("wedged").unwrap(),
        PluginWorkerObservation::Stopped
    );
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn stop_abandons_a_wedged_worker_after_the_deadline_instead_of_hanging() {
    let gate = ShutdownGate::new();
    let root = temp_root();
    write_plugin_with_runtime(&root, "wedged", &in_process_test_kind().to_string());
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(WedgedShutdownRuntime {
        gate: ShutdownGate(std::sync::Arc::clone(&gate.0)),
    }) as Arc<dyn PluginRuntime>);
    let manager = PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    );
    manager.scan().unwrap();
    manager.start("wedged").unwrap();
    manager.set_stop_deadline_for_test(Duration::from_millis(100));

    let started = Instant::now();
    let stopped = manager.stop("wedged");
    let elapsed = started.elapsed();
    assert!(
        matches!(stopped, Err(PluginLoaderError::Runtime(_))),
        "a wedged worker must fail the stop with a typed error, got {stopped:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "the stop must give up at the deadline, not join forever (took {elapsed:?})"
    );
    assert_eq!(
        manager.abandoned_worker_count(),
        1,
        "the abandoned thread must be counted, not silently detached"
    );
    assert_eq!(
        manager.observe_worker("wedged").unwrap(),
        PluginWorkerObservation::Uncertain
    );
    assert!(
        !manager.is_running("wedged"),
        "the instance must already be out of the map when the stop gives up"
    );
    // Release the wedged worker so the abandoned thread exits instead of
    // outliving the test.
    gate.release();
    let _ = fs::remove_dir_all(root);
}

#[test]
fn failed_package_stop_keeps_worker_and_admission_uncertain() {
    let gate = ShutdownGate::new();
    let root = temp_root();
    write_plugin_with_runtime(&root, "wedged", &in_process_test_kind().to_string());
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(WedgedShutdownRuntime {
        gate: ShutdownGate(Arc::clone(&gate.0)),
    }));
    let manager = Arc::new(PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    ));
    manager.scan().unwrap();
    manager.start("wedged").unwrap();
    manager.set_stop_deadline_for_test(Duration::from_millis(50));
    let lease = manager.begin_package_mutation("wedged").unwrap();
    assert!(lease.stop_if_present().is_err());
    assert_eq!(manager.uncertain_startup_worker_count_for_test("wedged"), 1);
    assert!(lease.finish().is_err());
    assert!(manager.start("wedged").is_err());
    assert!(manager.begin_package_mutation("wedged").is_err());
    assert!(!manager.stop_all_checked());
    gate.release();
    let _ = fs::remove_dir_all(root);
}

/// Runtime whose first owned worker thread exits immediately (a worker that
/// died before any caller observed it) and whose later starts fall back to
/// the generic worker, so the restarted generation can answer a call.
struct DeadWorkerThenGenericRuntime {
    spawns: std::sync::atomic::AtomicUsize,
}

impl PluginRuntime for DeadWorkerThenGenericRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        in_process_test_kind()
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

    fn spawn_worker(
        &self,
        _manifest: &PluginManifest,
        _directory: &Path,
        _ctx: &WorkerContext,
    ) -> Option<Result<ManagedWorker, PluginLoaderError>> {
        if self
            .spawns
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
            == 0
        {
            // The manager maps this instance as running, but the worker
            // thread is already finished — exactly the dead-but-mapped
            // generation `start` must detect and retire.
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let worker = std::thread::spawn(|| Ok::<(), PluginLoaderError>(()));
            Some(Ok(ManagedWorker::new(tx, worker)))
        } else {
            None
        }
    }
}

#[tokio::test]
async fn start_retires_a_dead_but_mapped_worker_and_spawns_a_fresh_generation() {
    let root = temp_root();
    write_plugin_with_runtime(&root, "demo", &in_process_test_kind().to_string());
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(DeadWorkerThenGenericRuntime {
        spawns: std::sync::atomic::AtomicUsize::new(0),
    }) as Arc<dyn PluginRuntime>);
    let manager = PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.clone(),
            source: PluginSource::Development,
        }],
        runtimes,
    );
    manager.scan().unwrap();

    manager.start("demo").unwrap();
    let dead_token = manager.running_generation("demo").unwrap();
    assert!(manager.is_running("demo"));
    // Wait out the dead worker thread deterministically.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !manager.worker_finished_for_test("demo") && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        manager.worker_finished_for_test("demo"),
        "the first generation's worker must have exited by now"
    );
    assert_eq!(
        manager.observe_worker("demo").unwrap(),
        PluginWorkerObservation::Uncertain
    );

    // Without the dead-generation check this returned Ok and left the map
    // pointing at a worker that can never answer again.
    manager.start("demo").unwrap();
    let fresh_token = manager.running_generation("demo").unwrap();
    assert_ne!(
        dead_token, fresh_token,
        "a dead generation must be replaced, not reported as already running"
    );
    let answer = manager
        .call("demo", &serde_json::json!({"type": "poll"}))
        .await;
    assert!(
        answer.is_ok(),
        "the fresh generation must answer, got {answer:?}"
    );
    assert!(
        manager.retired_worker_count() >= 1,
        "the dead generation's join handle must be retained for reaping, not dropped"
    );
    manager.stop_all();
    assert_eq!(
        manager.retired_worker_count(),
        0,
        "shutdown must reap every retained worker"
    );
    assert_eq!(
        manager.abandoned_worker_count(),
        0,
        "an already-finished worker must be joined, never abandoned"
    );
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn panic_retirement_keeps_join_ownership_until_the_next_reap() {
    let (manager, root) = scripted_manager(
        &["doomed"],
        Arc::new(|request| {
            if request.windows(6).any(|window| window == b"doomed") {
                panic!("native plugin bug");
            }
            Ok(b"null".to_vec())
        }),
    );
    manager.start("doomed").unwrap();
    let failed = manager
        .call_with_deadline(
            "doomed",
            &serde_json::json!({"type": "doomed"}),
            Instant::now() + Duration::from_secs(5),
        )
        .await;
    assert!(failed.is_err());
    assert!(!manager.is_running("doomed"));
    assert_eq!(
        manager.retired_worker_count(),
        1,
        "the hot path must retain the failed worker's join handle instead of dropping it"
    );
    // A package operation reaps finished retired workers without waiting for
    // a live one, and succeeds as soon as the failed worker is joined.
    let deadline = Instant::now() + Duration::from_secs(5);
    let lease = loop {
        if let Ok(lease) = manager.begin_package_mutation("doomed") {
            break lease;
        }
        assert!(
            Instant::now() < deadline,
            "finished worker was never reaped"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    };
    lease.finish().unwrap();
    assert_eq!(
        manager.retired_worker_count(),
        0,
        "the retained worker must be joined by an opportunistic reap"
    );
    assert_eq!(manager.abandoned_worker_count(), 0);
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn napi_vm_startup_handshake_is_bounded_by_the_manager_deadline() {
    let root = temp_root();
    let staged = root.join("napi-vm-hang");
    std::fs::create_dir_all(&staged).unwrap();
    copy_fixture_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/napi-vm-hang"),
        &staged,
    );
    let sibling = root.join("healthy-sibling");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join("plugin.json"), r#"{"schemaVersion":3,"id":"healthy-sibling","name":"Healthy","version":"1.0.0","runtime":"declarative"}"#).unwrap();
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    manager.scan().unwrap();
    manager.set_start_deadline_for_test(Duration::from_millis(200));

    let started = Instant::now();
    let result = manager.start("tiktools.napi-vm-hang");
    let elapsed = started.elapsed();
    match result {
        Err(PluginLoaderError::StartupUncertain(message)) => assert!(
            message.contains("did not finish loading within"),
            "the failure must name the missed startup deadline, got {message}"
        ),
        other => {
            panic!("a guest wedged in onLoad must fail the start at the deadline, got {other:?}")
        }
    }
    assert!(
        elapsed < Duration::from_secs(2),
        "the start must give up at its deadline (took {elapsed:?})"
    );
    assert!(!manager.is_running("tiktools.napi-vm-hang"));
    assert_eq!(
        manager.uncertain_startup_worker_count_for_test("tiktools.napi-vm-hang"),
        1
    );
    assert_eq!(
        manager.observe_worker("tiktools.napi-vm-hang").unwrap(),
        PluginWorkerObservation::Uncertain
    );
    assert!(matches!(
        manager.ensure_startup_known("tiktools.napi-vm-hang"),
        Err(PluginLoaderError::StartupUncertain(_))
    ));
    assert!(matches!(
        manager.start("tiktools.napi-vm-hang"),
        Err(PluginLoaderError::StartupUncertain(_))
    ));
    assert!(matches!(
        manager.stop_if_present("tiktools.napi-vm-hang"),
        Err(PluginLoaderError::StartupUncertain(_))
    ));
    // One unresolved plugin does not prevent an unrelated healthy startup.
    manager.start("healthy-sibling").unwrap();
    assert!(manager.stop_if_present("healthy-sibling").unwrap());
    assert!(!manager.stop_all_checked());
    // Once the bounded fixture actually exits, reaping releases the handle
    // but does not assert that its earlier external effects are known.
    let deadline = Instant::now() + Duration::from_secs(4);
    while manager.uncertain_startup_worker_count_for_test("tiktools.napi-vm-hang") != 0
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(20));
        assert!(!manager.stop_all_checked());
    }
    assert_eq!(
        manager.uncertain_startup_worker_count_for_test("tiktools.napi-vm-hang"),
        0
    );
    assert!(matches!(
        manager.ensure_startup_known("tiktools.napi-vm-hang"),
        Err(PluginLoaderError::StartupUncertain(_))
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn repo_example_is_discovered_loaded_and_unloaded() {
    let root = temp_root();
    fs::create_dir_all(root.join("repo.game")).unwrap();
    fs::write(
        root.join("repo.game/plugin.json"),
        include_str!("../../../tests/fixtures/plugins/repo-game.plugin.json"),
    )
    .unwrap();
    let manager = PluginManager::with_runtimes(
        plugin_roots(temp_root(), temp_root(), Some(root.clone())),
        RuntimeRegistry::new(),
    );
    let plugins = manager.scan().unwrap();
    assert_eq!(plugins.len(), 1);
    assert!(plugins[0].available);
    manager.start("repo.game").unwrap();
    assert!(manager.get("repo.game").unwrap().running);
    manager.stop("repo.game").unwrap();
    assert!(!manager.get("repo.game").unwrap().running);
    let _ = fs::remove_dir_all(root);
}
