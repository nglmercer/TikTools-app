use super::*;
use cloud_client::{
    WorkspaceEdit, WorkspaceKey, WorkspaceRecord, WorkspaceSnapshot, WorkspaceStore,
};
use std::cell::Cell;

struct EmptySource;
impl crate::CutoverSource for EmptySource {
    fn with_guarded_source<T>(
        &self,
        operation: impl FnOnce(
            &[PluginSelection],
            &[crate::V1CutoverReceipt],
        ) -> Result<T, SyncCoordError>,
    ) -> Result<T, SyncCoordError> {
        operation(&[], &[])
    }
}

fn marked_v2(sync: &SyncCoordinator, session: &Arc<cloud_client::Session>) {
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .require_product_outbox(&session.instance_origin(), session.account_id())
        .unwrap();
    drop(lease);
    let review = sync
        .guarded_marked_cutover_review(session, &[], &[])
        .unwrap();
    sync.execute_cutover_with_source(&review, &EmptySource)
        .unwrap();
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
}

fn remote(enabled: bool, revision: i64) -> WorkspaceRecord {
    WorkspaceRecord {
        collection: "plugin-selections".into(),
        key: "acme.plugin".into(),
        revision,
        deleted: false,
        value: Some(serde_json::json!({
            "schemaVersion": 1,
            "pluginId": "acme.plugin",
            "version": "1.0.0",
            "enabled": enabled
        })),
    }
}

fn replace_remote(store: &WorkspaceStore, enabled: bool, revision: i64) {
    let mut snapshot = store.load().unwrap().unwrap();
    snapshot
        .state
        .install_rebuild(WorkspaceSnapshot {
            schema_version: 2,
            cursor: revision,
            records: vec![remote(enabled, revision)],
        })
        .unwrap();
    store
        .save(Some(snapshot.generation), &snapshot.state)
        .unwrap();
}

#[tokio::test]
async fn cached_v2_remote_review_rejects_legacy_v2_without_product_marker() {
    let dir = TempDir::new("v2-cached-legacy-marker");
    let mock = MockSync::start("v2-legacy-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "legacy-v2-login").await;
    let session = auth.session().await.unwrap();
    let preview = sync.preview_v2_cutover(&session, &[]).unwrap();
    let lease = sync.protocol.try_cutover().unwrap();
    lease
        .prepare(&preview, &mock.origin, "v2-legacy-user", &[])
        .unwrap();
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "v2-legacy-user".into(),
    )
    .unwrap();
    workspace
        .bootstrap(&preview.selections(), preview.legacy())
        .unwrap();
    lease
        .finish(&preview, &mock.origin, "v2-legacy-user")
        .unwrap();
    drop(lease);
    sync.set_enabled(false).unwrap();
    let consent = sync.review_v2_resume(&session, None).unwrap();
    sync.confirm_v2_resume(&consent).unwrap();
    let store = WorkspaceStore::open(dir.join("a-sync"), &mock.origin, "v2-legacy-user").unwrap();
    replace_remote(&store, true, 1);
    // Legacy-format v2 remains readable by the protocol itself, but cannot
    // enter product admission without its independently required marker.
    assert!(sync.protocol.try_v2(&mock.origin, "v2-legacy-user").is_ok());
    assert!(matches!(
        sync.review_v2_product_remote(&session, "acme.plugin"),
        Err(SyncCoordError::Protocol(
            ProtocolFenceError::ProductOutboxNotEnabled
        ))
    ));
    mock.stop();
}

