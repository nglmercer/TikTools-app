use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn pull_count_includes_final_and_empty_pages_and_preserves_legacy_api() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let first = serde_json::json!({"changes":[
            {"collection":"plugin-selections","key":"acme.a","revision":1,"deleted":false,
             "value":{"schemaVersion":1,"pluginId":"acme.a","version":"1.0.0","enabled":true}},
            {"collection":"plugin-selections","key":"acme.b","revision":2,"deleted":false,
             "value":{"schemaVersion":1,"pluginId":"acme.b","version":"1.0.0","enabled":false}}
        ],"cursor":2,"has_more":false})
    .to_string();
    let empty = serde_json::json!({"changes":[],"cursor":2,"has_more":false}).to_string();
    let invalid = serde_json::json!({"changes":[],"cursor":1,"has_more":false}).to_string();
    let task = tokio::spawn(async move {
        for body in [
            r#"{"id":"alice"}"#.to_owned(),
            first,
            empty.clone(),
            invalid,
            empty,
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(body.as_bytes()).await.unwrap();
        }
    });
    let session = Session::connect(&origin, "fixture".into()).await.unwrap();
    let mut state = session.new_state();
    assert_eq!(
        session.pull_page_with_count(&mut state).await.unwrap(),
        (false, 2)
    );
    let saved = state.clone();
    assert_eq!(
        session.pull_page_with_count(&mut state).await.unwrap(),
        (false, 0)
    );
    assert!(matches!(
        session.pull_page_with_count(&mut state).await,
        Err(SyncError::Protocol)
    ));
    assert_eq!(state, saved);
    assert!(!session.pull_page(&mut state).await.unwrap());
    assert_eq!(state, saved);
    task.await.unwrap();
}

#[tokio::test]
async fn account_export_response_limit_allows_large_bounded_payloads() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let body = serde_json::json!({"payload": "x".repeat(MAX_RESPONSE)}).to_string();
    let task = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let _ = socket.read(&mut request).await;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(body.as_bytes()).await;
        }
    });
    let url = format!("http://{address}/");
    let response = reqwest::get(&url).await.unwrap();
    let value: serde_json::Value = decode_with_limit(response, MAX_ACCOUNT_EXPORT)
        .await
        .unwrap();
    assert_eq!(value["payload"].as_str().unwrap().len(), MAX_RESPONSE);
    let response = reqwest::get(&url).await.unwrap();
    assert!(matches!(
        decode::<serde_json::Value>(response).await,
        Err(SyncError::Protocol)
    ));
    task.await.unwrap();
}

