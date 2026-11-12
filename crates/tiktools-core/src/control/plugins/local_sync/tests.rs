use super::*;
use std::{path::PathBuf, sync::Arc};
use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore};
use tiktools_cloud_sync::SyncCoordinator;

struct Fixture {
    root: PathBuf,
    db: crate::db::DatabaseManager,
    auth: CloudAuth,
    sync: SyncCoordinator,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "plugin-mutation-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = crate::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let sidecar = paths.data.join("account.json");
        std::fs::write(
            &sidecar,
            br#"{"origin":"https://cloud.example","account_id":"alice"}"#,
        )
        .unwrap();
        let auth = CloudAuth::new(
            Some("https://cloud.example".into()),
            sidecar,
            Arc::new(MemoryTokenStore::new()),
        );
        let sync = SyncCoordinator::new(auth.clone(), paths.data.join("cloud-sync")).unwrap();
        let db = crate::db::DatabaseManager::new(paths);
        Self {
            root,
            db,
            auth,
            sync,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn reservation_requires_runtime_confirmation_and_detects_a_newer_edit() {
    let f = Fixture::new();
    f.sync.set_enabled(true).unwrap();
    let first =
        reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, false).unwrap();
    let entry =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    assert_eq!(entry, *first.candidate.as_ref().unwrap());
    let epoch = "a".repeat(64);
    // No runtime observation yet: no consumer can acknowledge this edit.
    assert!(!f.db.acknowledge_plugin_sync_intent(&entry, &epoch).unwrap());
    let second =
        reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, true).unwrap();
    assert!(confirm_local_plugin_mutation(&f.db, &first, &epoch).is_err());
    confirm_local_plugin_mutation(&f.db, &second, &epoch).unwrap();
    let updated =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    assert_eq!(updated.enabled, Some(true));
    assert_eq!(updated.runtime_epoch.as_deref(), Some(epoch.as_str()));
    assert!(!f.db.acknowledge_plugin_sync_intent(&entry, &epoch).unwrap());
}

#[test]
fn opt_out_records_local_edit_without_a_new_cloud_candidate() {
    let f = Fixture::new();
    f.sync.set_enabled(true).unwrap();
    reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, true).unwrap();
    f.sync.set_enabled(false).unwrap();
    let local =
        reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, false).unwrap();
    assert!(local.candidate.is_none());
    let old =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    assert_ne!(old.generation, old.source_generation);
}

#[test]
fn v2_local_reservation_preserves_local_edits_and_rejects_legacy_consent() {
    let f = Fixture::new();
    f.sync.set_enabled(true).unwrap();
    reserve_local_plugin_mutation(&f.sync, &f.db, "hotkeys", "1.0.0", true, true).unwrap();
    let previous =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    let prepared = std::cell::Cell::new(false);
    assert!(reserve_local_plugin_mutation_prepared_for(
        &f.sync,
        &f.db,
        "hotkeys",
        "1.0.0",
        true,
        false,
        LocalMutationProtocol::V2,
        |_| {
            prepared.set(true);
            Ok(())
        },
    )
    .is_err());
    assert!(!prepared.get());
    assert_eq!(
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap(),
        previous
    );

    f.sync.set_enabled(false).unwrap();
    let (local, ()) = reserve_local_plugin_mutation_prepared_for(
        &f.sync,
        &f.db,
        "hotkeys",
        "1.0.0",
        true,
        false,
        LocalMutationProtocol::V2,
        |has_candidate| {
            assert!(!has_candidate);
            Ok(())
        },
    )
    .unwrap();
    assert!(local.candidate.is_none());
    let pending =
        f.db.next_plugin_sync_intent("https://cloud.example", "alice")
            .unwrap()
            .unwrap();
    assert_eq!(pending.generation, previous.generation);
    assert_ne!(pending.source_generation, previous.source_generation);
}

