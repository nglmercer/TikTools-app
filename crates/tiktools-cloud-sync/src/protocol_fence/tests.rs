use super::*;
use cloud_client::{
    PluginSelection, StateSnapshot, StateStore, SyncState, WorkspaceState, WorkspaceStore,
};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::io::Read;
use std::time::{Duration, Instant};
use std::{
    fs::{self, OpenOptions},
    io::Write,
};

const ORIGIN: &str = "https://cloud.example";
const ACCOUNT: &str = "account-a";

#[test]
fn product_outbox_marker_is_identity_bound_and_corruption_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let gate = fence.try_cutover().unwrap();
    gate.require_product_outbox(ORIGIN, ACCOUNT).unwrap();
    assert!(matches!(
        gate.ensure_legacy_allowed(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::ProductOutboxRequired)
    ));
    let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], None).unwrap();
    assert!(matches!(
        gate.prepare(&preview, ORIGIN, ACCOUNT, &[]),
        Err(ProtocolFenceError::ProductOutboxRequired)
    ));
    assert!(gate.ensure_legacy_allowed(ORIGIN, "account-b").is_ok());
    drop(gate);
    let path = fence.product_outbox_path(ORIGIN, ACCOUNT).unwrap();
    fs::write(&path, b"{invalid").unwrap();
    let reopened = ProtocolFence::open(&dir).unwrap();
    let lease = reopened.try_v1().unwrap();
    assert!(matches!(
        lease.ensure_legacy_allowed(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::Store(StoreError::InvalidState))
    ));
}

#[test]
fn interrupted_marker_preparation_leaves_only_an_ignored_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("private");
    let fence = ProtocolFence::open(&private).unwrap();
    let mut temp = tempfile::Builder::new()
        .prefix(".v1-uncertain-tmp-")
        .tempfile_in(&private)
        .unwrap();
    temp.write_all(b"{\"unfinished\":").unwrap();
    temp.flush().unwrap();
    let (_file, path) = temp.keep().unwrap();
    assert!(path.is_file());
    assert_eq!(fence.uncertain_count(ORIGIN, ACCOUNT).unwrap(), 0);
    assert!(fence.uncertain_records(ORIGIN, ACCOUNT).unwrap().is_empty());
}

#[test]
fn marker_publication_never_overwrites_an_existing_destination() {
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("private");
    let fence = ProtocolFence::open(&private).unwrap();
    let prefix = format!("v1-uncertain-{}-", identity_hash(ORIGIN, ACCOUNT).unwrap());
    let destination = private.join(format!("{prefix}collision"));
    let mut existing = OpenOptions::new();
    existing.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        existing.mode(0o600);
    }
    existing
        .open(&destination)
        .unwrap()
        .write_all(b"original marker")
        .unwrap();
    let temp = tempfile::Builder::new()
        .prefix(".v1-uncertain-tmp-")
        .tempfile_in(&private)
        .unwrap();
    let temporary_path = temp.path().to_owned();
    let original = V1UncertainUpload {
        format_version: 1,
        origin: ORIGIN.into(),
        account_id: ACCOUNT.into(),
        source_generation: 1,
        key: "acme.plugin".into(),
        pending: Pending {
            base_revision: None,
            edit: cloud_client::Edit::Delete,
            conflict: false,
        },
        base_record: None,
    };
    assert!(matches!(
        fence.publish_uncertain(temp, destination.clone(), &original),
        Err(ProtocolFenceError::Io(ref error)) if error.kind() == io::ErrorKind::AlreadyExists
    ));
    assert_eq!(fs::read(&destination).unwrap(), b"original marker");
    assert!(!temporary_path.exists());
    assert_eq!(fence.uncertain_count(ORIGIN, ACCOUNT).unwrap(), 1);
}

