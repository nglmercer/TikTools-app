//! End-to-end coverage for the napi-vm runtime: a TypeScript-authored,
//! tsc-compiled fixture plugin is discovered, started, called with real
//! `PluginCall` JSON, and stopped through the public loader API. The guest
//! answers with `PluginCallResult` JSON; no second protocol is involved.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};
use tiktools_plugin_loader::{
    NapiVmPluginRuntime, PluginManager, PluginRoot, PluginRuntime, PluginSource,
};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/napi-vm-echo")
}

fn temp_root(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tiktools-napi-vm-{label}-{}-{suffix}",
        std::process::id()
    ))
}

fn copy_dir(source: &Path, target: &Path) {
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            fs::create_dir_all(&destination).unwrap();
            copy_dir(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), &destination).unwrap();
        }
    }
}

/// Stages the fixture under a plugin root so the manager test exercises
/// real discovery instead of a hand-built manifest.
fn staged_root() -> PathBuf {
    let root = temp_root("root");
    let staged = root.join("napi-vm-echo");
    fs::create_dir_all(&staged).unwrap();
    copy_dir(&fixture_dir(), &staged);
    root
}

fn fixture_manifest() -> PluginManifest {
    let bytes = fs::read(fixture_dir().join("plugin.json")).unwrap();
    PluginManifest::from_json_str(std::str::from_utf8(&bytes).unwrap()).unwrap()
}

#[test]
fn napi_vm_runtime_reports_its_kind() {
    assert_eq!(NapiVmPluginRuntime.kind(), PluginRuntimeKind::NapiVm);
}

#[test]
fn fixture_manifest_parses_as_napi_vm() {
    let manifest = fixture_manifest();
    assert_eq!(manifest.id, "tiktools.napi-vm-echo");
    assert_eq!(manifest.runtime, PluginRuntimeKind::NapiVm);
    assert_eq!(manifest.entry, "dist/index.js");
    manifest.validate_compatibility().unwrap();
    assert!(manifest.target_matches_current_platform());
    // The napi-vm compatibility subset rides along in the same file without
    // disturbing the TikTools parse.
    let raw: Value =
        serde_json::from_str(&fs::read_to_string(fixture_dir().join("plugin.json")).unwrap())
            .unwrap();
    assert_eq!(raw.get("apiVersion"), Some(&json!(1)));
}

