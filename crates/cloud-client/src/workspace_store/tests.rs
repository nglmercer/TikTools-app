use super::*;
use crate::{
    Session, WorkspaceChanges, WorkspaceEdit, WorkspaceKey, WorkspacePendingStatus,
    WorkspaceRecord, WorkspaceStateError,
};
use serde_json::json;
use std::sync::{Arc, Barrier};

fn state(account: &str) -> WorkspaceState {
    WorkspaceState::new("https://cloud.example".into(), account.into()).unwrap()
}
fn key() -> WorkspaceKey {
    WorkspaceKey::new("plugin-selections", "test.plugin").unwrap()
}
fn value(enabled: bool) -> serde_json::Value {
    json!({"schemaVersion":1,"pluginId":"test.plugin","version":"1.0.0","enabled":enabled})
}
fn record(revision: i64, enabled: bool) -> WorkspaceRecord {
    WorkspaceRecord {
        collection: "plugin-selections".into(),
        key: "test.plugin".into(),
        revision,
        deleted: false,
        value: Some(value(enabled)),
    }
}
fn private_test_path(root: &Path) -> PathBuf {
    let path = root.join("private-state");
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&path).unwrap();
    path
}

#[test]
fn durable_unknown_and_frozen_base_survive_restart_and_v1_file_is_untouched() {
    let root = tempfile::tempdir().unwrap();
    let store_root = private_test_path(root.path());
    let legacy = store_root.join("sync-state.json");
    let original = b"existing-v1-state-must-stay";
    fs::write(&legacy, original).unwrap();
    let store = WorkspaceStore::open(&store_root, "https://cloud.example", "account").unwrap();
    let mut source = state("account");
    source
        .apply_changes(WorkspaceChanges {
            changes: vec![record(1, false)],
            cursor: 1,
            has_more: false,
        })
        .unwrap();
    source
        .stage(key(), WorkspaceEdit::Put(value(true)))
        .unwrap();
    source.prepare_push(&key()).unwrap();
    source
        .apply_changes(WorkspaceChanges {
            changes: vec![record(2, true)],
            cursor: 2,
            has_more: false,
        })
        .unwrap();
    let saved = store.save(None, &source).unwrap();
    assert_eq!(saved.generation, 1);
    drop(store);
    let reopened = WorkspaceStore::open(&store_root, "https://cloud.example", "account").unwrap();
    let mut loaded = reopened.load().unwrap().unwrap();
    assert_eq!(loaded.state, source);
    let pending = loaded
        .state
        .entry(&key())
        .unwrap()
        .pending
        .as_ref()
        .unwrap();
    assert_eq!(pending.status, WorkspacePendingStatus::Unconfirmed);
    assert_eq!(pending.base_revision, Some(1));
    assert_eq!(pending.base_record, Some(record(1, false)));
    assert!(loaded.state.prepare_push(&key()).is_err());
    assert_eq!(fs::read(legacy).unwrap(), original);
}

#[test]
fn origin_account_partitions_preserve_all_pending_and_reject_foreign_state() {
    let root = tempfile::tempdir().unwrap();
    let store_root = private_test_path(root.path());
    let a = WorkspaceStore::open(&store_root, "https://cloud.example", "account-a").unwrap();
    let b = WorkspaceStore::open(&store_root, "https://cloud.example", "account-b").unwrap();
    let other = WorkspaceStore::open(&store_root, "https://other.example", "account-a").unwrap();
    let mut sa = state("account-a");
    sa.stage(key(), WorkspaceEdit::Put(value(true))).unwrap();
    a.save(None, &sa).unwrap();
    let mut sb = state("account-b");
    sb.stage(key(), WorkspaceEdit::Delete).unwrap();
    b.save(None, &sb).unwrap();
    assert_ne!(a.directory, b.directory);
    assert_ne!(a.directory, other.directory);
    assert!(other.load().unwrap().is_none());
    assert_eq!(a.load().unwrap().unwrap().state, sa);
    assert_eq!(b.load().unwrap().unwrap().state, sb);
    assert!(matches!(
        a.save(Some(1), &sb),
        Err(StoreError::WrongIdentity)
    ));
    assert_eq!(a.load().unwrap().unwrap().generation, 1);
    fs::copy(
        a.directory.join("workspace-state.json"),
        other.directory.join("workspace-state.json"),
    )
    .unwrap();
    assert!(matches!(other.load(), Err(StoreError::WrongIdentity)));
}