#[test]
fn cutover_fences_all_v1_leases_and_recovers_preparing_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let legacy = StateStore::open(&dir)
        .unwrap()
        .save(None, &SyncState::new(ORIGIN.into(), ACCOUNT.into()))
        .unwrap();
    let old_bytes = fs::read(dir.join("sync-state.json")).unwrap();
    let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], Some(&legacy)).unwrap();
    let v1 = fence.try_v1().unwrap();
    let another_v1 = ProtocolFence::open(&dir).unwrap().try_v1().unwrap();
    assert!(matches!(fence.try_cutover(), Err(ProtocolFenceError::Busy)));
    drop(v1);
    drop(another_v1);
    let cutover = fence.try_cutover().unwrap();
    cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]).unwrap();
    assert!(matches!(fence.try_v1(), Err(ProtocolFenceError::Busy)));
    drop(cutover);
    let v1 = fence.try_v1().unwrap();
    assert!(matches!(
        v1.ensure_v1(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::NotV1)
    ));
    assert!(v1.ensure_v1(ORIGIN, "account-b").is_ok());
    drop(v1);
    assert!(matches!(
        fence.try_v2(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::StaleCutover)
    ));
    let cutover = ProtocolFence::open(&dir).unwrap().try_cutover().unwrap();
    cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]).unwrap();
    assert!(matches!(
        cutover.finish(&preview, ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::StaleCutover)
    ));
    let store = WorkspaceStore::open(&dir, ORIGIN, ACCOUNT).unwrap();
    assert!(store
        .try_with_locked_snapshot::<_, StoreError>(|_| Ok(()))
        .unwrap()
        .is_some());
    let workspace = crate::WorkspaceCoordinator::open(&dir, ORIGIN.into(), ACCOUNT.into()).unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    cutover.finish(&preview, ORIGIN, ACCOUNT).unwrap();
    drop(cutover);
    let v2 = fence.try_v2(ORIGIN, ACCOUNT).unwrap();
    assert!(matches!(fence.try_cutover(), Err(ProtocolFenceError::Busy)));
    drop(v2);
    assert!(matches!(
        fence.try_v1().unwrap().ensure_v1(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::NotV1)
    ));
    assert_eq!(fs::read(dir.join("sync-state.json")).unwrap(), old_bytes);
}

