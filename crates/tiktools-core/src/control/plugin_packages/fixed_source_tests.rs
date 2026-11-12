use super::*;

#[tokio::test(start_paused = true)]
async fn update_deadline_bounds_a_stalled_catalog_and_is_shared_by_later_queries() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let catalog =
        cloud_client::CloudCatalog::new(&format!("http://{}", listener.local_addr().unwrap()))
            .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let task = tokio::spawn(async move {
        bounded_update_query(
            deadline,
            catalog.plugin_version_page("acme.plugin", Some("linux-x64-gnu"), Some(1), None),
        )
        .await
    });
    let (_socket, _) = listener.accept().await.unwrap();
    tokio::time::advance(Duration::from_secs(30)).await;
    // The HTTP client's own timeout may fire before the batch deadline;
    // either timeout is safe as long as it cannot produce approval.
    assert!(!matches!(task.await.unwrap(), Some(Ok(_))));
    assert!(bounded_update_query(deadline, async { 42 }).await.is_none());
}

#[test]
fn live_status_matches_the_exact_host_artifact_and_fails_closed() {
    let host = tiktools_plugin_api::HostCompat::new(semver::Version::new(1, 0, 0))
        .with_target("linux-x64-gnu");
    let mut document: PackageDocument = serde_json::from_value(serde_json::json!({
        "id": "acme.plugin", "name": "Example", "publisher":"acme", "releases": [{
            "version":"1.2.3", "runtime":"process", "schemaVersion":3, "protocolVersion":1,
            "artifacts":[
                {"target":"any", "url":"any.plugin", "size":1, "sha256":"a".repeat(64), "keyId":"key-a"},
                {"target":"linux-x64-gnu", "url":"native.plugin", "size":2, "sha256":"b".repeat(64), "keyId":"key-a"}
            ]
        }]
    })).unwrap();
    let any: cloud_client::CatalogVersion = serde_json::from_value(serde_json::json!({
        "version":"1.2.3", "target":"any", "sha256":"a".repeat(64), "size":"1", "yanked":false,
        "keyRevoked":false, "publisherApproved":true, "advisory":null
    }))
    .unwrap();
    let mut native = any.clone();
    native.target = "linux-x64-gnu".into();
    native.sha256 = "b".repeat(64);
    native.size = "2".into();
    let mut records = BTreeMap::from([((any.version.clone(), any.target.clone()), any)]);
    let status = |records: &BTreeMap<_, _>| {
        classify_live_releases(&document, &host, Some(records), |_| Ok(false))["1.2.3"].status
    };
    assert_eq!(
        status(&records),
        LiveReleaseStatus::Unknown,
        "an approved any artifact cannot approve the native artifact"
    );
    records.insert(
        (native.version.clone(), native.target.clone()),
        native.clone(),
    );
    assert_eq!(status(&records), LiveReleaseStatus::Approved);
    let mut missing_publisher = document.clone();
    missing_publisher.publisher = None;
    assert_eq!(
        classify_live_releases(&missing_publisher, &host, Some(&records), |_| Ok(false))["1.2.3"]
            .status,
        LiveReleaseStatus::Unknown,
        "a live catalog row cannot approve a release with no publisher"
    );
    missing_publisher.publisher = Some(String::new());
    assert_eq!(
        classify_live_releases(&missing_publisher, &host, Some(&records), |_| Ok(false))["1.2.3"]
            .status,
        LiveReleaseStatus::Unknown
    );
    let mut missing_key = document.clone();
    missing_key.releases[0].artifacts[1].key_id = None;
    assert_eq!(
        classify_live_releases(&missing_key, &host, Some(&records), |_| Ok(false))["1.2.3"].status,
        LiveReleaseStatus::Unknown,
        "a live catalog row cannot approve an artifact with no signing key"
    );
    missing_key.releases[0].artifacts[1].key_id = Some(String::new());
    assert_eq!(
        classify_live_releases(&missing_key, &host, Some(&records), |_| Ok(false))["1.2.3"].status,
        LiveReleaseStatus::Unknown
    );
    native.yanked = true;
    records.insert(
        (native.version.clone(), native.target.clone()),
        native.clone(),
    );
    assert_eq!(
        status(&records),
        LiveReleaseStatus::Yanked,
        "status may change within one generation"
    );
    native.yanked = false;
    native.key_revoked = true;
    records.insert(
        (native.version.clone(), native.target.clone()),
        native.clone(),
    );
    assert_eq!(status(&records), LiveReleaseStatus::KeyRevoked);
    native.key_revoked = false;
    native.publisher_approved = false;
    records.insert(
        (native.version.clone(), native.target.clone()),
        native.clone(),
    );
    assert_eq!(status(&records), LiveReleaseStatus::PublisherUnapproved);
    native.publisher_approved = true;
    native.sha256 = "c".repeat(64);
    records.insert((native.version.clone(), native.target.clone()), native);
    assert_eq!(
        status(&records),
        LiveReleaseStatus::Unknown,
        "digest mismatch must not approve"
    );
    assert_eq!(
        classify_live_releases(&document, &host, None, |_| Ok(false))["1.2.3"].status,
        LiveReleaseStatus::Unknown
    );
    // Equal digests on multiple targets may have different live states.
    // Old receipts cannot choose one; new receipts bind the exact target.
    document.releases[0].artifacts[0].sha256 = document.releases[0].artifacts[1].sha256.clone();
    records
        .get_mut(&("1.2.3".into(), "linux-x64-gnu".into()))
        .unwrap()
        .sha256 = "b".repeat(64);
    records
        .get_mut(&("1.2.3".into(), "any".into()))
        .unwrap()
        .sha256 = "b".repeat(64);
    records
        .get_mut(&("1.2.3".into(), "any".into()))
        .unwrap()
        .yanked = true;
    let ambiguous = classify_live_releases(&document, &host, Some(&records), |_| Ok(false));
    assert_eq!(ambiguous["1.2.3"].status, LiveReleaseStatus::Approved);
    let origin = "https://cloud.example";
    let historical_url = tiktools_plugin_registry::generation_files_url(origin, 3).unwrap();
    let mut source = RegistrySource::new("official", &historical_url).unwrap();
    source.cloud_origin = Some(origin.into());
    let mut receipt: InstallReceipt = serde_json::from_value(serde_json::json!({
        "id":"acme.plugin", "version":"1.2.3",
        "origin":{"kind":"registry", "registry":"official", "url":historical_url},
        "archiveDigest":"b".repeat(64), "signature":{"status":"verified", "publisher":"acme", "keyId":"key-a"},
        "runtime":"process", "installedAt":1, "updatedAt":1
    }))
    .unwrap();
    assert_eq!(
        installed_live_status(&receipt, &source, &document, &ambiguous),
        LiveReleaseStatus::Unknown
    );
    receipt.cloud_artifact = Some(tiktools_plugin_installer::CloudArtifactIdentity {
        target: "any".into(),
        generation: "3".into(),
    });
    assert_eq!(
        installed_live_status(&receipt, &source, &document, &ambiguous),
        LiveReleaseStatus::Yanked
    );
    receipt.cloud_artifact.as_mut().unwrap().target = "linux-x64-gnu".into();
    assert_eq!(
        installed_live_status(&receipt, &source, &document, &ambiguous),
        LiveReleaseStatus::Approved
    );
    receipt.signature.key_id = Some("other-key".into());
    assert_eq!(
        installed_live_status(&receipt, &source, &document, &ambiguous),
        LiveReleaseStatus::Unknown
    );
    receipt.signature.key_id = Some("key-a".into());
    receipt.signature.publisher = Some("other-publisher".into());
    assert_eq!(
        installed_live_status(&receipt, &source, &document, &ambiguous),
        LiveReleaseStatus::Unknown
    );
    receipt.signature.publisher = Some("acme".into());
    receipt.cloud_artifact.as_mut().unwrap().generation = "03".into();
    assert_eq!(
        installed_live_status(&receipt, &source, &document, &ambiguous),
        LiveReleaseStatus::Unknown
    );
    receipt.cloud_artifact.as_mut().unwrap().generation = "4".into();
    assert_eq!(
        installed_live_status(&receipt, &source, &document, &ambiguous),
        LiveReleaseStatus::Unknown
    );
    document.releases.push(document.releases[0].clone());
    assert_eq!(
        classify_live_releases(&document, &host, Some(&records), |_| Ok(false))["1.2.3"].status,
        LiveReleaseStatus::Unknown,
        "duplicate signed versions are ambiguous"
    );
}