#[test]
fn concurrent_generation_cas_has_one_winner_and_retains_the_other_staged_copy() {
    let root = tempfile::tempdir().unwrap();
    let store_root = private_test_path(root.path());
    let store = WorkspaceStore::open(&store_root, "https://cloud.example", "account").unwrap();
    store.save(None, &state("account")).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let mut threads = Vec::new();
    for enabled in [false, true] {
        let path = store_root.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            let store = WorkspaceStore::open(path, "https://cloud.example", "account").unwrap();
            let mut loaded = store.load().unwrap().unwrap();
            loaded
                .state
                .stage(key(), WorkspaceEdit::Put(value(enabled)))
                .unwrap();
            barrier.wait();
            (
                store.save(Some(loaded.generation), &loaded.state),
                loaded.state,
            )
        }));
    }
    let results: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(
        results.iter().filter(|(result, _)| result.is_ok()).count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|(result, _)| matches!(result, Err(StoreError::ConcurrentChange)))
            .count(),
        1
    );
    assert!(results
        .iter()
        .all(|(_, state)| state.pending_keys().len() == 1));
    assert_eq!(store.load().unwrap().unwrap().generation, 2);
}

#[test]
fn operation_lock_serializes_network_work_but_not_state_staging_and_is_identity_scoped() {
    let root = tempfile::tempdir().unwrap();
    let store_root = private_test_path(root.path());
    let first = WorkspaceStore::open(&store_root, "https://cloud.example", "account").unwrap();
    let second = WorkspaceStore::open(&store_root, "https://cloud.example", "account").unwrap();
    let foreign = WorkspaceStore::open(&store_root, "https://cloud.example", "other").unwrap();
    let guard = first.try_operation_lock().unwrap().unwrap();
    assert!(second.try_operation_lock().unwrap().is_none());
    let independent = foreign.try_operation_lock().unwrap().unwrap();
    let mut state = state("account");
    state.stage(key(), WorkspaceEdit::Delete).unwrap();
    second.save(None, &state).unwrap();
    assert_eq!(first.load().unwrap().unwrap().state, state);
    drop(guard);
    assert!(second.try_operation_lock().unwrap().is_some());
    drop(independent);
}

#[test]
fn invalid_unknown_format_deep_or_oversized_files_fail_without_replacement() {
    let root = tempfile::tempdir().unwrap();
    let store_root = private_test_path(root.path());
    let store = WorkspaceStore::open(&store_root, "https://cloud.example", "account").unwrap();
    let path = store.directory.join("workspace-state.json");
    for bytes in [
        b"{broken".to_vec(),
        json!({"formatVersion":3,"generation":1,"state":state("account")})
            .to_string()
            .into_bytes(),
        format!("{}0{}", "[".repeat(65), "]".repeat(65)).into_bytes(),
    ] {
        fs::write(&path, &bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert!(matches!(store.load(), Err(StoreError::InvalidState)));
        assert!(store.save(None, &state("account")).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    file.set_len(MAX_WORKSPACE_STATE_BYTES as u64 + 1).unwrap();
    drop(file);
    assert!(matches!(store.load(), Err(StoreError::InvalidState)));
}

#[cfg(unix)]
#[test]
fn state_namespace_and_files_refuse_nonprivate_paths_and_symlinks() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let root = tempfile::tempdir().unwrap();
    let unsafe_root = root.path().join("unsafe");
    fs::create_dir(&unsafe_root).unwrap();
    fs::set_permissions(&unsafe_root, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        WorkspaceStore::open(&unsafe_root, "https://cloud.example", "account"),
        Err(StoreError::UnsafeDirectory)
    ));
    let safe = root.path().join("safe");
    let store = WorkspaceStore::open(&safe, "https://cloud.example", "account").unwrap();
    let external = root.path().join("external");
    fs::write(&external, b"unrelated").unwrap();
    symlink(&external, store.directory.join("workspace-state.json")).unwrap();
    assert!(matches!(store.load(), Err(StoreError::UnsafeDirectory)));
    fs::remove_file(store.directory.join("workspace-state.json")).unwrap();
    store.save(None, &state("account")).unwrap();
    fs::set_permissions(
        store.directory.join("workspace-state.json"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(matches!(store.load(), Err(StoreError::UnsafeDirectory)));
    assert_eq!(fs::read(external).unwrap(), b"unrelated");
}

#[tokio::test]
async fn held_real_absence_response_cannot_clear_a_newer_durable_delete_after_reload() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    async fn read_head(stream: &mut TcpStream) {
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            let byte = stream.read_u8().await.unwrap();
            bytes.push(byte);
            assert!(bytes.len() < 16 * 1024);
        }
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (seen, seen_rx) = tokio::sync::oneshot::channel();
    let (release, release_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut login, _) = listener.accept().await.unwrap();
        read_head(&mut login).await;
        let body = br#"{"id":"account"}"#;
        login.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
        login.write_all(body).await.unwrap();
        login.shutdown().await.unwrap();
        let (mut lookup, _) = listener.accept().await.unwrap();
        read_head(&mut lookup).await;
        seen.send(()).unwrap();
        release_rx.await.unwrap();
        lookup
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
    });
    let session = Session::connect(&origin, "test-only-token".into())
        .await
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let store_root = private_test_path(root.path());
    let store = WorkspaceStore::open(&store_root, &origin, "account").unwrap();
    let mut original = WorkspaceState::new(origin, "account".into()).unwrap();
    let captured = original.stage(key(), WorkspaceEdit::Delete).unwrap();
    store.save(None, &original).unwrap();
    let reading =
        tokio::spawn(async move { session.workspace_read_for_resolution(&key()).await.unwrap() });
    seen_rx.await.unwrap();
    let mut latest = store.load().unwrap().unwrap();
    let newer = latest.state.stage(key(), WorkspaceEdit::Delete).unwrap();
    assert!(newer > captured);
    store.save(Some(latest.generation), &latest.state).unwrap();
    release.send(()).unwrap();
    let proof = reading.await.unwrap();
    let mut reloaded = store.load().unwrap().unwrap();
    reloaded.state.observe_read(&proof).unwrap();
    assert_eq!(reloaded.state, latest.state);
    assert!(matches!(
        reloaded.state.complete_absent_delete(&proof, captured),
        Err(WorkspaceStateError::StaleReview)
    ));
    assert_eq!(reloaded.state, latest.state);
    assert_eq!(store.load().unwrap().unwrap().state, latest.state);
    server.await.unwrap();
}