#[test]
fn completed_marked_cutover_requires_intact_product_marker() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let preview =
        CutoverPreview::new_marked(ORIGIN.into(), ACCOUNT.into(), &[], None, &[]).unwrap();
    let cutover = fence.try_cutover().unwrap();
    cutover.require_product_outbox(ORIGIN, ACCOUNT).unwrap();
    cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]).unwrap();
    let workspace = crate::WorkspaceCoordinator::open(&dir, ORIGIN.into(), ACCOUNT.into()).unwrap();
    workspace.bootstrap_reviewed(&preview).unwrap();
    cutover.finish(&preview, ORIGIN, ACCOUNT).unwrap();
    drop(cutover);
    assert!(fence.try_v2(ORIGIN, ACCOUNT).is_ok());
    assert!(matches!(
        workspace.stage(
            cloud_client::WorkspaceKey::new("preferences", "desktop").unwrap(),
            cloud_client::WorkspaceEdit::Put(
                serde_json::json!({"schemaVersion": 1, "theme": "dark"})
            ),
        ),
        Err(crate::WorkspaceCoordError::Protocol(
            ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    let path = fence.product_outbox_path(ORIGIN, ACCOUNT).unwrap();
    let original = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    // The immutable v3 protocol record still protects the partition
    // when the separate product marker is missing.
    assert!(matches!(
        workspace.stage(
            cloud_client::WorkspaceKey::new("preferences", "desktop").unwrap(),
            cloud_client::WorkspaceEdit::Put(
                serde_json::json!({"schemaVersion": 1, "theme": "dark"})
            ),
        ),
        Err(crate::WorkspaceCoordError::Protocol(
            ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert!(matches!(
        fence.try_v2(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::ProductOutboxNotEnabled)
    ));
    let cutover = fence.try_cutover().unwrap();
    assert!(matches!(
        cutover.resume_record(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::ProductOutboxNotEnabled)
    ));
    drop(cutover);
    fs::write(&path, b"corrupt").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    assert!(fence.try_v2(ORIGIN, ACCOUNT).is_err());
    assert!(workspace
        .stage(
            cloud_client::WorkspaceKey::new("preferences", "desktop").unwrap(),
            cloud_client::WorkspaceEdit::Put(
                serde_json::json!({"schemaVersion": 1, "theme": "dark"})
            ),
        )
        .is_err());
    let cutover = fence.try_cutover().unwrap();
    assert!(cutover.resume_record(ORIGIN, ACCOUNT).is_err());
    drop(cutover);
    fs::write(&path, original).unwrap();
    assert!(fence.try_v2(ORIGIN, ACCOUNT).is_ok());
    assert!(workspace.snapshot().unwrap().state.is_product_guarded());
    assert!(!workspace.snapshot().unwrap().state.has_product_receipts());
    fs::remove_file(path).unwrap();
    fs::remove_file(fence.record_path(ORIGIN, ACCOUNT).unwrap()).unwrap();
    assert!(matches!(
        workspace.stage(
            cloud_client::WorkspaceKey::new("preferences", "desktop").unwrap(),
            cloud_client::WorkspaceEdit::Put(
                serde_json::json!({"schemaVersion": 1, "theme": "dark"})
            ),
        ),
        Err(crate::WorkspaceCoordError::Protocol(
            ProtocolFenceError::ProductOutboxRequired
        ))
    ));
}

#[test]
fn marked_workspace_upgrade_preserves_intents_and_survives_lost_markers() {
    use cloud_client::{WorkspaceEdit, WorkspaceKey};
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let preview =
        CutoverPreview::new_marked(ORIGIN.into(), ACCOUNT.into(), &[], None, &[]).unwrap();
    let gate = fence.try_cutover().unwrap();
    gate.require_product_outbox(ORIGIN, ACCOUNT).unwrap();
    gate.prepare(&preview, ORIGIN, ACCOUNT, &[]).unwrap();

    // Simulate a marked workspace written by the previous software, before
    // a product-mode bit and the v3 envelope existed.
    let store = WorkspaceStore::open(&dir, ORIGIN, ACCOUNT).unwrap();
    let mut old = WorkspaceState::new(ORIGIN.into(), ACCOUNT.into()).unwrap();
    old.mark_bootstrap_complete();
    store.save(None, &old).unwrap();
    gate.finish(&preview, ORIGIN, ACCOUNT).unwrap();
    let key = WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
    let first = WorkspaceEdit::Put(
        serde_json::json!({"schemaVersion":1,"pluginId":"acme.plugin","version":"1.0.0","enabled":true}),
    );
    old.stage_product_decision(key.clone(), first, 1, &"a".repeat(64))
        .unwrap();
    old.prepare_push(&key).unwrap();
    old.stage_product_decision(key.clone(), WorkspaceEdit::Delete, 2, &"b".repeat(64))
        .unwrap();
    // Reproduce the exact legacy v2 envelope; current writers reject
    // saving unguarded product receipts until they are upgraded.
    let identity = serde_json::to_vec(&("workspace-v2", ORIGIN, ACCOUNT)).unwrap();
    let partition = Sha256::digest(identity)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let path = dir
        .join("workspace-v2")
        .join(partition)
        .join("workspace-state.json");
    fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "formatVersion": 2, "generation": 2, "state": &old
        }))
        .unwrap(),
    )
    .unwrap();
    let saved = store.load().unwrap().unwrap();
    assert!(!saved.state.is_product_guarded());
    assert!(gate.upgrade_marked_workspace(ORIGIN, ACCOUNT).unwrap());
    assert!(!gate.upgrade_marked_workspace(ORIGIN, ACCOUNT).unwrap());
    let upgraded = store.load().unwrap().unwrap();
    assert_eq!(upgraded.generation, saved.generation + 1);
    assert!(upgraded.state.is_product_guarded());
    assert_eq!(upgraded.state.entries, old.entries);
    assert_eq!(upgraded.state.cursor, old.cursor);
    assert_eq!(
        upgraded.state.next_local_generation,
        old.next_local_generation
    );
    assert!(upgraded.state.has_product_receipts());
    drop(gate);

    // Even simultaneous loss of both separate protocol markers cannot
    // make the upgraded workspace available through generic operations.
    fs::remove_file(fence.record_path(ORIGIN, ACCOUNT).unwrap()).unwrap();
    fs::remove_file(fence.product_outbox_path(ORIGIN, ACCOUNT).unwrap()).unwrap();
    let workspace = crate::WorkspaceCoordinator::open(&dir, ORIGIN.into(), ACCOUNT.into()).unwrap();
    assert!(matches!(
        workspace.stage(key, WorkspaceEdit::Delete),
        Err(crate::WorkspaceCoordError::Protocol(
            ProtocolFenceError::ProductOutboxRequired
        ))
    ));
    assert_eq!(store.load().unwrap().unwrap().state, upgraded.state);
}

#[test]
fn finish_holds_workspace_state_lock_until_protocol_marker_is_written() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], None).unwrap();
    let cutover = fence.try_cutover().unwrap();
    cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]).unwrap();
    let coordinator =
        crate::WorkspaceCoordinator::open(&dir, ORIGIN.into(), ACCOUNT.into()).unwrap();
    coordinator.bootstrap_reviewed(&preview).unwrap();
    let store = WorkspaceStore::open(&dir, ORIGIN, ACCOUNT).unwrap();
    let snapshot = store.load().unwrap().unwrap();
    let mut newer = snapshot.state.clone();
    let key = cloud_client::WorkspaceKey::new("plugin-selections", "acme.plugin").unwrap();
    let selection = PluginSelection {
        schema_version: 1,
        plugin_id: "acme.plugin".into(),
        version: "1.0.0".into(),
        enabled: true,
    };
    newer
        .stage(
            key,
            cloud_client::WorkspaceEdit::Put(serde_json::to_value(selection).unwrap()),
        )
        .unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let writer_dir = dir.clone();
    let mut writer = None;
    cutover
        .finish_with_hook(&preview, ORIGIN, ACCOUNT, || {
            assert!(store
                .try_with_locked_snapshot::<_, StoreError>(|_| Ok(()))
                .unwrap()
                .is_none());
            writer = Some(std::thread::spawn(move || {
                let store = WorkspaceStore::open(&writer_dir, ORIGIN, ACCOUNT).unwrap();
                started_tx.send(()).unwrap();
                let result = store.save(Some(snapshot.generation), &newer);
                done_tx.send(result.is_ok()).unwrap();
            }));
            started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
            assert!(done_rx.recv_timeout(Duration::from_millis(100)).is_err());
        })
        .unwrap();
    assert!(done_rx.recv_timeout(Duration::from_secs(3)).unwrap());
    writer.unwrap().join().unwrap();
    assert_eq!(store.load().unwrap().unwrap().state.entries.len(), 1);
    drop(cutover);
    assert!(fence.try_v2(ORIGIN, ACCOUNT).is_ok());
}

