use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tiktools_plugin_api::HostCompat;
use tiktools_plugin_package::{
    pack, sign_archive, KeyId, KeyRole, PackRequest, PackageArchive, Sha256Digest, SigningKey,
    TrustStore, TrustedKey,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    cloud_source, generation_files_url, join_url, latest_compatible, resolve, Advisory,
    AdvisorySeverity, DownloadRequest, Downloader, IndexWriter, RegistriesConfig, RegistryClient,
    RegistryError, RegistrySource, RejectionReason, ResolveRequest, SearchQuery,
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
            "tiktools-cloud-{name}-{}-{suffix}-{}",
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

/// Minimal Cloud origin mock: serves a static file tree, including the
/// generation pointer at `api/v1/registry/current` and immutable snapshots at
/// `api/v1/registry/generations/{n}/files/...`.
struct MockCloud {
    origin: String,
    root: PathBuf,
    requests: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl MockCloud {
    async fn start(root: PathBuf) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = requests.clone();
        let serve_root = root.clone();
        let task = tokio::spawn(async move {
            let root = serve_root;
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
            origin,
            root,
            requests,
            task,
        }
    }

    fn stop(self) {
        self.task.abort();
    }
}

struct Keys {
    root: SigningKey,
    index: SigningKey,
    publisher: SigningKey,
}

fn keys() -> Keys {
    keys_with("root-1", "index-1", "acme-1")
}

fn keys_with(root: &str, index: &str, publisher: &str) -> Keys {
    Keys {
        root: SigningKey::generate(KeyId::parse(root).unwrap(), None).unwrap(),
        index: SigningKey::generate(KeyId::parse(index).unwrap(), None).unwrap(),
        publisher: SigningKey::generate(KeyId::parse(publisher).unwrap(), Some("acme".to_owned()))
            .unwrap(),
    }
}

fn host() -> HostCompat {
    HostCompat::new(semver::Version::new(0, 4, 0)).with_target("linux-x64-gnu")
}

fn package(directory: &TempDir, id: &str, version: &str) -> PathBuf {
    let source = directory.join(&format!("src-{id}-{version}"));
    fs::create_dir_all(&source).unwrap();
    let manifest = serde_json::json!({
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
    fs::write(
        source.join("plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(source.join("tts"), format!("binary {version}")).unwrap();
    pack(&PackRequest {
        manifest: source.join("plugin.json"),
        entry: Some(source.join("tts")),
        output: directory.join(&format!("out/{id}-{version}.plugin")),
        target: None,
    })
    .unwrap()
    .path
}

fn signed_tree(tree: PathBuf, keys: &Keys, archives: &[PathBuf], description: &str) -> IndexWriter {
    let writer = IndexWriter::open(
        tree,
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
            Some(description.to_owned()),
            None,
        )
        .unwrap();
    for archive in archives {
        writer.add(archive, "stable", None).unwrap();
    }
    writer
}

fn gen_files(mock: &MockCloud, generation: u64) -> PathBuf {
    mock.root
        .join(format!("api/v1/registry/generations/{generation}/files"))
}

fn set_current(mock: &MockCloud, generation: u64) {
    let path = mock.root.join("api/v1/registry/current");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        serde_json::json!({
            "generation": generation,
            "inventorySha256": "0".repeat(64),
            "manifestSha256": "1".repeat(64),
            "rootFingerprint": "deadbeef",
            "indexKeyId": "index-1",
            "releaseCount": 1,
            "fileCount": 8,
            "activatedAt": "2026-01-01T00:00:00Z",
        })
        .to_string(),
    )
    .unwrap();
}

fn cloud_client(
    directory: &TempDir,
    origin: &str,
    roots: Vec<tiktools_plugin_package::PublicKey>,
) -> RegistryClient {
    let source = cloud_source("official", origin, roots).unwrap();
    assert!(source.cloud_origin.is_some());
    RegistryClient::new(
        directory.join("cache"),
        directory.join("registries.json"),
        vec![source],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixed_product_package_uses_the_same_generation_as_its_metadata() {
    let directory = TempDir::new("fixed-generation-package");
    let keys = keys();
    let v1 = package(&directory, "acme.tts", "1.0.0");
    let v2 = package(&directory, "acme.tts", "1.1.0");
    sign_archive(&v1, &keys.publisher).unwrap();
    sign_archive(&v2, &keys.publisher).unwrap();
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(gen_files(&mock, 1), &keys, &[v1], "first");
    signed_tree(gen_files(&mock, 2), &keys, &[v2], "second");
    set_current(&mock, 1);
    let source = cloud_source("official", &mock.origin, vec![keys.root.public_key()]).unwrap();
    let client = RegistryClient::new_fixed(
        directory.join("cache"),
        directory.join("registries.json"),
        vec![source],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    for (generation, version) in [(1, "1.0.0"), (2, "1.1.0")] {
        set_current(&mock, generation);
        let (source, document) = client.package("acme.tts", None).await.unwrap();
        assert_eq!(
            source.url,
            generation_files_url(&mock.origin, generation).unwrap()
        );
        let artifact = &document.release(version).unwrap().artifacts[0];
        let bytes = reqwest::get(join_url(&source.url, &artifact.url).unwrap())
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(Sha256Digest::of_bytes(&bytes), artifact.sha256);
    }
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixed_product_does_not_use_previous_generation_metadata_on_failure() {
    let directory = TempDir::new("fixed-generation-cache");
    let keys = keys();
    let archive = package(&directory, "acme.tts", "1.0.0");
    sign_archive(&archive, &keys.publisher).unwrap();
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(gen_files(&mock, 1), &keys, &[archive], "first");
    set_current(&mock, 1);
    let source = cloud_source("official", &mock.origin, vec![keys.root.public_key()]).unwrap();
    let client = RegistryClient::new_fixed(
        directory.join("cache"),
        directory.join("registries.json"),
        vec![source],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    assert!(client.package("acme.tts", None).await.is_ok());
    // Generation two has no signed files. Generation one's cached keys,
    // configuration and index may not satisfy this request.
    set_current(&mock, 2);
    assert!(client.package("acme.tts", None).await.is_err());
    set_current(&mock, 0);
    assert!(client.refresh("official").await.is_err());
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_discovery_pins_refresh_and_search() {
    let directory = TempDir::new("discovery");
    let keys = keys();
    let archive = package(&directory, "acme.tts", "1.0.0");
    let signed = sign_archive(&archive, &keys.publisher).unwrap();
    let _ = signed;
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(gen_files(&mock, 1), &keys, &[archive], "Staging cloud");
    set_current(&mock, 1);

    let client = cloud_client(&directory, &mock.origin, vec![keys.root.public_key()]);
    let status = client.refresh("official").await.unwrap();
    assert!(status.verified, "{status:?}");
    let expected = generation_files_url(&mock.origin, 1).unwrap();
    assert_eq!(status.url, expected);

    // The resolved generation endpoint is persisted, so restarts keep it.
    let saved = RegistriesConfig::load(&directory.join("registries.json")).unwrap();
    assert_eq!(saved.registries.len(), 1);
    assert_eq!(saved.registries[0].url, expected);
    assert_eq!(
        saved.registries[0].cloud_origin.as_deref(),
        Some(mock.origin.trim_end_matches('/'))
    );

    let results = client
        .search(&SearchQuery {
            query: "voice".to_owned(),
            ..SearchQuery::default()
        })
        .await;
    assert_eq!(results.total, 1, "{results:?}");
    assert_eq!(results.entries[0].registry, "official");

    let (source, document) = client.package("acme.tts", None).await.unwrap();
    assert_eq!(source.url, expected);
    assert_eq!(document.release("1.0.0").unwrap().channel, "stable");
    assert!(
        mock.requests.load(Ordering::Acquire) > 0,
        "discovery must hit the Cloud origin"
    );

    // A rebuilt client keeps the resolved generation URL without network.
    drop(client);
    let rebuilt = cloud_client(&directory, &mock.origin, vec![keys.root.public_key()]);
    assert_eq!(
        rebuilt.status("official").unwrap().url,
        expected,
        "restart must keep the resolved generation endpoint"
    );
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_signed_download_verifies_and_updates_across_generations() {
    let directory = TempDir::new("update");
    let keys = keys();
    let v1 = package(&directory, "acme.tts", "1.0.0");
    sign_archive(&v1, &keys.publisher).unwrap();
    let v2 = package(&directory, "acme.tts", "1.1.0");
    let added_v2 = sign_archive(&v2, &keys.publisher).unwrap();
    let _ = added_v2;
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(
        gen_files(&mock, 1),
        &keys,
        std::slice::from_ref(&v1),
        "gen one",
    );
    let writer2 = signed_tree(gen_files(&mock, 2), &keys, &[v1, v2.clone()], "gen two");
    writer2
        .advise(
            "acme.tts",
            "1.1.0",
            Some(Advisory {
                severity: AdvisorySeverity::High,
                message: "security fix".to_owned(),
                url: None,
            }),
        )
        .unwrap();
    set_current(&mock, 1);

    let client = cloud_client(&directory, &mock.origin, vec![keys.root.public_key()]);
    client.refresh("official").await.unwrap();
    let (source, document) = client.package("acme.tts", None).await.unwrap();
    assert!(source.url.contains("/generations/1/files"));
    let resolved = latest_compatible(&document, "stable", &host()).unwrap();
    assert_eq!(resolved.release.version, "1.0.0");

    let downloader = Downloader::new(directory.join("downloads")).unwrap();
    let url = join_url(&source.url, &resolved.artifact.url).unwrap();
    let downloaded = downloader
        .fetch(
            DownloadRequest {
                url: &url,
                expected_digest: Some(&resolved.artifact.sha256),
                expected_size: Some(resolved.artifact.size),
            },
            &|_, _| {},
            &std::sync::atomic::AtomicBool::new(false),
        )
        .await
        .unwrap();
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

    // Publish generation 2 and advance the pointer: the next operation picks
    // it up, downloads the new archive, and sees the advisory.
    set_current(&mock, 2);
    client.refresh("official").await.unwrap();
    let (source, document) = client.package("acme.tts", None).await.unwrap();
    assert!(source.url.contains("/generations/2/files"));
    let resolved = latest_compatible(&document, "stable", &host()).unwrap();
    assert_eq!(resolved.release.version, "1.1.0");
    assert_eq!(
        document
            .release("1.1.0")
            .unwrap()
            .advisory
            .as_ref()
            .unwrap()
            .message,
        "security fix"
    );
    let url = join_url(&source.url, &resolved.artifact.url).unwrap();
    let downloaded = downloader
        .fetch(
            DownloadRequest {
                url: &url,
                expected_digest: Some(&resolved.artifact.sha256),
                expected_size: Some(resolved.artifact.size),
            },
            &|_, _| {},
            &std::sync::atomic::AtomicBool::new(false),
        )
        .await
        .unwrap();
    assert_eq!(downloaded.digest, resolved.artifact.sha256);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_pinned_operations_survive_generation_changes() {
    let directory = TempDir::new("pinning");
    let keys = keys();
    let v1 = package(&directory, "acme.tts", "1.0.0");
    sign_archive(&v1, &keys.publisher).unwrap();
    let v2 = package(&directory, "acme.tts", "1.1.0");
    sign_archive(&v2, &keys.publisher).unwrap();
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(
        gen_files(&mock, 1),
        &keys,
        std::slice::from_ref(&v1),
        "gen one",
    );
    signed_tree(
        gen_files(&mock, 2),
        &keys,
        &[v1.clone(), v2.clone()],
        "gen two",
    );
    set_current(&mock, 1);

    let client = cloud_client(&directory, &mock.origin, vec![keys.root.public_key()]);
    client.refresh("official").await.unwrap();

    // An operation pinned to generation 1 keeps serving generation 1 bytes
    // after the pointer advances: this is the static-URL form of the same
    // snapshot the Cloud client pins per operation.
    let gen1_url = generation_files_url(&mock.origin, 1).unwrap();
    let mut pinned = RegistrySource::new("pinned", &gen1_url).unwrap();
    pinned.root_keys = vec![keys.root.public_key()];
    let pinned_client = RegistryClient::new(
        directory.join("pinned-cache"),
        directory.join("pinned.json"),
        vec![pinned],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    pinned_client.refresh("pinned").await.unwrap();

    set_current(&mock, 2);
    client.refresh("official").await.unwrap();

    let (_, old) = pinned_client.package("acme.tts", None).await.unwrap();
    assert_eq!(old.release("1.0.0").unwrap().version, "1.0.0");
    assert!(
        old.release("1.1.0").is_none(),
        "the pinned generation-1 snapshot must not see generation-2 releases"
    );
    let (_, current) = client.package("acme.tts", None).await.unwrap();
    assert!(
        current.release("1.1.0").is_some(),
        "the advanced client serves generation 2"
    );
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_offline_serves_cached_signed_metadata() {
    let directory = TempDir::new("offline");
    let keys = keys();
    let archive = package(&directory, "acme.tts", "1.0.0");
    sign_archive(&archive, &keys.publisher).unwrap();
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(gen_files(&mock, 1), &keys, &[archive], "gen one");
    set_current(&mock, 1);

    let client = cloud_client(&directory, &mock.origin, vec![keys.root.public_key()]);
    client.refresh("official").await.unwrap();
    assert_eq!(
        client.package("acme.tts", None).await.unwrap().1.id,
        "acme.tts"
    );
    // Prime the catalog cache while online; search needs it.
    let online = client
        .search(&SearchQuery {
            query: "voice".to_owned(),
            ..SearchQuery::default()
        })
        .await;
    assert_eq!(online.total, 1, "{online:?}");

    mock.stop();
    // Origin gone: cached signed metadata and installed plugins keep working.
    let (_, document) = client.package("acme.tts", None).await.unwrap();
    assert_eq!(document.release("1.0.0").unwrap().channel, "stable");
    let results = client
        .search(&SearchQuery {
            query: "voice".to_owned(),
            ..SearchQuery::default()
        })
        .await;
    assert_eq!(results.total, 1, "{results:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_rejects_wrong_roots_tampering_and_digest_mismatch() {
    let directory = TempDir::new("tamper");
    let keys = keys();
    let archive = package(&directory, "acme.tts", "1.0.0");
    sign_archive(&archive, &keys.publisher).unwrap();
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(gen_files(&mock, 1), &keys, &[archive], "gen one");
    set_current(&mock, 1);

    let impostor = SigningKey::generate(KeyId::parse("root-x").unwrap(), None).unwrap();
    let wrong = cloud_client(&directory, &mock.origin, vec![impostor.public_key()]);
    assert!(matches!(
        wrong.refresh("official").await,
        Err(RegistryError::Signature { .. })
    ));

    let client = cloud_client(&directory, &mock.origin, vec![keys.root.public_key()]);
    client.refresh("official").await.unwrap();
    // Resolve the document before tampering so the digest check below runs
    // against the cached signed metadata.
    let (_, document) = client.package("acme.tts", None).await.unwrap();
    let resolved = latest_compatible(&document, "stable", &host()).unwrap();

    // Tamper with the archive bytes first: the digest check fails while the
    // cached document still verifies, so only the archive is re-downloaded.
    let tree = gen_files(&mock, 1);
    let plugin = fs::read_dir(tree.join("packages/acme.tts"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut bytes = fs::read(&plugin).unwrap();
    bytes[100] ^= 0xff;
    fs::write(&plugin, bytes).unwrap();
    let downloader = Downloader::new(directory.join("downloads")).unwrap();
    let url = join_url(
        &generation_files_url(&mock.origin, 1).unwrap(),
        &resolved.artifact.url,
    )
    .unwrap();
    assert!(matches!(
        downloader
            .fetch(
                DownloadRequest {
                    url: &url,
                    expected_digest: Some(&resolved.artifact.sha256),
                    expected_size: None,
                },
                &|_, _| {},
                &std::sync::atomic::AtomicBool::new(false),
            )
            .await,
        Err(RegistryError::DigestMismatch { .. })
    ));

    // Tamper with the signed package document instead: a client without a
    // cached copy rejects it during verification.
    let doc = gen_files(&mock, 1).join("index/ac/me/acme.tts.json");
    let tampered = fs::read_to_string(&doc)
        .unwrap()
        .replace("audio.output", "points.write");
    fs::write(&doc, tampered).unwrap();
    let fresh_dir = TempDir::new("tamper-fresh");
    let fresh = cloud_client(&fresh_dir, &mock.origin, vec![keys.root.public_key()]);
    let tampered_result = fresh.package("acme.tts", None).await;
    assert!(
        matches!(tampered_result, Err(RegistryError::Signature { .. })),
        "tampered index must fail verification, got: {tampered_result:?}"
    );
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_enforces_yanks_revocations_and_reports_no_generation() {
    let directory = TempDir::new("yank");
    let keys = keys();
    let v1 = package(&directory, "acme.tts", "1.0.0");
    sign_archive(&v1, &keys.publisher).unwrap();
    let v2 = package(&directory, "acme.tts", "1.1.0");
    sign_archive(&v2, &keys.publisher).unwrap();
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(
        gen_files(&mock, 1),
        &keys,
        std::slice::from_ref(&v1),
        "gen one",
    );
    let writer2 = signed_tree(
        gen_files(&mock, 2),
        &keys,
        &[v1.clone(), v2.clone()],
        "gen two",
    );
    writer2.yank("acme.tts", "1.0.0", true).unwrap();
    let revoked = PackageArchive::open(&v1).unwrap().digest().unwrap();
    writer2.revoke_digest(&revoked).unwrap();
    set_current(&mock, 1);

    let client = cloud_client(&directory, &mock.origin, vec![keys.root.public_key()]);
    client.refresh("official").await.unwrap();

    set_current(&mock, 2);
    client.refresh("official").await.unwrap();
    let (_, document) = client.package("acme.tts", None).await.unwrap();
    // Latest skips the yanked release.
    let resolved = latest_compatible(&document, "stable", &host()).unwrap();
    assert_eq!(resolved.release.version, "1.1.0");
    // An exact pin of the yanked release is rejected unless explicitly allowed.
    let exact = semver::VersionReq::parse("=1.0.0").unwrap();
    let rejected = resolve(
        &document,
        &ResolveRequest {
            requirement: Some(&exact),
            ..ResolveRequest::default()
        },
        &host(),
    )
    .unwrap_err();
    assert!(rejected
        .iter()
        .any(|rejection| rejection.reason == RejectionReason::Yanked));
    let allowed = resolve(
        &document,
        &ResolveRequest {
            requirement: Some(&exact),
            allow_yanked: true,
            ..ResolveRequest::default()
        },
        &host(),
    )
    .unwrap();
    assert_eq!(allowed.release.version, "1.0.0");
    assert!(
        client.is_revoked(&revoked),
        "the revoked generation-1 digest must stay rejected"
    );

    // No published generation is an availability state surfaced to the UI.
    fs::remove_file(mock.root.join("api/v1/registry/current")).unwrap();
    let missing = client.refresh("official").await.unwrap_err();
    assert!(
        matches!(missing, RegistryError::NoPublishedGeneration { .. }),
        "{missing:?}"
    );
    assert!(
        client
            .status("official")
            .unwrap()
            .error
            .is_some_and(|error| error.contains("no published generation")),
        "refresh failure must be visible in the registry status"
    );
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_interrupted_downloads_fail_closed_and_retry() {
    let directory = TempDir::new("interrupted");
    let keys = keys();
    let archive = package(&directory, "acme.tts", "1.0.0");
    sign_archive(&archive, &keys.publisher).unwrap();
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(gen_files(&mock, 1), &keys, &[archive], "gen one");
    set_current(&mock, 1);

    let client = cloud_client(&directory, &mock.origin, vec![keys.root.public_key()]);
    client.refresh("official").await.unwrap();
    let (source, document) = client.package("acme.tts", None).await.unwrap();
    let resolved = latest_compatible(&document, "stable", &host()).unwrap();
    let url = join_url(&source.url, &resolved.artifact.url).unwrap();
    let downloader = Downloader::new(directory.join("downloads")).unwrap();

    let cancelled = std::sync::atomic::AtomicBool::new(true);
    assert!(matches!(
        downloader
            .fetch(
                DownloadRequest {
                    url: &url,
                    expected_digest: Some(&resolved.artifact.sha256),
                    expected_size: Some(resolved.artifact.size),
                },
                &|_, _| {},
                &cancelled,
            )
            .await,
        Err(RegistryError::Cancelled)
    ));
    let retried = downloader
        .fetch(
            DownloadRequest {
                url: &url,
                expected_digest: Some(&resolved.artifact.sha256),
                expected_size: Some(resolved.artifact.size),
            },
            &|_, _| {},
            &std::sync::atomic::AtomicBool::new(false),
        )
        .await
        .unwrap();
    assert_eq!(retried.digest, resolved.artifact.sha256);
    mock.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_third_party_registries_keep_working() {
    let directory = TempDir::new("third-party");
    let cloud_keys = keys();
    let archive = package(&directory, "acme.tts", "1.0.0");
    sign_archive(&archive, &cloud_keys.publisher).unwrap();
    let mock = MockCloud::start(directory.join("origin")).await;
    signed_tree(gen_files(&mock, 1), &cloud_keys, &[archive], "gen one");
    set_current(&mock, 1);

    // Third-party static tree under its own trust anchor. Key ids must be
    // distinct: the trust store is keyed by key id, not by registry.
    let other_keys = keys_with("troot-1", "tindex-1", "tacme-1");
    let other_archive = package(&directory, "other.tool", "2.0.0");
    sign_archive(&other_archive, &other_keys.publisher).unwrap();
    let third_dir = directory.join("third");
    fs::create_dir_all(&third_dir).unwrap();
    let writer = IndexWriter::open(
        third_dir.clone(),
        "third",
        Some(SigningKey::from_json(&other_keys.index.to_json()).unwrap()),
    );
    writer
        .init(
            &other_keys.root,
            vec![
                TrustedKey::new(other_keys.index.public_key(), KeyRole::Index),
                TrustedKey::new(other_keys.publisher.public_key(), KeyRole::Publisher)
                    .publisher("acme"),
            ],
            Some("Third party".to_owned()),
            None,
        )
        .unwrap();
    writer.add(&other_archive, "stable", None).unwrap();
    let third = MockCloud::start(third_dir).await;

    let mut official =
        cloud_source("official", &mock.origin, vec![cloud_keys.root.public_key()]).unwrap();
    official.priority = 100;
    let mut third_source = RegistrySource::new("acme", &third.origin).unwrap();
    third_source.root_keys = vec![other_keys.root.public_key()];
    let client = RegistryClient::new(
        directory.join("cache"),
        directory.join("registries.json"),
        vec![official, third_source],
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    for status in client.refresh_all().await {
        assert!(status.error.is_none(), "{status:?}");
    }
    let results = client.search(&SearchQuery::default()).await;
    assert_eq!(results.total, 2, "{results:?}");
    assert!(client.package("acme.tts", None).await.is_ok());
    assert!(client.package("other.tool", None).await.is_ok());

    // Disabling the third-party source leaves the official one untouched.
    client.set_enabled("acme", false).unwrap();
    let results = client.search(&SearchQuery::default()).await;
    assert_eq!(results.total, 1, "{results:?}");
    assert_eq!(results.entries[0].registry, "official");
    mock.stop();
    third.stop();
}

#[test]
fn cloud_source_config_round_trips_and_preserves_resolved_urls() {
    let mut config = RegistriesConfig::default();
    let source = cloud_source(
        "official",
        "https://cloud.example",
        vec![keys().root.public_key()],
    )
    .unwrap();
    config.registries.push(source);
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(
        json["registries"][0]["cloudOrigin"],
        serde_json::Value::String("https://cloud.example".to_owned())
    );
    // Older configurations without the field keep loading.
    let legacy = serde_json::json!({
        "version": 1,
        "registries": [{"name": "acme", "url": "https://acme.example/v1"}],
        "defaults": {},
    });
    let loaded: RegistriesConfig = serde_json::from_value(legacy).unwrap();
    assert!(loaded.registries[0].cloud_origin.is_none());

    // A restart keeps a previously resolved generation endpoint.
    let resolved = generation_files_url("https://cloud.example", 7).unwrap();
    let mut restarted = config.clone();
    restarted.registries[0].url = resolved.clone();
    restarted.merge_builtin(vec![cloud_source(
        "official",
        "https://cloud.example",
        vec![keys().root.public_key()],
    )
    .unwrap()]);
    assert_eq!(restarted.registries[0].url, resolved);
}
