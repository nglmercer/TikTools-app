use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use tiktools_plugin_api::HostCompat;
use tiktools_plugin_package::{
    pack, sign_archive, KeyId, KeyRole, PackRequest, SigningKey, TrustStore, TrustedKey,
};
use tiktools_plugin_registry::{
    Advisory, AdvisorySeverity, Downloader, IndexWriter, RegistryClient, RegistrySource,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::{
    check_updates, ActivationHooks, ChangeKind, CommitOptions, Consent, InstallErrorCode,
    Installer, InstallerConfig, JobKind, JobRegistry, JobReporter, JobStage, JobStatus, NoopHooks,
    NoopReporter, PackageOrigin, PackageSources, PackageSpec, PackageSpecInput, SignaturePolicy,
    SignatureStatus, UpdateKind,
};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tiktools-installer-{name}-{}-{suffix}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::AcqRel)
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

fn host() -> HostCompat {
    HostCompat::new(semver::Version::new(0, 4, 0))
}

fn installer(root: &TempDir) -> Installer {
    installer_with(root, Arc::new(RwLock::new(TrustStore::new())))
}

fn installer_with(root: &TempDir, trust: Arc<RwLock<TrustStore>>) -> Installer {
    Installer::new(
        InstallerConfig {
            plugins_dir: root.join("plugins"),
            store_dir: root.join("store"),
            host: host(),
            retain_versions: 2,
            provider_endpoints: None,
        },
        trust,
    )
}

fn digest_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_archive(path: &Path, files: &[(&str, &[u8])], with_checksums: bool) {
    let mut writer = ZipWriter::new(File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, bytes) in files {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    if with_checksums {
        let checksums: BTreeMap<_, _> = files
            .iter()
            .map(|(name, bytes)| (*name, digest_bytes(bytes)))
            .collect();
        writer.start_file("checksums.json", options).unwrap();
        writer
            .write_all(serde_json::to_string(&checksums).unwrap().as_bytes())
            .unwrap();
    }
    writer.finish().unwrap();
}

fn manifest(id: &str, version: &str, extra: serde_json::Value) -> Vec<u8> {
    let mut value = serde_json::json!({
        "schemaVersion": 3,
        "id": id,
        "name": "Demo",
        "version": version,
        "runtime": "process",
        "entry": "demo-host",
    });
    value
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    serde_json::to_vec(&value).unwrap()
}

fn demo_package(root: &TempDir, id: &str, version: &str, extra: serde_json::Value) -> PathBuf {
    let path = root.join(&format!("{id}-{version}.plugin"));
    let manifest = manifest(id, version, extra);
    let entry = format!("binary {version}");
    write_archive(
        &path,
        &[("plugin.json", &manifest), ("demo-host", entry.as_bytes())],
        true,
    );
    path
}

fn implicit(replace_existing: bool) -> CommitOptions {
    CommitOptions {
        consent: Consent::Implicit { replace_existing },
        policy: None,
    }
}

fn install_file(
    installer: &Installer,
    path: &Path,
    replace: bool,
) -> Result<crate::InstallOutcome, crate::InstallError> {
    let prepared =
        installer.prepare(path, PackageOrigin::File, SignaturePolicy::AllowWithConsent)?;
    installer.commit(&prepared, &implicit(replace), &NoopHooks, &NoopReporter)
}

#[derive(Default)]
struct RecordingHooks {
    calls: Mutex<Vec<String>>,
    fail_activation: AtomicBool,
}

impl ActivationHooks for RecordingHooks {
    fn deactivate(&self, plugin_id: &str) -> Result<bool, String> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("deactivate:{plugin_id}"));
        Ok(true)
    }

    fn activate(&self, plugin_id: &str, was_running: bool) -> Result<(), String> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("activate:{plugin_id}:{was_running}"));
        if self.fail_activation.swap(false, Ordering::AcqRel) {
            return Err("scan failed".to_owned());
        }
        Ok(())
    }
}

#[test]
fn installs_checksum_checked_package_atomically_with_receipt() {
    let root = TempDir::new("atomic");
    let installer = installer(&root);
    let outcome = install_file(
        &installer,
        &demo_package(&root, "demo", "1.0.0", serde_json::json!({})),
        false,
    )
    .unwrap();
    assert_eq!(outcome.change, ChangeKind::Install);
    assert!(root.join("plugins/demo/plugin.json").is_file());
    assert!(root.join("plugins/demo/demo-host").is_file());
    assert_eq!(outcome.receipt.origin, PackageOrigin::File);
    assert_eq!(outcome.receipt.signature.status, SignatureStatus::Unsigned);
    assert_eq!(installer.receipt("demo").unwrap().unwrap().version, "1.0.0");
    assert_eq!(
        fs::read_dir(root.join("plugins/.staging")).unwrap().count(),
        0
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(root.join("plugins/demo/demo-host"))
            .unwrap()
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0);
    }
}

