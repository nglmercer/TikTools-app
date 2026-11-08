use super::*;
use crate::{PluginInstance, PluginManager, PluginRoot, PluginSource, RuntimeRegistry};
use serde_json::json;
use std::{
    path::Path,
    sync::{atomic::AtomicUsize, Mutex},
};
use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};

struct RecordingRuntime {
    loads: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<Value>>>,
    hold: Arc<AtomicBool>,
}
struct RecordingInstance {
    requests: Arc<Mutex<Vec<Value>>>,
    hold: Arc<AtomicBool>,
}
impl PluginRuntime for RecordingRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Process
    }
    fn load(
        &self,
        _: &PluginManifest,
        _: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        panic!("isolated process must receive a private data directory")
    }
    fn load_isolated(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
        data: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        assert_eq!(data, directory.join(".data"));
        let settings: Value = serde_json::from_slice(
            &fs::read(data.join(&manifest.id).join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(settings, json!({"label":"visible"}));
        assert_eq!(
            fs::read(directory.join("entry.bin")).unwrap(),
            b"approved bytes"
        );
        Ok(Box::new(RecordingInstance {
            requests: Arc::clone(&self.requests),
            hold: Arc::clone(&self.hold),
        }))
    }
}
impl PluginInstance for RecordingInstance {
    fn id(&self) -> &str {
        "isolated"
    }
    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        self.requests
            .lock()
            .unwrap()
            .push(serde_json::from_slice(request).unwrap());
        while self.hold.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(br#"{"summary":"one effect","intents":[]}"#.to_vec())
    }
    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        Ok(())
    }
}

type IsolatedFixture = (
    PluginManager,
    PathBuf,
    Arc<AtomicUsize>,
    Arc<Mutex<Vec<Value>>>,
    Arc<AtomicBool>,
);
fn fixture() -> IsolatedFixture {
    static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "tiktools-isolated-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::SeqCst)
    ));
    let package = root.join("packages/demo");
    fs::create_dir_all(package.join(".data")).unwrap();
    fs::write(package.join("entry.bin"), b"approved bytes").unwrap();
    fs::write(
        package.join(".data/settings.json"),
        br#"{"ownerSecret":"must not be copied"}"#,
    )
    .unwrap();
    fs::write(package.join("plugin.json"), json!({"schemaVersion":3,"id":"isolated","name":"Isolated","version":"1.0.0","runtime":"process","entry":"entry.bin"}).to_string()).unwrap();
    let loads = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let hold = Arc::new(AtomicBool::new(false));
    let mut runtimes = RuntimeRegistry::default();
    runtimes.register(Arc::new(RecordingRuntime {
        loads: Arc::clone(&loads),
        requests: Arc::clone(&requests),
        hold: Arc::clone(&hold),
    }));
    let manager = PluginManager::with_runtimes(
        vec![PluginRoot {
            path: root.join("packages"),
            source: PluginSource::Development,
        }],
        runtimes,
    );
    manager.scan().unwrap();
    (manager, root, loads, requests, hold)
}

