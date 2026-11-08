use super::*;

fn profile() -> RemoteProfile {
    let mut p = RemoteProfile {
        version: 1,
        name: "desk".into(),
        profile_id: Uuid::new_v4().to_string(),
        origin: "https://127.0.0.1:8788".into(),
        host_id: Uuid::new_v4().to_string(),
        principal_id: Uuid::new_v4().to_string(),
        session_id: Uuid::new_v4().to_string(),
        client_kind: ClientKind::Agent,
        trust: TlsTrust::System,
        credential_key: String::new(),
        refresh_attempt: None,
    };
    p.credential_key = credential_key(&p);
    p
}
fn intent() -> Intent {
    Intent {
        command_id: Uuid::new_v4().to_string(),
        operation: Operation::Disable,
        resource_id: "welcome".into(),
        host_instance_id: Uuid::new_v4().to_string(),
        expected_revision: Some("1".into()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        created_at: timestamp(),
    }
}
fn fixture() -> (tempfile::TempDir, PathBuf, RemoteProfile) {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("profiles");
    directory::check_directory(&root, true).unwrap();
    (t, root, profile())
}
fn record(intent: Intent) -> Record {
    Record {
        intent,
        outcome: Outcome::Succeeded,
        revision: Some("2".into()),
        error_code: None,
        recorded_at: timestamp(),
        points_result: None,
    }
}

fn points_intent() -> Intent {
    Intent {
        operation: Operation::AdjustPoints,
        resource_id: "alice".into(),
        expected_revision: None,
        delta: Some(-3.0),
        currency_name: Some("Gems".into()),
        ..intent()
    }
}
fn event_intent(operation: Operation) -> Intent {
    Intent {
        operation,
        ..intent()
    }
}

#[test]
fn points_intents_have_no_cas_and_keep_verified_absolute_result_metadata() {
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    for invalid in [
        Intent {
            expected_revision: Some("0".into()),
            ..points_intent()
        },
        Intent {
            delta: Some(f64::NAN),
            ..points_intent()
        },
        Intent {
            currency_name: None,
            ..points_intent()
        },
        Intent {
            resource_id: "@alice".into(),
            ..points_intent()
        },
    ] {
        assert_eq!(
            j.begin(invalid).unwrap_err().code,
            "command_journal_invalid"
        );
        assert!(j.view().pending.is_none());
    }
    let original = points_intent();
    j.begin(original.clone()).unwrap();
    let mut result = record(original.clone());
    result.revision = Some("45".into());
    assert!(j.finish(result.clone()).is_err());
    result.points_result = Some(PointsResult {
        delta: 3.0,
        total_points: 4.0,
        level: 1,
    });
    assert!(j.finish(result.clone()).is_err());
    result.points_result = Some(PointsResult {
        delta: -3.0,
        total_points: 0.0,
        level: 1,
    });
    j.finish(result).unwrap();
    drop(j);
    let j = Journal::open(&root, &p).unwrap();
    assert_eq!(j.view().recent[0].intent, original);
    assert_eq!(
        j.view().recent[0]
            .points_result
            .as_ref()
            .unwrap()
            .total_points,
        0.0
    );
}

#[test]
#[ignore = "private subprocess invoked by the migration parent"]
fn journal_migration_process_fixture() {
    let root = PathBuf::from(std::env::var_os("TIKTOOLS_JOURNAL_FIXTURE_DIR").unwrap());
    assert!(root.starts_with(std::env::temp_dir()));
    let p: RemoteProfile =
        serde_json::from_slice(&fs::read(root.join("fixture-profile.input")).unwrap()).unwrap();
    let _j = Journal::open(&root, &p).unwrap();
    panic!("migration crash point was not reached");
}

#[test]
fn older_formats_migrate_to_v4_preserving_pending_ids_and_mixed_history_at_every_crash_boundary() {
    for (version, pending_points) in [(1, false), (2, false), (2, true), (3, false), (3, true)] {
        for stage in ["before", "renamed", "after"] {
            let (_t, root, p) = fixture();
            let j = Journal::open(&root, &p).unwrap();
            let completed = intent();
            j.begin(completed.clone()).unwrap();
            j.finish(record(completed)).unwrap();
            if version >= 2 {
                let completed = points_intent();
                j.begin(completed.clone()).unwrap();
                let mut r = record(completed);
                r.points_result = Some(PointsResult {
                    delta: -3.0,
                    total_points: 0.0,
                    level: 1,
                });
                j.finish(r).unwrap();
            }
            if version == 3 {
                let completed = event_intent(Operation::DeleteEvent);
                j.begin(completed.clone()).unwrap();
                j.finish(record(completed)).unwrap();
            }
            let pending = if version == 3 && !pending_points {
                event_intent(Operation::DisableEvent)
            } else if pending_points {
                points_intent()
            } else {
                intent()
            };
            j.begin(pending.clone()).unwrap();
            let history = serde_json::to_value(j.view().recent).unwrap();
            let path = j.path.clone();
            drop(j);
            let mut data: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            data["version"] = version.into();
            let old_bytes = serde_json::to_vec(&data).unwrap();
            fs::write(&path, &old_bytes).unwrap();
            fs::write(
                root.join("fixture-profile.input"),
                serde_json::to_vec(&p).unwrap(),
            )
            .unwrap();
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "remote::profiles::commands::tests::journal_migration_process_fixture",
                ])
                .env("TIKTOOLS_JOURNAL_FIXTURE_DIR", &root)
                .env("TIKTOOLS_JOURNAL_MIGRATION_CRASH", stage)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(87), "v{version}/{stage}");
            let file: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            assert_eq!(
                file["version"],
                if stage == "before" { version } else { VERSION }
            );
            if stage == "before" {
                assert_eq!(fs::read(&path).unwrap(), old_bytes);
            }
            let j = Journal::open(&root, &p).unwrap();
            assert_eq!(j.view().pending, Some(pending));
            assert_eq!(serde_json::to_value(j.view().recent).unwrap(), history);
            let migrated: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            assert_eq!(migrated["version"], VERSION);
            assert_eq!(
                j.begin(event_intent(Operation::DisableEvent))
                    .unwrap_err()
                    .code,
                "command_pending"
            );
        }
    }
}