#[test]
fn rejects_bad_archives_without_touching_plugins() {
    let root = TempDir::new("bad");
    let installer = installer(&root);
    let zip = root.join("demo.zip");
    write_archive(
        &zip,
        &[(
            "plugin.json",
            &manifest("demo", "1.0.0", serde_json::json!({})),
        )],
        true,
    );
    let error = installer
        .prepare(&zip, PackageOrigin::File, SignaturePolicy::AllowWithConsent)
        .unwrap_err();
    assert!(error.message.contains(".plugin extension"));
    assert_eq!(error.code, InstallErrorCode::InvalidPackage);

    let missing = root.join("missing.plugin");
    write_archive(&missing, &[("demo-host", b"{}")], false);
    assert_eq!(
        installer
            .prepare(
                &missing,
                PackageOrigin::File,
                SignaturePolicy::AllowWithConsent
            )
            .unwrap_err()
            .code,
        InstallErrorCode::InvalidPackage
    );

    let evil = root.join("evil.plugin");
    write_archive(
        &evil,
        &[
            (
                "plugin.json",
                &manifest("demo", "1.0.0", serde_json::json!({})),
            ),
            ("../evil.txt", b"evil"),
        ],
        false,
    );
    let error = install_file(&installer, &evil, false).unwrap_err();
    assert!(error.message.contains("unsafe path"), "{error}");
    assert!(!root.join("plugins/demo").exists());
    assert!(!root.join("evil.txt").exists());
}

#[test]
fn existing_plugin_requires_replace_and_updates_keep_previous_version() {
    let root = TempDir::new("replace");
    let installer = installer(&root);
    install_file(
        &installer,
        &demo_package(&root, "demo", "1.0.0", serde_json::json!({})),
        false,
    )
    .unwrap();
    let v2 = demo_package(&root, "demo", "2.0.0", serde_json::json!({}));
    let error = install_file(&installer, &v2, false).unwrap_err();
    assert_eq!(error.code, InstallErrorCode::AlreadyInstalled);
    assert!(error.message.contains("already installed"));
    assert!(fs::read_to_string(root.join("plugins/demo/plugin.json"))
        .unwrap()
        .contains("1.0.0"));

    let outcome = install_file(&installer, &v2, true).unwrap();
    assert_eq!(outcome.change, ChangeKind::Update);
    assert_eq!(outcome.previous_version.as_deref(), Some("1.0.0"));
    assert!(fs::read_to_string(root.join("plugins/demo/plugin.json"))
        .unwrap()
        .contains("2.0.0"));
    let versions = installer.versions("demo").unwrap();
    assert_eq!(versions.installed.as_deref(), Some("2.0.0"));
    assert_eq!(versions.retained.len(), 1);
    assert_eq!(versions.retained[0].version, "1.0.0");
}

#[test]
fn consent_tokens_bind_the_reviewed_package() {
    let root = TempDir::new("consent");
    let installer = installer(&root);
    let path = demo_package(
        &root,
        "demo",
        "1.0.0",
        serde_json::json!({"permissions": ["audio.output"]}),
    );
    let prepared = installer
        .prepare(
            &path,
            PackageOrigin::File,
            SignaturePolicy::AllowWithConsent,
        )
        .unwrap();
    let preview = installer.preview(&prepared);
    assert!(preview.requires_sideload_consent);
    assert_eq!(preview.permission_diff.added_permissions, ["audio.output"]);

    let stale = installer.commit(
        &prepared,
        &CommitOptions {
            consent: Consent::Token("stale".to_owned()),
            policy: None,
        },
        &NoopHooks,
        &NoopReporter,
    );
    let error = stale.unwrap_err();
    assert_eq!(error.code, InstallErrorCode::ConsentRequired);
    assert!(error.details.is_some());

    installer
        .commit(
            &prepared,
            &CommitOptions {
                consent: Consent::Token(preview.consent_token.clone()),
                policy: None,
            },
            &NoopHooks,
            &NoopReporter,
        )
        .unwrap();
    let reinstall = installer.preview(&prepared);
    assert_eq!(reinstall.change, ChangeKind::Reinstall);
    assert_ne!(reinstall.consent_token, preview.consent_token);
}

