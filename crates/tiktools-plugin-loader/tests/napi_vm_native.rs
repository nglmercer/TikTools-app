//! Native addon coverage for the napi-vm runtime: a bundled napi-rs
//! package (built from `tests/fixtures/native-tsfn`) is staged into a copy
//! of the `napi-vm-native` guest fixture through the SDK staging library,
//! declared in `nativeAddons` as `{ package, root }`, and driven through
//! load, sync calls, idle TSFN delivery, shutdown, and reload. TikTools
//! selects the exact host `.node` from the package root and exposes it as
//! a native `require()` alias; guests load it through `node:module`
//! `createRequire` without executing any package loader. Negative tests
//! pin the authorization boundary: undeclared files, missing or ambiguous
//! host binaries, and untrusted manifests all fail closed.
//!
//! The guest under test keeps its natural shape:
//!
//! ```ts
//! import { createRequire } from 'node:module';
//!
//! const require = createRequire(import.meta.url);
//! const { startListener, stopListener } = require('rdev-node');
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
use tiktools_plugin_api::{manifest::current_napi_target, PluginManifest};
use tiktools_plugin_loader::{
    NapiVmPluginRuntime, PluginManager, PluginRoot, PluginRuntime, PluginSource,
};
use tiktools_plugin_sdk::native_stage::{stage_native_package, NativeStageRequest};

static BUILD_ONCE: OnceLock<PathBuf> = OnceLock::new();
static STAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Platform files staged into the fixture package. Only the exact host
/// file carries real bytes; the rest are placeholders selection must
/// ignore.
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
    format!("placeholder native binary for {platform}; never selected on this host").into_bytes()
}

/// Writes one `.node` per platform key into a source package directory.
/// Only the exact host key carries real fixture bytes; every other key —
/// foreign targets and the same-platform libc twin alike — carries
/// placeholders that could never initialize. A successful load therefore
/// proves exact selection: any wrong pick would fail its binary preflight
/// instead. A throwing `index.js` tripwire proves package loader code
/// never executes: guests load the addon purely through its native alias.
fn write_node_binaries(package_dir: &Path) {
    let exact_host_key = current_napi_target();
    assert!(
        CASCADE_KEYS.contains(&exact_host_key.as_str()),
        "test host {exact_host_key} is not covered by the staged platform files"
    );
    let library_bytes = fs::read(native_library()).unwrap();
    for key in CASCADE_KEYS {
        let file = format!("node-rdev.{key}.node");
        let bytes = if key == exact_host_key {
            library_bytes.clone()
        } else {
            placeholder_bytes(key)
        };
        fs::write(package_dir.join(&file), &bytes).unwrap();
    }
    fs::write(
        package_dir.join("index.js"),
        "throw new Error('package loader must not execute');",
    )
    .unwrap();
}

/// Assembles the `rdev-node` source package in scratch (fixture manifest
/// plus the multi-platform `.node` tree) and stages it into the plugin
/// through the SDK staging library, the same call a real plugin build
/// makes instead of `npm install`. Overwrites the file skeleton the
/// fixture copy ships.
fn stage_rdev_package(staged: &Path, label: &str) {
    let source = scratch_root(label);
    fs::create_dir_all(&source).unwrap();
    copy_dir(&fixture_dir().join("node_modules/rdev-node"), &source);
    write_node_binaries(&source);
    let report = stage_native_package(&NativeStageRequest {
        package: "rdev-node",
        source: &source,
        plugin_dir: staged,
        root: "node_modules/rdev-node",
        overwrite: true,
    })
    .unwrap();
    assert_eq!(
        report.node_binaries.len(),
        CASCADE_KEYS.len(),
        "staging must retain every platform binary: {report:?}"
    );
    fs::remove_dir_all(&source).ok();
}

