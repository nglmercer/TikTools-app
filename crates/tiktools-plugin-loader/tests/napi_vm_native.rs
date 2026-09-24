//! Native addon coverage for the napi-vm runtime: a bundled napi-rs
//! package (built from `tests/fixtures/native-tsfn`) is staged into a copy
//! of the `napi-vm-native` guest fixture, declared in `nativeAddons` with
//! computed SHA-256 digests, and driven through load, sync calls, idle
//! TSFN delivery, shutdown, and reload. Negative tests pin the
//! authorization boundary: undeclared files, wrong digests, and missing
//! host artifacts all fail closed.
//!
//! The guest under test keeps its natural shape:
//!
//! ```ts
//! import { startListener } from 'rdev-node';
//! ```

#![cfg(feature = "napi-vm-node-api")]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tiktools_plugin_api::{
    manifest::{current_target, host_native_artifact_keys},
    PluginManifest,
};
use tiktools_plugin_loader::{
    NapiVmPluginRuntime, PluginManager, PluginRoot, PluginRuntime, PluginSource,
};

static BUILD_ONCE: OnceLock<PathBuf> = OnceLock::new();
static STAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Platform keys the fixture loader cascade probes, in probe order. Keep in
/// sync with `node_modules/rdev-node/index.js` in the guest fixture.
const CASCADE_KEYS: [&str; 8] = [
    "win32-x64-msvc",
    "win32-arm64-msvc",
    "linux-x64-gnu",
    "linux-x64-musl",
    "linux-arm64-gnu",
    "linux-arm64-musl",
    "darwin-x64",
    "darwin-arm64",
];

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/napi-vm-native")
}

fn fixture_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native-tsfn")
}

