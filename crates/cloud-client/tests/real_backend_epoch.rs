//! Cross-repository fixture. The backend test supplies synthetic accounts on
//! stdin and runs this ignored test against an isolated loopback server.
use std::{io::Read, time::Duration};

use cloud_client::{
    CloudCatalog, Session, SyncError, WorkspaceEdit, WorkspaceKey, WorkspacePendingStatus,
    WorkspaceSnapshot, WorkspaceState, WorkspaceStore,
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureInput {
    origin: String,
    first_token: String,
    second_token: String,
}

#[tokio::test]
#[ignore = "run only from the isolated RemakeBackend PostgreSQL fixture"]
async fn real_backend_epoch_contract() {
    let mut input = Vec::new();
    std::io::stdin()
        .take(16_385)
        .read_to_end(&mut input)
        .expect("read fixture input");
    assert!(input.len() <= 16_384, "fixture input is oversized");
    let input: FixtureInput = serde_json::from_slice(&input).expect("invalid fixture input");
    let origin = url::Url::parse(&input.origin).expect("invalid fixture origin");
    assert_eq!(origin.scheme(), "http");
    assert_eq!(origin.host_str(), Some("127.0.0.1"));
    assert!(origin.port().is_some());
    assert_eq!(origin.path(), "/");
    assert!(origin.username().is_empty() && origin.password().is_none());
    assert!(origin.query().is_none() && origin.fragment().is_none());

    tokio::time::timeout(Duration::from_secs(30), async move {
        let catalog = CloudCatalog::new(&input.origin).expect("catalog setup");
        let capabilities = catalog.workspace_sync_capabilities().await.expect("capabilities");
        assert!(capabilities.write_epoch_barrier && capabilities.write_epoch_enforcement);
        let first = Session::connect(&input.origin, input.first_token.clone()).await.expect("first session");
        let another_device = Session::connect(&input.origin, input.first_token).await.expect("second device session");
        let other = Session::connect(&input.origin, input.second_token).await.expect("other account session");
        assert_ne!(first.account_id(), other.account_id());

        let old_epoch = first.workspace_write_epoch().await.expect("initial epoch");
        assert_eq!(old_epoch.decimal(), "0");
        assert_eq!(old_epoch.epoch_required(), Some(false));
        let preference = json!({"schemaVersion":1,"theme":"dark"});
        let before = first.workspace_put("preferences", "desktop", preference.clone(), None).await.expect("legacy write before migration");

        // Save an exact prepared request and a newer local edit to real disk.
        let root = tempfile::tempdir().expect("private temporary state");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = WorkspaceStore::open(root.path(), first.instance_origin(), first.account_id()).expect("workspace store");
        let key = WorkspaceKey::new("plugin-selections", "acme.hotkeys").unwrap();
        let value = json!({"schemaVersion":1,"pluginId":"acme.hotkeys","version":"1.0.0","enabled":true});
        let mut state = WorkspaceState::new(first.instance_origin(), first.account_id().into()).unwrap();
        state.install_rebuild(WorkspaceSnapshot { schema_version: 2, cursor: 0, records: vec![] }).unwrap();
        state.mark_bootstrap_complete();
        state.mark_product_guarded();
        state.stage_product_decision(key.clone(), WorkspaceEdit::Put(value.clone()), 1, &"a".repeat(64)).unwrap();
        let proof = state.product_queued_proof(&key).unwrap();
        let ticket = state.prepare_product_push_fenced(&proof, &old_epoch).unwrap();
        state.stage(key.clone(), WorkspaceEdit::Put(json!({"schemaVersion":1,"pluginId":"acme.hotkeys","version":"1.0.0","enabled":false}))).unwrap();
        store.save(None, &state).expect("persist original and next");
        let reopened = WorkspaceStore::open(root.path(), first.instance_origin(), first.account_id()).unwrap();
        let snapshot = reopened.load().unwrap().unwrap();
        let entry = snapshot.state.entry(&key).unwrap();
        let pending = entry.pending.as_ref().unwrap();
        assert_eq!(pending.status, WorkspacePendingStatus::Unconfirmed);
        assert_eq!(pending.write_epoch.as_ref(), ticket.write_epoch());
        assert!(entry.next.is_some());

        let required = another_device.workspace_require_write_epoch().await.expect("enable required epoch");
        assert_eq!(required.epoch_required(), Some(true));
        assert!(required.is_after(&old_epoch).unwrap());
        assert!(matches!(first.workspace_put("preferences", "desktop", preference, Some(before.revision)).await, Err(SyncError::Http(428))));
        assert!(matches!(first.workspace_put_fenced(&key.collection, &key.key, value.clone(), None, ticket.write_epoch().unwrap()).await, Err(SyncError::Http(412))));

        let barrier = another_device.workspace_write_barrier().await.expect("barrier");
        assert!(barrier.is_after(&required).unwrap());
        assert!(matches!(first.workspace_put_fenced(&key.collection, &key.key, value, None, &required).await, Err(SyncError::Http(412))));
        let new_value = json!({"schemaVersion":1,"theme":"light"});
        let written = first.workspace_put_fenced("preferences", "desktop", new_value, Some(before.revision), &barrier).await.expect("fenced write");
        assert!(matches!(first.workspace_delete_fenced("preferences", "desktop", written.revision, &required).await, Err(SyncError::Http(412))));
        let deleted = first.workspace_delete_fenced("preferences", "desktop", written.revision, &barrier).await.expect("fenced delete");
        assert!(deleted.deleted);
        let other_epoch = other.workspace_write_epoch().await.expect("other account epoch");
        assert_eq!(other_epoch.decimal(), "0");
        assert_eq!(other_epoch.epoch_required(), Some(false));
        assert!(matches!(other.workspace_put_fenced("preferences", "desktop", json!({"schemaVersion":1}), None, &barrier).await, Err(SyncError::WrongAccount)));
        let retained = reopened.load().unwrap().unwrap();
        let pending = retained.state.entry(&key).unwrap().pending.as_ref().unwrap();
        assert_eq!(pending.status, WorkspacePendingStatus::Unconfirmed);
        assert_eq!(pending.write_epoch.as_ref(), ticket.write_epoch());
        assert!(retained.state.entry(&key).unwrap().next.is_some());
    }).await.expect("isolated SDK fixture timed out");
}
