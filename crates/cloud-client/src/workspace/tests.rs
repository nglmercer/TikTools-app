//! Exercise v2 through real HTTP sockets, including native authentication and
//! conditional writes. The fixture never opens an application or sync store.
mod write_epoch;
use super::*;
use std::collections::BTreeMap;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

#[derive(Debug)]
struct Request {
    method: String,
    target: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    chunked: bool,
}
impl Reply {
    fn json(value: Value) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: serde_json::to_vec(&value).unwrap(),
            chunked: false,
        }
    }
    fn status(status: u16) -> Self {
        Self {
            status,
            ..Self::json(json!({}))
        }
    }
    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}
async fn request(socket: &mut TcpStream) -> Request {
    let mut bytes = Vec::new();
    let end = loop {
        let mut chunk = [0; 4096];
        let read = socket.read(&mut chunk).await.unwrap();
        assert!(read > 0, "HTTP request ended before its headers");
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(position) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
            break position + 4;
        }
        assert!(bytes.len() <= 16 * 1024, "oversized HTTP headers");
    };
    let header = std::str::from_utf8(&bytes[..end]).unwrap();
    let mut lines = header.split("\r\n");
    let mut first = lines.next().unwrap().split_whitespace();
    let method = first.next().unwrap().to_owned();
    let target = first.next().unwrap().to_owned();
    let headers: BTreeMap<_, _> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>().unwrap())
        .unwrap_or(0);
    assert!(length <= portable::MAX_DOCUMENT_BYTES + 1024);
    while bytes.len() < end + length {
        let mut chunk = [0; 4096];
        let read = socket.read(&mut chunk).await.unwrap();
        assert!(read > 0, "HTTP request ended before its body");
        bytes.extend_from_slice(&chunk[..read]);
    }
    Request {
        method,
        target,
        headers,
        body: bytes[end..end + length].to_vec(),
    }
}
async fn mock(replies: Vec<Reply>) -> (String, JoinHandle<Vec<Request>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for reply in replies {
            let (mut socket, _) =
                tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                    .await
                    .expect("client did not send expected request")
                    .unwrap();
            requests.push(request(&mut socket).await);
            let mut head = format!(
                "HTTP/1.1 {} Fixture\r\nContent-Type: application/json\r\nConnection: close\r\n",
                reply.status
            );
            for (name, value) in &reply.headers {
                head.push_str(&format!("{name}: {value}\r\n"));
            }
            if reply.chunked {
                head.push_str("Transfer-Encoding: chunked\r\n\r\n");
            } else if !reply
                .headers
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            {
                head.push_str(&format!("Content-Length: {}\r\n\r\n", reply.body.len()));
            } else {
                head.push_str("\r\n");
            }
            // A bounded decoder may close before the fixture finishes its body.
            if socket.write_all(head.as_bytes()).await.is_ok() {
                if reply.chunked {
                    for chunk in reply.body.chunks(1024) {
                        if socket
                            .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                            .await
                            .is_err()
                            || socket.write_all(chunk).await.is_err()
                            || socket.write_all(b"\r\n").await.is_err()
                        {
                            break;
                        }
                    }
                    let _ = socket.write_all(b"0\r\n\r\n").await;
                } else {
                    let _ = socket.write_all(&reply.body).await;
                }
            }
        }
        requests
    });
    (origin, task)
}
fn record(revision: i64, value: Option<Value>) -> Value {
    json!({"collection":"preferences","key":"desktop","revision":revision,"deleted":value.is_none(),"value":value})
}
fn preference(theme: &str) -> Value {
    json!({"schemaVersion":1,"theme":theme})
}

fn workspace_capability_document() -> Value {
    json!({
        "apiVersion": 2, "instanceMode": "single", "userConfigurableRegistries": false,
        "sync": {"basePath": "/api/v2/sync", "schemaVersion": 2,
            "collections": portable::COLLECTIONS, "maxRecords": 1000,
            "maxBytes": 8 * 1024 * 1024, "maxDocumentBytes": portable::MAX_DOCUMENT_BYTES,
            "conflictStatus": 409, "expiredCursorStatus": 410, "preconditionsRequired": true},
        "catalog": {"ignored": "additional capabilities remain extensible"}
    })
}