#[test]
fn incompatible_packages_are_rejected_before_activation() {
    let root = TempDir::new("incompatible");
    let installer = installer(&root);
    let path = demo_package(
        &root,
        "demo",
        "1.0.0",
        serde_json::json!({"engines": {"tiktools": ">=9.0.0"}}),
    );
    let prepared = installer
        .prepare(
            &path,
            PackageOrigin::File,
            SignaturePolicy::AllowWithConsent,
        )
        .unwrap();
    assert!(installer.preview(&prepared).incompatibility.is_some());
    let error = installer
        .commit(&prepared, &implicit(false), &NoopHooks, &NoopReporter)
        .unwrap_err();
    assert_eq!(error.code, InstallErrorCode::Incompatible);
    assert!(!root.join("plugins/demo").exists());
}

#[test]
fn rollback_restores_previous_version_and_pins_it() {
    let root = TempDir::new("rollback");
    let installer = installer(&root);
    let hooks = RecordingHooks::default();
    for version in ["1.0.0", "1.1.0"] {
        let prepared = installer
            .prepare(
                &demo_package(&root, "demo", version, serde_json::json!({})),
                PackageOrigin::File,
                SignaturePolicy::AllowWithConsent,
            )
            .unwrap();
        installer
            .commit(&prepared, &implicit(true), &hooks, &NoopReporter)
            .unwrap();
    }
    let outcome = installer
        .rollback("demo", None, &hooks, &NoopReporter)
        .unwrap();
    assert_eq!(outcome.receipt.version, "1.0.0");
    assert!(outcome.receipt.policy.pinned);
    assert_eq!(outcome.change, ChangeKind::Downgrade);
    assert!(fs::read_to_string(root.join("plugins/demo/plugin.json"))
        .unwrap()
        .contains("1.0.0"));
    let versions = installer.versions("demo").unwrap();
    assert_eq!(
        versions
            .retained
            .iter()
            .map(|entry| entry.version.as_str())
            .collect::<Vec<_>>(),
        ["1.1.0"]
    );
    let forward = installer
        .rollback("demo", Some("1.1.0"), &hooks, &NoopReporter)
        .unwrap();
    assert_eq!(forward.receipt.version, "1.1.0");
    assert!(hooks
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| call == "activate:demo:true"));
    assert_eq!(
        installer
            .rollback("demo", Some("9.9.9"), &hooks, &NoopReporter)
            .unwrap_err()
            .code,
        InstallErrorCode::NotFound
    );
}

#[test]
fn retention_prunes_the_oldest_versions() {
    let root = TempDir::new("retention");
    let installer = installer(&root);
    for version in ["1.0.0", "1.1.0", "1.2.0", "1.3.0"] {
        install_file(
            &installer,
            &demo_package(&root, "demo", version, serde_json::json!({})),
            true,
        )
        .unwrap();
    }
    let versions = installer.versions("demo").unwrap();
    let retained: Vec<_> = versions
        .retained
        .iter()
        .map(|entry| entry.version.as_str())
        .collect();
    assert_eq!(retained, ["1.1.0", "1.2.0"]);
    assert!(!root.join("store/versions/demo/1.0.0").exists());
}

#[test]
fn activation_failures_restore_the_previous_package() {
    let root = TempDir::new("activation");
    let installer = installer(&root);
    install_file(
        &installer,
        &demo_package(&root, "demo", "1.0.0", serde_json::json!({})),
        false,
    )
    .unwrap();
    let hooks = RecordingHooks::default();
    hooks.fail_activation.store(true, Ordering::Release);
    let prepared = installer
        .prepare(
            &demo_package(&root, "demo", "2.0.0", serde_json::json!({})),
            PackageOrigin::File,
            SignaturePolicy::AllowWithConsent,
        )
        .unwrap();
    let error = installer
        .commit(&prepared, &implicit(true), &hooks, &NoopReporter)
        .unwrap_err();
    assert_eq!(error.code, InstallErrorCode::IoError);
    assert!(fs::read_to_string(root.join("plugins/demo/plugin.json"))
        .unwrap()
        .contains("1.0.0"));
    assert_eq!(installer.receipt("demo").unwrap().unwrap().version, "1.0.0");
}

#[test]
fn uninstall_removes_package_receipt_versions_and_optionally_data() {
    let root = TempDir::new("uninstall");
    let installer = installer(&root);
    for version in ["1.0.0", "2.0.0"] {
        install_file(
            &installer,
            &demo_package(&root, "demo", version, serde_json::json!({})),
            true,
        )
        .unwrap();
    }
    let data = root.join("plugin-data/demo");
    fs::create_dir_all(&data).unwrap();
    installer
        .uninstall("demo", Some(&data), &NoopHooks)
        .unwrap();
    assert!(!root.join("plugins/demo").exists());
    assert!(installer.receipt("demo").unwrap().is_none());
    assert!(!root.join("store/versions/demo").exists());
    assert!(!data.exists());
    assert_eq!(
        installer
            .uninstall("demo", None, &NoopHooks)
            .unwrap_err()
            .code,
        InstallErrorCode::NotInstalled
    );
}

