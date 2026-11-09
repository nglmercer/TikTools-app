use super::*;
use serde_json::json;

fn state() -> WorkspaceState {
    WorkspaceState::new("https://cloud.example".into(), "account".into()).unwrap()
}
fn key(id: &str) -> WorkspaceKey {
    WorkspaceKey::new("plugin-selections", id).unwrap()
}
fn selection(id: &str, enabled: bool) -> Value {
    json!({"schemaVersion":1,"pluginId":id,"version":"1.0.0","enabled":enabled})
}
fn record(id: &str, revision: i64, enabled: bool) -> WorkspaceRecord {
    WorkspaceRecord {
        collection: "plugin-selections".into(),
        key: id.into(),
        revision,
        deleted: false,
        value: Some(selection(id, enabled)),
    }
}
fn read(target: WorkspaceKey, record: Option<WorkspaceRecord>) -> WorkspaceRead {
    WorkspaceRead {
        origin: "https://cloud.example".into(),
        account_id: "account".into(),
        target,
        record,
    }
}
fn page(records: Vec<WorkspaceRecord>, cursor: i64) -> WorkspaceChanges {
    WorkspaceChanges {
        changes: records,
        cursor,
        has_more: false,
    }
}
fn template() -> Value {
    json!({"templateVersion":1,"id":"same","title":"Template","actions":[{"name":"Log","typeId":"core.log","config":{"message":"safe"}}],"event":{"name":"Event","trigger":"tiktok.chat"}})
}

#[test]
fn all_five_collections_use_composite_keys_and_refuse_invalid_portable_data_atomically() {
    let mut s = state();
    for (collection, id, value) in [
        (
            "plugin-selections",
            "test.plugin",
            selection("test.plugin", false),
        ),
        (
            "plugin-settings",
            "test.plugin",
            json!({"schemaVersion":1,"pluginId":"test.plugin","settings":{"volume":0.5}}),
        ),
        ("rule-templates", "same", template()),
        (
            "profiles",
            "same",
            json!({"profileVersion":1,"id":"same","name":"Profile","templates":[{"template":template()}]}),
        ),
        (
            "preferences",
            "desktop",
            json!({"schemaVersion":1,"theme":"dark"}),
        ),
    ] {
        s.stage(
            WorkspaceKey::new(collection, id).unwrap(),
            WorkspaceEdit::Put(value),
        )
        .unwrap();
    }
    assert_eq!(s.entries.len(), 5);
    s.validate().unwrap();
    let before = s.clone();
    assert!(s
        .stage(
            WorkspaceKey {
                collection: "unknown".into(),
                key: "same".into()
            },
            WorkspaceEdit::Delete
        )
        .is_err());
    assert!(s.stage(WorkspaceKey::new("plugin-settings","test.plugin").unwrap(),WorkspaceEdit::Put(json!({"schemaVersion":1,"pluginId":"test.plugin","settings":{"apiKey":"literal-secret"}}))).is_err());
    assert_eq!(s, before);
}

#[test]
fn prepared_request_is_frozen_and_own_ack_retains_newer_local_edit_without_skipping_cursor() {
    let mut s = state();
    let target = key("test.plugin");
    s.apply_changes(page(vec![record("test.plugin", 1, false)], 1))
        .unwrap();
    let original = s
        .stage(
            target.clone(),
            WorkspaceEdit::Put(selection("test.plugin", true)),
        )
        .unwrap();
    let ticket = s.prepare_push(&target).unwrap();
    assert_eq!(ticket.local_generation(), original);
    assert_eq!(ticket.base_revision(), Some(1));
    let newer = s
        .stage(
            target.clone(),
            WorkspaceEdit::Put(selection("test.plugin", false)),
        )
        .unwrap();
    assert!(newer > original);
    assert_eq!(
        s.entry(&target).unwrap().pending.as_ref().unwrap().edit,
        *ticket.edit()
    );
    s.acknowledge(&ticket, record("test.plugin", 2, true))
        .unwrap();
    let pending = s.entry(&target).unwrap().pending.as_ref().unwrap();
    assert_eq!(pending.local_generation, newer);
    assert_eq!(pending.base_revision, Some(2));
    assert_eq!(pending.status, WorkspacePendingStatus::Queued);
    assert_eq!(s.cursor, 1);
    assert!(s.ready_remote().is_empty());
    assert!(matches!(
        s.acknowledge(&ticket, record("test.plugin", 2, true)),
        Err(WorkspaceStateError::StaleReview)
    ));
    s.apply_changes(page(vec![record("test.plugin", 2, true)], 2))
        .unwrap();
    assert_eq!(s.cursor, 2);
    let next = s.prepare_push(&target).unwrap();
    s.acknowledge(&next, record("test.plugin", 3, false))
        .unwrap();
    assert_eq!(s.ready_remote().len(), 1);
    assert_eq!(s.cursor, 2);
    s.validate().unwrap();
}