#[tokio::test]
async fn workspace_capability_discovery_is_anonymous_and_checks_contract() {
    let good = workspace_capability_document();
    let mut wrong_precondition = good.clone();
    wrong_precondition["sync"]["preconditionsRequired"] = json!(false);
    let mut wrong_collections = good.clone();
    wrong_collections["sync"]["collections"] = json!(["preferences", "preferences"]);
    let mut wrong_path = good.clone();
    wrong_path["sync"]["basePath"] = json!("https://attacker.invalid/api/v2/sync");
    let mut oversized = good.clone();
    oversized["padding"] = json!("x".repeat(64 * 1024));
    let (origin, server) = mock(vec![
        Reply::json(good),
        Reply::json(wrong_precondition),
        Reply::json(wrong_collections),
        Reply::json(wrong_path),
        Reply::json(oversized),
        Reply::status(404),
    ])
    .await;
    let catalog = CloudCatalog::new(&origin).unwrap();
    let capabilities = catalog.workspace_sync_capabilities().await.unwrap();
    assert_eq!(capabilities.max_records, 1000);
    assert_eq!(
        capabilities.max_document_bytes,
        portable::MAX_DOCUMENT_BYTES as u32
    );
    for _ in 0..4 {
        assert!(matches!(
            catalog.workspace_sync_capabilities().await,
            Err(SyncError::Protocol)
        ));
    }
    assert!(matches!(
        catalog.workspace_sync_capabilities().await,
        Err(SyncError::Http(404))
    ));
    for request in server.await.unwrap() {
        assert_eq!(request.method, "GET");
        assert_eq!(request.target, "/api/v2/capabilities");
        assert!(!request.headers.contains_key("authorization"));
        assert!(!request.headers.contains_key("cookie"));
        assert!(request.body.is_empty());
    }
}
fn template() -> Value {
    json!({"templateVersion":1,"id":"gift-rule","title":"Gift rule",
        "actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"Hello"}}],
        "event":{"name":"Chat","enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all"}})
}
fn document_meta(id: Uuid, revision: i64, status: &str) -> Value {
    json!({"id":id,"kind":"template","slug":"gift-rule","revision":revision,"title":"Gift rule","sha256":"a".repeat(64),"status":status})
}
fn document_view(id: Uuid, revision: i64, status: &str) -> Value {
    let mut value = document_meta(id, revision, status);
    value["document"] = template();
    value["sha256"] = json!(document_digest(&value["document"]));
    value
}
fn document_digest(value: &Value) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(serde_json::to_vec(value).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn plugin() -> Value {
    json!({"id":"acme.plugin","latestStableVersion":"1.0.0","metadataPath":"/api/v1/registry/generations/17/files/index/ac/me/acme.plugin.json",
        "versions":[{"version":"1.0.0","target":"any","sha256":"a".repeat(64),"size":100,"yanked":false,"keyRevoked":false,"publisherApproved":true,"advisory":null}]})
}
fn profile() -> Reply {
    Reply::json(json!({"id":"account-a"}))
}
async fn native_session(replies: Vec<Reply>) -> (Session, JoinHandle<Vec<Request>>) {
    let mut all = vec![profile()];
    all.extend(replies);
    let (origin, task) = mock(all).await;
    (
        Session::connect(&origin, "private-native-token".into())
            .await
            .unwrap(),
        task,
    )
}
fn native(requests: &[Request]) {
    for request in requests {
        assert_eq!(
            request.headers.get("authorization").map(String::as_str),
            Some("Bearer private-native-token")
        );
        assert!(!request.headers.contains_key("cookie"));
        assert!(!request.headers.contains_key("origin"));
    }
}

#[tokio::test]
async fn workspace_conditional_crud_keeps_v2_cursors_and_wire_preconditions() {
    let dark = preference("dark");
    let light = preference("light");
    let tombstone = record(9, None);
    let (session, task) = native_session(vec![
        Reply::json(record(3, Some(dark.clone()))),
        Reply::json(record(3, Some(dark.clone()))),
        Reply::json(record(8, Some(light.clone()))),
        Reply::json(tombstone.clone()),
        Reply::json(json!({"schemaVersion":2,"cursor":9,"records":[tombstone.clone()]})),
        Reply::json(json!({"changes":[tombstone],"cursor":9,"has_more":false})),
    ])
    .await;
    assert_eq!(
        session
            .workspace_put("preferences", "desktop", dark.clone(), None)
            .await
            .unwrap()
            .revision,
        3
    );
    assert_eq!(
        session
            .workspace_record("preferences", "desktop")
            .await
            .unwrap()
            .value,
        Some(dark.clone())
    );
    assert_eq!(
        session
            .workspace_put("preferences", "desktop", light.clone(), Some(3))
            .await
            .unwrap()
            .revision,
        8
    );
    assert!(
        session
            .workspace_delete("preferences", "desktop", 8)
            .await
            .unwrap()
            .deleted
    );
    assert_eq!(session.workspace_snapshot().await.unwrap().cursor, 9);
    assert_eq!(session.workspace_changes(8).await.unwrap().cursor, 9);
    let requests = task.await.unwrap();
    native(&requests);
    assert_eq!(requests[0].target, "/api/v1/native/me");
    for request in &requests[1..5] {
        assert_eq!(
            request.target,
            "/api/v2/sync/collections/preferences/desktop"
        );
    }
    assert_eq!(requests[1].method, "PUT");
    assert_eq!(requests[1].headers["if-none-match"], "*");
    assert!(!requests[1].headers.contains_key("if-match"));
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[1].body).unwrap(),
        dark
    );
    assert_eq!(requests[2].method, "GET");
    assert_eq!(requests[3].headers["if-match"], "\"3\"");
    assert!(!requests[3].headers.contains_key("if-none-match"));
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[3].body).unwrap(),
        light
    );
    assert_eq!(requests[4].method, "DELETE");
    assert_eq!(requests[4].headers["if-match"], "\"8\"");
    assert!(requests[4].body.is_empty());
    assert_eq!(requests[5].target, "/api/v2/sync/export");
    assert_eq!(requests[6].target, "/api/v2/sync/changes?cursor=8");
}

#[tokio::test]
async fn workspace_responses_reject_wrong_identity_body_and_regressing_revisions() {
    let dark = preference("dark");
    let mut wrong_key = record(8, Some(dark.clone()));
    wrong_key["key"] = json!("other");
    let mut wrong_collection = record(8, Some(dark.clone()));
    wrong_collection["collection"] = json!("profiles");
    let mut unknown = record(8, Some(dark.clone()));
    unknown["authority"] = json!(true);
    for body in [
        wrong_key,
        wrong_collection,
        unknown,
        record(8, Some(preference("light"))),
        record(8, None),
        record(6, Some(dark.clone())),
        record(0, Some(dark.clone())),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session
                .workspace_put("preferences", "desktop", dark.clone(), Some(7))
                .await,
            Err(SyncError::Protocol)
        ));
        assert_eq!(task.await.unwrap().len(), 2);
    }
    for body in [record(6, None), record(8, Some(dark.clone()))] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session.workspace_delete("preferences", "desktop", 7).await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    // An identical conditional write or repeated tombstone may preserve its revision.
    let (session, task) = native_session(vec![
        Reply::json(record(7, Some(dark.clone()))),
        Reply::json(record(7, None)),
    ])
    .await;
    assert_eq!(
        session
            .workspace_put("preferences", "desktop", dark, Some(7))
            .await
            .unwrap()
            .revision,
        7
    );
    assert_eq!(
        session
            .workspace_delete("preferences", "desktop", 7)
            .await
            .unwrap()
            .revision,
        7
    );
    task.await.unwrap();
}