#[tokio::test]
async fn refuses_redirects_before_forwarding_bearer() {
    let destination = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = destination.local_addr().unwrap();
    let source = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", source.local_addr().unwrap());
    let handler = tokio::spawn(async move {
        let (mut socket, _) = source.accept().await.unwrap();
        let mut request = [0; 4096];
        let _ = socket.read(&mut request).await.unwrap();
        socket.write_all(format!("HTTP/1.1 302 Found\r\nLocation: http://{target}/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    });
    assert!(matches!(
        Session::connect(&origin, "test-token".into()).await,
        Err(SyncError::Http(302))
    ));
    handler.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), destination.accept())
            .await
            .is_err()
    );
}

#[test]
fn deletion_blocker_pages_reject_invalid_roles_and_cursors() {
    let member = DeletionMembership {
        publisher_id: Uuid::from_u128(1),
        namespace: "example".into(),
        role: "owner".into(),
    };
    let mut page = DeletionBlockerPage {
        cloud_roles: vec!["admin".into()],
        memberships: vec![member.clone()],
        next_after: None,
    };
    assert!(page.validate(None).is_ok());
    page.memberships.push(member);
    assert!(page.validate(None).is_err());
    page.memberships.pop();
    page.next_after = Some(Uuid::from_u128(1));
    assert!(page.validate(None).is_err());
    page.next_after = None;
    page.cloud_roles.push("admin".into());
    assert!(page.validate(None).is_err());
    page.cloud_roles.clear();
    assert!(page.validate(Some(Uuid::from_u128(1))).is_err());
}

#[test]
fn rate_limit_retry_delay_is_bounded_and_unambiguous() {
    let mut headers = reqwest::header::HeaderMap::new();
    assert_eq!(retry_delay(&headers), None);
    headers.insert(reqwest::header::RETRY_AFTER, "60".parse().unwrap());
    assert_eq!(retry_delay(&headers), Some(60));
    headers.insert(reqwest::header::RETRY_AFTER, "86401".parse().unwrap());
    assert_eq!(retry_delay(&headers), None);
    headers.insert(reqwest::header::RETRY_AFTER, "0".parse().unwrap());
    assert_eq!(retry_delay(&headers), None);
    headers.append(reqwest::header::RETRY_AFTER, "60".parse().unwrap());
    assert_eq!(retry_delay(&headers), None);
}

#[test]
fn sync_export_rejects_inconsistent_or_unrecognized_records() {
    let good = Record {
        collection: COLLECTION.into(),
        key: "acme.plugin".into(),
        revision: 1,
        deleted: true,
        value: None,
    };
    let mut export = SyncExport {
        schema_version: 1,
        cursor: 1,
        records: vec![good.clone()],
    };
    assert!(export.validate().is_ok());
    export.records.push(good);
    assert!(matches!(export.validate(), Err(SyncError::Protocol)));
    export.records.pop();
    export.cursor = 0;
    assert!(matches!(export.validate(), Err(SyncError::Protocol)));
    export.cursor = 1;
    export.records[0].collection = "other".into();
    assert!(matches!(export.validate(), Err(SyncError::Protocol)));
    assert!(serde_json::from_str::<SyncExport>(
        r#"{"schemaVersion":1,"cursor":0,"records":[],"token":"secret"}"#
    )
    .is_err());
}

#[test]
fn account_export_is_account_bound_bounded_and_secret_free() {
    let raw = serde_json::json!({
        "schemaVersion": 1,
        "profile": {"id":"user-1","email":"user@example.test","name":"User"},
        "cloudRoles": ["moderator"],
        "publisherMemberships": [{"namespace":"acme","role":"owner"}],
        "publisherKeysRequested": [],
        "releasesSubmitted": [],
        "uploadsRequested": [],
        "reviewsRecorded": [],
        "publicationRequests": [],
        "ownershipTransfers": [],
        "auditEventsByActor": [],
        "yanksRecorded": [],
        "advisoriesRecorded": [],
        "sync": {"schemaVersion":1,"cursor":0,"records":[]}
    });
    let mut export: AccountExport = serde_json::from_value(raw).unwrap();
    assert!(export.validate("user-1").is_ok());
    assert!(matches!(
        export.validate("user-2"),
        Err(SyncError::WrongAccount)
    ));
    export.publisher_memberships[0]["sessionToken"] = serde_json::json!("secret");
    assert!(matches!(
        export.validate("user-1"),
        Err(SyncError::Protocol)
    ));
    export.publisher_memberships.clear();
    export.cloud_roles.push("owner".into());
    assert!(matches!(
        export.validate("user-1"),
        Err(SyncError::Protocol)
    ));
}

#[test]
fn offline_edits_require_explicit_resolution() {
    let mut state = SyncState::new("https://cloud.example".into(), "user".into());
    state.stage("acme.plugin".into(), Edit::Delete).unwrap();
    assert!(state.entries["acme.plugin"].pending.is_some());
    assert!(state
        .stage(
            "acme.plugin".into(),
            Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "not-a-version".into(),
                enabled: true
            })
        )
        .is_err());
    assert!(state.entries["acme.plugin"].pending.is_some());
    state.discard("acme.plugin").unwrap();
    assert!(state.entries["acme.plugin"].pending.is_none());
}

fn selection(version: &str) -> PluginSelection {
    PluginSelection {
        schema_version: 1,
        plugin_id: "acme.a".into(),
        version: version.into(),
        enabled: true,
    }
}

fn record(revision: i64, value: Option<PluginSelection>) -> Record {
    Record {
        collection: "plugin-selections".into(),
        key: "acme.a".into(),
        revision,
        deleted: value.is_none(),
        value,
    }
}

