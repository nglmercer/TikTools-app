use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tiktools_plugin_api::PluginManifest;
use tiktools_plugin_package::{KeysDocument, PackageArchive};
use tiktools_plugin_registry::{shard_path, PackageDocument};

use crate::{run, scaffold::render_template, validate::publish_issues, TEMPLATES};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tiktools-devkit-{name}-{}-{suffix}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::AcqRel)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }

    fn arg(&self, path: &str) -> String {
        self.join(path).display().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn validate_all_fails_if_any_manifest_is_missing_or_not_publishable() {
    let directory = TempDir::new("validate-all");
    let files = render_template("declarative", "acme.demo", "acme", None, "{}");
    fs::write(directory.join("plugin.json"), &files["plugin.json"]).unwrap();
    fs::write(directory.join("CHANGELOG.md"), &files["CHANGELOG.md"]).unwrap();
    let valid = directory.0.to_string_lossy().to_string();
    let missing = directory.arg("missing.json");
    assert!(run(args(&["validate-all", &valid])).is_ok());
    assert!(run(args(&["validate-all", &valid, &missing])).is_err());
    assert!(run(args(&["validate-all", &valid, &valid])).is_err());
    assert!(run(args(&["validate-all"])).is_err());
}

#[cfg(unix)]
#[test]
fn keygen_writes_private_keys_safely_and_records_explicit_scopes() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let dir = TempDir::new("scoped-key");
    let path = dir.0.to_string_lossy().to_string();
    run(args(&[
        "keygen",
        "--id",
        "official-key",
        "--publisher",
        "tiktools",
        "--out",
        &path,
        "--scope",
        "tiktools.*",
        "--scope",
        "sonicboom.server",
    ]))
    .unwrap();
    let private = dir.join("official-key.key.json");
    assert_eq!(
        fs::metadata(&private).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let public: tiktools_plugin_package::TrustedKey =
        serde_json::from_slice(&fs::read(dir.join("official-key.pub.json")).unwrap()).unwrap();
    assert!(public.allows_package("sonicboom.server"));
    assert!(!public.allows_package("sonicboom.other"));
    assert!(run(args(&[
        "keygen", "--id", "bad", "--role", "root", "--scope", "*", "--out", &path
    ]))
    .is_err());
    let victim = dir.join("victim");
    fs::write(&victim, b"unchanged").unwrap();
    symlink(&victim, dir.join("link.key.json")).unwrap();
    assert!(run(args(&[
        "keygen",
        "--id",
        "link",
        "--publisher",
        "tiktools",
        "--out",
        &path,
        "--force"
    ]))
    .is_err());
    assert_eq!(fs::read(victim).unwrap(), b"unchanged");
}

#[test]
fn every_template_renders_a_publishable_manifest() {
    for template in TEMPLATES {
        let files = render_template(template, "acme.demo", "acme", None, "{ path = \"../sdk\" }");
        let manifest = PluginManifest::from_json_str(&files["plugin.json"])
            .unwrap_or_else(|error| panic!("{template}: {error}\n{}", files["plugin.json"]));
        manifest.validate_compatibility().unwrap();
        assert_eq!(manifest.id, "acme.demo");
        let directory = TempDir::new(template);
        fs::write(directory.join("CHANGELOG.md"), &files["CHANGELOG.md"]).unwrap();
        let issues = publish_issues(&manifest, &directory.0, false);
        assert!(issues.is_empty(), "{template}: {issues:?}");
        assert!(
            files.values().all(|contents| !contents.contains("{{id}}")
                && !contents.contains("{{name}}")
                && !contents.contains("{{crate}}")),
            "{template} left placeholders"
        );
    }
    let rust = render_template(
        "process-rust",
        "acme.demo",
        "acme",
        Some("Acme Demo"),
        "{ path = \"../sdk\" }",
    );
    assert!(rust["src/main.rs"].contains("tiktools_process_plugin!(AcmeDemoPlugin);"));
    assert!(rust["Cargo.toml"].contains("tiktools-plugin-sdk = { path = \"../sdk\" }"));
    let native = render_template("native-rust", "acme.demo", "acme", None, "{}");
    assert!(native["Cargo.toml"].contains("crate-type = [\"cdylib\"]"));
    assert!(native["src/lib.rs"].contains("tiktools_export_native_plugin!"));
    let napi = render_template("napi-vm-ts", "acme.demo", "acme", None, "{}");
    let manifest = PluginManifest::from_json_str(&napi["plugin.json"]).unwrap();
    assert_eq!(manifest.name, "acme-demo");
    let webview = render_template("webview-ui", "acme.demo", "acme", None, "{}");
    let manifest = PluginManifest::from_json_str(&webview["plugin.json"]).unwrap();
    assert!(manifest.ui.is_some());
}

#[test]
fn new_rejects_invalid_ids_and_non_empty_directories() {
    let directory = TempDir::new("new");
    assert!(run(args(&["new", "Bad Id", "--dir", &directory.arg("bad")])).is_err());
    fs::create_dir_all(directory.join("taken")).unwrap();
    fs::write(directory.join("taken/file"), b"x").unwrap();
    assert!(run(args(&[
        "new",
        "acme.demo",
        "--dir",
        &directory.arg("taken")
    ]))
    .is_err());
    run(args(&[
        "new",
        "acme.demo",
        "--template",
        "declarative",
        "--dir",
        &directory.arg("fresh"),
    ]))
    .unwrap();
    assert!(directory.join("fresh/plugin.json").is_file());
    assert!(directory.join("fresh/.gitignore").is_file());
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn scaffold_pack_sign_and_publish_to_a_static_registry() {
    let directory = TempDir::new("publish");
    let plugin = directory.arg("plugin");
    let keys = directory.arg("keys");
    run(args(&[
        "new",
        "acme.hooks",
        "--template",
        "declarative",
        "--dir",
        &plugin,
    ]))
    .unwrap();
    run(args(&["validate", &plugin, "--publish"])).unwrap();
    run(args(&[
        "keygen", "--id", "root-1", "--role", "root", "--out", &keys,
    ]))
    .unwrap();
    run(args(&[
        "keygen", "--id", "index-1", "--role", "index", "--out", &keys,
    ]))
    .unwrap();
    run(args(&[
        "keygen",
        "--id",
        "acme-1",
        "--publisher",
        "acme",
        "--out",
        &keys,
    ]))
    .unwrap();
    assert!(run(args(&[
        "keygen",
        "--id",
        "acme-1",
        "--publisher",
        "acme",
        "--out",
        &keys
    ]))
    .is_err());

    let out = directory.arg("dist");
    run(args(&[
        "pack",
        &plugin,
        "--out-dir",
        &out,
        "--sign",
        &directory.arg("keys/acme-1.key.json"),
    ]))
    .unwrap();
    let archive = directory.join("dist/acme.hooks-0.1.0.plugin");
    let inspection = PackageArchive::open(&archive).unwrap().inspect().unwrap();
    assert!(inspection.signature.is_some());
    assert_eq!(
        inspection.changelog.as_deref().map(str::trim),
        Some("# Changelog\n\n## 0.1.0\n\n- First release.")
    );
    run(args(&[
        "verify",
        &archive.display().to_string(),
        "--trust",
        &directory.arg("keys/acme-1.pub.json"),
    ]))
    .unwrap();

    let registry = directory.arg("registry");
    run(args(&[
        "index",
        "init",
        &registry,
        "--name",
        "test",
        "--root-key",
        &directory.arg("keys/root-1.key.json"),
        "--index-key",
        &directory.arg("keys/index-1.key.json"),
    ]))
    .unwrap();
    let unsigned_publisher = run(args(&[
        "publish",
        &archive.display().to_string(),
        "--registry",
        &registry,
        "--index-key",
        &directory.arg("keys/index-1.key.json"),
    ]));
    assert!(
        unsigned_publisher.is_err(),
        "publisher key is not delegated yet"
    );
    run(args(&[
        "index",
        "delegate",
        &registry,
        "--root-key",
        &directory.arg("keys/root-1.key.json"),
        "--delegate",
        &directory.arg("keys/acme-1.pub.json"),
    ]))
    .unwrap();
    let keys_document =
        KeysDocument::parse(&fs::read(directory.join("registry/keys.json")).unwrap()).unwrap();
    assert_eq!(keys_document.keys.len(), 2);
    run(args(&[
        "publish",
        &archive.display().to_string(),
        "--registry",
        &registry,
        "--index-key",
        &directory.arg("keys/index-1.key.json"),
    ]))
    .unwrap();
    let document: PackageDocument = serde_json::from_value(read_json(
        &directory.join("registry").join(shard_path("acme.hooks")),
    ))
    .unwrap();
    assert_eq!(document.releases.len(), 1);
    assert_eq!(
        document.releases[0].artifacts[0].key_id.as_deref(),
        Some("acme-1")
    );
    assert!(directory
        .join("registry")
        .join(format!("{}.sig", shard_path("acme.hooks")))
        .is_file());

    run(args(&[
        "index",
        "yank",
        &registry,
        "acme.hooks@0.1.0",
        "--index-key",
        &directory.arg("keys/index-1.key.json"),
    ]))
    .unwrap();
    let document: PackageDocument = serde_json::from_value(read_json(
        &directory.join("registry").join(shard_path("acme.hooks")),
    ))
    .unwrap();
    assert!(document.releases[0].yanked);
    let catalog = read_json(&directory.join("registry/catalog/page-1.json"));
    assert_eq!(catalog["total"], 0);
}

#[test]
fn dev_stage_copies_a_validated_package() {
    let directory = TempDir::new("dev");
    let plugin = directory.arg("plugin");
    run(args(&[
        "new",
        "acme.dev",
        "--template",
        "declarative",
        "--dir",
        &plugin,
    ]))
    .unwrap();
    let into = directory.arg("dev-plugins");
    run(args(&["dev", "stage", &plugin, "--into", &into])).unwrap();
    assert!(directory.join("dev-plugins/acme.dev/plugin.json").is_file());
    assert!(directory
        .join("dev-plugins/acme.dev/checksums.json")
        .is_file());
    run(args(&["dev", "stage", &plugin, "--into", &into])).unwrap();
    assert_eq!(
        fs::read_dir(directory.join("dev-plugins")).unwrap().count(),
        1
    );
}

#[test]
fn legacy_pack_flags_still_work() {
    let directory = TempDir::new("legacy");
    fs::write(
        directory.join("plugin.json"),
        br#"{"schemaVersion":2,"id":"legacy","name":"Legacy","version":"1.0.0","runtime":"process","entry":"legacy"}"#,
    )
    .unwrap();
    fs::write(directory.join("built"), b"binary").unwrap();
    run(args(&[
        "pack",
        "--manifest",
        &directory.arg("plugin.json"),
        "--entry",
        &directory.arg("built"),
        "--output",
        &directory.arg("out/legacy.plugin"),
        "--target",
        "linux-x64-gnu",
    ]))
    .unwrap();
    let inspection = PackageArchive::open(directory.join("out/legacy.plugin"))
        .unwrap()
        .inspect()
        .unwrap();
    assert_eq!(inspection.manifest.targets, ["linux-x64-gnu"]);
}