#[tokio::test]
async fn workspace_snapshot_and_changes_validate_complete_identity_and_cursor_links() {
    let row = record(3, Some(preference("dark")));
    for body in [
        json!({"schemaVersion":1,"cursor":3,"records":[]}),
        json!({"schemaVersion":2,"cursor":-1,"records":[]}),
        json!({"schemaVersion":2,"cursor":2,"records":[row.clone()]}),
        json!({"schemaVersion":2,"cursor":3,"records":[row.clone(),row.clone()]}),
        json!({"schemaVersion":2,"cursor":3,"records":[],"origin":"https://other.example"}),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session.workspace_snapshot().await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    for body in [
        json!({"changes":[row.clone()],"cursor":2,"has_more":false}),
        json!({"changes":[row.clone(),row.clone()],"cursor":3,"has_more":false}),
        json!({"changes":[],"cursor":3,"has_more":false}),
        json!({"changes":[],"cursor":2,"has_more":true}),
        json!({"changes":[],"cursor":2,"hasMore":false}),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session.workspace_changes(2).await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    let (session, task) = native_session(vec![Reply::json(
        json!({"changes":[],"cursor":3,"has_more":false}),
    )])
    .await;
    assert_eq!(session.workspace_changes(3).await.unwrap().cursor, 3);
    task.await.unwrap();
}

#[tokio::test]
async fn invalid_portable_input_never_dispatches_or_selects_an_external_origin() {
    let (session, task) = native_session(vec![]).await;
    for (collection, key, value) in [
        ("other", "desktop", preference("dark")),
        ("preferences", "../desktop", preference("dark")),
        (
            "preferences",
            "desktop",
            json!({"schemaVersion":1,"theme":"dark","registryUrl":"https://other.example"}),
        ),
        (
            "plugin-settings",
            "acme.plugin",
            json!({"schemaVersion":1,"pluginId":"acme.plugin","settings":{"token":"literal-secret"}}),
        ),
    ] {
        assert!(matches!(
            session.workspace_put(collection, key, value, None).await,
            Err(SyncError::Configuration)
        ));
    }
    assert!(matches!(
        session.workspace_changes(-1).await,
        Err(SyncError::Configuration)
    ));
    assert!(matches!(
        session.workspace_delete("preferences", "desktop", 0).await,
        Err(SyncError::Configuration)
    ));
    assert!(matches!(
        session
            .workspace_put("preferences", "desktop", preference("dark"), Some(0))
            .await,
        Err(SyncError::Configuration)
    ));
    assert_eq!(task.await.unwrap().len(), 1);
    for origin in [
        "http://example.test",
        "https://cloud.example/path",
        "https://user:password@cloud.example",
        "https://cloud.example/?next=https://other.example",
    ] {
        assert!(matches!(
            CloudCatalog::new(origin),
            Err(SyncError::Configuration)
        ));
    }
}

#[tokio::test]
async fn catalog_is_anonymous_and_pins_all_paths_to_one_origin() {
    let id = Uuid::from_u128(1);
    let replies = vec![
        Reply::json(json!({"untrustedUrl":"https://other.example"})),
        Reply::json(json!({"generation":17,"items":[],"nextCursor":null})),
        Reply::json(json!({"generation":17,"plugin":plugin()})),
        Reply::json(json!({"items":[],"nextCursor":null})),
        Reply::json(document_view(id, 2, "published")),
    ];
    let (origin, task) = mock(replies).await;
    let catalog = CloudCatalog::new(&origin).unwrap();
    catalog.capabilities().await.unwrap();
    catalog
        .plugins("rose & gift", Some("acme.plugin"), Some(17))
        .await
        .unwrap();
    catalog.plugin("acme.plugin", Some(17)).await.unwrap();
    catalog.documents(Some("template"), Some(id)).await.unwrap();
    catalog.document(id, Some(2)).await.unwrap();
    let requests = task.await.unwrap();
    assert_eq!(requests[0].target, "/api/v2/capabilities");
    let parsed = url::Url::parse(&format!("{origin}{}", requests[1].target)).unwrap();
    let query: BTreeMap<_, _> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    assert_eq!(query["q"], "rose & gift");
    assert_eq!(query["after"], "acme.plugin");
    assert_eq!(query["generation"], "17");
    assert_eq!(
        requests[2].target,
        "/api/v2/catalog/plugins/acme.plugin?generation=17"
    );
    assert_eq!(
        requests[3].target,
        format!("/api/v2/catalog/documents?kind=template&after={id}")
    );
    assert_eq!(
        requests[4].target,
        format!("/api/v2/catalog/documents/{id}/versions/2")
    );
    for request in requests {
        assert_eq!(request.method, "GET");
        assert!(request.body.is_empty());
        assert!(!request.headers.contains_key("authorization"));
        assert!(!request.headers.contains_key("cookie"));
    }
}

#[tokio::test]
async fn native_and_anonymous_v2_requests_refuse_redirects_and_surface_server_outcomes() {
    let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!("http://{}/stolen", destination.local_addr().unwrap());
    let (session, task) = native_session(vec![
        Reply::status(302).header("Location", &location),
        Reply::status(401),
        Reply::status(409),
        Reply::status(410),
        Reply::status(429).header("Retry-After", "60"),
        Reply::status(507),
    ])
    .await;
    assert!(matches!(
        session.workspace_snapshot().await,
        Err(SyncError::Http(302))
    ));
    assert!(matches!(
        session.workspace_snapshot().await,
        Err(SyncError::Unauthorized)
    ));
    assert!(matches!(
        session.workspace_snapshot().await,
        Err(SyncError::Conflict)
    ));
    assert!(matches!(
        session.workspace_snapshot().await,
        Err(SyncError::CursorExpired)
    ));
    assert!(matches!(
        session.workspace_snapshot().await,
        Err(SyncError::RateLimited {
            retry_after_seconds: Some(60)
        })
    ));
    assert!(matches!(
        session.workspace_snapshot().await,
        Err(SyncError::Http(507))
    ));
    native(&task.await.unwrap());
    let (origin, task) = mock(vec![Reply::status(307).header("Location", &location)]).await;
    assert!(matches!(
        CloudCatalog::new(&origin).unwrap().capabilities().await,
        Err(SyncError::Http(307))
    ));
    task.await.unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), destination.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn v2_response_limit_checks_declared_and_streamed_bytes() {
    let over = (MAX_V2_RESPONSE + 1).to_string();
    let (session, task) =
        native_session(vec![Reply::status(200).header("Content-Length", &over)]).await;
    assert!(matches!(
        session.workspace_snapshot().await,
        Err(SyncError::Protocol)
    ));
    task.await.unwrap();
    // Use the exact same streaming decoder with a small bound so this regression
    // proves chunk accounting without allocating a 32 MiB test document.
    let mut reply = Reply::json(json!({"source":"x".repeat(4096)}));
    reply.chunked = true;
    let (origin, task) = mock(vec![reply]).await;
    let response = reqwest::Client::new().get(origin).send().await.unwrap();
    assert!(matches!(
        decode_with_limit::<Value>(response, 1024).await,
        Err(SyncError::Protocol)
    ));
    task.await.unwrap();
}

#[tokio::test]
async fn v2_account_export_rejects_other_account_and_invalid_workspace() {
    for body in [
        json!({"schemaVersion":1,"account":{"profile":{"id":"account-a"}},"workspaceSync":{"schemaVersion":2,"cursor":0,"records":[]}}),
        json!({"schemaVersion":2,"account":{"profile":{"id":"account-b"}},"workspaceSync":{"schemaVersion":2,"cursor":0,"records":[]}}),
        json!({"schemaVersion":2,"account":{"profile":{"id":"account-a"}},"workspaceSync":{"schemaVersion":2,"cursor":0,"records":[record(1,None)]}}),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session.export_account_v2().await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    let valid = json!({"schemaVersion":2,"account":{"profile":{"id":"account-a"}},"workspaceSync":{"schemaVersion":2,"cursor":0,"records":[]},"library":{"items":[]}});
    let (session, task) = native_session(vec![Reply::json(valid.clone())]).await;
    assert_eq!(session.export_account_v2().await.unwrap(), valid);
    assert_eq!(task.await.unwrap()[1].target, "/api/v2/me/export");
}

#[tokio::test]
async fn private_library_mutations_use_native_session_and_strong_revision_headers() {
    let id = Uuid::from_u128(3);
    let document = template();
    let created = json!({"id":id,"revision":1,"status":"draft"});
    let (session, task) = native_session(vec![
        Reply::json(created.clone()),
        Reply::json(json!({"items":[],"nextCursor":null})),
        Reply::json(document_view(id, 1, "draft")),
        Reply::json(json!({"id":id,"revision":2,"status":"draft"})),
        Reply::status(204),
        Reply::status(204),
        Reply::status(204),
    ])
    .await;
    assert_eq!(
        session
            .library_create("template", document.clone())
            .await
            .unwrap(),
        created
    );
    session.library_documents(Some(id)).await.unwrap();
    session.library_document(id).await.unwrap();
    session
        .library_update(id, "template", document.clone(), 1)
        .await
        .unwrap();
    session.library_submit(id, 1).await.unwrap();
    session
        .library_withdraw(id, 1, "Replace outdated guidance")
        .await
        .unwrap();
    session.library_delete(id, 2).await.unwrap();
    let requests = task.await.unwrap();
    native(&requests);
    assert_eq!(requests[1].method, "POST");
    assert_eq!(requests[1].target, "/api/v2/library/documents");
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[1].body).unwrap(),
        json!({"kind":"template","document":document})
    );
    assert_eq!(
        requests[2].target,
        format!("/api/v2/library/documents?after={id}")
    );
    assert_eq!(
        requests[3].target,
        format!("/api/v2/library/documents/{id}")
    );
    assert_eq!(requests[4].method, "PUT");
    assert_eq!(requests[4].headers["if-match"], "\"1\"");
    assert_eq!(
        requests[5].target,
        format!("/api/v2/library/documents/{id}/versions/1/submit")
    );
    assert_eq!(
        requests[6].target,
        format!("/api/v2/library/documents/{id}/versions/1/withdraw")
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[6].body).unwrap(),
        json!({"reason":"Replace outdated guidance"})
    );
    assert_eq!(requests[7].method, "DELETE");
    assert_eq!(
        requests[7].target,
        format!("/api/v2/library/documents/{id}")
    );
    assert_eq!(requests[7].headers["if-match"], "\"2\"");
    assert!(!requests[7].headers.contains_key("if-none-match"));
}

