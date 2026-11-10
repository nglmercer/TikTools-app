use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tiktools_plugin_api::{HostCompat, PluginRuntimeKind};
use tiktools_plugin_package::{
    pack, sign_archive, KeyId, KeyRole, PackRequest, PackageArchive, Sha256Digest, SigningKey,
    TrustStore, TrustedKey,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    join_url, latest_compatible, resolve, shard_path, Advisory, AdvisorySeverity, DownloadRequest,
    Downloader, IndexWriter, PackageDocument, RegistriesConfig, RegistryClient, RegistryError,
    RegistrySource, RejectionReason, ResolveRequest, SearchQuery,
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
            "tiktools-registry-{name}-{}-{suffix}-{}",
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

struct StaticServer {
    base: String,
    requests: Arc<AtomicUsize>,
    _task: tokio::task::JoinHandle<()>,
}

impl StaticServer {
    async fn start(root: PathBuf) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let root = root.clone();
                let counter = counter.clone();
                tokio::spawn(async move {
                    let mut buffer = vec![0_u8; 16 * 1024];
                    let Ok(read) = socket.read(&mut buffer).await else {
                        return;
                    };
                    counter.fetch_add(1, Ordering::AcqRel);
                    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .split('?')
                        .next()
                        .unwrap_or("/")
                        .trim_start_matches('/')
                        .to_owned();
                    let header = |name: &str| {
                        request.lines().find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.trim()
                                .eq_ignore_ascii_case(name)
                                .then(|| value.trim().to_owned())
                        })
                    };
                    let response = match fs::read(root.join(&path)) {
                        Ok(body) => {
                            let etag = format!("\"{}\"", Sha256Digest::of_bytes(&body));
                            if header("if-none-match").as_deref() == Some(etag.as_str()) {
                                (304, "Not Modified", Vec::new(), etag, None)
                            } else if let Some(start) = header("range")
                                .and_then(|range| {
                                    range.strip_prefix("bytes=").map(ToOwned::to_owned)
                                })
                                .and_then(|range| range.trim_end_matches('-').parse::<usize>().ok())
                                .filter(|start| *start < body.len())
                            {
                                let total = body.len();
                                (
                                    206,
                                    "Partial Content",
                                    body[start..].to_vec(),
                                    etag,
                                    Some(format!("bytes {start}-{}/{total}", total - 1)),
                                )
                            } else {
                                (200, "OK", body, etag, None)
                            }
                        }
                        Err(_) => (404, "Not Found", b"missing".to_vec(), String::new(), None),
                    };
                    let (status, reason, body, etag, range) = response;
                    let mut head = format!(
                        "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n",
                        body.len()
                    );
                    if !etag.is_empty() {
                        head.push_str(&format!("etag: {etag}\r\naccept-ranges: bytes\r\n"));
                    }
                    if let Some(range) = range {
                        head.push_str(&format!("content-range: {range}\r\n"));
                    }
                    head.push_str("\r\n");
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                });
            }
        });
        Self {
            base,
            requests,
            _task: task,
        }
    }
}

struct Keys {
    root: SigningKey,
    index: SigningKey,
    publisher: SigningKey,
}

fn keys() -> Keys {
    Keys {
        root: SigningKey::generate(KeyId::parse("root-1").unwrap(), None).unwrap(),
        index: SigningKey::generate(KeyId::parse("index-1").unwrap(), None).unwrap(),
        publisher: SigningKey::generate(KeyId::parse("acme-1").unwrap(), Some("acme".to_owned()))
            .unwrap(),
    }
}

