use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tiktools_plugin_sdk::native_stage::*;

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Scratch package source under the system temp dir, unique per test.
fn fixture_source(label: &str) -> PathBuf {
    let id = FIXTURE_COUNTER.fetch_add(1, Ordering::AcqRel);
    let root = std::env::temp_dir().join(format!(
        "tiktools-native-stage-{label}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_source(root: &Path, name: &str, bytes: &[u8]) {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}

/// Minimal napi-rs-shaped package: manifest, loaders, types, and two
/// platform binaries. Callers mutate the manifest for negative tests.
fn write_package(root: &Path) {
    write_source(
        root,
        "package.json",
        br#"{
  "name": "rdev-node",
  "version": "1.0.1",
  "main": "index.js",
  "types": "./index.d.ts",
  "exports": { ".": { "types": "./index.d.ts", "import": "./index.mjs", "require": "./index.js" } },
  "files": ["index.js", "index.mjs", "index.d.ts", "*.node"]
}"#,
    );
    write_source(root, "index.js", b"module.exports = {};");
    write_source(root, "index.mjs", b"export default {};");
    write_source(root, "index.d.ts", b"export {};");
    write_source(root, "node-rdev.linux-x64-gnu.node", b"fake-linux");
    write_source(root, "node-rdev.win32-x64-msvc.node", b"fake-win32");
}

fn stage_request<'a>(
    package: &'a str,
    source: &'a Path,
    plugin_dir: &'a Path,
    root: &'a str,
) -> NativeStageRequest<'a> {
    NativeStageRequest {
        package,
        source,
        plugin_dir,
        root,
        overwrite: false,
    }
}

#[test]
fn stages_package_honoring_files_allowlist() {
    let source = fixture_source("happy");
    write_package(&source);
    // Build byproducts a checkout carries but `files` excludes.
    write_source(&source, "src/lib.rs", b"fn main() {}");
    fs::create_dir_all(source.join("node_modules/dep")).unwrap();
    write_source(&source, "node_modules/dep/index.js", b"{}");

    let plugin = fixture_source("happy-plugin");
    let report = stage_native_package(&stage_request(
        "rdev-node",
        &source,
        &plugin,
        "node_modules/rdev-node",
    ))
    .unwrap();
    assert_eq!(report.root, "node_modules/rdev-node");
    assert_eq!(
        report.files,
        vec![
            "node_modules/rdev-node/index.d.ts",
            "node_modules/rdev-node/index.js",
            "node_modules/rdev-node/index.mjs",
            "node_modules/rdev-node/node-rdev.linux-x64-gnu.node",
            "node_modules/rdev-node/node-rdev.win32-x64-msvc.node",
            "node_modules/rdev-node/package.json",
        ]
    );
    assert_eq!(
        report.node_binaries,
        vec![
            "node_modules/rdev-node/node-rdev.linux-x64-gnu.node",
            "node_modules/rdev-node/node-rdev.win32-x64-msvc.node",
        ]
    );
    assert!(plugin.join("node_modules/rdev-node/index.js").is_file());
    assert!(!plugin.join("node_modules/rdev-node/src").exists());
    assert!(!plugin.join("node_modules/rdev-node/node_modules").exists());

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}

#[test]
fn checkout_directories_are_never_traversed() {
    // Even with a `files` allowlist, dependency trees are pruned
    // before symlink checks: a source checkout nests symlinks under
    // node_modules that staging must neither copy nor choke on.
    let source = fixture_source("prune");
    write_package(&source);
    fs::create_dir_all(source.join("node_modules/dep")).unwrap();
    write_source(&source, "node_modules/dep/index.js", b"{}");
    #[cfg(unix)]
    std::os::unix::fs::symlink("index.js", source.join("node_modules/dep/link.js")).unwrap();

    let plugin = fixture_source("prune-plugin");
    let report = stage_native_package(&stage_request(
        "rdev-node",
        &source,
        &plugin,
        "node_modules/rdev-node",
    ))
    .unwrap();
    assert!(
        !report
            .files
            .iter()
            .any(|file| file.contains("node_modules/rdev-node/node_modules")),
        "{report:?}"
    );
    assert!(!plugin.join("node_modules/rdev-node/node_modules").exists());

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}