#[test]
fn prepared_cutover_binds_full_legacy_snapshot_across_restart() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let store = StateStore::open(&dir).unwrap();
    let legacy = store
        .save(None, &SyncState::new(ORIGIN.into(), ACCOUNT.into()))
        .unwrap();
    let reviewed = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], Some(&legacy)).unwrap();
    let cutover = fence.try_cutover().unwrap();
    cutover.prepare(&reviewed, ORIGIN, ACCOUNT, &[]).unwrap();
    drop(cutover);

    // A writer outside the protocol changes only the legacy cursor; there
    // are still no staged entries, so a digest of entries alone collides.
    let mut changed = legacy.state.clone();
    changed.cursor = 1;
    let changed = store.save(Some(legacy.generation), &changed).unwrap();
    let fresh = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], Some(&changed)).unwrap();
    assert_eq!(reviewed.entries(), fresh.entries());
    let cutover = ProtocolFence::open(&dir).unwrap().try_cutover().unwrap();
    assert!(matches!(
        cutover.prepare(&fresh, ORIGIN, ACCOUNT, &[]),
        Err(ProtocolFenceError::StaleCutover)
    ));
    assert!(matches!(
        cutover.prepare(&reviewed, ORIGIN, ACCOUNT, &[]),
        Err(ProtocolFenceError::StaleCutover)
    ));
    drop(cutover);

    // Returning the bytes to their old values still has a new source
    // generation and cannot revive the original review.
    let restored = store.save(Some(changed.generation), &legacy.state).unwrap();
    let restored_preview =
        CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], Some(&restored)).unwrap();
    let cutover = ProtocolFence::open(&dir).unwrap().try_cutover().unwrap();
    assert!(matches!(
        cutover.prepare(&restored_preview, ORIGIN, ACCOUNT, &[]),
        Err(ProtocolFenceError::StaleCutover)
    ));
    assert!(matches!(
        cutover.finish(&restored_preview, ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::StaleCutover)
    ));
}