/// Stages a native plugin at `staged`: copies the guest fixture, stages
/// the `rdev-node` package, and injects a `{ package, root }`
/// `nativeAddons` declaration. The `mutate` hook lets negative tests
/// corrupt the declaration afterwards.
fn stage_native_plugin_at(staged: &Path, mutate: impl FnOnce(&mut Value)) -> PluginManifest {
    fs::create_dir_all(staged).unwrap();
    copy_dir(&fixture_dir(), staged);
    stage_rdev_package(staged, "rdev-source");

    let manifest_path = staged.join("plugin.json");
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["nativeAddons"] = json!([{
        "package": "rdev-node",
        "root": "node_modules/rdev-node",
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
fn same_platform_twin_is_not_selected() {
    if !current_napi_target().ends_with("-gnu") && !current_napi_target().ends_with("-musl") {
        eprintln!("skipping twin test: test host has no libc twin to discriminate");
        return;
    }
    let (staged, manifest) = stage_native_plugin("exact-select", |_| {});
    // The libc twin of the exact host binary ships as placeholder bytes
    // that could never initialize: the load below succeeds only because
    // selection picks the exact host file and nothing else.
    let twin = if current_napi_target().ends_with("-gnu") {
        current_napi_target().replace("-gnu", "-musl")
    } else {
        current_napi_target().replace("-musl", "-gnu")
    };
    let twin_bytes =
        fs::read(staged.join(format!("node_modules/rdev-node/node-rdev.{twin}.node"))).unwrap();
    assert!(twin_bytes.starts_with(b"placeholder"), "{twin}");

    let mut instance = NapiVmPluginRuntime.load(&manifest, &staged).unwrap();
    let result = call_json(instance.as_mut(), &action_request("native.sum"));
    assert_eq!(result.get("summary"), Some(&json!("sum:42")), "{result}");
    instance.shutdown().unwrap();

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn musl_target_selects_musl_binary() {
    // GNU hosts prove their own side in every load test above; this builds
    // a musl binary and runs it, proving libc detection, the napi-rs host
    // spelling, and exact selection from the musl side. Only the target
    // toolchain gates the test: any other build failure is reported.
    if !cfg!(target_os = "linux") {
        eprintln!("skipping musl probe: musl libc only exists on Linux");
        return;
    }
    let target_dir = scratch_root("musl-probe-target");
    let output = std::process::Command::new("cargo")
        .args([
            "build",
            "--offline",
            "--locked",
            "--target",
            "x86_64-unknown-linux-musl",
            "--manifest-path",
        ])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/musl-probe/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .output()
        .expect("cargo is required to build the musl probe");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("may not be installed") || stderr.contains("can't find crate") {
            eprintln!("skipping musl probe: x86_64-unknown-linux-musl target missing");
            return;
        }
        panic!("musl probe build failed: {stderr}");
    }
    let probe = target_dir.join("x86_64-unknown-linux-musl/debug/tiktools-musl-probe");
    let output = std::process::Command::new(&probe)
        .output()
        .expect("musl probe must execute");
    assert!(
        output.status.success(),
        "musl probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("MUSL PROBE OK") && stdout.contains("node-rdev.linux-x64-musl.node"),
        "{stdout:?}"
    );
}

#[test]
fn alias_require_resolves_the_same_package() {
    let (staged, manifest) = stage_native_plugin("alias-require", |_| {});
    let mut instance = NapiVmPluginRuntime.load(&manifest, &staged).unwrap();
    // A fresh alias lookup resolves the same allowlisted binary as the
    // module-top `createRequire` binding.
    let result = call_json(instance.as_mut(), &action_request("native.require"));
    assert_eq!(
        result.get("summary"),
        Some(&json!("require:function:42")),
        "{result}"
    );
    instance.shutdown().unwrap();

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn untrusted_native_addon_is_rejected() {
    let dedicated = scratch_root("untrusted-root");
    let housed = dedicated.join("napi-vm-native");
    let manifest = stage_native_plugin_at(&housed, |manifest| {
        manifest.as_object_mut().unwrap().remove("trust");
    });
    // Without the explicit opt-out the manifest is Sandboxed by default.
    assert_eq!(manifest.trust, tiktools_plugin_api::PluginTrust::Sandboxed);

    // Direct runtime loads refuse.
    let error = match NapiVmPluginRuntime.load(&manifest, &housed) {
        Ok(_) => panic!("untrusted native addon must fail"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("not trusted"), "{error}");

    // Managed loads refuse with the discovered source.
    let manager = PluginManager::new(vec![PluginRoot {
        path: dedicated.clone(),
        source: PluginSource::Development,
    }]);
    manager.scan().unwrap();
    let error = match manager.start(&manifest.id) {
        Ok(()) => panic!("untrusted managed start must fail"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("not trusted"), "{error}");

    fs::remove_dir_all(&dedicated).ok();
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
    // Ship every binary but declare nothing: no alias, no loading. The
    // guest's `require('rdev-node')` falls through to the package's
    // throwing `index.js` tripwire instead of reaching any binary.
    stage_rdev_package(&staged, "rdev-source");
    let manifest =
        PluginManifest::from_json_str(&fs::read_to_string(staged.join("plugin.json")).unwrap())
            .unwrap();
    assert!(manifest.native_addons.is_empty());

    let error = match NapiVmPluginRuntime.load(&manifest, &staged) {
        Ok(_) => panic!("load without a nativeAddons declaration must fail"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("package loader must not execute"), "{error}");

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn missing_host_binary_fails_load_with_clear_error() {
    let (staged, manifest) = stage_native_plugin("no-host-binary", |_| {});
    fs::remove_file(staged.join(format!(
        "node_modules/rdev-node/node-rdev.{}.node",
        current_napi_target()
    )))
    .unwrap();

    let error = match NapiVmPluginRuntime.load(&manifest, &staged) {
        Ok(_) => panic!("load with no host binary must fail"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("has no binary for") && error.contains(&current_napi_target()),
        "{error}"
    );

    fs::remove_dir_all(&staged).ok();
}

#[test]
fn multiple_matching_host_binaries_fail_load() {
    let (staged, manifest) = stage_native_plugin("ambiguous-binary", |_| {});
    // A second file matching the host suffix: selection must refuse to
    // guess between them.
    fs::write(
        staged.join(format!(
            "node_modules/rdev-node/other.{}.node",
            current_napi_target()
        )),
        b"ambiguous host binary",
    )
    .unwrap();

    let error = match NapiVmPluginRuntime.load(&manifest, &staged) {
        Ok(_) => panic!("load with ambiguous host binaries must fail"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("expected exactly one"), "{error}");

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

    // Stopping releases the listener: starting again works.
    let result = call_json(instance.as_mut(), &action_request("native.start"));
    assert_eq!(
        result.get("summary"),
        Some(&json!("started:true")),
        "{result}"
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
        "trust": "trusted",
        "apiVersion": 1,
        "nativeAddons": [{
            "package": "rdev-node",
            "root": "node_modules/rdev-node",
        }],
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    let entry_bytes =
        b"export default { call() { return { logs: [], intents: [], events: [] }; } };";
    let package_bytes = br#"{"name":"rdev-node","version":"1.0.0"}"#;
    let files: &[(&str, &[u8])] = &[
        ("plugin.json", &manifest_bytes),
        ("dist/index.js", entry_bytes),
        ("node_modules/rdev-node/package.json", package_bytes),
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
    assert_eq!(installed.manifest.native_addons[0].package, "rdev-node");
    assert_eq!(
        installed.manifest.native_addons[0].root,
        "node_modules/rdev-node"
    );
    for name in [
        "node_modules/rdev-node/package.json",
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