#[test]
fn proposals_require_a_private_copy_revision_and_cannot_be_smuggled_into_older_formats() {
    let operations = [
        Operation::CreateEventDraft,
        Operation::UpdateEventDraft,
        Operation::CreateActionDraft,
        Operation::UpdateActionDraft,
    ];
    for operation in operations {
        let original = Intent {
            operation,
            proposal_revision: Some("7".into()),
            ..intent()
        };
        assert!(original.validate().is_ok());
        assert!(Intent {
            proposal_revision: None,
            ..original.clone()
        }
        .validate()
        .is_err());
        for pending in [false, true] {
            for version in [1, 2, 3] {
                let (_t, root, p) = fixture();
                let j = Journal::open(&root, &p).unwrap();
                j.begin(original.clone()).unwrap();
                if !pending {
                    j.finish(record(original.clone())).unwrap();
                }
                let path = j.path.clone();
                drop(j);
                let mut data: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                data["version"] = version.into();
                let bytes = serde_json::to_vec(&data).unwrap();
                fs::write(&path, &bytes).unwrap();
                assert_eq!(
                    Journal::open(&root, &p).err().unwrap().code,
                    "command_journal_invalid"
                );
                assert_eq!(fs::read(&path).unwrap(), bytes);
            }
        }
    }
    for version in [1, 2, 3] {
        for pending in [false, true] {
            let (_t, root, p) = fixture();
            let j = Journal::open(&root, &p).unwrap();
            let i = intent();
            j.begin(i.clone()).unwrap();
            if !pending {
                j.finish(record(i)).unwrap();
            }
            let path = j.path.clone();
            drop(j);
            let mut data: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            data["version"] = version.into();
            if pending {
                data["pending"]["proposalRevision"] = serde_json::Value::Null;
            } else {
                data["recent"][0]["intent"]["proposalRevision"] = serde_json::Value::Null;
            }
            fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
            assert!(Journal::open(&root, &p).is_err());
        }
    }
    assert!(Intent {
        proposal_revision: Some("7".into()),
        ..intent()
    }
    .validate()
    .is_err());
}

