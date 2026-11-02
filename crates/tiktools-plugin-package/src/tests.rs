use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::{
    archive::normalized_archive_path, pack, pack::staged_entry_name, pack::validate_target,
    sign_archive, KeyId, KeyRole, KeysDocument, PackRequest, PackageArchive, PackageError,
    PackageErrorKind, Sha256Digest, SigningKey, TrustStore, TrustedKey, CHECKSUMS_FILE,
};

const DEMO_MANIFEST: &[u8] = br#"{"schemaVersion":2,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"index.js"}"#;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let count = COUNTER.fetch_add(1, Ordering::AcqRel);
        let path = std::env::temp_dir().join(format!(
            "tiktools-plugin-package-{name}-{}-{suffix}-{count}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_archive(path: &Path, files: &[(&str, &[u8])]) {
    let mut writer = ZipWriter::new(File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, bytes) in files {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap();
}

fn checksums_for(files: &[(&str, &[u8])]) -> String {
    let entries: std::collections::BTreeMap<_, _> = files
        .iter()
        .map(|(name, bytes)| (*name, Sha256Digest::of_bytes(bytes).to_string()))
        .collect();
    serde_json::to_string(&entries).unwrap()
}

fn demo_archive(directory: &TempDir, name: &str, manifest: &[u8]) -> PathBuf {
    let entry = br#"{"ready":true}"#;
    let checksums = checksums_for(&[("plugin.json", manifest), ("index.js", entry)]);
    let path = directory.join(name);
    write_archive(
        &path,
        &[
            ("plugin.json", manifest),
            ("index.js", entry),
            ("checksums.json", checksums.as_bytes()),
        ],
    );
    path
}

fn read_entry(archive: &Path, name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(File::open(archive).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

fn entry_names(archive: &Path) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(File::open(archive).unwrap()).unwrap();
    (0..archive.len())
        .map(|index| {
            archive
                .by_index(index)
                .unwrap()
                .name()
                .unwrap()
                .into_owned()
        })
        .collect()
}

#[test]
fn rejects_archive_path_traversal() {
    assert!(normalized_archive_path("../../outside").is_err());
    assert!(normalized_archive_path("plugin\\..\\outside").is_err());
    assert!(normalized_archive_path("./plugin.json").unwrap().is_some());
    assert!(normalized_archive_path("./").unwrap().is_none());
}

#[test]
fn rejects_invalid_utf8_archive_names() {
    let directory = TempDir::new("utf8");
    let path = directory.join("invalid-name.plugin");
    write_archive(&path, &[("plugin.json", b"{}")]);
    let mut bytes = fs::read(&path).unwrap();
    for (signature, flags_offset, name_offset) in
        [(&b"PK\x03\x04"[..], 6, 30), (&b"PK\x01\x02"[..], 8, 46)]
    {
        let header = bytes.windows(4).position(|part| part == signature).unwrap();
        bytes[header + flags_offset + 1] |= 0x08;
        bytes[header + name_offset] = 0xff;
    }
    fs::write(&path, bytes).unwrap();
    let archive = PackageArchive::open(&path).unwrap();
    assert!(archive.inspect().is_err());
    let staging = directory.join("staging");
    fs::create_dir_all(&staging).unwrap();
    assert!(archive.extract(&staging).is_err());
}

#[test]
fn rejects_wrong_extension() {
    let directory = TempDir::new("extension");
    let path = directory.join("demo.zip");
    write_archive(&path, &[("plugin.json", DEMO_MANIFEST)]);
    let error = PackageArchive::open(&path).unwrap_err();
    assert!(error.to_string().contains(".plugin extension"));
}

#[test]
fn inspects_manifest_and_extracts_checksum_checked_tree() {
    let directory = TempDir::new("extract");
    let path = demo_archive(&directory, "demo.plugin", DEMO_MANIFEST);
    let archive = PackageArchive::open(&path).unwrap();
    let inspection = archive.inspect().unwrap();
    assert_eq!(inspection.manifest.id, "demo");
    assert!(inspection.checksums.is_some());
    assert!(inspection.signature.is_none());

    let staging = directory.join("staging");
    fs::create_dir_all(&staging).unwrap();
    let extracted = archive.extract(&staging).unwrap();
    assert_eq!(extracted.manifest.version, "1.0.0");
    assert!(extracted.root.join("index.js").is_file());
    assert_eq!(
        archive.digest().unwrap(),
        Sha256Digest::of_file(&path).unwrap()
    );
}

#[test]
fn rejects_unsafe_traversal_entries() {
    let directory = TempDir::new("traversal");
    let path = directory.join("evil.plugin");
    write_archive(
        &path,
        &[("plugin.json", DEMO_MANIFEST), ("../evil.txt", b"evil")],
    );
    let staging = directory.join("staging");
    fs::create_dir_all(&staging).unwrap();
    let error = PackageArchive::open(&path)
        .unwrap()
        .extract(&staging)
        .unwrap_err();
    assert!(error.to_string().contains("unsafe path"), "{error}");
    assert!(!directory.join("evil.txt").exists());
}

#[test]
fn rejects_missing_manifest_missing_checksums_and_tampering() {
    let directory = TempDir::new("integrity");
    let staging = directory.join("staging");

    let missing_manifest = directory.join("missing.plugin");
    write_archive(&missing_manifest, &[("index.js", b"{}")]);
    fs::create_dir_all(staging.join("a")).unwrap();
    assert!(matches!(
        PackageArchive::open(&missing_manifest)
            .unwrap()
            .extract(&staging.join("a")),
        Err(PackageError::MissingManifest)
    ));

    let unchecked = directory.join("unchecked.plugin");
    write_archive(
        &unchecked,
        &[("plugin.json", DEMO_MANIFEST), ("index.js", b"{}")],
    );
    fs::create_dir_all(staging.join("b")).unwrap();
    assert!(matches!(
        PackageArchive::open(&unchecked)
            .unwrap()
            .extract(&staging.join("b")),
        Err(PackageError::MissingChecksums(_))
    ));

    let tampered = directory.join("tampered.plugin");
    let checksums = checksums_for(&[("plugin.json", DEMO_MANIFEST), ("index.js", b"original")]);
    write_archive(
        &tampered,
        &[
            ("plugin.json", DEMO_MANIFEST),
            ("index.js", b"tampered"),
            ("checksums.json", checksums.as_bytes()),
        ],
    );
    fs::create_dir_all(staging.join("c")).unwrap();
    let error = PackageArchive::open(&tampered)
        .unwrap()
        .extract(&staging.join("c"))
        .unwrap_err();
    assert!(matches!(error, PackageError::ChecksumMismatch { .. }));
    assert_eq!(error.kind(), PackageErrorKind::InvalidPackage);

    let unlisted = directory.join("unlisted.plugin");
    let checksums = checksums_for(&[("plugin.json", DEMO_MANIFEST), ("index.js", b"{}")]);
    write_archive(
        &unlisted,
        &[
            ("plugin.json", DEMO_MANIFEST),
            ("index.js", b"{}"),
            ("extra.bin", b"extra"),
            ("checksums.json", checksums.as_bytes()),
        ],
    );
    fs::create_dir_all(staging.join("d")).unwrap();
    assert!(matches!(
        PackageArchive::open(&unlisted)
            .unwrap()
            .extract(&staging.join("d")),
        Err(PackageError::UnlistedFile(_))
    ));
}

#[test]
fn incompatible_manifests_are_classified() {
    let directory = TempDir::new("incompatible");
    let manifest = br#"{"schemaVersion":9,"id":"demo","name":"Demo","version":"1.0.0","runtime":"process","entry":"index.js"}"#;
    let path = demo_archive(&directory, "future.plugin", manifest);
    let error = PackageArchive::open(&path).unwrap().inspect().unwrap_err();
    assert_eq!(error.kind(), PackageErrorKind::Incompatible);
}

#[test]
fn supported_targets_validate() {
    for target in tiktools_plugin_api::manifest::PLUGIN_TARGETS {
        assert_eq!(validate_target(target).unwrap(), target);
    }
    for unsupported in ["winx64", "linux64", "x86_64-unknown-linux-musl", ""] {
        assert!(validate_target(unsupported).is_err(), "{unsupported}");
    }
}

#[test]
fn staged_entry_suffix_follows_requested_target() {
    assert_eq!(
        staged_entry_name("plugin", Some("win32-x64-msvc"), true),
        "plugin.exe"
    );
    assert_eq!(
        staged_entry_name("plugin.exe", Some("win32-x64-msvc"), true),
        "plugin.exe"
    );
    assert_eq!(
        staged_entry_name("plugin", Some("linux-x64-gnu"), true),
        "plugin"
    );
    assert_eq!(
        staged_entry_name("dist/index.js", Some("win32-x64-msvc"), false),
        "dist/index.js"
    );
}

fn source_package(directory: &TempDir, manifest: serde_json::Value) -> (PathBuf, PathBuf) {
    let manifest_path = directory.join("plugin.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let entry_path = directory.join("built-entry");
    fs::write(&entry_path, b"binary").unwrap();
    (manifest_path, entry_path)
}

fn process_manifest() -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 2,
        "id": "target-test",
        "name": "Target Test",
        "version": "1.0.0",
        "runtime": "process",
        "entry": "target-test",
        "protocolVersion": 1,
    })
}

#[test]
fn pack_injects_target_without_touching_source_manifest() {
    let directory = TempDir::new("pack-target");
    let source = process_manifest();
    let (manifest, entry) = source_package(&directory, source.clone());
    let output = directory.join("out/target-test-1.0.0-linux-x64-gnu.plugin");
    let packed = pack(&PackRequest {
        manifest: manifest.clone(),
        entry: Some(entry),
        output: output.clone(),
        target: Some("linux-x64-gnu".to_owned()),
    })
    .unwrap();
    assert_eq!(packed.manifest.targets, ["linux-x64-gnu"]);
    assert_eq!(packed.digest, Sha256Digest::of_file(&output).unwrap());
    let after: serde_json::Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(after, source);
    let checksums: serde_json::Value =
        serde_json::from_slice(&read_entry(&output, "target-test/checksums.json")).unwrap();
    assert!(checksums.get("plugin.json").is_some());
    assert!(checksums.get("target-test").is_some());
    let staging = directory.join("staging");
    fs::create_dir_all(&staging).unwrap();
    PackageArchive::open(&output)
        .unwrap()
        .extract(&staging)
        .unwrap();
}

#[cfg(unix)]
#[test]
fn pack_records_unix_modes() {
    let directory = TempDir::new("pack-modes");
    let (manifest, entry) = source_package(&directory, process_manifest());
    let output = directory.join("modes.plugin");
    pack(&PackRequest {
        manifest,
        entry: Some(entry),
        output: output.clone(),
        target: Some("linux-x64-gnu".to_owned()),
    })
    .unwrap();
    let mut archive = zip::ZipArchive::new(File::open(&output).unwrap()).unwrap();
    assert_eq!(
        archive
            .by_name("target-test/target-test")
            .unwrap()
            .unix_mode(),
        Some(0o100755)
    );
    assert_eq!(
        archive
            .by_name("target-test/plugin.json")
            .unwrap()
            .unix_mode(),
        Some(0o100644)
    );
}

#[test]
fn pack_rejects_targets_for_portable_runtimes() {
    let directory = TempDir::new("pack-wasm");
    let mut manifest = process_manifest();
    manifest["runtime"] = serde_json::json!("wasm");
    let (manifest, entry) = source_package(&directory, manifest);
    assert!(pack(&PackRequest {
        manifest,
        entry: Some(entry),
        output: directory.join("out.plugin"),
        target: Some("linux-x64-gnu".to_owned()),
    })
    .is_err());
}

#[test]
fn pack_keeps_node_modules_for_napi_vm_and_fresh_entry_bytes() {
    let directory = TempDir::new("pack-napi");
    let manifest = serde_json::json!({
        "schemaVersion": 3,
        "id": "native-test",
        "name": "NativeTest",
        "version": "1.0.0",
        "runtime": "napi-vm",
        "entry": "dist/index.js",
    });
    let (manifest, _) = source_package(&directory, manifest);
    fs::create_dir_all(directory.join("dist")).unwrap();
    fs::write(directory.join("dist/index.js"), b"stale").unwrap();
    let entry = directory.join("built-index.js");
    fs::write(&entry, b"fresh").unwrap();
    let package = directory.join("node_modules/rdev-node");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("index.js"), b"loader").unwrap();
    fs::write(package.join("node-rdev.linux-x64-gnu.node"), b"binary").unwrap();
    let output = directory.join("native-test.plugin");
    pack(&PackRequest {
        manifest,
        entry: Some(entry),
        output: output.clone(),
        target: None,
    })
    .unwrap();
    assert_eq!(read_entry(&output, "native-test/dist/index.js"), b"fresh");
    assert_eq!(
        read_entry(
            &output,
            "native-test/node_modules/rdev-node/node-rdev.linux-x64-gnu.node"
        ),
        b"binary"
    );
}