#[test]
fn runtime_identity_is_needed_only_for_an_opted_in_candidate() {
    let f = Fixture::new();
    let (local, value) = reserve_local_plugin_mutation_prepared(
        &f.sync,
        &f.db,
        "acme.local",
        "1.0.0",
        true,
        false,
        |has_candidate| {
            assert!(!has_candidate);
            Ok("local-only")
        },
    )
    .unwrap();
    assert!(local.candidate.is_none());
    assert_eq!(value, "local-only");

    f.sync.set_enabled(true).unwrap();
    let error = reserve_local_plugin_mutation_prepared(
        &f.sync,
        &f.db,
        "acme.blocked",
        "1.0.0",
        true,
        true,
        |has_candidate| {
            assert!(has_candidate);
            Err::<(), _>(OperationError::unavailable("no runtime identity"))
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "unavailable");
    assert!(f
        .db
        .next_plugin_sync_intent("https://cloud.example", "alice")
        .unwrap()
        .is_none());
    let snapshot = f.db.load_behavior_snapshot().unwrap();
    assert!(!snapshot["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .any(|plugin| plugin["id"] == "acme.blocked"));
}

#[test]
fn sqlite_adapter_requires_confirmed_exact_source_and_retains_receipt() {
    use crate::cloud_sync::product_outbox::SqliteProductOutbox;
    use tiktools_cloud_sync::{V1LocalEditContext, V1Pending, V1ProductOutbox};

    let f = Fixture::new();
    f.sync.set_enabled(true).unwrap();
    let reservation =
        reserve_local_plugin_mutation(&f.sync, &f.db, "acme.plugin", "1.0.0", true, false).unwrap();
    let entry = reservation.candidate.as_ref().unwrap();
    let context = V1LocalEditContext {
        origin: entry.origin.clone(),
        account_id: entry.account_id.clone(),
        config_generation: entry.config_generation,
    };
    let epoch = "a".repeat(64);
    let mut adapter = SqliteProductOutbox::new(&f.db, &epoch);
    assert!(adapter.next(&context).is_err());
    confirm_local_plugin_mutation(&f.db, &reservation, &epoch).unwrap();
    let candidate = adapter.next(&context).unwrap().unwrap();
    let pending = V1Pending {
        base_revision: Some(4),
        edit: candidate.edit.clone(),
        conflict: false,
    };
    let mut changed = candidate.clone();
    changed.generation += 1;
    assert!(!adapter.acknowledge(&context, &changed, &pending).unwrap());
    assert!(f
        .db
        .next_plugin_sync_intent(&context.origin, &context.account_id)
        .unwrap()
        .is_some());
    let candidate = adapter.next(&context).unwrap().unwrap();
    assert!(adapter.acknowledge(&context, &candidate, &pending).unwrap());
    assert!(adapter
        .validate_pending(&context, "acme.plugin", &pending)
        .unwrap());
    let mut conflict = pending.clone();
    conflict.conflict = true;
    let proof = adapter
        .conflict_source_proof(&context, "acme.plugin", &conflict)
        .unwrap()
        .unwrap();
    assert_eq!(proof.generation(), candidate.generation);
    let mut rebased = pending.clone();
    rebased.base_revision = Some(5);
    assert!(!adapter
        .validate_pending(&context, "acme.plugin", &rebased)
        .unwrap());
    rebased.conflict = true;
    assert!(adapter
        .conflict_source_proof(&context, "acme.plugin", &rebased)
        .unwrap()
        .is_none());
    // Even an older writer that failed to advance the source generation
    // must not authorize review while the persisted state differs.
    rusqlite::Connection::open(f.db.paths().automation_database())
        .unwrap()
        .execute(
            "UPDATE behavior_plugins SET enabled=1 WHERE id='acme.plugin'",
            [],
        )
        .unwrap();
    assert!(adapter
        .conflict_source_proof(&context, "acme.plugin", &conflict)
        .unwrap()
        .is_none());
    reserve_local_plugin_mutation(&f.sync, &f.db, "acme.plugin", "1.0.0", true, true).unwrap();
    assert!(!adapter
        .validate_pending(&context, "acme.plugin", &pending)
        .unwrap());
    assert!(adapter
        .conflict_source_proof(&context, "acme.plugin", &conflict)
        .unwrap()
        .is_none());
}

#[test]
fn runtime_epoch_is_stable_per_host_and_changes_after_recreation() {
    let f = Fixture::new();
    let first = crate::cloud_sync::CloudSync::new(f.db.paths(), &f.auth);
    let second = crate::cloud_sync::CloudSync::new(f.db.paths(), &f.auth);
    let a = first.runtime_epoch().unwrap();
    assert_eq!(a, first.runtime_epoch().unwrap());
    assert_eq!(a.len(), 64);
    assert_ne!(a, second.runtime_epoch().unwrap());
}