#[test]
fn guest_answers_action_and_poll_calls() {
    let manifest = fixture_manifest();
    let runtime = NapiVmPluginRuntime;
    let mut instance = runtime.load(&manifest, &fixture_dir()).unwrap();
    assert_eq!(instance.id(), "tiktools.napi-vm-echo");

    // Action call: the guest is async, so this also proves Promise results
    // are awaited by the host envelope.
    let request = json!({
        "type": "action",
        "action": { "typeId": "echo.ping", "config": {} },
        "event": {},
    });
    let response = instance
        .handle_message(serde_json::to_vec(&request).unwrap().as_slice())
        .unwrap();
    let result: Value = serde_json::from_slice(&response).unwrap();
    assert_eq!(
        result.get("summary"),
        Some(&json!("echo:echo.ping@tiktools.napi-vm-echo")),
        "{result}"
    );
    assert_eq!(
        result.get("logs"),
        Some(&json!(["saw action for tiktools.napi-vm-echo"])),
        "{result}"
    );

    // Poll call: the guest observes the host-supplied context argument.
    let response = instance.handle_message(br#"{"type":"poll"}"#).unwrap();
    let result: Value = serde_json::from_slice(&response).unwrap();
    assert_eq!(
        result.get("events"),
        Some(&json!([{ "type": "echo.tick", "data": { "plugin": "tiktools.napi-vm-echo" } }])),
        "{result}"
    );

    instance.shutdown().unwrap();
}

#[tokio::test]
async fn manager_discovers_starts_calls_and_stops_napi_vm_plugin() {
    let root = staged_root();
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    let discovered = manager.scan().unwrap();
    let plugin = discovered
        .iter()
        .find(|plugin| plugin.manifest.id == "tiktools.napi-vm-echo")
        .expect("fixture plugin is discovered");
    assert!(plugin.available, "{:?}", plugin.reason);
    assert_eq!(plugin.manifest.runtime, PluginRuntimeKind::NapiVm);

    manager.start("tiktools.napi-vm-echo").unwrap();
    assert!(manager.is_running("tiktools.napi-vm-echo"));

    let result = manager
        .call(
            "tiktools.napi-vm-echo",
            &json!({
                "type": "action",
                "action": { "typeId": "echo.ping", "config": {} },
                "event": {},
            }),
        )
        .await
        .unwrap();
    assert_eq!(
        result.get("summary"),
        Some(&json!("echo:echo.ping@tiktools.napi-vm-echo")),
        "{result}"
    );

    manager.stop("tiktools.napi-vm-echo").unwrap();
    assert!(!manager.is_running("tiktools.napi-vm-echo"));

    fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn guest_push_delivers_to_bus_without_polling() {
    let root = temp_root("push-root");
    let staged = root.join("napi-vm-push");
    fs::create_dir_all(&staged).unwrap();
    copy_dir(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/napi-vm-push"),
        &staged,
    );
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    manager.scan().unwrap();
    manager.start("tiktools.napi-vm-push").unwrap();
    let mut bus = manager.subscribe_emitted_events();

    // One action fires one emit; no poll happens anywhere in this test.
    let result = manager
        .call(
            "tiktools.napi-vm-push",
            &json!({
                "type": "action",
                "action": { "typeId": "push.fire", "config": {} },
                "event": {},
            }),
        )
        .await
        .unwrap();
    assert_eq!(result.get("summary"), Some(&json!("emitted")), "{result}");
    let emitted = tokio::time::timeout(std::time::Duration::from_secs(5), bus.recv())
        .await
        .expect("push must arrive without polling")
        .unwrap();
    assert_eq!(emitted.plugin_id, "tiktools.napi-vm-push");
    assert_eq!(emitted.event_type, "push.tick");
    assert_eq!(emitted.data, json!({"n": 1}));

    // Batch push returns the accepted count and delivers each event.
    let result = manager
        .call(
            "tiktools.napi-vm-push",
            &json!({
                "type": "action",
                "action": { "typeId": "push.fire-many", "config": {} },
                "event": {},
            }),
        )
        .await
        .unwrap();
    assert_eq!(result.get("summary"), Some(&json!("emitted:2")), "{result}");
    for _ in 0..2 {
        tokio::time::timeout(std::time::Duration::from_secs(5), bus.recv())
            .await
            .expect("batch push must arrive")
            .unwrap();
    }

    // Undeclared types throw to the guest: the action fails and the bus
    // stays silent.
    let error = manager
        .call(
            "tiktools.napi-vm-push",
            &json!({
                "type": "action",
                "action": { "typeId": "push.undeclared", "config": {} },
                "event": {},
            }),
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("undeclared"), "{error}");

    manager.stop("tiktools.napi-vm-push").unwrap();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn guest_without_call_export_fails_closed() {
    let root = temp_root("no-call");
    let staged = root.join("broken");
    fs::create_dir_all(staged.join("dist")).unwrap();
    fs::write(
        staged.join("plugin.json"),
        r#"{"schemaVersion":3,"id":"broken.napi-vm","name":"BrokenNapiVm","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js","capabilities":[],"apiVersion":1}"#,
    )
    .unwrap();
    fs::write(
        staged.join("dist/index.js"),
        "export default { onLoad() {} };\n",
    )
    .unwrap();

    let manifest =
        PluginManifest::from_json_str(&fs::read_to_string(staged.join("plugin.json")).unwrap())
            .unwrap();
    let mut instance = NapiVmPluginRuntime.load(&manifest, &staged).unwrap();
    let error = instance
        .handle_message(br#"{"type":"poll"}"#)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("must export call(request, context)"),
        "{error}"
    );
    instance.shutdown().unwrap();

    fs::remove_dir_all(&root).ok();
}

#[test]
fn missing_entry_fails_load() {
    let root = temp_root("missing-entry");
    let staged = root.join("missing");
    fs::create_dir_all(&staged).unwrap();
    fs::write(
        staged.join("plugin.json"),
        r#"{"schemaVersion":3,"id":"missing.napi-vm","name":"MissingNapiVm","version":"1.0.0","runtime":"napi-vm","entry":"dist/index.js","capabilities":[],"apiVersion":1}"#,
    )
    .unwrap();

    let manifest =
        PluginManifest::from_json_str(&fs::read_to_string(staged.join("plugin.json")).unwrap())
            .unwrap();
    let error = match NapiVmPluginRuntime.load(&manifest, &staged) {
        Ok(_) => panic!("load with a missing entry must fail"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("failed to load"), "{error}");

    fs::remove_dir_all(&root).ok();
}