#[test]
fn pack_ignores_node_modules_and_lockfiles_for_other_runtimes() {
    let directory = TempDir::new("pack-process");
    let (manifest, entry) = source_package(&directory, process_manifest());
    fs::create_dir_all(directory.join("node_modules/x")).unwrap();
    fs::write(directory.join("node_modules/x/index.js"), b"loader").unwrap();
    fs::write(directory.join("native-libs.lock.json"), b"{}").unwrap();
    let output = directory.join("out.plugin");
    pack(&PackRequest {
        manifest,
        entry: Some(entry),
        output: output.clone(),
        target: None,
    })
    .unwrap();
    let names = entry_names(&output);
    assert!(
        names.iter().all(|name| !name.contains("node_modules")),
        "{names:?}"
    );
    assert!(
        names.iter().all(|name| !name.contains("native-libs")),
        "{names:?}"
    );
}

#[test]
fn pack_ships_lockfile_for_declared_native_libs() {
    let directory = TempDir::new("pack-lockfile");
    let manifest = serde_json::json!({
        "schemaVersion": 3,
        "id": "native-test",
        "name": "Native Test",
        "version": "1.0.0",
        "runtime": "napi-vm",
        "entry": "dist/index.js",
        "nativeAddons": [{"package": "rdev-node", "root": "node_modules/rdev-node"}],
        "nativeLibs": [{"package": "rdev-node", "version": "1.0.1"}],
    });
    let (manifest, _) = source_package(&directory, manifest);
    fs::create_dir_all(directory.join("dist")).unwrap();
    let entry = directory.join("dist/index.js");
    fs::write(&entry, b"fresh").unwrap();
    fs::write(
        directory.join("native-libs.lock.json"),
        br#"{"version":1,"packages":{}}"#,
    )
    .unwrap();
    let output = directory.join("out.plugin");
    pack(&PackRequest {
        manifest,
        entry: Some(entry),
        output: output.clone(),
        target: Some("linux-x64-gnu".to_owned()),
    })
    .unwrap();
    assert_eq!(
        read_entry(&output, "native-test/native-libs.lock.json"),
        br#"{"version":1,"packages":{}}"#
    );
}