#[tokio::test]
async fn cached_v2_remote_review_revalidates_session_consent_snapshot_and_pending_edit() {
    let dir = TempDir::new("v2-cached-remote-review");
    let mock = MockSync::start("v2-remote-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "first-login").await;
    let session = auth.session().await.unwrap();
    marked_v2(&sync, &session);
    let workspace = crate::WorkspaceCoordinator::open(
        dir.join("a-sync"),
        mock.origin.clone(),
        "v2-remote-user".into(),
    )
    .unwrap();
    let store = WorkspaceStore::open(dir.join("a-sync"), &mock.origin, "v2-remote-user").unwrap();
    replace_remote(&store, true, 1);
    let review = sync
        .review_v2_product_remote(&session, "acme.plugin")
        .unwrap();
    let called = Cell::new(false);
    sync.with_reviewed_v2_product_remote(&review, |context, record| {
        assert_eq!(context.origin, mock.origin);
        assert_eq!(context.account_id, "v2-remote-user");
        assert_eq!(record, &remote(true, 1));
        called.set(true);
        Ok(())
    })
    .unwrap();
    assert!(called.get());

    // A fresh cached revision invalidates the old review before admission.
    replace_remote(&store, false, 2);
    assert!(sync
        .with_reviewed_v2_product_remote::<()>(&review, |_, _| panic!("stale review admitted"))
        .is_err());
    let review = sync
        .review_v2_product_remote(&session, "acme.plugin")
        .unwrap();
    let target = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
    workspace
        .stage(target, WorkspaceEdit::Put(remote(true, 1).value.unwrap()))
        .unwrap();
    assert!(sync
        .with_reviewed_v2_product_remote::<()>(&review, |_, _| panic!("pending edit admitted"))
        .is_err());
    assert!(sync
        .review_v2_product_remote(&session, "acme.plugin")
        .is_err());

    mock.stop();
}

#[tokio::test]
async fn cached_v2_remote_review_rejects_optout_and_foreign_coordinator() {
    let dir = TempDir::new("v2-cached-remote-consent");
    let mock = MockSync::start("v2-consent-user").await;
    let (auth, sync) = coordinator(&dir, &mock, "a", Arc::new(MemoryTokenStore::new()));
    login(&auth, "consent-login").await;
    let session = auth.session().await.unwrap();
    marked_v2(&sync, &session);
    let store = WorkspaceStore::open(dir.join("a-sync"), &mock.origin, "v2-consent-user").unwrap();
    replace_remote(&store, true, 1);
    let review = sync
        .review_v2_product_remote(&session, "acme.plugin")
        .unwrap();
    let other = SyncCoordinator::new(auth.clone(), dir.join("a-sync")).unwrap();
    assert!(other
        .with_reviewed_v2_product_remote::<()>(&review, |_, _| panic!(
            "foreign coordinator admitted"
        ))
        .is_err());

    let marker = std::fs::read_dir(dir.join("a-sync"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("v1-product-outbox-")
        })
        .unwrap();
    let marker_contents = std::fs::read(&marker).unwrap();
    std::fs::remove_file(&marker).unwrap();
    assert!(sync
        .review_v2_product_remote(&session, "acme.plugin")
        .is_err());
    assert!(sync
        .with_reviewed_v2_product_remote::<()>(&review, |_, _| panic!("missing marker admitted"))
        .is_err());
    // Corruption must fail closed at both stages too. Restore the same file
    // in place so Unix's private permissions remain valid for the next checks.
    std::fs::write(&marker, b"invalid").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    assert!(sync
        .review_v2_product_remote(&session, "acme.plugin")
        .is_err());
    assert!(sync
        .with_reviewed_v2_product_remote::<()>(&review, |_, _| panic!("damaged marker admitted"))
        .is_err());
    std::fs::write(&marker, marker_contents).unwrap();
    // This workspace remains unchanged and contains no pending edit, so only
    // the changed session can explain rejection of the earlier review.
    auth.logout().await.unwrap();
    login(&auth, "second-consent-login").await;
    assert!(sync
        .with_reviewed_v2_product_remote::<()>(&review, |_, _| panic!("old login admitted"))
        .is_err());
    let current_session = auth.session().await.unwrap();
    let review = sync
        .review_v2_product_remote(&current_session, "acme.plugin")
        .unwrap();
    sync.set_enabled(false).unwrap();
    assert!(sync
        .with_reviewed_v2_product_remote::<()>(&review, |_, _| panic!("opt-out admitted"))
        .is_err());
    assert!(sync
        .review_v2_product_remote(&current_session, "acme.plugin")
        .is_err());
    mock.stop();
}