/// Pending local edit at base revision 2, with the server at revision 2 holding `1.0.0`.
fn state_with_pending_upgrade(session: &Session) -> SyncState {
    let mut state = session.new_state();
    state.cursor = 2;
    state.entries.insert(
        "acme.a".into(),
        Entry {
            remote: Some(record(2, Some(selection("1.0.0")))),
            pending: Some(Pending {
                base_revision: Some(2),
                edit: Edit::Put(selection("1.1.0")),
                conflict: false,
            }),
        },
    );
    state
}

#[test]
fn remote_identical_to_a_pending_edit_resolves_without_conflict() {
    let mut state = SyncState::new("https://cloud.example".into(), "user".into());
    state
        .stage("acme.a".into(), Edit::Put(selection("1.1.0")))
        .unwrap();
    state.apply(record(3, Some(selection("1.1.0")))).unwrap();
    let entry = &state.entries["acme.a"];
    assert!(
        entry.pending.is_none(),
        "identical remote must clear the edit"
    );
    assert_eq!(entry.remote.as_ref().unwrap().revision, 3);
}

#[test]
fn remote_identical_tombstone_resolves_a_pending_delete() {
    let mut state = SyncState::new("https://cloud.example".into(), "user".into());
    state.stage("acme.a".into(), Edit::Delete).unwrap();
    state.apply(record(4, None)).unwrap();
    assert!(state.entries["acme.a"].pending.is_none());
}

#[test]
fn differing_remote_still_requires_explicit_resolution() {
    let mut state = SyncState::new("https://cloud.example".into(), "user".into());
    state.entries.insert(
        "acme.a".into(),
        Entry {
            remote: Some(record(2, Some(selection("1.0.0")))),
            pending: Some(Pending {
                base_revision: Some(2),
                edit: Edit::Put(selection("1.1.0")),
                conflict: false,
            }),
        },
    );
    state.apply(record(3, Some(selection("2.0.0")))).unwrap();
    let pending = state.entries["acme.a"].pending.as_ref().expect("edit kept");
    assert!(pending.conflict);
    assert_eq!(pending.base_revision, Some(2));
}

/// Serves one canned HTTP response per entry, in order, then stops.
async fn serve(responses: Vec<(&'static str, String)>) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        for (status, body) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 8192];
            let _ = socket.read(&mut request).await.unwrap();
            let head = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(body.as_bytes()).await.unwrap();
        }
    });
    (origin, task)
}

fn changes_page(record: Record, cursor: i64) -> String {
    serde_json::json!({"changes": [record], "cursor": cursor, "has_more": false}).to_string()
}

#[tokio::test]
async fn push_conflict_that_the_feed_resolves_identically_succeeds_automatically() {
    let account = r#"{"id":"alice"}"#.to_owned();
    let (origin, task) = serve(vec![
        ("200 OK", account),
        ("409 Conflict", String::new()),
        (
            "200 OK",
            changes_page(record(3, Some(selection("1.1.0"))), 3),
        ),
    ])
    .await;
    let session = Session::connect(&origin, "fixture".into()).await.unwrap();
    let mut state = state_with_pending_upgrade(&session);
    session.push(&mut state, "acme.a").await.unwrap();
    task.await.unwrap();
    let entry = &state.entries["acme.a"];
    assert!(entry.pending.is_none());
    assert_eq!(entry.remote.as_ref().unwrap().revision, 3);
    assert_eq!(state.cursor, 3);
}

#[tokio::test]
async fn push_conflict_with_a_different_remote_stays_manual() {
    let account = r#"{"id":"alice"}"#.to_owned();
    let (origin, task) = serve(vec![
        ("200 OK", account),
        ("409 Conflict", String::new()),
        (
            "200 OK",
            changes_page(record(3, Some(selection("2.0.0"))), 3),
        ),
    ])
    .await;
    let session = Session::connect(&origin, "fixture".into()).await.unwrap();
    let mut state = state_with_pending_upgrade(&session);
    assert!(matches!(
        session.push(&mut state, "acme.a").await,
        Err(SyncError::Conflict)
    ));
    task.await.unwrap();
    let pending = state.entries["acme.a"].pending.as_ref().expect("edit kept");
    assert!(pending.conflict);
}