#[test]
fn adoption_creates_receipts_for_unmanaged_packages() {
    let root = TempDir::new("adopt");
    let installer = installer(&root);
    fs::create_dir_all(root.join("plugins/legacy")).unwrap();
    fs::write(
        root.join("plugins/legacy/plugin.json"),
        manifest(
            "legacy",
            "0.9.0",
            serde_json::json!({"permissions": ["http"]}),
        ),
    )
    .unwrap();
    let adopted = installer.adopt(["legacy", "missing"]).unwrap();
    assert_eq!(adopted.len(), 1);
    assert_eq!(adopted[0].origin, PackageOrigin::Adopted);
    assert_eq!(adopted[0].granted_permissions, ["http"]);
    assert!(installer.adopt(["legacy"]).unwrap().is_empty());
}

#[test]
fn concurrent_operations_on_one_plugin_are_busy() {
    let root = TempDir::new("busy");
    let installer = installer(&root);
    install_file(
        &installer,
        &demo_package(&root, "demo", "1.0.0", serde_json::json!({})),
        false,
    )
    .unwrap();
    struct ReentrantHooks<'a>(&'a Installer);
    impl ActivationHooks for ReentrantHooks<'_> {
        fn deactivate(&self, plugin_id: &str) -> Result<bool, String> {
            let error = self.0.uninstall(plugin_id, None, &NoopHooks).unwrap_err();
            assert_eq!(error.code, InstallErrorCode::Busy);
            Ok(false)
        }
        fn activate(&self, _plugin_id: &str, _was_running: bool) -> Result<(), String> {
            Ok(())
        }
    }
    let prepared = installer
        .prepare(
            &demo_package(&root, "demo", "1.1.0", serde_json::json!({})),
            PackageOrigin::File,
            SignaturePolicy::AllowWithConsent,
        )
        .unwrap();
    installer
        .commit(
            &prepared,
            &implicit(true),
            &ReentrantHooks(&installer),
            &NoopReporter,
        )
        .unwrap();
}

fn publisher_key() -> SigningKey {
    SigningKey::generate(KeyId::parse("acme-1").unwrap(), Some("acme".to_owned())).unwrap()
}

fn signed_package(root: &TempDir, key: &SigningKey, id: &str, version: &str) -> PathBuf {
    let source = root.join(&format!("src-{id}-{version}"));
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("plugin.json"),
        manifest(
            id,
            version,
            serde_json::json!({"publisher": "acme", "permissions": ["audio.output"]}),
        ),
    )
    .unwrap();
    fs::write(source.join("demo-host"), format!("binary {version}")).unwrap();
    let output = pack(&PackRequest {
        manifest: source.join("plugin.json"),
        entry: Some(source.join("demo-host")),
        output: root.join(&format!("signed/{id}-{version}.plugin")),
        target: None,
    })
    .unwrap();
    sign_archive(&output.path, key).unwrap();
    output.path
}

#[test]
fn signature_policies_distinguish_verified_untrusted_and_unsigned() {
    let root = TempDir::new("signatures");
    let key = publisher_key();
    let signed = signed_package(&root, &key, "acme.demo", "1.0.0");
    let unsigned = demo_package(&root, "acme.other", "1.0.0", serde_json::json!({}));

    let untrusted = installer(&root);
    let prepared = untrusted
        .prepare(
            &signed,
            PackageOrigin::File,
            SignaturePolicy::AllowWithConsent,
        )
        .unwrap();
    assert_eq!(prepared.signature.status, SignatureStatus::Untrusted);
    assert!(untrusted.preview(&prepared).requires_sideload_consent);
    assert_eq!(
        untrusted
            .prepare(&signed, PackageOrigin::File, SignaturePolicy::Require)
            .unwrap_err()
            .code,
        InstallErrorCode::UntrustedPublisher
    );
    assert_eq!(
        untrusted
            .prepare(&unsigned, PackageOrigin::File, SignaturePolicy::Require)
            .unwrap_err()
            .code,
        InstallErrorCode::SignatureRequired
    );

    let mut store = TrustStore::new();
    store.insert(
        TrustedKey::new(key.public_key(), KeyRole::Publisher).publisher("acme"),
        "local",
    );
    let trusted_root = TempDir::new("signatures-trusted");
    let trusted = installer_with(&trusted_root, Arc::new(RwLock::new(store)));
    let prepared = trusted
        .prepare(&signed, PackageOrigin::File, SignaturePolicy::Require)
        .unwrap();
    assert_eq!(prepared.signature.status, SignatureStatus::Verified);
    assert_eq!(prepared.signature.publisher.as_deref(), Some("acme"));
    let preview = trusted.preview(&prepared);
    assert!(!preview.requires_sideload_consent);
    trusted
        .commit(
            &prepared,
            &CommitOptions {
                consent: Consent::Token(preview.consent_token),
                policy: None,
            },
            &NoopHooks,
            &NoopReporter,
        )
        .unwrap();
    assert!(trusted
        .receipt("acme.demo")
        .unwrap()
        .unwrap()
        .signature
        .is_verified());
}