#[test]
fn guarded_publication_rejection_preserves_previous_generation() {
    let root = tempfile::tempdir().unwrap();
    let store_root = private_test_path(root.path());
    let store = WorkspaceStore::open(&store_root, "https://cloud.example", "account").unwrap();
    let original = state("account");
    store.save(None, &original).unwrap();
    let mut next = original.clone();
    next.stage(key(), WorkspaceEdit::Put(value(true))).unwrap();
    assert!(matches!(
        store.save_with_publication_guard::<StoreError>(Some(1), &next, |_publish| {
            Err(StoreError::InvalidState)
        }),
        Err(StoreError::InvalidState)
    ));
    let loaded = store.load().unwrap().unwrap();
    assert_eq!(loaded.generation, 1);
    assert_eq!(loaded.state, original);
    assert!(matches!(
        store.save_with_publication_guard::<StoreError>(Some(1), &next, |_publish| Ok(())),
        Err(StoreError::InvalidState)
    ));
    assert_eq!(store.load().unwrap().unwrap().state, original);
    let advanced = store
        .save_with_publication_guard::<StoreError>(Some(1), &next, |publish| publish())
        .unwrap();
    assert_eq!(advanced.generation, 2);
    let invoked = std::sync::atomic::AtomicBool::new(false);
    assert!(matches!(
        store.save_with_publication_guard::<StoreError>(Some(1), &original, |_publish| {
            invoked.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }),
        Err(StoreError::ConcurrentChange)
    ));
    assert!(!invoked.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(store.load().unwrap().unwrap().state, next);
}

#[test]
fn guard_cannot_hide_failed_or_repeated_publication() {
    let root = tempfile::tempdir().unwrap();
    let store_root = private_test_path(root.path());
    let store = WorkspaceStore::open(&store_root, "https://cloud.example", "account").unwrap();
    let original = state("account");
    // The callback is intentionally faulty: after creating a directory at the
    // destination it ignores the failed atomic rename and reports success.
    assert!(matches!(
        store.save_with_publication_guard::<StoreError>(None, &original, |publish| {
            fs::create_dir(store.directory.join("workspace-state.json")).unwrap();
            assert!(publish().is_err());
            Ok(())
        }),
        Err(StoreError::InvalidState)
    ));
    fs::remove_dir(store.directory.join("workspace-state.json")).unwrap();
    assert!(store.load().unwrap().is_none());

    // If a broken guard publishes twice, it must not turn the second failure
    // into a success. The first rename may already have committed.
    assert!(matches!(
        store.save_with_publication_guard::<StoreError>(None, &original, |publish| {
            publish()?;
            assert!(publish().is_err());
            Ok(())
        }),
        Err(StoreError::InvalidState)
    ));
    assert_eq!(store.load().unwrap().unwrap().state, original);
}