#[tokio::test]
async fn filtered_public_catalog_uses_target_query_and_monotonic_pagination() {
    let first = Uuid::from_u128(20);
    let last = Uuid::from_u128(70);
    let items: Vec<_> = (21..=70)
        .map(|n| document_meta(Uuid::from_u128(n), 1, "published"))
        .collect();
    let (origin, task) = mock(vec![
        Reply::json(json!({"generation":17,"items":[plugin()],"nextCursor":null})),
        Reply::json(json!({"generation":17,"plugin":plugin()})),
        Reply::json(json!({"items":items,"nextCursor":last})),
        Reply::json(json!({"items":[],"nextCursor":null})),
    ])
    .await;
    let catalog = CloudCatalog::new(&origin).unwrap();
    catalog
        .plugins_filtered(
            "gift & chat",
            None,
            Some(17),
            Some("x86_64-unknown-linux-gnu"),
        )
        .await
        .unwrap();
    catalog
        .plugin_for_target("acme.plugin", Some(17), Some("x86_64-unknown-linux-gnu"))
        .await
        .unwrap();
    catalog
        .documents_filtered(Some("template"), Some("gift & chat"), Some(first))
        .await
        .unwrap();
    catalog
        .documents_filtered(Some("template"), Some("gift & chat"), Some(last))
        .await
        .unwrap();
    let requests = task.await.unwrap();
    for (index, target) in [
        (0, "/api/v2/catalog/plugins"),
        (1, "/api/v2/catalog/plugins/acme.plugin"),
    ] {
        let url = url::Url::parse(&format!("{origin}{}", requests[index].target)).unwrap();
        assert_eq!(url.path(), target);
        assert_eq!(
            url.query_pairs()
                .find(|(key, _)| key == "target")
                .unwrap()
                .1,
            "x86_64-unknown-linux-gnu"
        );
    }
    for (index, after) in [(2, first), (3, last)] {
        let url = url::Url::parse(&format!("{origin}{}", requests[index].target)).unwrap();
        let query: BTreeMap<_, _> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(query["q"], "gift & chat");
        assert_eq!(query["after"], after.to_string());
        assert_eq!(query["kind"], "template");
    }
    assert!(requests
        .iter()
        .all(|request| !request.headers.contains_key("authorization")));
}