#[test]
fn pack_includes_webview_ui_changelog_and_declarative_packages() {
    let directory = TempDir::new("pack-ui");
    let manifest = serde_json::json!({
        "schemaVersion": 3,
        "id": "ui-test",
        "name": "UI Test",
        "version": "1.0.0",
        "runtime": "declarative",
        "changelog": "CHANGELOG.md",
        "ui": {
            "apiVersion": 1,
            "mode": "webview",
            "entry": "ui/dist/index.html",
            "pages": [{"id": "main", "title": {"default": "Main"}}]
        }
    });
    let (manifest, _) = source_package(&directory, manifest);
    fs::create_dir_all(directory.join("ui/dist/assets")).unwrap();
    fs::write(directory.join("ui/dist/index.html"), b"<html></html>").unwrap();
    fs::write(directory.join("ui/dist/assets/app.js"), b"app").unwrap();
    fs::write(directory.join("ui/source.ts"), b"secret source").unwrap();
    fs::write(directory.join("CHANGELOG.md"), b"# 1.0.0\n").unwrap();
    let output = directory.join("ui.plugin");
    pack(&PackRequest {
        manifest,
        entry: None,
        output: output.clone(),
        target: None,
    })
    .unwrap();
    let names = entry_names(&output);
    assert!(
        names.contains(&"ui-test/ui/dist/index.html".to_owned()),
        "{names:?}"
    );
    assert!(
        names.contains(&"ui-test/ui/dist/assets/app.js".to_owned()),
        "{names:?}"
    );
    assert!(
        names.contains(&"ui-test/CHANGELOG.md".to_owned()),
        "{names:?}"
    );
    assert!(
        !names.iter().any(|name| name.ends_with("source.ts")),
        "{names:?}"
    );
    let inspection = PackageArchive::open(&output).unwrap().inspect().unwrap();
    assert_eq!(inspection.changelog.as_deref(), Some("# 1.0.0\n"));
}