#[test]
fn legacy_preparation_is_refused_but_completed_v2_remains_readable() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], None).unwrap();
    let legacy_digest = Sha256::digest(serde_json::to_vec(preview.entries()).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut record = ProtocolRecord {
        format_version: 1,
        origin: ORIGIN.into(),
        account_id: ACCOUNT.into(),
        mode: ProtocolMode::Preparing,
        candidate_sha256: legacy_digest,
    };
    fence.write_record(&record).unwrap();
    let cutover = fence.try_cutover().unwrap();
    assert!(matches!(
        cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]),
        Err(ProtocolFenceError::StaleCutover)
    ));
    assert!(matches!(
        cutover.finish(&preview, ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::StaleCutover)
    ));
    drop(cutover);

    // Existing completed protocol selection must not silently revert to v1.
    record.mode = ProtocolMode::V2;
    fence.write_record(&record).unwrap();
    assert!(fence.try_v2(ORIGIN, ACCOUNT).is_ok());
    let v1 = fence.try_v1().unwrap();
    assert!(matches!(
        v1.ensure_v1(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::NotV1)
    ));
}

#[test]
fn oversized_cutover_and_outbox_markers_do_not_disable_v1() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let origin = format!("https://{}.example", "a".repeat(5000));
    // A syntactically valid origin may still exceed a marker's byte cap.
    let preview = CutoverPreview::new(origin.clone(), ACCOUNT.into(), &[], None).unwrap();
    let cutover = fence.try_cutover().unwrap();
    assert!(matches!(
        cutover.prepare(&preview, &origin, ACCOUNT, &[]),
        Err(ProtocolFenceError::Store(StoreError::InvalidState))
    ));
    assert!(!fence.record_path(&origin, ACCOUNT).unwrap().exists());
    assert!(matches!(
        cutover.require_product_outbox(&origin, ACCOUNT),
        Err(ProtocolFenceError::Store(StoreError::InvalidState))
    ));
    assert!(!fence
        .product_outbox_path(&origin, ACCOUNT)
        .unwrap()
        .exists());
    drop(cutover);
    let v1 = fence.try_v1().unwrap();
    assert!(v1.ensure_v1(&origin, ACCOUNT).is_ok());
    assert!(v1.ensure_legacy_allowed(&origin, ACCOUNT).is_ok());
}

#[test]
fn oversized_uncertain_marker_fails_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let origin = format!("https://{}.example", "a".repeat(9000));
    let mut state = SyncState::new(origin.clone(), ACCOUNT.into());
    state
        .stage("acme.plugin".into(), cloud_client::Edit::Delete)
        .unwrap();
    let snapshot = StateSnapshot {
        generation: 1,
        state,
    };
    let v1 = fence.try_v1().unwrap();
    assert!(matches!(
        v1.prepare_upload(&snapshot, "acme.plugin"),
        Err(ProtocolFenceError::Store(StoreError::InvalidState))
    ));
    assert_eq!(fence.uncertain_count(&origin, ACCOUNT).unwrap(), 0);
    assert!(fence
        .uncertain_records(&origin, ACCOUNT)
        .unwrap()
        .is_empty());
    assert!(v1.ensure_v1(&origin, ACCOUNT).is_ok());
}