#[test]
fn product_specs_cannot_choose_another_registry_or_network_url() {
    for input in [
        PackageSpecInput::Spec {
            spec: "other:acme.plugin@1.0.0".into(),
        },
        PackageSpecInput::Registry {
            id: "acme.plugin".into(),
            version: None,
            registry: Some("other".into()),
        },
        PackageSpecInput::Url {
            url: "https://other.example/plugin.plugin".into(),
        },
        PackageSpecInput::Spec {
            spec: "https://other.example/plugin.plugin".into(),
        },
    ] {
        assert!(product_package_spec(input).is_err());
    }
    assert!(matches!(
        product_package_spec(PackageSpecInput::Spec {
            spec: "acme.plugin@1.0.0".into()
        }),
        Ok(PackageSpec::Registry { .. })
    ));
    assert!(matches!(
        product_package_spec(PackageSpecInput::File {
            path: "/tmp/example.plugin".into()
        }),
        Ok(PackageSpec::File(_))
    ));
}

#[test]
fn current_cloud_status_must_match_exact_artifact_and_remain_approved() {
    let digest = "a".repeat(64);
    let mut item: cloud_client::CatalogVersion = serde_json::from_value(serde_json::json!({
        "version": "1.2.3", "target": "linux-x64-gnu", "sha256": digest,
        "size": "42", "yanked": false, "keyRevoked": false,
        "publisherApproved": true, "advisory": null
    }))
    .unwrap();
    let check = |item: &cloud_client::CatalogVersion, target: &str, size| {
        validate_current_cloud_release(item, "1.2.3", target, &"a".repeat(64), size)
    };
    assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_ok());
    assert!(check(&item, "any", 42).is_none());
    assert!(check(&item, "linux-x64-gnu", 43).is_none());
    item.yanked = true;
    assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_err());
    item.yanked = false;
    item.key_revoked = true;
    assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_err());
    item.key_revoked = false;
    item.publisher_approved = false;
    assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_err());
    item.publisher_approved = true;
    item.advisory = Some(cloud_client::CatalogAdvisory {
        severity: "high".into(),
        message: None,
        url: None,
    });
    assert!(check(&item, "linux-x64-gnu", 42).unwrap().is_err());
}