#[tokio::test]
async fn preparation_is_inert_and_dispatch_uses_frozen_bytes_private_settings_and_one_instance() {
    let (manager, root, loads, requests, _) = fixture();
    let digest = crate::plugin_artifact_fingerprint(&manager.get("isolated").unwrap()).unwrap();
    let snapshot = root.join("one-invocation");
    let prepared = manager
        .prepare_isolated_call(
            "isolated",
            &digest,
            snapshot.clone(),
            json!({"label":"visible"}),
        )
        .unwrap();
    assert_eq!(loads.load(Ordering::SeqCst), 0);
    assert!(!manager.is_running("isolated"));
    fs::write(
        root.join("packages/demo/entry.bin"),
        b"later Owner replacement",
    )
    .unwrap();
    let result = prepared.dispatch(json!({"type":"action","action":{"typeId":"isolated.send","config":{"text":"public"}},"event":{"type":"tiktok.chat"},"settings":{"ownerSecret":"JSON cannot inject settings"}}), Duration::from_secs(1)).unwrap().wait().await.unwrap();
    assert_eq!(result["summary"], "one effect");
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert_eq!(
        requests.lock().unwrap()[0]["settings"],
        json!({"label":"visible"})
    );
    assert!(!manager.is_running("isolated"));
    assert!(!snapshot.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn changed_artifact_cannot_prepare_and_unadmitted_snapshot_drop_is_inert() {
    let (manager, root, loads, _, _) = fixture();
    let digest = crate::plugin_artifact_fingerprint(&manager.get("isolated").unwrap()).unwrap();
    let snapshot = root.join("unadmitted");
    let prepared = manager
        .prepare_isolated_call(
            "isolated",
            &digest,
            snapshot.clone(),
            json!({"label":"visible"}),
        )
        .unwrap();
    drop(prepared);
    assert!(!snapshot.exists());
    assert_eq!(loads.load(Ordering::SeqCst), 0);
    fs::write(root.join("packages/demo/entry.bin"), b"changed").unwrap();
    assert!(manager
        .prepare_isolated_call("isolated", &digest, root.join("changed"), json!({}))
        .is_err());
    assert_eq!(loads.load(Ordering::SeqCst), 0);
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn timeout_is_unknown_and_cannot_automatically_reinvoke() {
    let (manager, root, loads, requests, hold) = fixture();
    hold.store(true, Ordering::Release);
    let digest = crate::plugin_artifact_fingerprint(&manager.get("isolated").unwrap()).unwrap();
    let prepared = manager
        .prepare_isolated_call(
            "isolated",
            &digest,
            root.join("timeout"),
            json!({"label":"visible"}),
        )
        .unwrap();
    let call = prepared
        .dispatch(json!({"type":"action"}), Duration::from_secs(1))
        .unwrap();
    for _ in 0..250 {
        if !requests.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let error = call.wait().await.unwrap_err();
    assert!(error.to_string().contains("unknown"));
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert_eq!(requests.lock().unwrap().len(), 1);
    hold.store(false, Ordering::Release);
    // Wait for the admitted thread to finish cleanup; this never dispatches.
    for _ in 0..100 {
        if !root.join("timeout").exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn isolated_vm_cannot_publish_to_owner_bus_or_reuse_owner_guest_state() {
    let root = std::env::temp_dir().join(format!(
        "tiktools-isolated-vm-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let package = root.join("packages/demo");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("index.js"),
        r#"
import { emit } from 'tiktools:events';
let calls = 0;
export default { call() {
  calls += 1;
  let ownerBus = 'closed';
  try { emit({type:'isolated.tick',data:{calls}}); ownerBus = 'open'; } catch (_) {}
  return {summary:ownerBus,calls};
} };
"#,
    )
    .unwrap();
    fs::write(
        package.join("plugin.json"),
        json!({
            "schemaVersion":3,"id":"isolated.vm","name":"VM","version":"1.0.0",
            "runtime":"napi-vm","entry":"index.js","capabilities":["events.publish"],
            "eventTypes":[{"type":"isolated.tick","title":{"default":"Tick"}}]
        })
        .to_string(),
    )
    .unwrap();
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.join("packages"),
        source: PluginSource::Development,
    }]);
    manager.scan().unwrap();
    let mut owner_bus = manager.subscribe_emitted_events();
    let digest = crate::plugin_artifact_fingerprint(&manager.get("isolated.vm").unwrap()).unwrap();
    for index in 0..2 {
        let prepared = manager
            .prepare_isolated_call(
                "isolated.vm",
                &digest,
                root.join(format!("invocation-{index}")),
                json!({}),
            )
            .unwrap();
        let response = prepared
            .dispatch(json!({"type":"action"}), Duration::from_secs(5))
            .unwrap()
            .wait()
            .await
            .unwrap();
        assert_eq!(response, json!({"summary":"closed","calls":1}));
        assert!(owner_bus.try_recv().is_err());
        assert!(!manager.is_running("isolated.vm"));
    }
    let _ = fs::remove_dir_all(root);
}