#[test]
fn v1_cannot_smuggle_points_or_invalid_pending_state_through_migration() {
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    j.begin(points_intent()).unwrap();
    let path = j.path.clone();
    drop(j);
    let mut data: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    data["version"] = 1.into();
    let bytes = serde_json::to_vec(&data).unwrap();
    fs::write(&path, &bytes).unwrap();
    assert_eq!(
        Journal::open(&root, &p).err().unwrap().code,
        "command_journal_invalid"
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn durable_intent_is_exclusive_bound_and_never_replaced_until_resolved() {
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    let original = intent();
    j.begin(original.clone()).unwrap();
    assert_eq!(
        Journal::open(&root, &p).err().unwrap().code,
        "command_journal_in_use"
    );
    assert_eq!(j.begin(intent()).unwrap_err().code, "command_pending");
    assert_eq!(j.remove_resolved().unwrap_err().code, "command_pending");
    let bytes = fs::read(&j.path).unwrap();
    assert!(!String::from_utf8(bytes).unwrap().contains("accessToken"));
    drop(j);
    let j = Journal::open(&root, &p).unwrap();
    assert_eq!(j.view().pending, Some(original.clone()));
    assert_eq!(j.finish(record(intent())).unwrap_err().code, "stale_view");
    j.finish(record(original.clone())).unwrap();
    drop(j);
    let j = Journal::open(&root, &p).unwrap();
    assert!(j.view().pending.is_none());
    assert_eq!(j.view().recent[0].intent, original);
    assert_eq!(
        j.begin(original).unwrap_err().code,
        "command_journal_invalid"
    );
}
#[test]
fn another_origin_host_principal_session_or_profile_cannot_load_the_intent() {
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    j.begin(intent()).unwrap();
    let path = j.path.clone();
    drop(j);
    for field in ["origin", "host", "principal", "session", "profile"] {
        let mut q = p.clone();
        match field {
            "origin" => q.origin = "https://127.0.0.1:9999".into(),
            "host" => q.host_id = Uuid::new_v4().to_string(),
            "principal" => q.principal_id = Uuid::new_v4().to_string(),
            "session" => q.session_id = Uuid::new_v4().to_string(),
            _ => q.profile_id = Uuid::new_v4().to_string(),
        }
        q.credential_key = credential_key(&q);
        if field == "profile" {
            fs::copy(
                &path,
                root.join("commands")
                    .join(format!("{}.journal", q.profile_id)),
            )
            .unwrap();
        }
        assert_eq!(
            Journal::open(&root, &q).err().unwrap().code,
            "command_journal_invalid"
        );
    }
    assert!(Journal::open(&root, &p).unwrap().view().pending.is_some());
}
#[test]
fn invalid_oversized_or_unstructured_journal_data_cannot_be_loaded_or_persisted() {
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    let original = intent();
    j.begin(original.clone()).unwrap();
    let mut bad = record(original.clone());
    bad.error_code = Some("secret-marker".into());
    assert_eq!(j.finish(bad).unwrap_err().code, "command_journal_invalid");
    assert_eq!(j.view().pending, Some(original));
    let path = j.path.clone();
    let bytes = fs::read(&path).unwrap();
    drop(j);
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["rawGraph"] = serde_json::json!({"source":"private"});
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(
        Journal::open(&root, &p).err().unwrap().code,
        "command_journal_invalid"
    );
    fs::write(&path, vec![b'x'; BYTES as usize + 1]).unwrap();
    assert_eq!(
        Journal::open(&root, &p).err().unwrap().code,
        "command_journal_invalid"
    );
}
#[test]
fn persistence_failure_blocks_dispatch_and_later_mutations() {
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    let original = intent();
    fs::create_dir(&j.path).unwrap();
    assert!(j.begin(original.clone()).is_err());
    assert!(j.view().unavailable);
    assert_eq!(j.view().pending, Some(original));
    assert_eq!(
        j.begin(intent()).unwrap_err().code,
        "command_journal_unavailable"
    );
}
#[test]
fn failed_result_persistence_keeps_the_original_pending_id_for_recovery() {
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    let original = intent();
    j.begin(original.clone()).unwrap();
    let backup = j.path.with_extension("backup");
    fs::rename(&j.path, &backup).unwrap();
    fs::create_dir(&j.path).unwrap();
    assert!(j.finish(record(original.clone())).is_err());
    assert_eq!(j.view().pending, Some(original.clone()));
    assert!(j.view().unavailable);
    fs::remove_dir(&j.path).unwrap();
    fs::rename(&backup, &j.path).unwrap();
    drop(j);
    assert_eq!(
        Journal::open(&root, &p).unwrap().view().pending,
        Some(original)
    );
}
#[test]
fn confirmed_noop_deletes_can_keep_the_maximum_revision_without_overflow() {
    for operation in [Operation::Delete, Operation::DeleteEvent] {
        let (_t, root, p) = fixture();
        let j = Journal::open(&root, &p).unwrap();
        let mut i = intent();
        i.operation = operation;
        i.expected_revision = Some(i64::MAX.to_string());
        j.begin(i.clone()).unwrap();
        let mut r = record(i);
        r.revision = Some(i64::MAX.to_string());
        j.finish(r).unwrap();
        assert!(j.view().pending.is_none());
    }
}

#[test]
fn pending_is_never_pruned_and_resolved_metadata_history_has_a_fixed_bound() {
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    for _ in 0..(HISTORY + 4) {
        let i = intent();
        j.begin(i.clone()).unwrap();
        j.finish(record(i)).unwrap();
    }
    let unresolved = intent();
    j.begin(unresolved.clone()).unwrap();
    assert_eq!(j.view().recent.len(), HISTORY);
    drop(j);
    let j = Journal::open(&root, &p).unwrap();
    assert_eq!(j.view().pending, Some(unresolved));
    assert_eq!(j.view().recent.len(), HISTORY);
}
#[cfg(unix)]
#[test]
fn journal_rejects_public_permissions_symlinks_and_hard_links() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let (_t, root, p) = fixture();
    let j = Journal::open(&root, &p).unwrap();
    j.begin(intent()).unwrap();
    let path = j.path.clone();
    drop(j);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Journal::open(&root, &p).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let actual = path.with_extension("backup");
    fs::rename(&path, &actual).unwrap();
    symlink(&actual, &path).unwrap();
    assert!(Journal::open(&root, &p).is_err());
    fs::remove_file(&path).unwrap();
    fs::hard_link(&actual, &path).unwrap();
    assert!(Journal::open(&root, &p).is_err());
    fs::remove_file(&path).unwrap();
    fs::rename(&actual, &path).unwrap();
    assert!(Journal::open(&root, &p).unwrap().view().pending.is_some());
}

#[test]
#[ignore = "subprocess fixture, invoked only by the parent crash test"]
fn journal_process_death_fixture() {
    let root = PathBuf::from(
        std::env::var_os("TIKTOOLS_JOURNAL_FIXTURE_DIR")
            .expect("private fixture directory required"),
    );
    assert!(root.starts_with(std::env::temp_dir()));
    let profile: RemoteProfile =
        serde_json::from_slice(&fs::read(root.join("fixture-profile.input")).unwrap()).unwrap();
    let journal = Journal::open(&root, &profile).unwrap();
    let stage = std::env::var("TIKTOOLS_JOURNAL_FIXTURE_STAGE").unwrap();
    if stage == "before_intent" {
        std::process::exit(86);
    }
    let i = match std::env::var("TIKTOOLS_JOURNAL_FIXTURE_OPERATION")
        .unwrap()
        .as_str()
    {
        "adjust-points" => points_intent(),
        "disable-event" => event_intent(Operation::DisableEvent),
        "delete-event" => event_intent(Operation::DeleteEvent),
        "disable" => intent(),
        other => panic!("unknown fixture operation {other}"),
    };
    journal.begin(i.clone()).unwrap();
    if stage == "after_intent" {
        std::process::exit(86);
    }
    assert_eq!(stage, "after_result");
    let mut result = record(i);
    if result.intent.operation == Operation::AdjustPoints {
        result.points_result = Some(PointsResult {
            delta: -3.0,
            total_points: 0.0,
            level: 1,
        });
    }
    journal.finish(result).unwrap();
    std::process::exit(86);
}
#[test]
fn process_death_preserves_only_synced_intents_and_confirmed_outcomes() {
    for operation in ["disable", "adjust-points", "disable-event", "delete-event"] {
        for stage in ["before_intent", "after_intent", "after_result"] {
            let (_t, root, p) = fixture();
            let input = root.join("fixture-profile.input");
            let mut file = options().write(true).create_new(true).open(input).unwrap();
            serde_json::to_writer(&mut file, &p).unwrap();
            file.sync_all().unwrap();
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "remote::profiles::commands::tests::journal_process_death_fixture",
                ])
                .env("TIKTOOLS_JOURNAL_FIXTURE_DIR", &root)
                .env("TIKTOOLS_JOURNAL_FIXTURE_STAGE", stage)
                .env("TIKTOOLS_JOURNAL_FIXTURE_OPERATION", operation)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(86), "{stage}");
            let recovered = Journal::open(&root, &p).unwrap().view();
            assert_eq!(
                recovered.pending.is_some(),
                stage == "after_intent",
                "{operation}/{stage}"
            );
            assert_eq!(
                recovered.recent.len(),
                usize::from(stage == "after_result"),
                "{operation}/{stage}"
            );
            let recovered_intent = recovered
                .pending
                .as_ref()
                .or_else(|| recovered.recent.first().map(|r| &r.intent));
            if let Some(intent) = recovered_intent {
                assert_eq!(serde_json::to_value(intent.operation).unwrap(), operation);
            }
        }
    }
}

