//! Device-native plugin probes, independent of Bun and desktop native addons.
#![cfg(target_os = "android")]
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use tiktools_plugin_api::PluginManifest;
use tiktools_plugin_loader::{
    NapiVmPluginRuntime, NativePluginRuntime, PluginRuntime, ProcessPluginRuntime,
};

fn directory(label: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("tt-plugin-{}-{unique}-{label}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}
fn manifest(runtime: &str, entry: &str) -> PluginManifest {
    PluginManifest::from_json_str(
        &json!({
            "schemaVersion": 3, "apiVersion": 1, "id": "portable", "name": "Portable",
            "version": "1.0.0", "runtime": runtime, "entry": entry,
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn portable_vm_executes_without_node_or_bun() {
    let root = directory("vm");
    let manifest = manifest("napi-vm", "index.js");
    fs::write(
        root.join("plugin.json"),
        json!({
            "schemaVersion": 3, "apiVersion": 1, "id": "portable", "name": "Portable",
            "version": "1.0.0", "runtime": "napi-vm", "entry": "index.js",
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        root.join("index.js"),
        r#"export default {
        async call(request, context) {
            return {summary: "Android 😀", logs: [], intents: [], events: []};
        }
    };"#,
    )
    .unwrap();
    let mut instance = NapiVmPluginRuntime.load(&manifest, &root).unwrap();
    let response = instance.handle_message(br#"{"type":"poll"}"#).unwrap();
    let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
    assert_eq!(response["summary"], "Android 😀");
    instance.shutdown().unwrap();
    drop(instance);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn process_plugin_uses_native_interpreter_and_existing_frames() {
    let root = directory("process");
    let python =
        PathBuf::from(std::env::var_os("PREFIX").expect("native Termux PREFIX")).join("bin/python");
    assert!(python.is_file(), "pkg install python for process fixture");
    let entry = root.join("plugin");
    fs::write(&entry, format!("#!{}\n{}", python.display(), r#"
import json, struct, sys, os
while True:
    header = sys.stdin.buffer.read(4)
    if not header: break
    request = json.loads(sys.stdin.buffer.read(struct.unpack('<I', header)[0]))
    result = {'summary': 'Android 😀', 'pluginId': os.environ['TIKTOOLS_PLUGIN_ID'], 'prefix': os.environ.get('PREFIX'), 'tmpdir': os.environ.get('TMPDIR'), 'cookieAbsent': 'TIKTOOLS_SESSION_COOKIE' not in os.environ}
    response = json.dumps({'protocolVersion': request['protocolVersion'], 'id': request['id'], 'ok': True, 'result': result}).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(response)) + response)
    sys.stdout.buffer.flush()
"#)).unwrap();
    fs::set_permissions(&entry, fs::Permissions::from_mode(0o700)).unwrap();
    let manifest = manifest("process", "plugin");
    let mut instance = ProcessPluginRuntime.load(&manifest, &root).unwrap();
    for _ in 0..2 {
        let bytes = instance.handle_message(br#"{"type":"poll"}"#).unwrap();
        let response: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(response["summary"], "Android 😀");
        assert_eq!(response["pluginId"], "portable");
        assert_eq!(response["prefix"], std::env::var("PREFIX").unwrap());
        if let Ok(tmpdir) = std::env::var("TMPDIR") {
            assert_eq!(response["tmpdir"], tmpdir);
        }
        assert_eq!(response["cookieAbsent"], true);
    }
    instance.shutdown().unwrap();
    drop(instance);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_library_loading_fails_before_opening_unvalidated_binary() {
    let root = directory("native");
    let error = NativePluginRuntime
        .load(&manifest("native", "desktop.so"), &root)
        .err()
        .expect("Android native backend unavailable");
    assert!(
        error.to_string().contains("validated native ABI"),
        "{error}"
    );
    fs::remove_dir_all(root).unwrap();
}