#[test]
fn ambiguous_uploads_are_per_identity_and_survive_restart() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let first = fence.try_v1().unwrap();
    let second = fence.try_v1().unwrap();
    let network = first.try_network().unwrap();
    assert!(matches!(
        second.try_network(),
        Err(ProtocolFenceError::Busy)
    ));
    drop(network);
    let mut source = SyncState::new(ORIGIN.into(), ACCOUNT.into());
    source
        .stage(
            "acme.plugin".into(),
            cloud_client::Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "1.0.0".into(),
                enabled: false,
            }),
        )
        .unwrap();
    let uncertain = first
        .prepare_upload(
            &StateSnapshot {
                generation: 1,
                state: source.clone(),
            },
            "acme.plugin",
        )
        .unwrap();
    source
        .stage(
            "acme.plugin".into(),
            cloud_client::Edit::Put(PluginSelection {
                schema_version: 1,
                plugin_id: "acme.plugin".into(),
                version: "2.0.0".into(),
                enabled: true,
            }),
        )
        .unwrap();
    let known = second
        .prepare_upload(
            &StateSnapshot {
                generation: 2,
                state: source.clone(),
            },
            "acme.plugin",
        )
        .unwrap();
    second.acknowledge_upload(known).unwrap();
    drop(uncertain); // Simulates cancellation after dispatch.
    drop(first);
    drop(second);
    let cutover = ProtocolFence::open(&dir).unwrap().try_cutover().unwrap();
    assert_eq!(cutover.uncertain_uploads(ORIGIN, ACCOUNT).unwrap(), 1);
    let frozen = cutover.uncertain_details(ORIGIN, ACCOUNT).unwrap();
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].key(), "acme.plugin");
    assert!(
        matches!(&frozen[0].pending().edit, cloud_client::Edit::Put(value) if value.version == "1.0.0" && !value.enabled)
    );
    assert!(frozen[0].base_record().is_none());
    assert_eq!(cutover.uncertain_uploads(ORIGIN, "account-b").unwrap(), 0);
    let preview = CutoverPreview::new(ORIGIN.into(), ACCOUNT.into(), &[], None).unwrap();
    assert!(matches!(
        cutover.prepare(&preview, ORIGIN, ACCOUNT, &[]),
        Err(ProtocolFenceError::UncertainV1)
    ));
}

#[test]
fn corrupt_protocol_record_is_never_treated_as_v1() {
    let temp = tempfile::tempdir().unwrap();
    let fence = ProtocolFence::open(temp.path().join("private")).unwrap();
    let path = fence.record_path(ORIGIN, ACCOUNT).unwrap();
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).unwrap().write_all(b"bad").unwrap();
    assert!(matches!(
        fence.try_v1().unwrap().ensure_v1(ORIGIN, ACCOUNT),
        Err(ProtocolFenceError::Store(StoreError::InvalidState))
    ));
}