#[test]
fn event_operations_have_fixed_scope_and_cannot_become_workflow_or_action_commands() {
    use tiktools_collaboration::{Capability, ResourceKind};
    for (operation, wire, method, capability) in [
        (
            Operation::DisableEvent,
            "disable-event",
            "automation.disable",
            Capability::AutomationToggle,
        ),
        (
            Operation::DeleteEvent,
            "delete-event",
            "automation.delete",
            Capability::AutomationEdit,
        ),
    ] {
        assert_eq!(serde_json::to_value(operation).unwrap(), wire);
        assert_eq!(operation.method(), method);
        assert_eq!(operation.feature(), "automationEventDeactivate");
        assert_eq!(operation.capability(), capability);
        assert_eq!(operation.kind(), ResourceKind::Automation);
        assert_eq!(operation.scope(), "event");
        assert!(operation.is_event());
        let mut value = serde_json::to_value(event_intent(operation)).unwrap();
        value["scope"] = "action".into();
        assert!(
            serde_json::from_value::<Intent>(value).is_err(),
            "scope is derived from the operation, never supplied by the page/file"
        );
    }
    for operation in [
        Operation::Disable,
        Operation::Delete,
        Operation::AdjustPoints,
    ] {
        assert_eq!(operation.scope(), "host");
        assert!(!operation.is_event());
    }
}