#[tokio::test]
async fn fixed_client_ignores_legacy_sources_and_persisted_trust_controls() {
    let directory = TempDir::new("fixed-sources");
    let cloud = StaticServer::start(directory.join("cloud")).await;
    let foreign = StaticServer::start(directory.join("foreign")).await;
    let trusted = keys();
    let impostor = keys();
    let builtin =
        crate::cloud_source("official", &cloud.base, vec![trusted.root.public_key()]).unwrap();
    let mut spoofed =
        crate::cloud_source("official", &foreign.base, vec![impostor.root.public_key()]).unwrap();
    spoofed.enabled = false;
    spoofed.priority = 999;
    spoofed.scopes = vec!["evil.*".into()];
    let custom = RegistrySource::new("custom", &foreign.base).unwrap();
    let path = directory.join("registries.json");
    let mut stored = RegistriesConfig {
        registries: vec![spoofed, custom],
        ..Default::default()
    };
    stored.defaults.allow_sideload = false;
    stored.save(&path).unwrap();
    let client = RegistryClient::new_fixed(
        directory.join("cache"),
        &path,
        vec![builtin.clone()],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    let effective = client.config();
    assert_eq!(effective.registries, vec![builtin]);
    assert!(!effective.defaults.allow_sideload);
    let trust = client.trust();
    {
        let pinned = trust.read().unwrap();
        let root = pinned.get(&KeyId::parse("root-1").unwrap()).unwrap();
        assert_eq!(root.key, trusted.root.public_key());
    }
    assert!(matches!(
        client.add(RegistrySource::new("new", &foreign.base).unwrap()),
        Err(RegistryError::FixedSources)
    ));
    assert!(matches!(
        client.remove("custom"),
        Err(RegistryError::FixedSources)
    ));
    assert!(matches!(
        client.set_enabled("official", false),
        Err(RegistryError::FixedSources)
    ));
    assert!(matches!(
        client.fetch_root(&foreign.base).await,
        Err(RegistryError::FixedSources)
    ));
    assert!(matches!(
        client.package("acme.plugin", Some("custom")).await,
        Err(RegistryError::UnknownRegistry(_))
    ));
    assert_eq!(foreign.requests.load(Ordering::Acquire), 0);
    let mut defaults = client.defaults();
    defaults.channel = "beta".into();
    client.set_defaults(defaults).unwrap();
    let persisted = RegistriesConfig::load(&path).unwrap();
    assert!(persisted
        .registries
        .iter()
        .any(|source| source.name == "custom"));
    assert_eq!(persisted.defaults.channel, "beta");
    let unpinned = RegistryClient::new_fixed(
        directory.join("empty-cache"),
        &path,
        Vec::new(),
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    assert!(unpinned.config().registries.is_empty());
    assert!(unpinned.trust().read().unwrap().is_empty());
}

#[tokio::test]
async fn fixed_client_rejects_untrusted_generation_and_stale_network_fallback() {
    let directory = TempDir::new("fixed-generation");
    let cloud = StaticServer::start(directory.join("cloud")).await;
    let builtin =
        crate::cloud_source("official", &cloud.base, vec![keys().root.public_key()]).unwrap();
    let mut stored_source = builtin.clone();
    stored_source.url = format!("{}/api/v1/registry/generations/../other/files", cloud.base);
    let mut stored = RegistriesConfig::default();
    stored.registries.push(stored_source);
    let path = directory.join("registries.json");
    stored.save(&path).unwrap();
    let client = RegistryClient::new_fixed(
        directory.join("cache"),
        &path,
        vec![builtin.clone()],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    assert_eq!(client.config().registries[0].url, cloud.base);

    let mut generation = builtin.clone();
    generation.url = crate::generation_files_url(&cloud.base, 7).unwrap();
    let mut stored = RegistriesConfig::default();
    stored.registries.push(generation.clone());
    stored.save(&path).unwrap();
    let client = RegistryClient::new_fixed(
        directory.join("cache2"),
        &path,
        vec![builtin],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    // The current-generation endpoint is missing; a fixed product instance
    // must not silently fall back to a previously cached generation.
    assert!(client.refresh("official").await.is_err());
}

#[tokio::test]
async fn fixed_client_does_not_follow_cloud_redirects_to_another_origin() {
    let directory = TempDir::new("fixed-redirect");
    let foreign = StaticServer::start(directory.join("foreign")).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let destination = foreign.base.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 2048];
        let _ = socket.read(&mut request).await;
        let response = format!("HTTP/1.1 302 Found\r\nLocation: {destination}/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    let builtin = crate::cloud_source("official", &origin, vec![keys().root.public_key()]).unwrap();
    let client = RegistryClient::new_fixed(
        directory.join("cache"),
        directory.join("registries.json"),
        vec![builtin],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    assert!(client.refresh("official").await.is_err());
    server.await.unwrap();
    assert_eq!(foreign.requests.load(Ordering::Acquire), 0);
}

fn host() -> HostCompat {
    HostCompat::new(semver::Version::new(0, 4, 0)).with_target("linux-x64-gnu")
}

fn package(
    directory: &TempDir,
    id: &str,
    version: &str,
    extra: serde_json::Value,
    key: Option<&SigningKey>,
) -> PathBuf {
    let source = directory.join(&format!("src-{id}-{version}"));
    fs::create_dir_all(&source).unwrap();
    let mut manifest = serde_json::json!({
        "schemaVersion": 3,
        "id": id,
        "name": "Acme TTS",
        "version": version,
        "runtime": "process",
        "entry": "tts",
        "publisher": "acme",
        "license": "MIT",
        "categories": ["audio"],
        "tags": ["tts", "voice"],
        "description": "Reads chat aloud",
        "permissions": ["audio.output"],
    });
    manifest
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    fs::write(
        source.join("plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(source.join("tts"), format!("binary {version}")).unwrap();
    let output = pack(&PackRequest {
        manifest: source.join("plugin.json"),
        entry: Some(source.join("tts")),
        output: directory.join(&format!("out/{id}-{version}.plugin")),
        target: None,
    })
    .unwrap();
    if let Some(key) = key {
        sign_archive(&output.path, key).unwrap();
    }
    output.path
}

fn signed_registry(directory: &TempDir, keys: &Keys) -> IndexWriter {
    let writer = IndexWriter::open(
        directory.join("registry"),
        "test",
        Some(SigningKey::from_json(&keys.index.to_json()).unwrap()),
    );
    writer
        .init(
            &keys.root,
            vec![
                TrustedKey::new(keys.index.public_key(), KeyRole::Index),
                TrustedKey::new(keys.publisher.public_key(), KeyRole::Publisher).publisher("acme"),
            ],
            Some("Test registry".to_owned()),
            None,
        )
        .unwrap();
    writer
}

fn client(
    directory: &TempDir,
    base: &str,
    roots: Vec<tiktools_plugin_package::PublicKey>,
) -> RegistryClient {
    let mut source = RegistrySource::new("test", base).unwrap();
    source.root_keys = roots;
    RegistryClient::new(
        directory.join("cache"),
        directory.join("registries.json"),
        vec![source],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap()
}

#[test]
fn shards_are_stable_and_padded() {
    assert_eq!(shard_path("acme.tts"), "index/ac/me/acme.tts.json");
    assert_eq!(shard_path("ab"), "index/ab/__/ab.json");
    assert_eq!(shard_path("ab..foo"), "index/ab..foo.json");
    assert_eq!(shard_path("a..b"), "index/a./.b/a..b.json");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signed_registry_serves_canonical_parent_shard_without_changing_safe_paths() {
    let directory = TempDir::new("canonical-shard");
    let keys = keys();
    let writer = IndexWriter::open(
        directory.join("registry"),
        "test",
        Some(SigningKey::from_json(&keys.index.to_json()).unwrap()),
    );
    writer
        .init(
            &keys.root,
            vec![
                TrustedKey::new(keys.index.public_key(), KeyRole::Index),
                TrustedKey::new(keys.publisher.public_key(), KeyRole::Publisher).publisher("ab"),
            ],
            None,
            None,
        )
        .unwrap();
    let archive = package(
        &directory,
        "ab..foo",
        "1.0.0",
        serde_json::json!({"publisher":"ab"}),
        Some(&keys.publisher),
    );
    writer.add(&archive, "stable", None).unwrap();
    assert!(directory.join("registry/index/ab..foo.json").is_file());
    assert!(directory.join("registry/index/ab..foo.json.sig").is_file());
    let server = StaticServer::start(directory.join("registry")).await;
    let client = client(&directory, &server.base, vec![keys.root.public_key()]);
    assert!(client.refresh("test").await.unwrap().verified);
    let (_, document) = client.package("ab..foo", None).await.unwrap();
    assert_eq!(document.id, "ab..foo");
    assert!(document.release("1.0.0").is_some());
}

#[tokio::test]
async fn invalid_plugin_ids_are_rejected_before_registry_io() {
    let directory = TempDir::new("invalid-plugin-id");
    let writer = IndexWriter::open(directory.join("registry"), "test", None);
    assert!(writer.yank("../../outside", "1.0.0", true).is_err());
    assert!(!directory.join("registry").exists());
    let client = client(&directory, "http://127.0.0.1:9", vec![]);
    assert!(matches!(
        client.package("../../outside", None).await,
        Err(RegistryError::InvalidDocument { .. })
    ));
}

#[test]
fn urls_require_https_except_localhost() {
    assert!(join_url("https://plugins.example/v1", "config.json")
        .unwrap()
        .ends_with("/v1/config.json"));
    assert!(join_url("http://127.0.0.1:9/v1/", "index/a.json").is_ok());
    assert!(join_url("http://plugins.example/v1", "config.json").is_err());
    assert!(join_url("https://plugins.example/v1", "file:///etc/passwd").is_err());
}

#[test]
fn scoped_registries_claim_their_namespace() {
    let mut config = RegistriesConfig::default();
    let mut acme = RegistrySource::new("acme", "https://acme.example/v1").unwrap();
    acme.scopes = vec!["acme.*".to_owned()];
    acme.priority = -5;
    config.registries.push(acme);
    config
        .registries
        .push(RegistrySource::new("official", "https://official.example/v1").unwrap());
    let names = |id: &str| -> Vec<String> {
        config
            .candidates_for(id)
            .into_iter()
            .map(|source| source.name.clone())
            .collect()
    };
    assert_eq!(names("acme.tts"), ["acme"]);
    assert_eq!(names("other.tool"), ["official"]);
}

fn document(releases: serde_json::Value) -> PackageDocument {
    serde_json::from_value(serde_json::json!({
        "id": "acme.tts",
        "name": "Acme TTS",
        "releases": releases,
    }))
    .unwrap()
}

fn release(version: &str, channel: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut value = serde_json::json!({
        "version": version,
        "channel": channel,
        "runtime": "process",
        "schemaVersion": 3,
        "protocolVersion": 1,
        "artifacts": [{"target": "any", "url": "a.plugin", "size": 1, "sha256": "0".repeat(64)}],
    });
    value
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    value
}

#[test]
fn resolver_honours_channels_yanks_engines_and_targets() {
    let document = document(serde_json::json!([
        release("1.0.0", "stable", serde_json::json!({})),
        release("1.1.0", "stable", serde_json::json!({"yanked": true})),
        release("1.2.0-beta.1", "beta", serde_json::json!({})),
        release(
            "1.3.0",
            "stable",
            serde_json::json!({"engines": {"tiktools": ">=0.5.0"}})
        ),
        release(
            "1.4.0",
            "stable",
            serde_json::json!({"artifacts": [{"target": "win32-x64-msvc", "url": "w.plugin", "size": 1, "sha256": "1".repeat(64)}]})
        ),
    ]));
    let host = host();
    let stable = latest_compatible(&document, "stable", &host).unwrap();
    assert_eq!(stable.release.version, "1.0.0");
    let beta = latest_compatible(&document, "beta", &host).unwrap();
    assert_eq!(beta.release.version, "1.2.0-beta.1");

    let exact = semver::VersionReq::parse("=1.1.0").unwrap();
    let rejected = resolve(
        &document,
        &ResolveRequest {
            requirement: Some(&exact),
            ..ResolveRequest::default()
        },
        &host,
    )
    .unwrap_err();
    assert!(rejected.iter().any(
        |rejection| rejection.version == "1.1.0" && rejection.reason == RejectionReason::Yanked
    ));
    let pinned = resolve(
        &document,
        &ResolveRequest {
            requirement: Some(&exact),
            allow_yanked: true,
            ..ResolveRequest::default()
        },
        &host,
    )
    .unwrap();
    assert_eq!(pinned.release.version, "1.1.0");

    let newest = semver::VersionReq::parse(">=1.3.0").unwrap();
    let rejected = resolve(
        &document,
        &ResolveRequest {
            requirement: Some(&newest),
            ..ResolveRequest::default()
        },
        &host,
    )
    .unwrap_err();
    assert!(rejected
        .iter()
        .any(|rejection| matches!(rejection.reason, RejectionReason::Incompatible { .. })));
    assert!(rejected
        .iter()
        .any(|rejection| matches!(rejection.reason, RejectionReason::NoArtifact { .. })));

    let wasm_only = crate::tests::document(serde_json::json!([release(
        "2.0.0",
        "stable",
        serde_json::json!({"runtime": "wasm"})
    )]));
    assert!(latest_compatible(&wasm_only, "stable", &host).is_none());
    assert!(latest_compatible(
        &wasm_only,
        "stable",
        &host.with_runtime(PluginRuntimeKind::Wasm)
    )
    .is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signed_registry_round_trip_with_search_download_and_cache() {
    let directory = TempDir::new("round-trip");
    let keys = keys();
    let writer = signed_registry(&directory, &keys);
    let v1 = package(
        &directory,
        "acme.tts",
        "1.0.0",
        serde_json::json!({}),
        Some(&keys.publisher),
    );
    let v2 = package(
        &directory,
        "acme.tts",
        "1.1.0",
        serde_json::json!({}),
        Some(&keys.publisher),
    );
    writer.add(&v1, "stable", None).unwrap();
    let added = writer.add(&v2, "stable", None).unwrap();
    writer
        .advise(
            "acme.tts",
            "1.0.0",
            Some(Advisory {
                severity: AdvisorySeverity::High,
                message: "upgrade".to_owned(),
                url: None,
            }),
        )
        .unwrap();
    let server = StaticServer::start(directory.join("registry")).await;
    let client = client(&directory, &server.base, vec![keys.root.public_key()]);

    let status = client.refresh("test").await.unwrap();
    assert!(status.verified);
    assert_eq!(status.description.as_deref(), Some("Test registry"));

    let results = client
        .search(&SearchQuery {
            query: "voice".to_owned(),
            ..SearchQuery::default()
        })
        .await;
    assert_eq!(results.total, 1, "{results:?}");
    assert_eq!(results.entries[0].entry.latest, "1.1.0");
    assert_eq!(results.entries[0].registry, "test");

    let (source, document) = client.package("acme.tts", None).await.unwrap();
    assert_eq!(source.name, "test");
    assert!(document.release("1.0.0").unwrap().advisory.is_some());
    let resolved = latest_compatible(&document, "stable", &host()).unwrap();
    assert_eq!(resolved.release.version, "1.1.0");
    assert_eq!(resolved.artifact.sha256.to_string(), added.sha256);

    let downloader = Downloader::new(directory.join("downloads")).unwrap();
    let url = join_url(&source.url, &resolved.artifact.url).unwrap();
    let progress = AtomicU64::new(0);
    let report = |done: u64, _total: Option<u64>| progress.store(done, Ordering::Release);
    let downloaded = downloader
        .fetch(
            DownloadRequest {
                url: &url,
                expected_digest: Some(&resolved.artifact.sha256),
                expected_size: Some(resolved.artifact.size),
            },
            &report,
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
    assert!(!downloaded.cached);
    assert_eq!(progress.load(Ordering::Acquire), resolved.artifact.size);
    let inspection = PackageArchive::open(&downloaded.path)
        .unwrap()
        .inspect()
        .unwrap();
    let trust = client.trust();
    let verified = trust
        .read()
        .unwrap()
        .verify_package(
            inspection.signature.as_ref().unwrap(),
            inspection.checksums.as_ref().unwrap(),
            "acme.tts",
            0,
        )
        .unwrap();
    assert_eq!(verified.publisher, "acme");
    assert_eq!(verified.source, "test");

    let before = server.requests.load(Ordering::Acquire);
    let again = downloader
        .fetch(
            DownloadRequest {
                url: &url,
                expected_digest: Some(&resolved.artifact.sha256),
                expected_size: None,
            },
            &|_, _| {},
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
    assert!(again.cached);
    assert_eq!(server.requests.load(Ordering::Acquire), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tampered_documents_and_unknown_roots_are_rejected() {
    let directory = TempDir::new("tampered");
    let keys = keys();
    let writer = signed_registry(&directory, &keys);
    let package = package(
        &directory,
        "acme.tts",
        "1.0.0",
        serde_json::json!({}),
        Some(&keys.publisher),
    );
    writer.add(&package, "stable", None).unwrap();
    let document_path = directory.join("registry").join(shard_path("acme.tts"));
    let tampered = fs::read_to_string(&document_path)
        .unwrap()
        .replace("audio.output", "points.write");
    fs::write(&document_path, tampered).unwrap();
    let server = StaticServer::start(directory.join("registry")).await;

    let client_dir = TempDir::new("tampered-client");
    let verified = client(&client_dir, &server.base, vec![keys.root.public_key()]);
    assert!(matches!(
        verified.package("acme.tts", None).await,
        Err(RegistryError::Signature { .. })
    ));

    let impostor = SigningKey::generate(KeyId::parse("root-x").unwrap(), None).unwrap();
    let other_dir = TempDir::new("impostor-client");
    let wrong_root = client(&other_dir, &server.base, vec![impostor.public_key()]);
    assert!(matches!(
        wrong_root.refresh("test").await,
        Err(RegistryError::Signature { .. })
    ));

    let root = verified.fetch_root(&server.base).await.unwrap();
    assert_eq!(root.keys, vec![keys.root.public_key()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn downloads_verify_digests_resume_and_cancel() {
    let directory = TempDir::new("downloads");
    let served = directory.join("served");
    fs::create_dir_all(&served).unwrap();
    let bytes: Vec<u8> = (0..200_000_u32).map(|value| (value % 251) as u8).collect();
    fs::write(served.join("pkg.plugin"), &bytes).unwrap();
    let digest = Sha256Digest::of_bytes(&bytes);
    let server = StaticServer::start(served).await;
    let url = format!("{}/pkg.plugin", server.base);
    let downloader = Downloader::new(directory.join("cache")).unwrap();

    let wrong = Sha256Digest::of_bytes(b"other");
    assert!(matches!(
        downloader
            .fetch(
                DownloadRequest {
                    url: &url,
                    expected_digest: Some(&wrong),
                    expected_size: None,
                },
                &|_, _| {},
                &AtomicBool::new(false),
            )
            .await,
        Err(RegistryError::DigestMismatch { .. })
    ));

    let cancelled = AtomicBool::new(true);
    assert!(matches!(
        downloader
            .fetch(
                DownloadRequest {
                    url: &url,
                    expected_digest: Some(&digest),
                    expected_size: None,
                },
                &|_, _| {},
                &cancelled,
            )
            .await,
        Err(RegistryError::Cancelled)
    ));

    let partial_dir = directory.join("cache/partial");
    let partial = fs::read_dir(&partial_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(&partial, &bytes[..50_000]).unwrap();
    let resumed = downloader
        .fetch(
            DownloadRequest {
                url: &url,
                expected_digest: Some(&digest),
                expected_size: Some(bytes.len() as u64),
            },
            &|_, _| {},
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
    assert_eq!(resumed.digest, digest);
    assert_eq!(fs::read(&resumed.path).unwrap(), bytes);
    assert_eq!(downloader.prune(0).unwrap(), bytes.len() as u64);
    assert!(!Path::new(&resumed.path).exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registries_can_be_added_and_removed() {
    let directory = TempDir::new("config");
    let client = RegistryClient::new(
        directory.join("cache"),
        directory.join("registries.json"),
        Vec::new(),
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    let source = RegistrySource::new("acme", "https://plugins.acme.example/v1").unwrap();
    client.add(source.clone()).unwrap();
    assert!(matches!(
        client.add(source),
        Err(RegistryError::DuplicateRegistry(_))
    ));
    let saved = RegistriesConfig::load(&directory.join("registries.json")).unwrap();
    assert_eq!(saved.registries.len(), 1);
    client.set_enabled("acme", false).unwrap();
    assert!(client.config().ordered().is_empty());
    client.remove("acme").unwrap();
    assert!(client.statuses().is_empty());
    assert!(RegistrySource::new("Bad Name", "https://x.example").is_err());
    assert!(RegistrySource::new("insecure", "http://x.example").is_err());
}
