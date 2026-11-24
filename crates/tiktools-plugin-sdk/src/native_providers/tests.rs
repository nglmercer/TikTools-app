use super::*;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    task::JoinHandle,
};

static STUB_COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch(label: &str) -> PathBuf {
    let id = STUB_COUNTER.fetch_add(1, Ordering::AcqRel);
    let root = std::env::temp_dir().join(format!(
        "tiktools-native-providers-{label}-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// Minimal stub origin. Routes are filled after binding (metadata
/// documents embed the base URL); absent paths 404. Mirrors the
/// declarative-HTTP test stub: one read per connection,
/// `connection: close` framing.
async fn stub_origin() -> (
    String,
    Arc<std::sync::Mutex<BTreeMap<String, Vec<u8>>>>,
    JoinHandle<()>,
) {
    let routes = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
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
                let path = path.split('?').next().unwrap_or(path);
                let (status, body) = routes
                    .lock()
                    .ok()
                    .and_then(|routes| routes.get(path).cloned())
                    .map_or_else(|| (404, b"not found".to_vec()), |body| (200, body));
                let reason = if status == 200 { "OK" } else { "Not Found" };
                let header = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/octet-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(header.as_bytes()).await;
                let _ = socket.write_all(&body).await;
            });
        }
    });
    (base, routes, task)
}

/// Build a gzip tarball from `(archive_path, bytes)` entries.
fn make_tarball(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut builder = tar::Builder::new(encoder);
    for (path, bytes) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, path, *bytes).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

fn ssri_sha512(bytes: &[u8]) -> String {
    use base64::Engine;
    format!(
        "sha512-{}",
        base64::engine::general_purpose::STANDARD.encode(Sha512::digest(bytes))
    )
}

fn npm_fixture_tarball() -> Vec<u8> {
    make_tarball(&[
        (
            "package/package.json",
            br#"{"name":"fixture-pkg","version":"1.2.3","main":"index.js","files":["index.js","*.node"]}"#,
        ),
        ("package/index.js", b"module.exports = {};"),
        ("package/node-stem.linux-x64-gnu.node", b"gnu-bytes"),
        ("package/node-stem.linux-x64-musl.node", b"musl-bytes"),
    ])
}

fn npm_declaration() -> NativeLibDeclaration {
    NativeLibDeclaration {
        package: "fixture-pkg".to_owned(),
        version: "1.2.3".to_owned(),
        provider: NativeLibProvider::Npm,
        repo: None,
        tag: None,
        binary: None,
    }
}

fn github_declaration() -> NativeLibDeclaration {
    NativeLibDeclaration {
        package: "gh-pkg".to_owned(),
        version: "0.3.0".to_owned(),
        provider: NativeLibProvider::Github,
        repo: Some("owner/gh-pkg".to_owned()),
        tag: Some("v0.3.0".to_owned()),
        binary: Some("node-stem".to_owned()),
    }
}

#[test]
fn target_matcher_requires_dot_boundary_and_top_level() {
    assert!(node_serves_target(
        "node-stem.linux-x64-gnu.node",
        "linux-x64-gnu"
    ));
    assert!(!node_serves_target(
        "node-stem.linux-x64-musl.node",
        "linux-x64-gnu"
    ));
    // No dot boundary: a longer triple ending in the target name.
    assert!(!node_serves_target(
        "stemxlinux-x64-gnu.node",
        "linux-x64-gnu"
    ));
    // Subdirectory binaries never match: the loader selects top-level.
    assert!(!node_serves_target(
        "dist/node-stem.linux-x64-gnu.node",
        "linux-x64-gnu"
    ));
    assert!(!node_serves_target("index.js", "linux-x64-gnu"));
}

#[test]
fn integrity_and_pin_verifiers_fail_closed() {
    let bytes = b"payload";
    assert!(verify_integrity(bytes, &ssri_sha512(bytes)));
    assert!(!verify_integrity(bytes, "sha512-broken!!!"));
    assert!(!verify_integrity(bytes, "md5-AAAAAAAAAAAAAAAAAAAAAA=="));
    assert!(!verify_integrity(bytes, "not-an-integrity-value"));
    assert!(!verify_integrity(b"other", &ssri_sha512(bytes)));
    let pin = sha256_pin(bytes);
    assert!(verify_pin(bytes, &pin));
    assert!(!verify_pin(b"other", &pin));
    assert!(!verify_pin(bytes, "sha256:xyz"));
    assert!(!verify_pin(bytes, "md5:d41d8cd98f00b204e9800998ecf8427e"));
}