#[test]
fn package_specs_parse_every_form() {
    assert_eq!(
        "acme.tts".parse::<PackageSpec>().unwrap(),
        PackageSpec::Registry {
            registry: None,
            id: "acme.tts".to_owned(),
            requirement: None
        }
    );
    let exact: PackageSpec = "official:acme.tts@1.2.0".parse().unwrap();
    assert!(exact.exact());
    assert_eq!(exact.to_string(), "official:acme.tts@=1.2.0");
    let range: PackageSpec = "acme.tts@^1.2".parse().unwrap();
    assert!(!range.exact());
    assert!(matches!(
        "./dist/acme.plugin".parse::<PackageSpec>().unwrap(),
        PackageSpec::File(_)
    ));
    assert!(matches!(
        r"C:\plugins\acme.plugin".parse::<PackageSpec>().unwrap(),
        PackageSpec::File(_)
    ));
    assert!(matches!(
        "file:acme.zip".parse::<PackageSpec>().unwrap(),
        PackageSpec::File(_)
    ));
    assert!(matches!(
        "https://cdn.example/acme.plugin"
            .parse::<PackageSpec>()
            .unwrap(),
        PackageSpec::Url(_)
    ));
    assert!("http://cdn.example/acme.plugin"
        .parse::<PackageSpec>()
        .is_err());
    assert!("Bad Id".parse::<PackageSpec>().is_err());
    assert!("acme.tts@not-a-range!".parse::<PackageSpec>().is_err());
    let input: PackageSpec = PackageSpecInput::Registry {
        id: "acme.tts".to_owned(),
        version: Some("latest".to_owned()),
        registry: None,
    }
    .try_into()
    .unwrap();
    assert_eq!(input.plugin_id(), Some("acme.tts"));
}

#[test]
fn jobs_track_progress_cancellation_and_listeners() {
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    let jobs = JobRegistry::new(Some(Arc::new(move |_snapshot: &crate::JobSnapshot| {
        counter.fetch_add(1, Ordering::AcqRel);
    })));
    let job = jobs.start(JobKind::Install, Some("demo".to_owned()), None);
    job.stage(JobStage::Download);
    job.progress(1, Some(10));
    job.progress(2, Some(10));
    job.progress(10, Some(10));
    assert_eq!(jobs.get(job.id()).unwrap().progress.unwrap().done, 10);
    assert!(jobs.active_for("demo").is_some());
    assert!(jobs.cancel(job.id()));
    assert!(job.check().is_err());
    job.finish::<()>(&Err(crate::InstallError::cancelled()));
    let snapshot = jobs.get(job.id()).unwrap();
    assert_eq!(snapshot.status, JobStatus::Cancelled);
    assert!(!jobs.cancel(job.id()));
    assert!(jobs.active_for("demo").is_none());
    assert!(seen.load(Ordering::Acquire) >= 5);
}

struct StaticServer {
    base: String,
    _task: tokio::task::JoinHandle<()>,
}