#[test]
fn without_files_skips_checkout_directories() {
    let source = fixture_source("no-files");
    write_package(&source);
    let manifest = fs::read_to_string(source.join("package.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    value.as_object_mut().unwrap().remove("files");
    fs::write(
        source.join("package.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    write_source(&source, "stray.txt", b"stray");
    write_source(&source, "target/debug/lib.node", b"nope");
    write_source(&source, ".git/HEAD", b"ref");

    let plugin = fixture_source("no-files-plugin");
    let report = stage_native_package(&stage_request(
        "rdev-node",
        &source,
        &plugin,
        "node_modules/rdev-node",
    ))
    .unwrap();
    assert!(report.files.iter().any(|file| file.ends_with("stray.txt")));
    assert!(!report.files.iter().any(|file| file.contains("target/")));
    assert!(!report.files.iter().any(|file| file.contains(".git/")));

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}

#[test]
fn rejects_mismatched_package_name() {
    let source = fixture_source("name");
    write_package(&source);
    let plugin = fixture_source("name-plugin");
    let error = stage_native_package(&stage_request(
        "other-pkg",
        &source,
        &plugin,
        "node_modules/other-pkg",
    ))
    .unwrap_err();
    assert!(
        matches!(error, NativeStageError::PackageNameMismatch { .. }),
        "{error}"
    );

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}

#[test]
fn rejects_traversal_and_absolute_roots() {
    let source = fixture_source("roots");
    write_package(&source);
    let plugin = fixture_source("roots-plugin");
    for root in ["../escape", "/absolute", "", "nested/../../escape"] {
        let error =
            stage_native_package(&stage_request("rdev-node", &source, &plugin, root)).unwrap_err();
        assert!(
            matches!(error, NativeStageError::InvalidRoot(_)),
            "{root}: {error}"
        );
    }
    assert!(!plugin.join("escape").exists());

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}

#[test]
fn rejects_invalid_package_names() {
    let source = fixture_source("names");
    write_package(&source);
    let plugin = fixture_source("names-plugin");
    for package in ["", "../evil", "@scope", "has space", "a/b/c"] {
        let error = stage_native_package(&stage_request(
            package,
            &source,
            &plugin,
            "node_modules/pkg",
        ))
        .unwrap_err();
        assert!(
            matches!(error, NativeStageError::InvalidPackage(_)),
            "{package}: {error}"
        );
    }

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}

#[test]
fn requires_entrypoints_and_binaries() {
    // Missing export target.
    let source = fixture_source("entry");
    write_package(&source);
    fs::remove_file(source.join("index.mjs")).unwrap();
    let plugin = fixture_source("entry-plugin");
    let error = stage_native_package(&stage_request(
        "rdev-node",
        &source,
        &plugin,
        "node_modules/rdev-node",
    ))
    .unwrap_err();
    assert!(
        matches!(error, NativeStageError::MissingEntrypoint(_)),
        "{error}"
    );

    // No .node files at all.
    let bare = fixture_source("nobin");
    write_package(&bare);
    fs::remove_file(bare.join("node-rdev.linux-x64-gnu.node")).unwrap();
    fs::remove_file(bare.join("node-rdev.win32-x64-msvc.node")).unwrap();
    let bare_plugin = fixture_source("nobin-plugin");
    let error = stage_native_package(&stage_request(
        "rdev-node",
        &bare,
        &bare_plugin,
        "node_modules/rdev-node",
    ))
    .unwrap_err();
    assert!(
        matches!(error, NativeStageError::NoNativeBinaries(_)),
        "{error}"
    );

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
    fs::remove_dir_all(&bare).ok();
    fs::remove_dir_all(&bare_plugin).ok();
}

#[test]
fn refuses_symlinks_and_existing_destinations() {
    let source = fixture_source("links");
    write_package(&source);
    #[cfg(unix)]
    std::os::unix::fs::symlink("index.js", source.join("evil.js")).unwrap();
    let plugin = fixture_source("links-plugin");
    #[cfg(unix)]
    {
        let error = stage_native_package(&stage_request(
            "rdev-node",
            &source,
            &plugin,
            "node_modules/rdev-node",
        ))
        .unwrap_err();
        assert!(
            matches!(error, NativeStageError::SymlinkRefused(_)),
            "{error}"
        );
        fs::remove_file(source.join("evil.js")).unwrap();
    }

    stage_native_package(&stage_request(
        "rdev-node",
        &source,
        &plugin,
        "node_modules/rdev-node",
    ))
    .unwrap();
    let error = stage_native_package(&stage_request(
        "rdev-node",
        &source,
        &plugin,
        "node_modules/rdev-node",
    ))
    .unwrap_err();
    assert!(
        matches!(error, NativeStageError::AlreadyStaged(_)),
        "{error}"
    );

    let mut retry = stage_request("rdev-node", &source, &plugin, "node_modules/rdev-node");
    retry.overwrite = true;
    stage_native_package(&retry).unwrap();

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}

#[test]
fn native_only_packages_need_no_javascript_entrypoint() {
    let source = fixture_source("native-only");
    write_source(
        &source,
        "package.json",
        br#"{"name": "rdev-node", "version": "1.0.1"}"#,
    );
    write_source(&source, "node-rdev.linux-x64-gnu.node", b"fake-linux");
    write_source(&source, "node-rdev.win32-x64-msvc.node", b"fake-win32");

    let plugin = fixture_source("native-only-plugin");
    let report = stage_native_package(&stage_request(
        "rdev-node",
        &source,
        &plugin,
        "node_modules/rdev-node",
    ))
    .unwrap();
    assert_eq!(report.files.len(), 3);
    assert_eq!(report.node_binaries.len(), 2);

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}

#[test]
fn scoped_packages_stage_under_nested_roots() {
    let source = fixture_source("scoped");
    write_package(&source);
    let manifest = fs::read_to_string(source.join("package.json")).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    value.as_object_mut().unwrap().insert(
        "name".to_owned(),
        serde_json::Value::String("@scope/rdev-node".to_owned()),
    );
    fs::write(
        source.join("package.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();

    let plugin = fixture_source("scoped-plugin");
    let report = stage_native_package(&stage_request(
        "@scope/rdev-node",
        &source,
        &plugin,
        "node_modules/@scope/rdev-node",
    ))
    .unwrap();
    assert_eq!(report.files.len(), 6);
    assert!(plugin
        .join("node_modules/@scope/rdev-node/index.js")
        .is_file());

    fs::remove_dir_all(&source).ok();
    fs::remove_dir_all(&plugin).ok();
}