#[test]
fn child_shared_lease() {
    let Some(dir) = std::env::var_os("TIKTOOLS_FENCE_CHILD_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    let fence = ProtocolFence::open(&dir).unwrap();
    let _lease = fence.try_v1().unwrap();
    fs::write(dir.join("child-ready"), b"ready").unwrap();
    let until = Instant::now() + Duration::from_secs(20);
    while !dir.join("child-exit").exists() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(dir.join("child-exit").exists());
}

#[test]
fn child_product_emission_lease() {
    let Some(dir) = std::env::var_os("TIKTOOLS_PRODUCT_EMISSION_CHILD_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    let fence = ProductEmissionFence::open(&dir).unwrap();
    let _lease = fence.try_emission().unwrap();
    fs::write(dir.join("emission-child-ready"), b"ready").unwrap();
    let until = Instant::now() + Duration::from_secs(20);
    while !dir.join("emission-child-exit").exists() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(dir.join("emission-child-exit").exists());
}

#[test]
fn product_emission_and_untracked_mutation_exclude_each_other_across_processes() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("product-fence");
    let fence = ProductEmissionFence::open(&dir).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "protocol_fence::tests::child_product_emission_lease",
        ])
        .env("TIKTOOLS_PRODUCT_EMISSION_CHILD_DIR", &dir)
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !dir.join("emission-child-ready").exists() && Instant::now() < until {
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(10));
    }
    if !dir.join("emission-child-ready").exists() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("child did not hold product emission lease");
    }
    assert!(matches!(
        fence.try_untracked_edit(),
        Err(ProtocolFenceError::Busy)
    ));
    fs::write(dir.join("emission-child-exit"), b"done").unwrap();
    assert!(child.wait().unwrap().success());
    let mutation = fence.try_untracked_edit().unwrap();
    assert!(matches!(
        fence.try_emission(),
        Err(ProtocolFenceError::Busy)
    ));
    drop(mutation);
    assert!(fence.try_emission().is_ok());
}

#[cfg(unix)]
#[test]
fn dropped_lease_unlocks_before_forked_child_executes() {
    use std::os::unix::{net::UnixStream, process::CommandExt};

    let temp = tempfile::tempdir().unwrap();
    let fence = ProtocolFence::open(temp.path().join("private")).unwrap();
    // A raw lock serves as a control: closing it in the parent leaves it
    // held until the inherited descriptor is closed by the child.
    let raw = fence.lock_file_named("fork-control.lock").unwrap();
    FileExt::try_lock_exclusive(&raw).unwrap();
    let gate = fence.try_cutover().unwrap();
    let (mut parent, mut child) = UnixStream::pair().unwrap();
    parent
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    child
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "protocol_fence::tests::child_shared_lease"])
        .env_remove("TIKTOOLS_FENCE_CHILD_DIR")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // The closure performs only socket reads/writes, and reports both
    // failures to spawn without panicking or allocating in the child.
    unsafe {
        command.pre_exec(move || {
            child.write_all(&[1])?;
            let mut resume = [0u8; 1];
            child.read_exact(&mut resume)?;
            Ok(())
        });
    }
    // Command::spawn waits for exec, so perform it on another thread while
    // the main test observes the deliberately paused pre-exec child.
    let handle = std::thread::spawn(move || command.spawn().and_then(|mut child| child.wait()));
    let mut ready = [0u8; 1];
    parent.read_exact(&mut ready).unwrap();
    drop(raw);
    drop(gate);
    let control = fence.lock_file_named("fork-control.lock").unwrap();
    let control_held = matches!(FileExt::try_lock_shared(&control), Err(ref error) if error.kind() == io::ErrorKind::WouldBlock);
    let lease = fence.try_v1();
    parent.write_all(&[1]).unwrap();
    assert!(handle.join().unwrap().unwrap().success());
    assert!(control_held, "control flock must remain held across fork");
    assert!(
        lease.is_ok(),
        "the dropped lease must release its flock before exec"
    );
}

#[test]
fn cutover_waits_for_v1_in_a_separate_process() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("private");
    let fence = ProtocolFence::open(&dir).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "protocol_fence::tests::child_shared_lease"])
        .env("TIKTOOLS_FENCE_CHILD_DIR", &dir)
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !dir.join("child-ready").exists() && Instant::now() < until {
        assert!(child.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(10));
    }
    if !dir.join("child-ready").exists() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("child did not acquire v1 lease");
    }
    assert!(matches!(fence.try_cutover(), Err(ProtocolFenceError::Busy)));
    fs::write(dir.join("child-exit"), b"done").unwrap();
    assert!(child.wait().unwrap().success());
    assert!(fence.try_cutover().is_ok());
}