#[test]
fn lost_reply_equal_body_remains_unconfirmed_until_fresh_explicit_review() {
    let mut s = state();
    let target = key("test.plugin");
    let generation = s
        .stage(
            target.clone(),
            WorkspaceEdit::Put(selection("test.plugin", true)),
        )
        .unwrap();
    let ticket = s.prepare_push(&target).unwrap();
    // The server committed but its HTTP response was lost. Observing equal
    // source does not prove this operation was the original successful writer.
    s.apply_changes(page(vec![record("test.plugin", 1, true)], 1))
        .unwrap();
    s.observe_read(&read(target.clone(), Some(record("test.plugin", 1, true))))
        .unwrap();
    assert_eq!(
        s.entry(&target).unwrap().pending.as_ref().unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    assert!(matches!(
        s.prepare_push(&target),
        Err(WorkspaceStateError::Conflict)
    ));
    let before = s.clone();
    assert!(matches!(
        s.resolve(
            &read(target.clone(), Some(record("test.plugin", 2, false))),
            Some(1),
            generation,
            true
        ),
        Err(WorkspaceStateError::StaleReview)
    ));
    assert!(matches!(
        s.resolve(
            &read(target.clone(), Some(record("test.plugin", 1, true))),
            Some(1),
            generation + 1,
            true
        ),
        Err(WorkspaceStateError::StaleReview)
    ));
    assert_eq!(s, before);
    s.resolve(
        &read(target.clone(), Some(record("test.plugin", 1, true))),
        Some(1),
        generation,
        true,
    )
    .unwrap();
    assert_eq!(
        s.entry(&target)
            .unwrap()
            .pending
            .as_ref()
            .unwrap()
            .base_revision,
        Some(1)
    );
    assert!(matches!(
        s.acknowledge(&ticket, record("test.plugin", 1, true)),
        Err(WorkspaceStateError::StaleReview)
    ));
    // V2 conditional no-ops may retain their current revision.
    let next = s.prepare_push(&target).unwrap();
    s.acknowledge(&next, record("test.plugin", 1, true))
        .unwrap();
    assert!(s.pending_keys().is_empty());
}

#[test]
fn delete_without_base_requires_verified_absence_or_explicit_conflict_resolution() {
    let mut s = state();
    let target = key("test.plugin");
    let first_generation = s.stage(target.clone(), WorkspaceEdit::Delete).unwrap();
    assert!(matches!(
        s.prepare_push(&target),
        Err(WorkspaceStateError::NeedsRead)
    ));
    let absence = read(target.clone(), None);
    s.observe_read(&absence).unwrap();
    assert_eq!(s.pending_keys(), vec![target.clone()]);
    s.complete_absent_delete(&absence, first_generation)
        .unwrap();
    assert!(s.pending_keys().is_empty());
    assert!(s.entries.is_empty());
    let generation = s.stage(target.clone(), WorkspaceEdit::Delete).unwrap();
    s.observe_read(&read(target.clone(), Some(record("test.plugin", 1, false))))
        .unwrap();
    assert_eq!(
        s.entry(&target).unwrap().pending.as_ref().unwrap().status,
        WorkspacePendingStatus::Conflict
    );
    s.resolve(
        &read(target.clone(), Some(record("test.plugin", 1, false))),
        Some(1),
        generation,
        true,
    )
    .unwrap();
    let ticket = s.prepare_push(&target).unwrap();
    let tombstone = WorkspaceRecord {
        value: None,
        deleted: true,
        ..record("test.plugin", 2, false)
    };
    s.acknowledge(&ticket, tombstone).unwrap();
    s.stage(
        target.clone(),
        WorkspaceEdit::Put(selection("test.plugin", true)),
    )
    .unwrap();
    assert_eq!(s.prepare_push(&target).unwrap().base_revision(), Some(2));
}

#[test]
fn held_absence_proof_cannot_clear_a_newer_equal_delete_generation() {
    let mut s = state();
    let target = key("test.plugin");
    let captured = s.stage(target.clone(), WorkspaceEdit::Delete).unwrap();
    let held_absence = read(target.clone(), None);
    let newer = s.stage(target.clone(), WorkspaceEdit::Delete).unwrap();
    assert!(newer > captured);
    let before = s.clone();
    s.observe_read(&held_absence).unwrap();
    assert_eq!(s, before);
    assert!(matches!(
        s.complete_absent_delete(&held_absence, captured),
        Err(WorkspaceStateError::StaleReview)
    ));
    assert_eq!(s, before);
    // An actual new read after capturing the newer intent can finish it.
    s.complete_absent_delete(&read(target, None), newer)
        .unwrap();
    assert!(s.entries.is_empty());
}

#[test]
fn source_pages_are_atomic_and_cannot_clear_unknown_or_rewind_ahead_ack() {
    let mut s = state();
    let target = key("test.plugin");
    s.stage(
        target.clone(),
        WorkspaceEdit::Put(selection("test.plugin", true)),
    )
    .unwrap();
    let ticket = s.prepare_push(&target).unwrap();
    s.acknowledge(&ticket, record("test.plugin", 5, true))
        .unwrap();
    s.apply_changes(page(
        vec![
            record("test.other", 1, false),
            record("test.plugin", 2, false),
        ],
        2,
    ))
    .unwrap();
    assert_eq!(
        s.entry(&target).unwrap().remote.as_ref().unwrap().revision,
        5
    );
    let before = s.clone();
    assert!(s
        .apply_changes(page(
            vec![
                record("test.new", 3, false),
                record("test.plugin", 5, false)
            ],
            5
        ))
        .is_err());
    assert_eq!(s, before);
    assert_eq!(s.cursor, 2);
}

#[test]
fn expired_cursor_blocks_dispatch_and_application_until_explicit_complete_snapshot_rebuild() {
    let mut s = state();
    let target = key("test.plugin");
    s.apply_changes(page(vec![record("test.plugin", 1, false)], 1))
        .unwrap();
    s.stage(
        target.clone(),
        WorkspaceEdit::Put(selection("test.plugin", true)),
    )
    .unwrap();
    let ticket = s.prepare_push(&target).unwrap();
    s.stage(target.clone(), WorkspaceEdit::Delete).unwrap();
    s.require_rebuild();
    assert!(s.ready_remote().is_empty());
    assert!(matches!(
        s.prepare_push(&target),
        Err(WorkspaceStateError::NeedsRebuild)
    ));
    let before = s.clone();
    assert!(s
        .apply_changes(page(vec![record("test.plugin", 2, true)], 2))
        .is_err());
    assert_eq!(s, before);
    s.install_rebuild(WorkspaceSnapshot {
        schema_version: 2,
        cursor: 2,
        records: vec![record("test.plugin", 2, true)],
    })
    .unwrap();
    assert!(!s.needs_rebuild);
    assert_eq!(s.cursor, 2);
    assert!(s.ready_remote().is_empty());
    assert_eq!(
        s.entry(&target).unwrap().pending.as_ref().unwrap().status,
        WorkspacePendingStatus::Unconfirmed
    );
    assert!(matches!(
        s.entry(&target).unwrap().next.as_ref().unwrap().edit,
        WorkspaceEdit::Delete
    ));
    s.mark_conflict(&ticket).unwrap();
    s.validate().unwrap();
}

#[test]
fn snapshot_review_cannot_rewind_cursor_point_reads_retained_ids_or_immutable_body() {
    let mut s = state();
    let target = key("test.plugin");
    s.apply_changes(page(vec![record("test.plugin", 2, true)], 2))
        .unwrap();
    s.observe_read(&read(target.clone(), Some(record("test.plugin", 5, true))))
        .unwrap();
    s.require_rebuild();
    let before = s.clone();
    for (cursor, records) in [
        (1, vec![]),
        (4, vec![record("test.plugin", 4, true)]),
        (5, vec![]),
        (6, vec![record("test.plugin", 4, true)]),
        (6, vec![record("test.plugin", 5, false)]),
    ] {
        assert!(matches!(
            s.install_rebuild(WorkspaceSnapshot {
                schema_version: 2,
                cursor,
                records
            }),
            Err(WorkspaceStateError::StaleReview)
        ));
        assert_eq!(s, before);
    }
    s.install_rebuild(WorkspaceSnapshot {
        schema_version: 2,
        cursor: 6,
        records: vec![record("test.plugin", 6, false)],
    })
    .unwrap();
    assert_eq!(s.cursor, 6);
}

#[test]
fn resolution_is_bound_to_account_current_remote_and_latest_local_generation() {
    let mut s = state();
    let target = key("test.plugin");
    let old = s
        .stage(
            target.clone(),
            WorkspaceEdit::Put(selection("test.plugin", true)),
        )
        .unwrap();
    s.prepare_push(&target).unwrap();
    let latest = s.stage(target.clone(), WorkspaceEdit::Delete).unwrap();
    let before = s.clone();
    let mut foreign = read(target.clone(), None);
    foreign.account_id = "other".into();
    assert!(matches!(
        s.resolve(&foreign, None, latest, true),
        Err(WorkspaceStateError::WrongIdentity)
    ));
    assert!(matches!(
        s.resolve(&read(target.clone(), None), None, old, true),
        Err(WorkspaceStateError::StaleReview)
    ));
    assert_eq!(s, before);
    s.resolve(&read(target, None), None, latest, false).unwrap();
    assert!(s.entries.is_empty());
}

#[test]
fn v1_cutover_preserves_source_files_and_pending_choices_without_copying_base_or_cursor() {
    let mut legacy = SyncState::new("https://cloud.example".into(), "account".into());
    legacy.cursor = 99;
    legacy
        .stage(
            "test.plugin".into(),
            crate::Edit::Put(crate::PluginSelection {
                schema_version: 1,
                plugin_id: "test.plugin".into(),
                version: "1.0.0".into(),
                enabled: true,
            }),
        )
        .unwrap();
    legacy
        .entries
        .get_mut("test.plugin")
        .unwrap()
        .pending
        .as_mut()
        .unwrap()
        .base_revision = Some(77);
    let source = serde_json::to_vec(&legacy).unwrap();
    let mut s = state();
    s.stage(
        key("test.plugin"),
        WorkspaceEdit::Put(selection("test.plugin", false)),
    )
    .unwrap();
    assert_eq!(s.import_v1_pending(&legacy).unwrap(), 1);
    assert_eq!(s.cursor, 0);
    let entry = s.entry(&key("test.plugin")).unwrap();
    assert_eq!(entry.pending.as_ref().unwrap().base_revision, None);
    assert_eq!(
        entry.pending.as_ref().unwrap().status,
        WorkspacePendingStatus::Conflict
    );
    assert_eq!(
        entry.next.as_ref().unwrap().edit,
        WorkspaceEdit::Put(selection("test.plugin", false))
    );
    assert_eq!(serde_json::to_vec(&legacy).unwrap(), source);
    s.mark_bootstrap_complete();
    let before = s.clone();
    assert!(s.import_v1_pending(&legacy).is_err());
    assert_eq!(s, before);
    let mut foreign = legacy;
    foreign.account_id = "other".into();
    assert!(matches!(
        state().import_v1_pending(&foreign),
        Err(WorkspaceStateError::WrongIdentity)
    ));
}

#[test]
fn entry_and_value_capacity_failures_leave_every_prior_edit_intact() {
    let mut s = state();
    for index in 0..MAX_ENTRIES {
        let target = key(&format!("test.p{index:04}"));
        s.entries.push(WorkspaceEntry {
            target,
            remote: None,
            pending: Some(WorkspacePending {
                base_revision: None,
                base_record: None,
                edit: WorkspaceEdit::Delete,
                local_generation: index as u64 + 1,
                status: WorkspacePendingStatus::Queued,
            }),
            next: None,
        });
    }
    s.next_local_generation = MAX_ENTRIES as u64 + 1;
    s.validate().unwrap();
    let before = s.clone();
    assert!(matches!(
        s.stage(key("test.too-many"), WorkspaceEdit::Delete),
        Err(WorkspaceStateError::TooLarge)
    ));
    assert_eq!(s, before);
    let mut s = state();
    let settings: serde_json::Map<String, Value> = (0..30)
        .map(|index| (format!("field{index}"), Value::String("x".repeat(8192))))
        .collect();
    for index in 0..34 {
        let id = format!("test.p{index:02}");
        s.stage(
            WorkspaceKey::new("plugin-settings", &id).unwrap(),
            WorkspaceEdit::Put(json!({"schemaVersion":1,"pluginId":id,"settings":settings})),
        )
        .unwrap();
    }
    let before = s.clone();
    assert!(matches!(
        s.stage(
            WorkspaceKey::new("plugin-settings", "test.overflow").unwrap(),
            WorkspaceEdit::Put(
                json!({"schemaVersion":1,"pluginId":"test.overflow","settings":settings})
            )
        ),
        Err(WorkspaceStateError::TooLarge)
    ));
    assert_eq!(s, before);
}

#[test]
fn jsonb_normalized_ack_feed_rebuild_and_resolution_preserve_exact_decimal_values() {
    let target = WorkspaceKey::new("plugin-settings", "test.plugin").unwrap();
    let sent:Value=serde_json::from_str(r#"{"schemaVersion":1,"pluginId":"test.plugin","settings":{"n":1e18,"precise":1.0000000000000001e18,"large":18446744073709551615,"array":[-0.0,1.0]}}"#).unwrap();
    let normalized:Value=serde_json::from_str(r#"{"schemaVersion":1,"pluginId":"test.plugin","settings":{"n":1000000000000000000,"precise":1000000000000000100,"large":18446744073709551615,"array":[0,1]}}"#).unwrap();
    let row = |value: Value| WorkspaceRecord {
        collection: target.collection.clone(),
        key: target.key.clone(),
        revision: 7,
        deleted: false,
        value: Some(value),
    };
    for legacy_echo in [false, true] {
        let mut s = state();
        s.stage(target.clone(), WorkspaceEdit::Put(sent.clone()))
            .unwrap();
        let ticket = s.prepare_push(&target).unwrap();
        // Older servers echoed the input, while new ones return normalized JSONB.
        s.acknowledge(
            &ticket,
            row(if legacy_echo {
                sent.clone()
            } else {
                normalized.clone()
            }),
        )
        .unwrap();
        assert!(s.entry(&target).unwrap().pending.is_none());
        assert_eq!(s.cursor, 0);
        let before = s.clone();
        let mut wrong = normalized.clone();
        wrong["settings"]["precise"] = json!(1_000_000_000_000_000_128_u64);
        assert!(matches!(
            s.apply_changes(page(vec![row(wrong)], 7)),
            Err(WorkspaceStateError::Invalid)
        ));
        assert_eq!(s, before);
        s.apply_changes(page(vec![row(normalized.clone())], 7))
            .unwrap();
        assert_eq!(s.cursor, 7);
        let mut rebuilt = before.clone();
        rebuilt
            .install_rebuild(WorkspaceSnapshot {
                schema_version: 2,
                cursor: 7,
                records: vec![row(normalized.clone())],
            })
            .unwrap();
        assert_eq!(rebuilt.cursor, 7);
        let mut resolving = before;
        let generation = resolving
            .stage(target.clone(), WorkspaceEdit::Put(sent.clone()))
            .unwrap();
        resolving
            .resolve(
                &read(target.clone(), Some(row(normalized.clone()))),
                Some(7),
                generation,
                false,
            )
            .unwrap();
        assert!(resolving.entry(&target).unwrap().pending.is_none());
    }
    // A feed can arrive between prepare and the direct ACK at the same revision.
    let mut s = state();
    s.stage(target.clone(), WorkspaceEdit::Put(sent.clone()))
        .unwrap();
    let ticket = s.prepare_push(&target).unwrap();
    s.apply_changes(page(vec![row(sent)], 7)).unwrap();
    s.acknowledge(&ticket, row(normalized)).unwrap();
    assert!(s.entry(&target).unwrap().pending.is_none());
}

#[test]
fn product_decision_receipt_survives_restart_and_snapshot_rebuild_without_restage() {
    let mut s = state();
    s.mark_bootstrap_complete();
    let target = key("acme.plugin");
    let edit = WorkspaceEdit::Put(selection("acme.plugin", true));
    let digest = "a".repeat(64);
    let (generation, staged) = s
        .stage_product_decision(target.clone(), edit.clone(), 7, &digest)
        .unwrap();
    assert!(staged);
    let mut s: WorkspaceState = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    s.validate().unwrap();
    let before = s.clone();
    assert_eq!(
        s.stage_product_decision(target.clone(), edit.clone(), 7, &digest)
            .unwrap(),
        (generation, false)
    );
    assert_eq!(s, before);
    assert!(s
        .stage_product_decision(target.clone(), edit.clone(), 6, &digest)
        .is_err());
    assert!(s
        .stage_product_decision(target.clone(), edit.clone(), 7, &"b".repeat(64))
        .is_err());
    assert!(s
        .stage_product_decision(target.clone(), WorkspaceEdit::Delete, 7, &digest)
        .is_err());
    assert_eq!(s, before);
    s.require_rebuild();
    s.install_rebuild(WorkspaceSnapshot {
        schema_version: 2,
        records: vec![],
        cursor: 0,
    })
    .unwrap();
    let after_rebuild = s.clone();
    assert_eq!(
        s.stage_product_decision(target.clone(), edit.clone(), 7, &digest)
            .unwrap(),
        (generation, false)
    );
    assert_eq!(s, after_rebuild);
    let ticket = s.prepare_push(&target).unwrap();
    s.acknowledge(&ticket, record("acme.plugin", 1, true))
        .unwrap();
    assert!(s.pending_keys().is_empty());
    let after_delivery = s.clone();
    assert_eq!(
        s.stage_product_decision(target.clone(), edit.clone(), 7, &digest)
            .unwrap(),
        (generation, false)
    );
    assert_eq!(s, after_delivery);
    let (_, staged) = s
        .stage_product_decision(target, edit, 8, &"b".repeat(64))
        .unwrap();
    assert!(staged);
}

#[test]
fn product_recovery_proof_tracks_only_the_latest_uncertain_decision() {
    let mut s = state();
    s.mark_bootstrap_complete();
    let target = key("acme.plugin");
    let first = WorkspaceEdit::Put(selection("acme.plugin", true));
    let later = WorkspaceEdit::Put(selection("acme.plugin", false));
    s.stage_product_decision(target.clone(), first, 7, &"a".repeat(64))
        .unwrap();
    assert!(s.product_recovery_proof(&target).is_none());
    s.prepare_push(&target).unwrap();
    let original = s.product_recovery_proof(&target).unwrap();
    assert_eq!(original.source_generation(), 7);
    s.stage_product_decision(target.clone(), later.clone(), 8, &"b".repeat(64))
        .unwrap();
    assert!(s.verify_product_recovery(&original).is_err());
    let latest = s.product_recovery_proof(&target).unwrap();
    assert_eq!(latest.source_generation(), 8);
    assert_eq!(latest.edit(), &later);
    let reloaded: WorkspaceState =
        serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    assert_eq!(reloaded.product_recovery_proof(&target), Some(latest));
    // A generic newer local edit is not covered by an older product receipt.
    s.stage(
        target.clone(),
        WorkspaceEdit::Put(selection("acme.plugin", true)),
    )
    .unwrap();
    assert!(s.product_recovery_proof(&target).is_none());
}