#[test]
fn old_formats_reject_event_operations_in_pending_and_resolved_history_without_rewriting() {
    for version in [1, 2] {
        for operation in [Operation::DisableEvent, Operation::DeleteEvent] {
            for completed in [false, true] {
                let (_t, root, p) = fixture();
                let j = Journal::open(&root, &p).unwrap();
                let intent = event_intent(operation);
                j.begin(intent.clone()).unwrap();
                if completed {
                    j.finish(record(intent)).unwrap();
                }
                let path = j.path.clone();
                drop(j);
                let mut value: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                value["version"] = version.into();
                let bytes = serde_json::to_vec(&value).unwrap();
                fs::write(&path, &bytes).unwrap();
                assert_eq!(
                    Journal::open(&root, &p).err().unwrap().code,
                    "command_journal_invalid",
                    "v{version}/{operation:?}/completed={completed}"
                );
                assert_eq!(
                    fs::read(&path).unwrap(),
                    bytes,
                    "invalid old data must never be migrated or dropped"
                );
            }
        }
    }
}

#[test]
fn event_intents_require_cas_without_points_fields_and_share_the_existing_pending_gate() {
    for operation in [Operation::DisableEvent, Operation::DeleteEvent] {
        let (_t, root, p) = fixture();
        let j = Journal::open(&root, &p).unwrap();
        for invalid in [
            Intent {
                expected_revision: None,
                ..event_intent(operation)
            },
            Intent {
                expected_revision: Some("01".into()),
                ..event_intent(operation)
            },
            Intent {
                delta: Some(1.0),
                ..event_intent(operation)
            },
            Intent {
                currency_name: Some("Gems".into()),
                ..event_intent(operation)
            },
            Intent {
                resource_id: " spaced ".into(),
                ..event_intent(operation)
            },
        ] {
            assert_eq!(
                j.begin(invalid).unwrap_err().code,
                "command_journal_invalid"
            );
            assert!(j.view().pending.is_none());
        }
        let original = event_intent(operation);
        j.begin(original.clone()).unwrap();
        for other in [
            intent(),
            points_intent(),
            event_intent(Operation::DisableEvent),
        ] {
            assert_eq!(j.begin(other).unwrap_err().code, "command_pending");
        }
        let mut result = record(original.clone());
        result.points_result = Some(PointsResult {
            delta: 1.0,
            total_points: 1.0,
            level: 1,
        });
        assert_eq!(
            j.finish(result).unwrap_err().code,
            "command_journal_invalid"
        );
        assert_eq!(j.view().pending, Some(original.clone()));
        j.finish(record(original.clone())).unwrap();
        let path = j.path.clone();
        drop(j);
        let data: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(data["version"], VERSION);
        let j = Journal::open(&root, &p).unwrap();
        assert_eq!(j.view().recent[0].intent, original);
        let reused = Intent {
            operation: Operation::Disable,
            ..original
        };
        assert_eq!(
            j.begin(reused).unwrap_err().code,
            "command_journal_invalid",
            "IDs remain shared across resource kinds"
        );
    }
}

