use super::*;
use super::{
    errors::map_failure,
    validation::{bounded_document, validate_item},
};
use reqwest::StatusCode;
use serde_json::json;
use tiktools_cloud_auth::CloudAuth;

fn item(kind: &str, version: Option<&str>, revision: Option<i64>) -> PackageItemParams {
    PackageItemParams {
        kind: kind.into(),
        id: "3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c".into(),
        version: version.map(str::to_owned),
        revision,
        required: true,
    }
}

#[test]
fn plugins_pin_versions_and_documents_pin_revisions() {
    assert!(validate_item(&item("plugin", Some("1.2.0"), None)).is_ok());
    assert!(validate_item(&item("plugin", None, None)).is_err());
    assert!(validate_item(&item("plugin", Some("1.2.0"), Some(3))).is_err());
    assert!(validate_item(&item("overlay", None, Some(2))).is_ok());
    assert!(validate_item(&item("overlay", None, None)).is_err());
    assert!(validate_item(&item("overlay", None, Some(0))).is_err());
    assert!(validate_item(&item("script", None, Some(1))).is_err());
}

#[test]
fn documents_must_be_objects_within_the_size_budget() {
    assert!(bounded_document(&json!({"overlayVersion": 1})).is_ok());
    assert!(bounded_document(&json!([1, 2])).is_err());
    let big = json!({"text": "x".repeat(MAX_DOCUMENT_BYTES)});
    assert!(bounded_document(&big).is_err());
}