/// Scratch space under the workspace target directory: `/tmp` may be a
/// small or full tmpfs on CI workers, while the target directory is known
/// to have room for build outputs.
fn scratch_root(label: &str) -> PathBuf {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let id = STAGE_COUNTER.fetch_add(1, Ordering::AcqRel);
    target_dir.join(format!(
        "test-scratch/napi-vm-native-{label}-{}-{id}",
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

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Builds the napi-rs fixture cdylib once per test binary and returns its
/// path. The build is offline and pinned by the fixture's own lockfile.
fn native_library() -> &'static Path {
    BUILD_ONCE.get_or_init(|| {
        let target_dir = scratch_root("fixture-target");
        let output = std::process::Command::new("cargo")
            .args(["build", "--offline", "--release", "--manifest-path"])
            .arg(fixture_crate_dir().join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target_dir)
            .output()
            .expect("cargo is required to build the napi-rs fixture");
        assert!(
            output.status.success(),
            "napi-rs fixture build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let library = if cfg!(target_os = "windows") {
            "tiktools_native_tsfn_fixture.dll"
        } else if cfg!(target_os = "macos") {
            "libtiktools_native_tsfn_fixture.dylib"
        } else {
            "libtiktools_native_tsfn_fixture.so"
        };
        let built = target_dir.join("release").join(library);
        assert!(
            built.is_file(),
            "fixture cdylib was not built: {}",
            built.display()
        );
        built
    })
}

fn placeholder_bytes(platform: &str) -> Vec<u8> {
    format!("placeholder native binary for {platform}; never authorized on this host").into_bytes()
}

/// Writes one `.node` per cascade key: real fixture bytes for host keys,
/// placeholders for foreign keys (never authorized on this host), and
/// returns the matching `artifacts` map with computed digests.
fn write_node_binaries(package_dir: &Path) -> serde_json::Map<String, Value> {
    let host_keys = host_native_artifact_keys();
    let library_bytes = fs::read(native_library()).unwrap();
    let mut artifacts = serde_json::Map::new();
    for key in CASCADE_KEYS {
        let file = format!("node-rdev.{key}.node");
        let bytes = if host_keys.iter().any(|host| host == key) {
            // Same-host twins carry real bytes: napi-vm preflights every
            // authorized file against the host binary format at load.
            library_bytes.clone()
        } else {
            placeholder_bytes(key)
        };
        fs::write(package_dir.join(&file), &bytes).unwrap();
        artifacts.insert(
            key.to_owned(),
            json!({"path": file, "sha256": sha256_hex(&bytes)}),
        );
    }
    artifacts
}

/// Stages a native plugin at `staged`: copies the guest fixture, writes the
/// multi-platform `.node` tree, and injects a `nativeAddons` declaration
/// with computed digests. The `mutate` hook lets negative tests corrupt
/// the declaration afterwards.
fn stage_native_plugin_at(staged: &Path, mutate: impl FnOnce(&mut Value)) -> PluginManifest {
    fs::create_dir_all(staged).unwrap();
    copy_dir(&fixture_dir(), staged);
    let artifacts = write_node_binaries(&staged.join("node_modules/rdev-node"));

    let manifest_path = staged.join("plugin.json");
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["nativeAddons"] = json!([{
        "package": "rdev-node",
        "root": "node_modules/rdev-node",
        "artifacts": artifacts,
    }]);
    mutate(&mut manifest);
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    PluginManifest::from_json_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap()
}

fn stage_native_plugin(label: &str, mutate: impl FnOnce(&mut Value)) -> (PathBuf, PluginManifest) {
    let staged = scratch_root(label);
    let manifest = stage_native_plugin_at(&staged, mutate);
    (staged, manifest)
}

fn call_json(instance: &mut dyn tiktools_plugin_loader::PluginInstance, request: &Value) -> Value {
    let response = instance
        .handle_message(&serde_json::to_vec(request).unwrap())
        .unwrap();
    serde_json::from_slice(&response).unwrap()
}

fn action_request(type_id: &str) -> Value {
    json!({
        "type": "action",
        "action": { "typeId": type_id, "config": {} },
        "event": {},
    })
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[test]
fn native_addon_loads_and_answers_sync_call() {
    let (staged, manifest) = stage_native_plugin("load", |_| {});
    assert_eq!(manifest.native_addons.len(), 1);

    let mut instance = NapiVmPluginRuntime.load(&manifest, &staged).unwrap();
    let result = call_json(instance.as_mut(), &action_request("native.sum"));
    assert_eq!(result.get("summary"), Some(&json!("sum:42")), "{result}");
    instance.shutdown().unwrap();

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn undeclared_dot_node_require_fails_closed() {
    let (staged, manifest) = stage_native_plugin("undeclared", |_| {});
    // Present on disk but never declared: the guest probe must fail.
    fs::write(staged.join("dist/evil.node"), b"not an authorized addon").unwrap();

    let mut instance = NapiVmPluginRuntime.load(&manifest, &staged).unwrap();
    let result = call_json(instance.as_mut(), &action_request("native.evil"));
    let summary = result
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        summary.contains("not allowlisted"),
        "undeclared .node must be refused: {summary}"
    );
    instance.shutdown().unwrap();

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn native_load_without_declaration_fails_closed() {
    let staged = scratch_root("no-declaration");
    fs::create_dir_all(&staged).unwrap();
    copy_dir(&fixture_dir(), &staged);
    // Ship every binary but declare nothing: no allowlist, no loading.
    write_node_binaries(&staged.join("node_modules/rdev-node"));
    let manifest =
        PluginManifest::from_json_str(&fs::read_to_string(staged.join("plugin.json")).unwrap())
            .unwrap();
    assert!(manifest.native_addons.is_empty());

    let error = match NapiVmPluginRuntime.load(&manifest, &staged) {
        Ok(_) => panic!("load without a nativeAddons declaration must fail"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("not configured") || error.contains("allowlist"),
        "{error}"
    );

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn wrong_digest_fails_load() {
    let (staged, manifest) = stage_native_plugin("wrong-hash", |manifest| {
        let host_key = host_native_artifact_keys().remove(0);
        let artifact = &mut manifest["nativeAddons"][0]["artifacts"][host_key];
        let mut digest = artifact["sha256"].as_str().unwrap().to_owned();
        let last = digest.pop().unwrap();
        digest.push(if last == '0' { '1' } else { '0' });
        artifact["sha256"] = Value::String(digest);
    });

    let error = match NapiVmPluginRuntime.load(&manifest, &staged) {
        Ok(_) => panic!("load with a wrong digest must fail"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("integrity"), "{error}");

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn missing_host_artifact_fails_load_with_clear_error() {
    let (staged, manifest) = stage_native_plugin("no-host-artifact", |manifest| {
        let artifacts = manifest["nativeAddons"][0]["artifacts"]
            .as_object_mut()
            .unwrap();
        artifacts.retain(|key, _| key.starts_with("fuchsia-"));
        artifacts.insert(
            "fuchsia-arm64".to_owned(),
            json!({
                "path": "node-rdev.fuchsia-arm64.node",
                "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            }),
        );
    });

    let error = match NapiVmPluginRuntime.load(&manifest, &staged) {
        Ok(_) => panic!("load with no host artifact must fail"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("has no native artifact for") && error.contains(&current_target()),
        "{error}"
    );

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn missing_declared_file_fails_load() {
    let (staged, manifest) = stage_native_plugin("missing-file", |_| {});
    let host_key = host_native_artifact_keys().remove(0);
    fs::remove_file(staged.join(format!("node_modules/rdev-node/node-rdev.{host_key}.node")))
        .unwrap();

    let error = match NapiVmPluginRuntime.load(&manifest, &staged) {
        Ok(_) => panic!("load with a missing artifact file must fail"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("missing"), "{error}");

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn tsfn_callback_is_delivered_while_vm_is_idle() {
    let (staged, manifest) = stage_native_plugin("tsfn-idle", |_| {});
    let mut instance = NapiVmPluginRuntime.load(&manifest, &staged).unwrap();

    // No events before the listener starts.
    let result = call_json(instance.as_mut(), &json!({"type": "poll"}));
    assert_eq!(result.get("events"), Some(&json!([])), "{result}");

    let result = call_json(instance.as_mut(), &action_request("native.start"));
    assert_eq!(
        result.get("summary"),
        Some(&json!("started:true")),
        "{result}"
    );

    // The VM thread is idle from here: no plugin request arrives while the
    // native worker fires its threadsafe-function event (~50ms) and the
    // owner thread pumps it into the guest.
    std::thread::sleep(Duration::from_millis(800));
    let polled_at = now_ms();
    let result = call_json(instance.as_mut(), &json!({"type": "poll"}));
    let events = result
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(events.len(), 1, "{result}");
    assert_eq!(events[0].get("type"), Some(&json!("native.key")));
    assert_eq!(
        events[0].pointer("/data/event"),
        Some(&json!("key:space")),
        "{result}"
    );
    // The guest callback ran while idle, long before this poll started. If
    // delivery only happened inside a call evaluation, the timestamps would
    // coincide instead of lagging by most of the idle sleep.
    let delivered_at = events[0]
        .pointer("/data/at")
        .and_then(Value::as_u64)
        .unwrap() as u128;
    assert!(
        delivered_at + 200 < polled_at,
        "callback was not delivered while idle: delivered_at={delivered_at} polled_at={polled_at}"
    );

    let result = call_json(instance.as_mut(), &action_request("native.stop"));
    assert_eq!(
        result.get("summary"),
        Some(&json!("stopped:true")),
        "{result}"
    );
    instance.shutdown().unwrap();

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn clean_shutdown_and_reload_with_active_listener() {
    // A dedicated discovery root: parallel tests stage siblings with the
    // same plugin id, so scanning the shared scratch directory would be
    // nondeterministic.
    let root = scratch_root("reload-root");
    let staged = root.join("napi-vm-native");
    let manifest = stage_native_plugin_at(&staged, |_| {});
    assert_eq!(manifest.id, "tiktools.napi-vm-native");
    let manager = PluginManager::new(vec![PluginRoot {
        path: root.clone(),
        source: PluginSource::Development,
    }]);
    let discovered = manager.scan().unwrap();
    assert!(
        discovered
            .iter()
            .any(|plugin| plugin.manifest.id == manifest.id && plugin.available),
        "{discovered:?}"
    );

    manager.start(&manifest.id).unwrap();
    let result = manager
        .call(&manifest.id, &action_request("native.start"))
        .unwrap();
    assert_eq!(
        result.get("summary"),
        Some(&json!("started:true")),
        "{result}"
    );
    // Stop while the native worker is still in flight: guest `onUnload`
    // stops the listener through its own API, then the host disposes the
    // napi-vm plugin, shuts down the native runtime, and joins the thread.
    manager.stop(&manifest.id).unwrap();
    assert!(!manager.is_running(&manifest.id));

    // Reload is a fresh generation: the addon initializes again and sync
    // native calls answer.
    manager.start(&manifest.id).unwrap();
    let result = manager
        .call(&manifest.id, &action_request("native.sum"))
        .unwrap();
    assert_eq!(result.get("summary"), Some(&json!("sum:42")), "{result}");
    manager.stop(&manifest.id).unwrap();

    fs::remove_dir_all(&root).ok();
}

#[cfg(feature = "plugin-install")]
#[test]
fn installer_retains_bundled_native_tree() {
    use std::io::Write;
    use tiktools_plugin_loader::PluginInstaller;

    let scratch = scratch_root("installer");
    fs::create_dir_all(&scratch).unwrap();
    let manifest = json!({
        "schemaVersion": 3,
        "id": "native-installed",
        "name": "NativeInstalled",
        "version": "1.0.0",
        "runtime": "napi-vm",
        "entry": "dist/index.js",
        "apiVersion": 1,
        "nativeAddons": [{
            "package": "rdev-node",
            "root": "node_modules/rdev-node",
            "artifacts": {
                "linux-x64-gnu": {
                    "path": "node-rdev.linux-x64-gnu.node",
                    "sha256": sha256_hex(b"fake-linux"),
                },
                "win32-x64-msvc": {
                    "path": "node-rdev.win32-x64-msvc.node",
                    "sha256": sha256_hex(b"fake-win32"),
                },
            },
        }],
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    let entry_bytes =
        b"export default { call() { return { logs: [], intents: [], events: [] }; } };";
    let loader_bytes = b"module.exports = {};";
    let files: &[(&str, &[u8])] = &[
        ("plugin.json", &manifest_bytes),
        ("dist/index.js", entry_bytes),
        ("node_modules/rdev-node/index.js", loader_bytes),
        (
            "node_modules/rdev-node/node-rdev.linux-x64-gnu.node",
            b"fake-linux",
        ),
        (
            "node_modules/rdev-node/node-rdev.win32-x64-msvc.node",
            b"fake-win32",
        ),
    ];
    let checksums: serde_json::Map<String, Value> = files
        .iter()
        .map(|(name, bytes)| (name.to_string(), Value::String(sha256_hex(bytes))))
        .collect();
    let checksums_bytes = serde_json::to_vec_pretty(&checksums).unwrap();

    let archive_path = scratch.join("native-installed.plugin");
    {
        let file = fs::File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, bytes) in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.start_file("checksums.json", options).unwrap();
        writer.write_all(&checksums_bytes).unwrap();
        writer.finish().unwrap();
    }

    let installed = PluginInstaller {
        plugin_directory: scratch.join("plugins"),
        staging_directory: scratch.join("staging"),
        replace_existing: false,
    }
    .install(&archive_path)
    .unwrap();
    assert_eq!(installed.manifest.id, "native-installed");
    assert_eq!(installed.manifest.native_addons.len(), 1);
    for name in [
        "node_modules/rdev-node/index.js",
        "node_modules/rdev-node/node-rdev.linux-x64-gnu.node",
        "node_modules/rdev-node/node-rdev.win32-x64-msvc.node",
    ] {
        assert!(
            installed.directory.join(name).is_file(),
            "archive dropped {name}"
        );
    }
    assert_eq!(
        fs::read(
            installed
                .directory
                .join("node_modules/rdev-node/node-rdev.linux-x64-gnu.node")
        )
        .unwrap(),
        b"fake-linux",
    );

    fs::remove_dir_all(&scratch).ok();
}