#[test]
fn event_records_validate_revision_changes_and_domain_specific_rejections() {
    for operation in [Operation::DisableEvent, Operation::DeleteEvent] {
        let original = event_intent(operation);
        let mut result = record(original);
        result.validate().unwrap();
        result.revision = Some("1".into());
        assert_eq!(result.validate().is_ok(), operation.is_delete());
        result.revision = Some("3".into());
        assert!(result.validate().is_err());
        result.revision = None;
        result.outcome = Outcome::Rejected;
        result.error_code = Some("automation_not_found".into());
        result.validate().unwrap();
        result.intent.operation = Operation::Disable;
        assert!(
            result.validate().is_err(),
            "a behavior rejection must not be relabeled as workflow history"
        );
    }
}

#[test]
fn unsupported_versions_and_caller_supplied_event_scope_do_not_replace_a_valid_journal() {
    for mutation in ["zero-version", "future-version", "scope"] {
        let (_t, root, p) = fixture();
        let j = Journal::open(&root, &p).unwrap();
        j.begin(event_intent(Operation::DisableEvent)).unwrap();
        let path = j.path.clone();
        drop(j);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match mutation {
            "zero-version" => value["version"] = 0.into(),
            "future-version" => value["version"] = (VERSION + 1).into(),
            "scope" => value["pending"]["scope"] = "action".into(),
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert_eq!(
            Journal::open(&root, &p).err().unwrap().code,
            "command_journal_invalid"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes, "{mutation}");
    }
}