fn publisher_key(id: &str, publisher: &str) -> SigningKey {
    SigningKey::generate(KeyId::parse(id).unwrap(), Some(publisher.to_owned())).unwrap()
}

fn packed_demo(directory: &TempDir, id: &str) -> PathBuf {
    let mut manifest = process_manifest();
    manifest["id"] = serde_json::json!(id);
    let (manifest, entry) = source_package(directory, manifest);
    pack(&PackRequest {
        manifest,
        entry: Some(entry),
        output: directory.join("signed.plugin"),
        target: None,
    })
    .unwrap()
    .path
}

#[test]
fn signing_keys_round_trip_and_sign_detached_documents() {
    let key = publisher_key("acme-2026a", "acme");
    let restored = SigningKey::from_json(&key.to_json()).unwrap();
    assert_eq!(restored.public_key(), key.public_key());
    let signature = key.sign_detached(b"document");
    let mut store = TrustStore::new();
    store.insert(
        TrustedKey::new(key.public_key(), KeyRole::Index),
        "official",
    );
    assert!(store
        .verify_detached(&signature, b"document", KeyRole::Index, 0)
        .is_ok());
    assert!(store
        .verify_detached(&signature, b"tampered", KeyRole::Index, 0)
        .is_err());
    assert!(store
        .verify_detached(&signature, b"document", KeyRole::Publisher, 0)
        .is_err());
}