#[tokio::test]
async fn current_cloud_status_rejects_live_yank_without_generation_change() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let yanked = Arc::new(AtomicBool::new(false));
    let state = Arc::clone(&yanked);
    let server = tokio::spawn(async move {
        for _ in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let read = socket.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..read])
                .contains("/api/v2/catalog/plugins/acme.plugin/version-pages"));
            let body = serde_json::json!({
                "generation": "1", "id": "acme.plugin",
                "metadataPath": "/api/v1/registry/generations/1/files/index/ac/me/acme.plugin.json",
                "items": [{"version": "1.2.3", "target": "linux-x64-gnu", "sha256": "a".repeat(64), "size": "42", "yanked": state.load(Ordering::Acquire), "keyRevoked": false, "publisherApproved": true, "advisory": null}],
                "nextCursor": null
            }).to_string();
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let catalog = cloud_client::CloudCatalog::new(&origin).unwrap();
    let digest = "a".repeat(64);
    assert!(verify_catalog_release(
        &catalog,
        "acme.plugin",
        "1.2.3",
        "linux-x64-gnu",
        &digest,
        42,
        1
    )
    .await
    .is_ok());
    yanked.store(true, Ordering::Release);
    assert!(verify_catalog_release(
        &catalog,
        "acme.plugin",
        "1.2.3",
        "linux-x64-gnu",
        &digest,
        42,
        1
    )
    .await
    .is_err());
    yanked.store(false, Ordering::Release);
    let listed = load_catalog_versions(&catalog, "acme.plugin", "linux-x64-gnu", 1)
        .await
        .unwrap();
    assert!(!listed[&("1.2.3".into(), "linux-x64-gnu".into())].yanked);
    yanked.store(true, Ordering::Release);
    let listed = load_catalog_versions(&catalog, "acme.plugin", "linux-x64-gnu", 1)
        .await
        .unwrap();
    assert!(listed[&("1.2.3".into(), "linux-x64-gnu".into())].yanked);
    server.await.unwrap();
}