#[test]
fn lockfile_roundtrips_and_rejects_versions() {
    let root = scratch("lock");
    let path = root.join(NATIVE_LIBS_LOCKFILE);
    let mut packages = BTreeMap::new();
    packages.insert(
        "pkg".to_owned(),
        LockedNativeLib {
            version: "1.0.0".to_owned(),
            provider: NativeLibProvider::Npm,
            tarball: Some("https://example.invalid/pkg.tgz".to_owned()),
            integrity: Some("sha512-AAAA".to_owned()),
            repo: None,
            tag: None,
            binary: None,
            assets: BTreeMap::from([("node-stem.linux-x64-gnu.node".to_owned(), sha256_pin(b"x"))]),
        },
    );
    let lockfile = NativeLibsLockfile {
        version: LOCKFILE_VERSION,
        packages,
    };
    write_lockfile(&path, &lockfile).unwrap();
    assert_eq!(read_lockfile(&path).unwrap(), lockfile);
    std::fs::write(&path, r#"{"version":999,"packages":{}}"#).unwrap();
    assert!(matches!(
        read_lockfile(&path),
        Err(NativeProviderError::LockVersion { version: 999, .. })
    ));
    let _ = std::fs::remove_dir_all(&root);
}

/// Patch the first header's name field to an evil path the safe
/// builder API refuses to emit, then repair the header checksum so
/// the archive parses and the extractor's own guards are tested.
fn make_evil_tarball(evil: &str) -> Vec<u8> {
    let placeholder = "package/placeholder-padded-to-cover-evil-paths.node";
    assert!(evil.len() < placeholder.len());
    let tarball = make_tarball(&[(placeholder, b"x" as &[u8])]);
    // Decompress, patch the raw header, recompress.
    let mut decoder = flate2::read::GzDecoder::new(tarball.as_slice());
    let mut raw = Vec::new();
    use std::io::Read;
    decoder.read_to_end(&mut raw).unwrap();
    assert!(raw.len() >= 512);
    raw[..placeholder.len()].fill(0);
    raw[..evil.len()].copy_from_slice(evil.as_bytes());
    // Recompute the ustar checksum: sum with the chksum field blanked.
    raw[148..156].fill(b' ');
    let sum: u32 = raw[..512].iter().map(|byte| *byte as u32).sum();
    let checksum = format!("{sum:06o}\0 ");
    raw[148..156].copy_from_slice(checksum.as_bytes());
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    use std::io::Write;
    encoder.write_all(&raw).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn tarball_extraction_rejects_evil_entries() {
    // Baseline: the fixture extracts.
    let files = extract_npm_tarball("pkg", &npm_fixture_tarball()).unwrap();
    assert_eq!(files.len(), 4);
    // Prefix escapes are expressible through the safe builder.
    let tarball = make_tarball(&[("elsewhere/file.node", b"x" as &[u8])]);
    assert!(extract_npm_tarball("pkg", &tarball).is_err());
    // Traversal and absolute paths need raw headers: the builder
    // refuses to emit them, so patch the bytes directly.
    for evil in [
        "package/../escape.node",
        "/absolute.node",
        "package/sub/../../escape.node",
    ] {
        let tarball = make_evil_tarball(evil);
        assert!(
            extract_npm_tarball("pkg", &tarball).is_err(),
            "{evil} should be rejected"
        );
    }
    // Symlinks are non-regular files: rejected without following.
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut builder = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_cksum();
    builder
        .append_link(&mut header, "package/link.node", "package/real.node")
        .unwrap();
    let tarball = builder.into_inner().unwrap().finish().unwrap();
    assert!(extract_npm_tarball("pkg", &tarball).is_err());
}

#[tokio::test]
async fn npm_pin_then_fetch_roundtrip() {
    let tarball = npm_fixture_tarball();
    let integrity = ssri_sha512(&tarball);
    let (base, routes, task) = stub_origin().await;
    routes.lock().unwrap().extend([
        (
            "/fixture-pkg/1.2.3".to_owned(),
            serde_json::to_vec(&serde_json::json!({
                "name": "fixture-pkg",
                "version": "1.2.3",
                "dist": {
                    "tarball": format!("{base}/tarballs/fixture-pkg-1.2.3.tgz"),
                    "integrity": integrity,
                },
            }))
            .unwrap(),
        ),
        ("/tarballs/fixture-pkg-1.2.3.tgz".to_owned(), tarball),
    ]);
    let endpoints = ProviderEndpoints {
        npm_registry: base.clone(),
        github_base: base,
    };
    let client = provider_client().unwrap();
    let declaration = npm_declaration();
    let locked = pin_native_lib(&client, &endpoints, &declaration)
        .await
        .unwrap();
    assert_eq!(locked.version, "1.2.3");
    assert_eq!(locked.provider, NativeLibProvider::Npm);
    assert_eq!(locked.assets.len(), 2);
    assert_eq!(
        locked.assets.get("node-stem.linux-x64-gnu.node").unwrap(),
        &sha256_pin(b"gnu-bytes")
    );

    let root = scratch("npm-fetch");
    let dest = root.join("source");
    let staged = fetch_native_lib(
        &client,
        &endpoints,
        &declaration,
        &locked,
        "linux-x64-gnu",
        &dest,
    )
    .await
    .unwrap();
    assert!(staged.contains(&"package.json".to_owned()));
    assert!(staged.contains(&"node-stem.linux-x64-gnu.node".to_owned()));
    assert!(!staged.iter().any(|file| file.contains("musl")));
    assert!(!dest.join("node-stem.linux-x64-musl.node").exists());

    // A target the tarball does not serve fails instead of staging
    // a binary-less tree.
    let missing = fetch_native_lib(
        &client,
        &endpoints,
        &declaration,
        &locked,
        "win32-x64-msvc",
        &root.join("missing"),
    )
    .await;
    assert!(
        matches!(missing, Err(NativeProviderError::NoBinaryForTarget { .. })),
        "{missing:?}"
    );
    // Unknown triples never reach the network.
    assert!(matches!(
        fetch_native_lib(
            &client,
            &endpoints,
            &declaration,
            &locked,
            "fuchsia-arm64",
            &root.join("nope"),
        )
        .await,
        Err(NativeProviderError::UnknownTarget { .. })
    ));
    task.abort();
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn github_pin_then_fetch_roundtrip() {
    let (base, routes, task) = stub_origin().await;
    let prefix = "/owner/gh-pkg/releases/download/v0.3.0";
    routes.lock().unwrap().extend([
        (
            format!("{prefix}/index.js"),
            b"module.exports = {};".to_vec(),
        ),
        (format!("{prefix}/index.d.ts"), b"export {};".to_vec()),
        (
            format!("{prefix}/node-stem.linux-x64-gnu.node"),
            b"gnu-bytes".to_vec(),
        ),
        (
            format!("{prefix}/node-stem.win32-x64-msvc.node"),
            b"win-bytes".to_vec(),
        ),
    ]);
    let endpoints = ProviderEndpoints {
        npm_registry: base.clone(),
        github_base: base,
    };
    let client = provider_client().unwrap();
    let declaration = github_declaration();
    // Pinning probes every known target; absent assets 404 and are
    // skipped, the rest are hashed.
    let locked = pin_native_lib(&client, &endpoints, &declaration)
        .await
        .unwrap();
    assert_eq!(locked.provider, NativeLibProvider::Github);
    assert_eq!(locked.assets.len(), 4);
    assert_eq!(
        locked.assets.get("index.js").unwrap(),
        &sha256_pin(b"module.exports = {};")
    );

    let root = scratch("gh-fetch");
    let dest = root.join("source");
    let staged = fetch_native_lib(
        &client,
        &endpoints,
        &declaration,
        &locked,
        "linux-x64-gnu",
        &dest,
    )
    .await
    .unwrap();
    assert_eq!(staged.len(), 4);
    assert!(dest.join("node-stem.linux-x64-gnu.node").is_file());
    assert!(!dest.join("node-stem.win32-x64-msvc.node").exists());
    let manifest = std::fs::read_to_string(dest.join("package.json")).unwrap();
    assert!(manifest.contains("\"binaryName\": \"node-stem\""));
    assert!(manifest.contains("\"name\": \"gh-pkg\""));

    // An unpinned target fails before any download for it.
    let missing = fetch_native_lib(
        &client,
        &endpoints,
        &declaration,
        &locked,
        "darwin-arm64",
        &root.join("missing"),
    )
    .await;
    assert!(
        matches!(missing, Err(NativeProviderError::NoBinaryForTarget { .. })),
        "{missing:?}"
    );
    task.abort();
    let _ = std::fs::remove_dir_all(&root);
}