#[test]
fn signed_packages_verify_against_publisher_scope() {
    let directory = TempDir::new("sign");
    let key = publisher_key("acme-2026a", "acme");
    let path = packed_demo(&directory, "acme.demo");
    sign_archive(&path, &key).unwrap();
    let archive = PackageArchive::open(&path).unwrap();
    let inspection = archive.inspect().unwrap();
    let signature = inspection.signature.clone().unwrap();
    let checksums = inspection.checksums.clone().unwrap();

    let mut store = TrustStore::new();
    store.insert(
        TrustedKey::new(key.public_key(), KeyRole::Publisher).publisher("acme"),
        "official",
    );
    let verified = store
        .verify_package(&signature, &checksums, "acme.demo", 0)
        .unwrap();
    assert_eq!(verified.publisher, "acme");
    assert_eq!(verified.source, "official");

    let scope = store
        .verify_package(&signature, &checksums, "other.demo", 0)
        .unwrap_err();
    assert_eq!(scope.kind(), PackageErrorKind::UntrustedPublisher);
    assert!(store
        .verify_package(&signature, b"{}", "acme.demo", 0)
        .is_err());

    let staging = directory.join("staging");
    fs::create_dir_all(&staging).unwrap();
    let extracted = archive.extract(&staging).unwrap();
    assert_eq!(extracted.signature, Some(signature));
    assert!(!entry_names(&path)
        .iter()
        .any(|name| name.ends_with(CHECKSUMS_FILE) && name.contains("signature")));
}