#[test]
fn overlay_package_and_library_failures_never_mention_teams() {
    for (status, body, code) in [
        (
            404,
            &br#"{"code":"notFound","requestId":"r-1"}"#[..],
            "not_found",
        ),
        (404, b"", "not_found"),
        (409, br#"{"code":"conflict"}"#, "conflict"),
        (403, br#"{"code":"forbidden"}"#, "forbidden"),
        (503, br#"{"code":"misconfigured"}"#, "unavailable"),
        (500, b"<html>", "unavailable"),
    ] {
        let error = map_failure(StatusCode::from_u16(status).unwrap(), body);
        assert_eq!(error.code, code);
        assert!(
            !error.message.to_lowercase().contains("team"),
            "{}",
            error.message
        );
    }
    let kept = map_failure(
        StatusCode::NOT_FOUND,
        br#"{"code":"notFound","requestId":"r-1","internal":"x"}"#,
    );
    assert_eq!(
        kept.data,
        Some(json!({"cloud": {"code": "notFound", "requestId": "r-1"}}))
    );
}

// ---- Capability gating and lifecycle, against a recording server ----------

use std::sync::{Arc, Mutex};
use tiktools_cloud_auth::{Persistence, TokenStore, TokenStoreError};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Store;
impl TokenStore for Store {
    fn persistence(&self) -> Persistence {
        Persistence::MemoryOnly
    }
    fn load(&self, _: &str, _: &str) -> Result<Option<secrecy::SecretString>, TokenStoreError> {
        Ok(Some(secrecy::SecretString::from("test-token".to_owned())))
    }
    fn store(&self, _: &str, _: &str, _: &cloud_client::Session) -> Result<(), TokenStoreError> {
        Ok(())
    }
    fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
        Ok(())
    }
}

type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

/// Serves `handler(method, target, body) -> (status, json)` and records
/// every request as `(method, target, body)`.
async fn serve<F>(handler: F) -> (CloudAuth, Seen, std::path::PathBuf)
where
    F: Fn(&str, &str, &str) -> (u16, Value) + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let seen: Seen = Arc::default();
    let log = seen.clone();
    let handler = Arc::new(handler);
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let (log, handler) = (log.clone(), handler.clone());
            tokio::spawn(async move {
                let mut bytes = Vec::new();
                let (head_end, length) = loop {
                    let mut buf = [0u8; 4096];
                    let Ok(n) = socket.read(&mut buf).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&bytes[..at]).to_ascii_lowercase();
                        let length = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length: "))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        break (at + 4, length);
                    }
                };
                while bytes.len() < head_end + length {
                    let mut buf = [0u8; 4096];
                    match socket.read(&mut buf).await {
                        Ok(n) if n > 0 => bytes.extend_from_slice(&buf[..n]),
                        _ => return,
                    }
                }
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let mut first = text.lines().next().unwrap_or_default().split_whitespace();
                let method = first.next().unwrap_or_default().to_owned();
                let target = first.next().unwrap_or_default().to_owned();
                let body = String::from_utf8_lossy(&bytes[head_end..]).into_owned();
                log.lock()
                    .unwrap()
                    .push((method.clone(), target.clone(), body.clone()));
                let (status, value) = if method == "GET" && target == "/api/v1/native/me" {
                    (200, json!({"id": "alice"}))
                } else {
                    handler(&method, &target, &body)
                };
                let payload = serde_json::to_vec(&value).unwrap();
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                );
                let _ = socket.write_all(head.as_bytes()).await;
                let _ = socket.write_all(&payload).await;
            });
        }
    });
    let dir = std::env::temp_dir().join(format!(
        "bundles-gate-{}-{}",
        std::process::id(),
        fastrand::u64(..)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let sidecar = dir.join("account.json");
    std::fs::write(
        &sidecar,
        serde_json::to_vec(&json!({"origin": origin, "account_id": "alice"})).unwrap(),
    )
    .unwrap();
    let auth = CloudAuth::new(Some(origin), sidecar, Arc::new(Store));
    auth.restore().await.unwrap();
    (auth, seen, dir)
}

const ID: &str = "3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c";

fn capabilities() -> Value {
    json!({
        "media": {"operations": ["upload", "list", "read", "rename", "delete"]},
        "overlays": {"operations": ["create", "list", "read", "revise", "revisions", "revision", "delete"]},
        "packages": {"operations": ["create", "read", "preflight", "delete"]},
        "library": {"fork": {"available": true}},
    })
}

fn paths(seen: &Seen) -> Vec<String> {
    seen.lock()
        .unwrap()
        .iter()
        .map(|(m, t, _)| format!("{m} {t}"))
        .filter(|p| !p.contains("/api/v1/native/me"))
        .collect()
}

#[tokio::test]
async fn an_unannounced_capability_never_reaches_the_feature_endpoint() {
    for document in [
        json!({}),
        json!({"media": {"operations": ["upload"]}}),
        json!({"media": {"operations": "list"}}),
        json!({"overlays": {"operations": ["list"]}, "library": {"fork": {"available": false}}}),
    ] {
        let doc = document.clone();
        let (auth, seen, dir) = serve(move |_, target, _| {
            if target == "/api/v2/capabilities" {
                (200, doc.clone())
            } else {
                (500, json!({}))
            }
        })
        .await;
        let fail = |error: OperationError, capability: &str| {
            assert_eq!(error.code, "capability_unavailable");
            assert_eq!(error.data.unwrap()["capability"], capability);
        };
        fail(
            list_assets(&auth, CloudPageQuery::default())
                .await
                .unwrap_err(),
            "media.operations",
        );
        fail(
            set_asset_label(
                &auth,
                AssetSetLabelParams {
                    id: ID.into(),
                    label: None,
                },
            )
            .await
            .unwrap_err(),
            "media.operations",
        );
        fail(
            delete_asset(&auth, AssetIdParams { id: ID.into() })
                .await
                .unwrap_err(),
            "media.operations",
        );
        if document.pointer("/overlays/operations/0").is_none() {
            fail(
                list_overlays(&auth, CloudPageQuery::default())
                    .await
                    .unwrap_err(),
                "overlays.operations",
            );
        }
        fail(
            delete_package(&auth, PackageIdParams { id: ID.into() })
                .await
                .unwrap_err(),
            "packages.operations",
        );
        fail(
            fork_catalog_document(
                &auth,
                CatalogForkParams {
                    id: ID.into(),
                    revision: None,
                },
            )
            .await
            .unwrap_err(),
            "library.fork",
        );
        assert!(
            paths(&seen).iter().all(|p| p == "GET /api/v2/capabilities"),
            "{:?}",
            paths(&seen)
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[tokio::test]
async fn an_instance_without_a_capability_document_is_an_older_backend() {
    let (auth, seen, dir) = serve(|_, _, _| (404, json!({}))).await;
    let error = list_assets(&auth, CloudPageQuery::default())
        .await
        .unwrap_err();
    assert_eq!(error.code, "capability_unavailable");
    assert_eq!(error.data.unwrap()["reason"], "capabilities_missing");
    assert_eq!(paths(&seen), vec!["GET /api/v2/capabilities"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn invalid_input_is_refused_before_any_network_use() {
    let (auth, seen, dir) = serve(|_, _, _| (500, json!({}))).await;
    let bad = |e: OperationError| assert_eq!(e.code, "invalid_params");
    bad(list_assets(
        &auth,
        CloudPageQuery {
            cursor: Some("a b".into()),
            limit: None,
        },
    )
    .await
    .unwrap_err());
    bad(list_assets(
        &auth,
        CloudPageQuery {
            cursor: None,
            limit: Some(51),
        },
    )
    .await
    .unwrap_err());
    bad(list_overlays(
        &auth,
        CloudPageQuery {
            cursor: None,
            limit: Some(0),
        },
    )
    .await
    .unwrap_err());
    bad(delete_asset(&auth, AssetIdParams { id: "../x".into() })
        .await
        .unwrap_err());
    for label in ["", "   ", "a\nb", "x\u{2028}y", &"l".repeat(121)] {
        bad(set_asset_label(
            &auth,
            AssetSetLabelParams {
                id: ID.into(),
                label: Some(label.into()),
            },
        )
        .await
        .unwrap_err());
    }
    bad(read_overlay_revision(
        &auth,
        OverlayRevisionParams {
            id: ID.into(),
            revision: 0,
        },
    )
    .await
    .unwrap_err());
    bad(fork_catalog_document(
        &auth,
        CatalogForkParams {
            id: ID.into(),
            revision: Some(0),
        },
    )
    .await
    .unwrap_err());
    assert!(paths(&seen).is_empty(), "{:?}", paths(&seen));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn announced_operations_send_the_documented_requests() {
    let (auth, seen, dir) = serve(|method, target, body| match (method, target) {
        (_, "/api/v2/capabilities") => (200, capabilities()),
        ("GET", t) if t.starts_with("/api/v3/assets?") => (
            200,
            json!({"items":[{"id":ID,"sha256":"a".repeat(64),"mime":"image/png","width":1,"height":1,"size":9,"createdAt":"2026-11-11T00:00:00Z","label":"Logo"}],"nextCursor":"abc_-="}),
        ),
        ("PATCH", t) if t == format!("/api/v3/assets/{ID}") => {
            let label: Value = serde_json::from_str(body).unwrap();
            (200, json!({"id":ID,"sha256":"a".repeat(64),"mime":"image/png","width":1,"height":1,"size":9,"createdAt":"2026-11-11T00:00:00Z","label":label["label"]}))
        }
        ("GET", t) if t == "/api/v3/overlays" || t.starts_with("/api/v3/overlays?") => (
            200,
            json!({"items":[{"id":ID,"kind":"follow","name":"Hi","revision":2,"status":"draft","createdAt":"c","updatedAt":"u","elementCount":3}],"nextCursor":null}),
        ),
        ("GET", t) if t.starts_with(&format!("/api/v3/overlays/{ID}/revisions?")) => (
            200,
            json!({"id":ID,"currentRevision":2,"items":[{"revision":2,"status":"draft","name":"Hi","sha256":"b".repeat(64),"createdAt":"c","current":true}],"nextCursor":null}),
        ),
        ("GET", t) if t == format!("/api/v3/overlays/{ID}/revisions/2") => (
            200,
            json!({"id":ID,"revision":2,"status":"draft","schemaVersion":1,"sha256":"b".repeat(64),"createdAt":"c","current":true,"document":{"overlayVersion":1}}),
        ),
        ("DELETE", t) if t.starts_with("/api/v3/") => (200, json!({"id":ID,"deleted":true})),
        ("POST", t) if t == format!("/api/v2/catalog/documents/{ID}/fork") => (
            201,
            json!({"id":"11111111-1111-4111-8111-111111111111","revision":1,"status":"draft","forkOf":{"documentId":ID,"revision":4}}),
        ),
        _ => (404, json!({})),
    })
    .await;
    let page = list_assets(
        &auth,
        CloudPageQuery {
            cursor: Some("c-1_=".into()),
            limit: Some(10),
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items[0].label.as_deref(), Some("Logo"));
    assert_eq!(page.next_cursor.as_deref(), Some("abc_-="));
    let renamed = set_asset_label(
        &auth,
        AssetSetLabelParams {
            id: ID.to_uppercase(),
            label: Some("  New  ".into()),
        },
    )
    .await
    .unwrap();
    assert_eq!(renamed.label.as_deref(), Some("New"));
    let cleared = set_asset_label(
        &auth,
        AssetSetLabelParams {
            id: ID.into(),
            label: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(cleared.label, None);
    assert_eq!(
        list_overlays(&auth, CloudPageQuery::default())
            .await
            .unwrap()
            .items[0]
            .element_count,
        3
    );
    let revisions = list_overlay_revisions(
        &auth,
        OverlayRevisionsQuery {
            id: ID.into(),
            cursor: None,
            limit: Some(5),
        },
    )
    .await
    .unwrap();
    assert!(revisions.items[0].current);
    let record = read_overlay_revision(
        &auth,
        OverlayRevisionParams {
            id: ID.into(),
            revision: 2,
        },
    )
    .await
    .unwrap();
    assert_eq!(record.document["overlayVersion"], 1);
    assert!(
        delete_asset(&auth, AssetIdParams { id: ID.into() })
            .await
            .unwrap()
            .deleted
    );
    assert!(
        delete_overlay(&auth, OverlayIdParams { id: ID.into() })
            .await
            .unwrap()
            .deleted
    );
    assert!(
        delete_package(&auth, PackageIdParams { id: ID.into() })
            .await
            .unwrap()
            .deleted
    );
    let fork = fork_catalog_document(
        &auth,
        CatalogForkParams {
            id: ID.into(),
            revision: Some(4),
        },
    )
    .await
    .unwrap();
    assert_eq!(fork.fork_of.revision, 4);
    let fork_no_revision = fork_catalog_document(
        &auth,
        CatalogForkParams {
            id: ID.into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(fork_no_revision.status, "draft");
    let log = seen.lock().unwrap().clone();
    let find = |m: &str, prefix: &str| {
        log.iter()
            .filter(|(rm, t, _)| rm == m && t.starts_with(prefix))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        find("GET", "/api/v3/assets?")[0].1,
        "/api/v3/assets?limit=10&cursor=c-1_%3D"
    );
    let patches = find("PATCH", "/api/v3/assets/");
    assert_eq!(
        patches[0].1,
        format!("/api/v3/assets/{ID}"),
        "ids are sent canonical"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&patches[0].2).unwrap(),
        json!({"label": "New"})
    );
    assert_eq!(
        serde_json::from_str::<Value>(&patches[1].2).unwrap(),
        json!({"label": null})
    );
    let forks = find("POST", "/api/v2/catalog/documents/");
    assert_eq!(
        serde_json::from_str::<Value>(&forks[0].2).unwrap(),
        json!({"revision": 4})
    );
    assert_eq!(forks[1].2, "", "no revision sends an empty body");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn blocked_deletes_and_fork_failures_are_distinct_typed_errors() {
    let (auth, _seen, dir) = serve(|method, target, _| match (method, target) {
        (_, "/api/v2/capabilities") => (200, capabilities()),
        ("DELETE", t) if t.starts_with("/api/v3/assets/") => {
            (409, json!({"code":"conflict","requestId":"r-1"}))
        }
        ("DELETE", t) if t.starts_with("/api/v3/overlays/") => (409, json!({"code":"conflict"})),
        ("DELETE", t) if t.starts_with("/api/v3/packages/") => (404, json!({"code":"notFound"})),
        ("POST", t) if t.ends_with("/fork") && t.contains("22222222") => (409, json!({})),
        ("POST", t) if t.ends_with("/fork") && t.contains("33333333") => (507, json!({})),
        ("POST", t) if t.ends_with("/fork") && t.contains("44444444") => (429, json!({})),
        ("POST", t) if t.ends_with("/fork") => (404, json!({})),
        _ => (500, json!({})),
    })
    .await;
    let asset = delete_asset(&auth, AssetIdParams { id: ID.into() })
        .await
        .unwrap_err();
    assert_eq!(asset.code, "conflict");
    assert_eq!(asset.data.as_ref().unwrap()["reason"], "assetReferenced");
    assert_eq!(asset.data.unwrap()["cloud"]["requestId"], "r-1");
    let overlay = delete_overlay(&auth, OverlayIdParams { id: ID.into() })
        .await
        .unwrap_err();
    assert_eq!(overlay.code, "conflict");
    assert_eq!(overlay.data.unwrap()["reason"], "overlayPinned");
    assert_eq!(
        delete_package(&auth, PackageIdParams { id: ID.into() })
            .await
            .unwrap_err()
            .code,
        "not_found"
    );
    let fork = |id: &str| CatalogForkParams {
        id: id.into(),
        revision: None,
    };
    let missing = fork_catalog_document(&auth, fork(ID)).await.unwrap_err();
    assert_eq!(
        (missing.code, missing.data.unwrap()["reason"].clone()),
        ("not_found", json!("notPublished"))
    );
    let owned = fork_catalog_document(&auth, fork("22222222-2222-4222-8222-222222222222"))
        .await
        .unwrap_err();
    assert_eq!(
        (owned.code, owned.data.unwrap()["reason"].clone()),
        ("conflict", json!("alreadyInLibrary"))
    );
    let full = fork_catalog_document(&auth, fork("33333333-3333-4333-8333-333333333333"))
        .await
        .unwrap_err();
    assert_eq!(
        (full.code, full.data.unwrap()["reason"].clone()),
        ("quota_exceeded", json!("libraryQuota"))
    );
    let limited = fork_catalog_document(&auth, fork("44444444-4444-4444-8444-444444444444"))
        .await
        .unwrap_err();
    assert_eq!(
        (limited.code, limited.data.unwrap()["reason"].clone()),
        ("unavailable", json!("rateLimited"))
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn quota_and_set_label_wire_shapes() {
    let error = map_failure(
        StatusCode::INSUFFICIENT_STORAGE,
        br#"{"code":"quotaExceeded"}"#,
    );
    assert_eq!(error.code, "quota_exceeded");
    // The label key is required: omitting it must not silently clear.
    assert!(serde_json::from_value::<AssetSetLabelParams>(json!({"id": ID})).is_err());
    assert!(
        serde_json::from_value::<AssetSetLabelParams>(json!({"id": ID, "label": null})).is_ok()
    );
}