#[tokio::test]
async fn workspace_head_and_key_pages_are_strict_and_keep_collection_identity() {
    let records:Vec<_>=(0..50).map(|n|{
        let key=format!("acme.plugin{n:02}");
        json!({"collection":"plugin-selections","key":key,"revision":n+1,"deleted":false,"value":{"schemaVersion":1,"pluginId":key,"version":"1.0.0","enabled":true}})
    }).collect();
    let (session, task) = native_session(vec![
        Reply::json(json!({"cursor":123})),
        Reply::json(json!({"records":records,"nextAfter":"acme.plugin49"})),
        Reply::json(json!({"records":[],"nextAfter":null})),
    ])
    .await;
    assert!(session.instance_origin().starts_with("http://127.0.0.1:"));
    assert_eq!(session.account_id(), "account-a");
    assert_eq!(session.workspace_head().await.unwrap(), 123);
    assert_eq!(
        session
            .workspace_collection("plugin-selections", None)
            .await
            .unwrap()["records"]
            .as_array()
            .unwrap()
            .len(),
        50
    );
    session
        .workspace_collection("plugin-selections", Some("acme.plugin49"))
        .await
        .unwrap();
    let requests = task.await.unwrap();
    native(&requests);
    assert_eq!(requests[1].target, "/api/v2/sync/head");
    assert_eq!(
        requests[2].target,
        "/api/v2/sync/collections/plugin-selections"
    );
    assert_eq!(
        requests[3].target,
        "/api/v2/sync/collections/plugin-selections?after=acme.plugin49"
    );
    for body in [
        json!({"cursor":-1}),
        json!({"cursor":1,"accountId":"other"}),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session.workspace_head().await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    for body in [
        json!({"records":[record(1,None)],"nextAfter":"desktop"}),
        json!({"records":[record(1,None),record(1,None)],"nextAfter":null}),
        json!({"records":[],"nextAfter":null,"cursor":1}),
        json!({"records":[]}),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session.workspace_collection("preferences", None).await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    let (session, task) = native_session(vec![Reply::json(
        json!({"records":[record(1,None)],"nextAfter":null}),
    )])
    .await;
    assert!(matches!(
        session.workspace_collection("plugin-settings", None).await,
        Err(SyncError::Protocol)
    ));
    task.await.unwrap();
}

#[tokio::test]
async fn private_document_filters_versions_and_conditional_delete_follow_actual_routes() {
    let id = Uuid::from_u128(4);
    let (session,task)=native_session(vec![
        Reply::json(json!({"items":[document_meta(id,3,"draft")],"nextCursor":null})),
        Reply::json(json!({"items":[document_meta(id,2,"published"),document_meta(id,3,"draft")],"nextCursor":null})),
        Reply::json(document_view(id,2,"published")),
        Reply::status(204),
    ]).await;
    session
        .library_documents_filtered(
            Some("template"),
            Some("gift & chat"),
            Some(Uuid::from_u128(3)),
        )
        .await
        .unwrap();
    session.library_versions(id, Some(1)).await.unwrap();
    session.library_version(id, 2).await.unwrap();
    session.library_delete(id, 3).await.unwrap();
    let requests = task.await.unwrap();
    native(&requests);
    let origin = session.instance_origin();
    let url = url::Url::parse(&format!("{origin}{}", requests[1].target)).unwrap();
    assert_eq!(url.path(), "/api/v2/library/documents");
    assert_eq!(
        url.query_pairs().find(|(k, _)| k == "q").unwrap().1,
        "gift & chat"
    );
    assert_eq!(
        requests[2].target,
        format!("/api/v2/library/documents/{id}/versions?after=1")
    );
    assert_eq!(
        requests[3].target,
        format!("/api/v2/library/documents/{id}/versions/2")
    );
    assert_eq!(requests[4].headers["if-match"], "\"3\"");
    assert_eq!(requests[4].method, "DELETE");
}

#[tokio::test]
async fn document_pages_and_reads_reject_malformed_metadata_identity_and_source() {
    let id = Uuid::from_u128(4);
    for body in [
        json!({"items":[document_meta(id,1,"draft"),document_meta(id,1,"draft")],"nextCursor":null}),
        json!({"items":[],"nextCursor":id}),
        json!({"items":[]}),
        json!({"items":[],"nextCursor":null,"authority":true}),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session
                .library_documents_filtered(Some("template"), None, None)
                .await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    let wrong_id = document_view(Uuid::from_u128(5), 2, "draft");
    let mut wrong_source = document_view(id, 2, "draft");
    wrong_source["document"]["id"] = json!("another-rule");
    let mut extra = document_view(id, 2, "draft");
    extra["trusted"] = json!(true);
    let mut credential = document_view(id, 2, "draft");
    credential["document"]["actions"][0]["config"]["token"] = json!("literal-private-token");
    let mut bad_hash = document_view(id, 2, "draft");
    bad_hash["sha256"] = json!("not-a-digest");
    for body in [
        wrong_id,
        wrong_source,
        extra,
        credential,
        bad_hash,
        document_view(id, 1, "draft"),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session.library_version(id, 2).await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    for body in [
        json!({"items":[document_meta(id,2,"draft"),document_meta(id,2,"draft")],"nextCursor":null}),
        json!({"items":[document_meta(Uuid::from_u128(5),2,"draft")],"nextCursor":null}),
        json!({"items":[],"nextCursor":2}),
    ] {
        let (session, task) = native_session(vec![Reply::json(body)]).await;
        assert!(matches!(
            session.library_versions(id, Some(1)).await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn catalog_uses_manifest_ids_and_v2_sync_supports_legacy_plugins() {
    for id in [
        "foo".to_owned(),
        "hotkeys".to_owned(),
        "textintel".to_owned(),
        "a..b".to_owned(),
        "ab..c".to_owned(),
        format!("a{}", "b".repeat(127)),
    ] {
        let chars: Vec<_> = id.chars().collect();
        let shard = |start| {
            (start..start + 2)
                .map(|index| chars.get(index).copied().unwrap_or('_'))
                .collect::<String>()
        };
        let mut entry = plugin();
        entry["id"] = json!(id);
        let path = if shard(2) == ".." {
            format!("index/{id}.json")
        } else {
            format!("index/{}/{}/{id}.json", shard(0), shard(2))
        };
        entry["metadataPath"] = json!(format!("/api/v1/registry/generations/17/files/{path}"));
        let (origin, task) = mock(vec![
            Reply::json(json!({"generation":17,"items":[entry.clone()],"nextCursor":null})),
            Reply::json(json!({"generation":17,"plugin":entry})),
            Reply::json(json!({"generation":17,"items":[],"nextCursor":null})),
        ])
        .await;
        let catalog = CloudCatalog::new(&origin).unwrap();
        assert!(catalog.plugins("", None, Some(17)).await.is_ok());
        assert!(catalog.plugin(&id, Some(17)).await.is_ok());
        assert!(catalog.plugins("", Some(&id), Some(17)).await.is_ok());
        assert_eq!(task.await.unwrap().len(), 3);
    }
    assert!(portable::valid_key("plugin-selections", "hotkeys"));
    assert!(portable::valid_key("plugin-selections", "a..b"));
    let mut unsafe_shard = plugin();
    unsafe_shard["id"] = json!("ab..c");
    unsafe_shard["metadataPath"] =
        json!("/api/v1/registry/generations/17/files/index/ab/../ab..c.json");
    let (origin, task) = mock(vec![Reply::json(
        json!({"generation":17,"items":[unsafe_shard],"nextCursor":null}),
    )])
    .await;
    assert!(matches!(
        CloudCatalog::new(&origin)
            .unwrap()
            .plugins("", None, Some(17))
            .await,
        Err(SyncError::Protocol)
    ));
    task.await.unwrap();
    for id in [
        "1foo".to_owned(),
        "A.foo".to_owned(),
        "a".to_owned(),
        format!("a{}", "b".repeat(128)),
    ] {
        let (origin, task) = mock(vec![]).await;
        let catalog = CloudCatalog::new(&origin).unwrap();
        assert!(matches!(
            catalog.plugin(&id, Some(17)).await,
            Err(SyncError::Configuration)
        ));
        assert!(matches!(
            catalog.plugins("", Some(&id), Some(17)).await,
            Err(SyncError::Configuration)
        ));
        assert!(task.await.unwrap().is_empty());
        let mut entry = plugin();
        entry["id"] = json!(id);
        let (origin, task) = mock(vec![Reply::json(
            json!({"generation":17,"items":[entry],"nextCursor":null}),
        )])
        .await;
        assert!(matches!(
            CloudCatalog::new(&origin)
                .unwrap()
                .plugins("", None, Some(17))
                .await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn catalog_accepts_release_semver_up_to_backend_128_byte_limit() {
    for length in [64, 128, 129] {
        let version = format!("1.0.0+{}", "a".repeat(length - 6));
        assert_eq!(version.len(), length);
        assert!(semver::Version::parse(&version).is_ok());
        let mut entry = plugin();
        entry["latestStableVersion"] = json!(version);
        entry["versions"][0]["version"] = json!(version);
        let (origin, task) = mock(vec![
            Reply::json(json!({"generation":17,"items":[entry.clone()],"nextCursor":null})),
            Reply::json(json!({"generation":17,"plugin":entry})),
        ])
        .await;
        let catalog = CloudCatalog::new(&origin).unwrap();
        let page = catalog.plugins("", None, Some(17)).await;
        let detail = catalog.plugin("acme.plugin", Some(17)).await;
        if length <= 128 {
            assert!(page.is_ok(), "page should accept {length}-byte version");
            assert!(detail.is_ok(), "detail should accept {length}-byte version");
        } else {
            assert!(matches!(page, Err(SyncError::Protocol)));
            assert!(matches!(detail, Err(SyncError::Protocol)));
        }
        assert_eq!(task.await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn filtered_catalog_rejects_generation_cursor_target_and_trust_path_drift() {
    let mut bad_path = plugin();
    bad_path["metadataPath"] = json!("https://other.example/root.json");
    let mut wrong_target = plugin();
    wrong_target["versions"][0]["target"] = json!("aarch64-apple-darwin");
    let mut wrong_latest = plugin();
    wrong_latest["latestStableVersion"] = json!("2.0.0");
    for body in [
        json!({"generation":16,"items":[plugin()],"nextCursor":null}),
        json!({"generation":17,"items":[plugin(),plugin()],"nextCursor":null}),
        json!({"generation":17,"items":[],"nextCursor":"acme.plugin"}),
        json!({"generation":17,"items":[bad_path],"nextCursor":null}),
        json!({"generation":17,"items":[wrong_target],"nextCursor":null}),
        json!({"generation":17,"items":[wrong_latest],"nextCursor":null}),
        json!({"generation":17,"items":[],"nextCursor":null,"rootKeys":[]}),
    ] {
        let (origin, task) = mock(vec![Reply::json(body)]).await;
        assert!(matches!(
            CloudCatalog::new(&origin)
                .unwrap()
                .plugins_filtered("", None, Some(17), Some("x86_64-unknown-linux-gnu"))
                .await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    let (origin, task) = mock(vec![Reply::json(
        json!({"items":[document_meta(Uuid::from_u128(4),1,"draft")],"nextCursor":null}),
    )])
    .await;
    assert!(matches!(
        CloudCatalog::new(&origin)
            .unwrap()
            .documents_filtered(None, None, None)
            .await,
        Err(SyncError::Protocol)
    ));
    task.await.unwrap();
}

#[tokio::test]
async fn added_methods_fail_closed_before_dispatch_and_require_real_no_content_success() {
    let id = Uuid::from_u128(4);
    let (session, task) = native_session(vec![]).await;
    assert!(matches!(
        session.workspace_collection("unknown", None).await,
        Err(SyncError::Configuration)
    ));
    assert!(matches!(
        session
            .workspace_collection("preferences", Some("../desktop"))
            .await,
        Err(SyncError::Configuration)
    ));
    assert!(matches!(
        session
            .library_documents_filtered(Some("unknown"), None, None)
            .await,
        Err(SyncError::Configuration)
    ));
    assert!(matches!(
        session
            .library_documents_filtered(None, Some(&"x".repeat(101)), None)
            .await,
        Err(SyncError::Configuration)
    ));
    assert!(matches!(
        session.library_versions(id, Some(-1)).await,
        Err(SyncError::Configuration)
    ));
    assert!(matches!(
        session.library_version(id, 0).await,
        Err(SyncError::Configuration)
    ));
    assert!(matches!(
        session.library_delete(id, 0).await,
        Err(SyncError::Configuration)
    ));
    let catalog = CloudCatalog::new(&session.instance_origin()).unwrap();
    assert!(matches!(
        catalog
            .plugins_filtered("", None, None, Some("../native"))
            .await,
        Err(SyncError::Configuration)
    ));
    task.await.unwrap();
    for operation in ["delete", "submit", "withdraw"] {
        let (session, task) = native_session(vec![Reply::json(json!({"ok":true}))]).await;
        let result = match operation {
            "delete" => session.library_delete(id, 1).await,
            "submit" => session.library_submit(id, 1).await,
            _ => session.library_withdraw(id, 1, "Reviewed reason").await,
        };
        assert!(matches!(result, Err(SyncError::Protocol)));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn library_body_digest_links_private_public_and_exported_source() {
    let id = Uuid::from_u128(41);
    let mut original = document_view(id, 2, "published");
    // The backend hashes serde_json's bytes, retaining float/integer distinctions
    // and exact native integers; this is not the Owner library canonical hash.
    original["document"]["actions"][0]["config"]=serde_json::from_str(r#"{"float":1.0,"tiny":1e-6,"exponent":1e21,"normalizedIntegerExponent":1000000000000000000,"normalizedZero":0.0,"largeInteger":18446744073709551615,"unicode":"ñ\\\n","array":[2,1]}"#).unwrap();
    original["sha256"] = json!(document_digest(&original["document"]));
    let encoded = serde_json::to_vec(&original["document"]).unwrap();
    assert!(std::str::from_utf8(&encoded)
        .unwrap()
        .contains("18446744073709551615"));
    assert_eq!(
        original["document"]["actions"][0]["config"]["largeInteger"].as_u64(),
        Some(u64::MAX)
    );
    let (session,task)=native_session(vec![Reply::json(original.clone()),Reply::json(original.clone()),Reply::json(json!({"schemaVersion":2,"account":{"profile":{"id":"account-a"}},"workspaceSync":{"schemaVersion":2,"cursor":0,"records":[]},"library":{"items":[original.clone()]}}))]).await;
    assert_eq!(session.library_document(id).await.unwrap(), original);
    assert_eq!(session.library_version(id, 2).await.unwrap(), original);
    assert_eq!(
        session.export_account_v2().await.unwrap()["library"]["items"][0],
        original
    );
    task.await.unwrap();
    let (origin, task) = mock(vec![Reply::json(original.clone())]).await;
    assert_eq!(
        CloudCatalog::new(&origin)
            .unwrap()
            .document(id, Some(2))
            .await
            .unwrap(),
        original
    );
    task.await.unwrap();

    let mut changed_body = original.clone();
    changed_body["document"]["actions"][0]["config"]["array"] = json!([1, 2]);
    let mut changed_hash = original.clone();
    changed_hash["sha256"] = json!("0".repeat(64));
    // A numerically equal integer encoding is a different body under this
    // backend protocol. Do not silently substitute the local ECMAScript hash.
    let mut changed_numeric_encoding = original.clone();
    changed_numeric_encoding["document"]["actions"][0]["config"]["float"] = json!(1);
    assert_ne!(
        document_digest(&changed_numeric_encoding["document"]),
        original["sha256"].as_str().unwrap()
    );
    for value in [changed_body, changed_hash, changed_numeric_encoding] {
        let (session,task)=native_session(vec![Reply::json(value.clone()),Reply::json(json!({"schemaVersion":2,"account":{"profile":{"id":"account-a"}},"workspaceSync":{"schemaVersion":2,"cursor":0,"records":[]},"library":{"items":[value.clone()]}}))]).await;
        assert!(matches!(
            session.library_version(id, 2).await,
            Err(SyncError::Protocol)
        ));
        assert!(matches!(
            session.export_account_v2().await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
        let (origin, task) = mock(vec![Reply::json(value)]).await;
        assert!(matches!(
            CloudCatalog::new(&origin).unwrap().document(id, None).await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn workspace_write_acknowledgement_accepts_jsonb_decimal_normalization_without_rounding() {
    let sent:Value=serde_json::from_str(r#"{"schemaVersion":1,"pluginId":"acme.plugin","settings":{"exponent":1e18,"precise":1.0000000000000001e18,"large":18446744073709551615,"nested":[-0.0,1.0]}}"#).unwrap();
    let normalized:Value=serde_json::from_str(r#"{"schemaVersion":1,"pluginId":"acme.plugin","settings":{"exponent":1000000000000000000,"precise":1000000000000000100,"large":18446744073709551615,"nested":[0.0,1]}}"#).unwrap();
    let response = |value: Value| json!({"collection":"plugin-settings","key":"acme.plugin","revision":7,"deleted":false,"value":value});
    let (session, task) = native_session(vec![Reply::json(response(normalized.clone()))]).await;
    let acknowledged = session
        .workspace_put("plugin-settings", "acme.plugin", sent.clone(), None)
        .await
        .unwrap();
    assert_eq!(acknowledged.value, Some(normalized.clone()));
    assert!(portable::values_equivalent(&sent, &normalized));
    let requests = task.await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[1].body).unwrap(),
        sent
    );
    let mut rounded = normalized.clone();
    rounded["settings"]["large"] = serde_json::from_str("1.8446744073709552e19").unwrap();
    let mut binary_cast = normalized;
    binary_cast["settings"]["precise"] = json!(1_000_000_000_000_000_128_u64);
    for invalid in [rounded, binary_cast] {
        let (session, task) = native_session(vec![Reply::json(response(invalid))]).await;
        assert!(matches!(
            session
                .workspace_put("plugin-settings", "acme.plugin", sent.clone(), None)
                .await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
}

#[tokio::test]
async fn bounded_catalog_pages_validate_cursors_and_keep_integer_precision() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let encode = |value: Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).unwrap());
    let summary_cursor = encode(
        json!({"kind":"summary","generation":9007199254740993_i64,"q":"acme","target":"linux","last":"acme.plugin"}),
    );
    let version_cursor = encode(
        json!({"kind":"versions","generation":9007199254740993_i64,"id":"acme.plugin","target":"linux","version":"1.0.0","last_target":"any"}),
    );
    let path = "/api/v1/registry/generations/9007199254740993/files/index/ac/me/acme.plugin.json";
    let summary = |id: &str| json!({"id":id,"metadataPath":format!("/api/v1/registry/generations/9007199254740993/files/index/ac/me/{id}.json"),"latestStableVersion":"1.0.0"});
    let version = |number: &str| json!({"version":number,"target":"any","sha256":"a".repeat(64),"size":"9007199254740995","yanked":false,"keyRevoked":false,"publisherApproved":true,"advisory":null});
    let (origin, task) = mock(vec![
        Reply::json(json!({"schemaVersion":1,"summaryPath":"/api/v2/catalog/plugin-summaries","versionPagePath":"/api/v2/catalog/plugins/{id}/version-pages","maxItems":50,"maxBytes":524288,"versionOrder":"ascii-version-target"})),
        Reply::json(json!({"generation":"9007199254740993","items":[summary("acme.plugin")],"nextCursor":summary_cursor})),
        Reply::json(json!({"generation":"9007199254740993","items":[summary("acme.second")],"nextCursor":null})),
        Reply::json(json!({"generation":"9007199254740993","id":"acme.plugin","metadataPath":path,"items":[version("1.0.0")],"nextCursor":version_cursor})),
        Reply::json(json!({"generation":"9007199254740993","id":"acme.plugin","metadataPath":path,"items":[version("1.0.1")],"nextCursor":null})),
    ]).await;
    let catalog = CloudCatalog::new(&origin).unwrap();
    catalog.pagination_capabilities().await.unwrap();
    let first = catalog
        .plugin_summaries(Some("acme"), Some("linux"), None, None)
        .await
        .unwrap();
    assert_eq!(first.generation.as_deref(), Some("9007199254740993"));
    let next = first.next_cursor.as_deref().unwrap();
    assert!(matches!(
        catalog
            .plugin_summaries(
                Some("other"),
                Some("linux"),
                Some(9007199254740993),
                Some(next)
            )
            .await,
        Err(SyncError::Configuration)
    ));
    let second = catalog
        .plugin_summaries(
            Some("acme"),
            Some("linux"),
            Some(9007199254740993),
            Some(next),
        )
        .await
        .unwrap();
    assert_eq!(second.items[0].id, "acme.second");
    let first = catalog
        .plugin_version_page("acme.plugin", Some("linux"), Some(9007199254740993), None)
        .await
        .unwrap();
    assert_eq!(first.items[0].size, "9007199254740995");
    let next = first.next_cursor.as_deref().unwrap();
    assert!(matches!(
        catalog
            .plugin_version_page("other", Some("linux"), Some(9007199254740993), Some(next))
            .await,
        Err(SyncError::Configuration)
    ));
    let second = catalog
        .plugin_version_page(
            "acme.plugin",
            Some("linux"),
            Some(9007199254740993),
            Some(next),
        )
        .await
        .unwrap();
    assert_eq!(second.items[0].version, "1.0.1");
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests
        .iter()
        .all(|r| !r.headers.contains_key("authorization")));
    assert!(requests[2].target.contains("generation=9007199254740993"));
}

#[tokio::test]
async fn bounded_catalog_rejects_wrong_paths_extra_fields_and_oversized_bodies() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    let wrong_cursor = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(
            &json!({"kind":"summary","generation":18,"q":null,"target":null,"last":"hotkeys"}),
        )
        .unwrap(),
    );
    let cases = [
        json!({"generation":"17","items":[{"id":"ab..foo","metadataPath":"/api/v1/registry/generations/17/files/index/ab/../ab..foo.json","latestStableVersion":null}],"nextCursor":null}),
        json!({"generation":"17","items":[],"nextCursor":null,"extra":1}),
        json!({"generation":"017","items":[],"nextCursor":null}),
        json!({"generation":"17","items":[{"id":"hotkeys","metadataPath":"/api/v1/registry/generations/17/files/index/ho/tk/hotkeys.json","latestStableVersion":null}],"nextCursor":wrong_cursor}),
    ];
    for value in cases {
        let (origin, task) = mock(vec![Reply::json(value)]).await;
        assert!(matches!(
            CloudCatalog::new(&origin)
                .unwrap()
                .plugin_summaries(None, None, Some(17), None)
                .await,
            Err(SyncError::Protocol)
        ));
        task.await.unwrap();
    }
    let (origin, task) = mock(vec![Reply::json(json!({"generation":"17","items":[{"id":"ab..foo","metadataPath":"/api/v1/registry/generations/17/files/index/ab..foo.json","latestStableVersion":null}],"nextCursor":null}))]).await;
    assert!(CloudCatalog::new(&origin)
        .unwrap()
        .plugin_summaries(None, None, Some(17), None)
        .await
        .is_ok());
    task.await.unwrap();
    let (origin, task) = mock(vec![Reply::json(
        json!({"generation":"17","items":[],"nextCursor":null,"padding":"x".repeat(512*1024)}),
    )])
    .await;
    assert!(CloudCatalog::new(&origin)
        .unwrap()
        .plugin_summaries(None, None, Some(17), None)
        .await
        .is_err());
    task.await.unwrap();
}

#[tokio::test]
async fn variables_are_discovered_as_an_optional_collection_and_staged_through_the_normal_path() {
    // The legacy list stays exactly the five v2 collections; `variables` is
    // announced separately, and unknown optional names are ignored.
    let mut announced = workspace_capability_document();
    announced["sync"]["optionalCollections"] = json!(["variables", "future-collection"]);
    let (origin, server) = mock(vec![
        Reply::json(workspace_capability_document()),
        Reply::json(announced),
    ])
    .await;
    let catalog = CloudCatalog::new(&origin).unwrap();
    assert!(
        !catalog
            .workspace_sync_capabilities()
            .await
            .unwrap()
            .variables
    );
    assert!(
        catalog
            .workspace_sync_capabilities()
            .await
            .unwrap()
            .variables
    );
    server.await.unwrap();

    let value = json!({"schemaVersion":1,"name":"commandPort","type":"number","value":46665});
    let stored = json!({"collection":"variables","key":"commandPort","revision":4,"deleted":false,"value":value});
    let page = json!({"records":[stored],"nextAfter":null});
    let (session, task) = native_session(vec![
        Reply::json(stored.clone()),
        Reply::json(page),
        Reply::json(stored.clone()),
    ])
    .await;
    let written = session
        .workspace_put("variables", "commandPort", value.clone(), None)
        .await
        .unwrap();
    assert_eq!(written.value, Some(value.clone()));
    session
        .workspace_collection("variables", None)
        .await
        .unwrap();
    assert!(session
        .workspace_record("variables", "commandPort")
        .await
        .is_ok());
    let requests = task.await.unwrap();
    assert_eq!(requests[1].method, "PUT");
    assert_eq!(
        requests[1].target,
        "/api/v2/sync/collections/variables/commandPort"
    );
    // A secret variable is refused before any request is made.
    let (session, task) = native_session(vec![]).await;
    for bad in [
        json!({"schemaVersion":1,"name":"apiKey","type":"secret","value":"x"}),
        json!({"schemaVersion":1,"name":"other","type":"text","value":"x"}),
        json!({"schemaVersion":1,"name":"apiKey","type":"text","value":1}),
    ] {
        assert!(matches!(
            session
                .workspace_put("variables", "apiKey", bad, None)
                .await,
            Err(SyncError::Configuration)
        ));
    }
    assert_eq!(task.await.unwrap().len(), 1, "only the profile lookup");
}

#[tokio::test]
async fn library_documents_accept_fork_provenance_only_on_private_metadata() {
    let id = Uuid::from_u128(77);
    let source = Uuid::from_u128(78);
    let mut forked = document_view(id, 1, "draft");
    forked["forkOf"] = json!({"documentId":source,"revision":2});
    let page = json!({"items":[document_meta(id, 1, "draft")],"nextCursor":null});
    let mut page_with_fork = page.clone();
    page_with_fork["items"][0]["forkOf"] = json!({"documentId":source,"revision":2});
    let mut unknown = forked.clone();
    unknown["forkOf"]["parentId"] = json!("x");
    let mut zero = forked.clone();
    zero["forkOf"]["revision"] = json!(0);
    let (session, task) = native_session(vec![
        Reply::json(forked.clone()),
        Reply::json(page_with_fork),
        Reply::json(unknown),
        Reply::json(zero),
        Reply::json(document_view(id, 1, "draft")),
    ])
    .await;
    assert_eq!(session.library_document(id).await.unwrap(), forked);
    session.library_documents(None).await.unwrap();
    for _ in 0..2 {
        assert!(matches!(
            session.library_document(id).await,
            Err(SyncError::Protocol)
        ));
    }
    assert!(session.library_document(id).await.is_ok());
    task.await.unwrap();
    // Public documents never carry provenance.
    let mut public = document_view(id, 1, "published");
    public["forkOf"] = json!({"documentId":source,"revision":1});
    let (origin, task) = mock(vec![Reply::json(public)]).await;
    assert!(matches!(
        CloudCatalog::new(&origin)
            .unwrap()
            .document(id, Some(1))
            .await,
        Err(SyncError::Protocol)
    ));
    task.await.unwrap();
}