#[test]
fn revoked_expired_and_unknown_keys_are_rejected() {
    let directory = TempDir::new("revoked");
    let key = publisher_key("acme-2026a", "acme");
    let path = packed_demo(&directory, "acme.demo");
    sign_archive(&path, &key).unwrap();
    let inspection = PackageArchive::open(&path).unwrap().inspect().unwrap();
    let signature = inspection.signature.unwrap();
    let checksums = inspection.checksums.unwrap();

    let empty = TrustStore::new();
    assert_eq!(
        empty
            .verify_package(&signature, &checksums, "acme.demo", 0)
            .unwrap_err()
            .kind(),
        PackageErrorKind::UntrustedPublisher
    );

    let mut revoked = TrustedKey::new(key.public_key(), KeyRole::Publisher).publisher("acme");
    revoked.revoked = true;
    let mut store = TrustStore::new();
    store.insert(revoked, "official");
    assert_eq!(
        store
            .verify_package(&signature, &checksums, "acme.demo", 0)
            .unwrap_err()
            .kind(),
        PackageErrorKind::Revoked
    );

    let mut expired = TrustedKey::new(key.public_key(), KeyRole::Publisher).publisher("acme");
    expired.not_after = Some(10);
    let mut store = TrustStore::new();
    store.insert(expired, "official");
    assert!(store
        .verify_package(&signature, &checksums, "acme.demo", 11)
        .is_err());
}

#[test]
fn delegations_never_replace_pinned_roots() {
    let root = SigningKey::generate(KeyId::parse("root-1").unwrap(), None).unwrap();
    let mut store = TrustStore::new();
    store.insert(
        TrustedKey::new(root.public_key(), KeyRole::Root),
        "official",
    );
    let forged = TrustedKey::new(root.public_key(), KeyRole::Index);
    let publisher = publisher_key("acme-1", "acme");
    let document = KeysDocument::new(vec![
        forged,
        TrustedKey::new(publisher.public_key(), KeyRole::Publisher).publisher("acme"),
    ]);
    store.extend_delegations(
        &KeysDocument::parse(&document.to_bytes()).unwrap(),
        "official",
    );
    assert_eq!(store.get(root.key_id()).unwrap().role, KeyRole::Root);
    assert!(store.get(publisher.key_id()).is_some());
    store.remove_source("official");
    assert!(store.is_empty());
}

#[test]
fn legacy_scopes_allow_grandfathered_ids() {
    let key = publisher_key("tiktools-1", "tiktools");
    let mut trusted = TrustedKey::new(key.public_key(), KeyRole::Publisher).publisher("tiktools");
    assert!(trusted.allows_package("tiktools.gateway"));
    assert!(!trusted.allows_package("sonicboom.server"));
    trusted.scopes = vec!["tiktools.*".to_owned(), "sonicboom.server".to_owned()];
    assert!(trusted.allows_package("sonicboom.server"));
    assert!(trusted.allows_package("tiktools.gateway"));
    assert!(!trusted.allows_package("tiktools."));
}

#[test]
fn native_entries_keep_their_name_on_windows_targets() {
    let directory = TempDir::new("pack-native");
    let mut manifest = process_manifest();
    manifest["runtime"] = serde_json::json!("native");
    manifest["entry"] = serde_json::json!("native/target-test.bin");
    let (manifest, entry) = source_package(&directory, manifest);
    let packed = pack(&PackRequest {
        manifest,
        entry: Some(entry),
        output: directory.join("native.plugin"),
        target: Some("win32-x64-msvc".to_owned()),
    })
    .unwrap();
    assert_eq!(packed.manifest.entry, "native/target-test.bin");
    assert!(entry_names(&packed.path).contains(&"target-test/native/target-test.bin".to_owned()));
}