impl StaticServer {
    async fn start(root: PathBuf) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let root = root.clone();
                tokio::spawn(async move {
                    let mut buffer = vec![0_u8; 8192];
                    let Ok(read) = socket.read(&mut buffer).await else {
                        return;
                    };
                    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .trim_start_matches('/')
                        .to_owned();
                    let (status, body) = match fs::read(root.join(&path)) {
                        Ok(body) => ("200 OK", body),
                        Err(_) => ("404 Not Found", b"missing".to_vec()),
                    };
                    let head = format!(
                        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                });
            }
        });
        Self { base, _task: task }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_loop_installs_by_id_and_reports_updates() {
    let root = TempDir::new("loop");
    let root_key = SigningKey::generate(KeyId::parse("root-1").unwrap(), None).unwrap();
    let index_key = SigningKey::generate(KeyId::parse("index-1").unwrap(), None).unwrap();
    let publisher = publisher_key();
    let writer = IndexWriter::open(
        root.join("registry"),
        "test",
        Some(SigningKey::from_json(&index_key.to_json()).unwrap()),
    );
    writer
        .init(
            &root_key,
            vec![
                TrustedKey::new(index_key.public_key(), KeyRole::Index),
                TrustedKey::new(publisher.public_key(), KeyRole::Publisher).publisher("acme"),
            ],
            None,
            None,
        )
        .unwrap();
    writer
        .add(
            &signed_package(&root, &publisher, "acme.demo", "1.0.0"),
            "stable",
            None,
        )
        .unwrap();
    let server = StaticServer::start(root.join("registry")).await;

    let trust = Arc::new(RwLock::new(TrustStore::new()));
    let mut source = RegistrySource::new("test", &server.base).unwrap();
    source.root_keys = vec![root_key.public_key()];
    let registry = Arc::new(
        RegistryClient::new(
            root.join("store/registries"),
            root.join("store/registries.json"),
            vec![source],
            trust.clone(),
        )
        .unwrap(),
    );
    let sources = PackageSources::new(
        registry.clone(),
        Downloader::new(root.join("store/cache")).unwrap(),
    );
    let installer = installer_with(&root, trust);
    let jobs = JobRegistry::new(None);
    let job = jobs.start(JobKind::Install, None, Some("acme.demo".to_owned()));

    let spec: PackageSpec = "acme.demo".parse().unwrap();
    let resolved = sources
        .fetch(
            &spec,
            "stable",
            installer.host(),
            &job,
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
    assert_eq!(resolved.policy, SignaturePolicy::Require);
    let prepared = installer
        .prepare(&resolved.path, resolved.origin.clone(), resolved.policy)
        .unwrap();
    resolved
        .expected
        .as_ref()
        .unwrap()
        .check(&prepared)
        .unwrap();
    let preview = installer.preview(&prepared);
    assert_eq!(preview.signature.status, SignatureStatus::Verified);
    let outcome = installer
        .commit(
            &prepared,
            &CommitOptions {
                consent: Consent::Token(preview.consent_token),
                policy: None,
            },
            &NoopHooks,
            &job,
        )
        .unwrap();
    job.finish(&Ok(()));
    assert_eq!(jobs.get(job.id()).unwrap().status, JobStatus::Succeeded);
    assert!(
        matches!(outcome.receipt.origin, PackageOrigin::Registry { ref registry, .. } if registry == "test")
    );

    let none = check_updates(
        &installer.receipts().list().unwrap(),
        &registry,
        installer.host(),
    )
    .await;
    assert!(none.updates.is_empty(), "{none:?}");

    writer
        .add(
            &signed_package(&root, &publisher, "acme.demo", "1.0.1"),
            "stable",
            None,
        )
        .unwrap();
    writer
        .advise(
            "acme.demo",
            "1.0.0",
            Some(Advisory {
                severity: AdvisorySeverity::Critical,
                message: "fixed in 1.0.1".to_owned(),
                url: None,
            }),
        )
        .unwrap();
    registry.refresh("test").await.unwrap();
    let report = check_updates(
        &installer.receipts().list().unwrap(),
        &registry,
        installer.host(),
    )
    .await;
    assert_eq!(report.updates.len(), 1, "{report:?}");
    let update = &report.updates[0];
    assert_eq!(update.latest.as_deref(), Some("1.0.1"));
    assert_eq!(update.kind, Some(UpdateKind::Patch));
    assert!(update.advisory.is_some());
    assert!(!update.auto_applicable);

    let missing: PackageSpec = "acme.missing".parse().unwrap();
    let error = sources
        .fetch(
            &missing,
            "stable",
            installer.host(),
            &job,
            &AtomicBool::new(false),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, InstallErrorCode::NotFound);

    let incompatible: PackageSpec = "acme.demo@>=5".parse().unwrap();
    let error = sources
        .fetch(
            &incompatible,
            "stable",
            installer.host(),
            &job,
            &AtomicBool::new(false),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, InstallErrorCode::NoCompatibleRelease);
}

#[cfg(feature = "native-providers")]
mod native_backfill {
    use super::*;
    use tiktools_plugin_api::manifest::current_napi_target;

    struct NpmFixture {
        base: String,
        tarball: Vec<u8>,
        routes: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
        _task: tokio::task::JoinHandle<()>,
    }

    impl NpmFixture {
        async fn start(host_node: &str, host_bytes: &[u8]) -> Self {
            let foreign_node = if host_node.ends_with("win32-x64-msvc.node") {
                "package/node-stem.linux-x64-gnu.node"
            } else {
                "package/node-stem.win32-x64-msvc.node"
            };
            let tarball = Self::tarball(host_node, host_bytes, foreign_node);
            let routes: Arc<Mutex<BTreeMap<String, Vec<u8>>>> = Arc::default();
            let serving = routes.clone();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let task = tokio::spawn(async move {
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else {
                        break;
                    };
                    let routes = serving.clone();
                    tokio::spawn(async move {
                        let mut buffer = vec![0u8; 8192];
                        let Ok(read) = socket.read(&mut buffer).await else {
                            return;
                        };
                        let request = String::from_utf8_lossy(&buffer[..read]);
                        let path = request
                            .lines()
                            .next()
                            .and_then(|line| line.split_whitespace().nth(1))
                            .unwrap_or("/");
                        let body = routes
                            .lock()
                            .ok()
                            .and_then(|routes| routes.get(path).cloned());
                        let (status, body) = body.map_or_else(
                            || ("404 Not Found", b"not found".to_vec()),
                            |body| ("200 OK", body),
                        );
                        let header = format!(
                            "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = socket.write_all(header.as_bytes()).await;
                        let _ = socket.write_all(&body).await;
                    });
                }
            });
            Self {
                base,
                tarball,
                routes,
                _task: task,
            }
        }

        fn tarball(host_node: &str, host_bytes: &[u8], foreign_node: &str) -> Vec<u8> {
            let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            for (path, bytes) in [
                (
                    "package/package.json",
                    br#"{"name":"fixture-pkg","version":"1.2.3","main":"index.js","files":["index.js","*.node"]}"#.as_slice(),
                ),
                ("package/index.js", b"module.exports = {};".as_slice()),
                (host_node, host_bytes),
                (foreign_node, b"foreign-bytes".as_slice()),
            ] {
                let mut header = tar::Header::new_gnu();
                header.set_entry_type(tar::EntryType::Regular);
                header.set_size(bytes.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, path, bytes).unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap()
        }

        async fn pin_lockfile(&self) -> String {
            use base64::Engine;
            use tiktools_plugin_api::{NativeLibDeclaration, NativeLibProvider};
            use tiktools_plugin_sdk::native_providers::{
                pin_native_lib, provider_client, NativeLibsLockfile, ProviderEndpoints,
                LOCKFILE_VERSION,
            };
            let integrity = format!(
                "sha512-{}",
                base64::engine::general_purpose::STANDARD
                    .encode(sha2::Sha512::digest(&self.tarball))
            );
            self.routes.lock().unwrap().extend([
                (
                    "/fixture-pkg/1.2.3".to_owned(),
                    serde_json::to_vec(&serde_json::json!({
                        "name": "fixture-pkg",
                        "version": "1.2.3",
                        "dist": {"tarball": format!("{}/t.tgz", self.base), "integrity": integrity},
                    }))
                    .unwrap(),
                ),
                ("/t.tgz".to_owned(), self.tarball.clone()),
            ]);
            let endpoints = ProviderEndpoints {
                npm_registry: self.base.clone(),
                github_base: self.base.clone(),
            };
            let declaration = NativeLibDeclaration {
                package: "fixture-pkg".to_owned(),
                version: "1.2.3".to_owned(),
                provider: NativeLibProvider::Npm,
                repo: None,
                tag: None,
                binary: None,
            };
            let locked = pin_native_lib(&provider_client().unwrap(), &endpoints, &declaration)
                .await
                .unwrap();
            serde_json::to_string_pretty(&NativeLibsLockfile {
                version: LOCKFILE_VERSION,
                packages: BTreeMap::from([("fixture-pkg".to_owned(), locked)]),
            })
            .unwrap()
        }
    }

    fn backfill_manifest() -> Vec<u8> {
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": 3,
            "id": "backfill",
            "name": "Backfill",
            "version": "1.0.0",
            "runtime": "napi-vm",
            "entry": "dist/index.js",
            "nativeAddons": [{"package": "fixture-pkg", "root": "node_modules/fixture-pkg"}],
            "nativeLibs": [{"package": "fixture-pkg", "version": "1.2.3"}],
        }))
        .unwrap()
    }

    const PACKAGE_JSON: &[u8] =
        br#"{"name":"fixture-pkg","version":"1.2.3","main":"index.js","files":["index.js","*.node"]}"#;

    fn installer_with_endpoints(root: &TempDir, endpoint: &str) -> Installer {
        Installer::new(
            InstallerConfig {
                plugins_dir: root.join("plugins"),
                store_dir: root.join("store"),
                host: host(),
                retain_versions: 2,
                provider_endpoints: Some((endpoint.to_owned(), endpoint.to_owned())),
            },
            Arc::default(),
        )
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn installer_backfills_missing_host_binary_from_lockfile() {
        let host_node = format!("package/node-stem.{}.node", current_napi_target());
        let fixture = NpmFixture::start(&host_node, b"host-bytes").await;
        let lockfile = fixture.pin_lockfile().await;
        let root = TempDir::new("backfill");
        let archive = root.join("backfill.plugin");
        let manifest = backfill_manifest();
        write_archive(
            &archive,
            &[
                ("plugin.json", &manifest),
                ("dist/index.js", b"export default {};"),
                ("node_modules/fixture-pkg/package.json", PACKAGE_JSON),
                ("node_modules/fixture-pkg/index.js", b"module.exports = {};"),
                ("native-libs.lock.json", lockfile.as_bytes()),
            ],
            true,
        );
        let installer = installer_with_endpoints(&root, &fixture.base);
        let result = install_file(&installer, &archive, false);
        if cfg!(target_os = "android") {
            assert!(result.is_err());
            return;
        }
        result.unwrap();
        let tree = root.join("plugins/backfill/node_modules/fixture-pkg");
        assert_eq!(
            fs::read(tree.join(format!("node-stem.{}.node", current_napi_target()))).unwrap(),
            b"host-bytes"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn installer_skips_fetch_for_complete_trees() {
        let host_node = format!("package/node-stem.{}.node", current_napi_target());
        let fixture = NpmFixture::start(&host_node, b"host-bytes").await;
        let lockfile = fixture.pin_lockfile().await;
        let root = TempDir::new("complete");
        let archive = root.join("complete.plugin");
        let manifest = backfill_manifest();
        let host_name = format!(
            "node_modules/fixture-pkg/node-stem.{}.node",
            current_napi_target()
        );
        write_archive(
            &archive,
            &[
                ("plugin.json", &manifest),
                ("dist/index.js", b"export default {};"),
                ("node_modules/fixture-pkg/package.json", PACKAGE_JSON),
                ("node_modules/fixture-pkg/index.js", b"module.exports = {};"),
                (host_name.as_str(), b"host-bytes"),
                ("native-libs.lock.json", lockfile.as_bytes()),
            ],
            true,
        );
        let installer = installer_with_endpoints(&root, "http://127.0.0.1:9");
        install_file(&installer, &archive, false).unwrap();
    }

    #[test]
    fn installer_rejects_missing_binary_without_lockfile() {
        let root = TempDir::new("nolock");
        let archive = root.join("nolock.plugin");
        let manifest = backfill_manifest();
        write_archive(
            &archive,
            &[
                ("plugin.json", &manifest),
                ("dist/index.js", b"export default {};"),
                ("node_modules/fixture-pkg/package.json", PACKAGE_JSON),
            ],
            true,
        );
        let installer = installer_with_endpoints(&root, "http://127.0.0.1:9");
        let error = install_file(&installer, &archive, false).unwrap_err();
        assert!(error.message.contains("native-libs.lock.json"), "{error}");
        assert!(!root.join("plugins/backfill").exists());
    }
}

#[test]
fn installer_retains_bundled_native_tree() {
    let root = TempDir::new("native-tree");
    let manifest = serde_json::to_vec(&serde_json::json!({
        "schemaVersion": 3,
        "id": "native-installed",
        "name": "NativeInstalled",
        "version": "1.0.0",
        "runtime": "napi-vm",
        "entry": "dist/index.js",
        "trust": "trusted",
        "nativeAddons": [{"package": "rdev-node", "root": "node_modules/rdev-node"}],
    }))
    .unwrap();
    let archive = root.join("native-installed.plugin");
    write_archive(
        &archive,
        &[
            ("plugin.json", &manifest),
            ("dist/index.js", b"export default {};"),
            (
                "node_modules/rdev-node/package.json",
                br#"{"name":"rdev-node","version":"1.0.0"}"#,
            ),
            (
                "node_modules/rdev-node/node-rdev.linux-x64-gnu.node",
                b"fake-linux",
            ),
            (
                "node_modules/rdev-node/node-rdev.win32-x64-msvc.node",
                b"fake-win32",
            ),
        ],
        true,
    );
    let outcome = install_file(&installer(&root), &archive, false).unwrap();
    assert_eq!(outcome.receipt.runtime, "napi-vm");
    let tree = root.join("plugins/native-installed/node_modules/rdev-node");
    assert_eq!(
        fs::read(tree.join("node-rdev.linux-x64-gnu.node")).unwrap(),
        b"fake-linux"
    );
    assert!(tree.join("node-rdev.win32-x64-msvc.node").is_file());
    assert!(tree.join("package.json").is_file());
}
