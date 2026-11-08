use super::super::DatabaseManager;
use super::*;
use crate::{control::WorkflowMutation, paths::AppPaths};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Barrier},
};

struct Fixture {
    db: DatabaseManager,
    root: PathBuf,
}
fn paths(root: PathBuf) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    }
}
impl Fixture {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("tt-command-{}-{nonce:x}", std::process::id()));
        let paths = paths(root.clone());
        paths.ensure_directories().unwrap();
        Self {
            db: DatabaseManager::new(paths),
            root,
        }
    }
    fn counts(&self) -> (i64, i64, i64) {
        let db = self.db.open(&self.db.automation_path()).unwrap();
        (
            db.query_row("SELECT count(*) FROM control_commands", [], |row| {
                row.get(0)
            })
            .unwrap(),
            db.query_row("SELECT count(*) FROM control_command_outbox", [], |row| {
                row.get(0)
            })
            .unwrap(),
            db.query_row("SELECT count(*) FROM control_resource_outbox", [], |row| {
                row.get(0)
            })
            .unwrap(),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn command(id: &str, expected: &str) -> CommandTransaction {
    CommandTransaction {
        authorization: None,
        principal_id: "moderator".into(),
        session_id: "device-a".into(),
        request: CommandRequest {
            command_id: id.into(),
            host_instance_id: "instance-a".into(),
            expected_revision: expected.into(),
        },
        current_instance_id: "instance-a".into(),
        max_ids_per_principal: 100_000,
    }
}
fn save(name: &str) -> WorkflowMutation {
    WorkflowMutation::Save(
        json!({"id":"welcome","name":name,"enabled":false,"nodes":[],"edges":[]}),
    )
}

#[test]
fn duplicate_returns_the_original_outcome_after_another_writer_changes_the_resource() {
    let f = Fixture::new();
    let request = command("save-once", "0");
    let (original, fresh) =
        f.db.execute_workflow_command(&save("Original"), &request)
            .unwrap();
    assert!(fresh);
    assert_eq!(
        original.command.as_ref().unwrap().state,
        CommandState::Succeeded
    );
    f.db.set_workflow_enabled("welcome", true, None).unwrap();
    let (replayed, fresh) =
        f.db.execute_workflow_command(&save("Original"), &request)
            .unwrap();
    assert!(!fresh);
    assert_eq!(replayed, original);
    assert_eq!(
        f.db.load_workflow("welcome").unwrap().value.unwrap()["enabled"],
        true
    );
    assert_eq!(f.counts(), (1, 1, 2));
    let outcome =
        f.db.command_outcome("moderator", "save-once")
            .unwrap()
            .unwrap();
    assert_eq!(outcome.result, original.value);
    assert_eq!(outcome.resource_versions[0].revision, "1");
    assert!(f
        .db
        .command_outcome("someone-else", "save-once")
        .unwrap()
        .is_none());
}

#[test]
fn canonical_payloads_deduplicate_but_changes_to_method_params_or_preconditions_do_not() {
    let f = Fixture::new();
    let one: Value = serde_json::from_str(
        r#"{"id":"welcome","name":"Canonical","extra":{"a":1,"b":[{"x":2,"y":3}]}}"#,
    )
    .unwrap();
    let reordered: Value = serde_json::from_str(
        r#"{"extra":{"b":[{"y":3,"x":2}],"a":1},"name":"Canonical","id":"welcome"}"#,
    )
    .unwrap();
    let original =
        f.db.execute_workflow_command(&WorkflowMutation::Save(one), &command("canonical", "0"))
            .unwrap()
            .0;
    assert_eq!(
        f.db.execute_workflow_command(
            &WorkflowMutation::Save(reordered),
            &command("canonical", "0")
        )
        .unwrap(),
        (original, false)
    );
    assert!(matches!(
        f.db.execute_workflow_command(&save("Other"), &command("canonical", "0")),
        Err(DatabaseError::CommandIdReused)
    ));
    assert!(matches!(
        f.db.execute_workflow_command(
            &WorkflowMutation::Delete("welcome".into()),
            &command("canonical", "0")
        ),
        Err(DatabaseError::CommandIdReused)
    ));
    assert!(matches!(
        f.db.execute_workflow_command(&save("Other"), &command("canonical", "1")),
        Err(DatabaseError::CommandIdReused)
    ));
    assert_eq!(f.counts(), (1, 1, 1));
}

#[test]
fn concurrent_duplicates_commit_business_state_and_each_outbox_only_once() {
    let f = Fixture::new();
    let gate = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let db = f.db.clone();
            let gate = gate.clone();
            std::thread::spawn(move || {
                gate.wait();
                db.execute_workflow_command(&save("Concurrent"), &command("concurrent", "0"))
            })
        })
        .collect();
    let mut replies: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap().unwrap())
        .collect();
    assert_eq!(replies.iter().filter(|(_, fresh)| *fresh).count(), 1);
    assert_eq!(replies.pop().unwrap().0, replies.pop().unwrap().0);
    assert_eq!(f.counts(), (1, 1, 1));
}

#[test]
fn retained_duplicates_survive_restart_but_new_stale_instance_commands_do_not() {
    let f = Fixture::new();
    let original =
        f.db.execute_workflow_command(&save("Restart"), &command("committed", "0"))
            .unwrap()
            .0;
    let reopened = DatabaseManager::new(paths(f.root.clone()));
    let mut request = command("committed", "0");
    request.current_instance_id = "instance-b".into();
    let (duplicate, fresh) = reopened
        .execute_workflow_command(&save("Restart"), &request)
        .unwrap();
    assert!(!fresh);
    assert_eq!(duplicate.value, original.value);
    assert_eq!(duplicate.command, original.command);
    assert_eq!(duplicate.host_instance_id.as_deref(), Some("instance-b"));
    request.request.command_id = "new-id".into();
    request.request.expected_revision = "1".into();
    assert!(matches!(
        reopened.execute_workflow_command(&save("Must not execute"), &request),
        Err(DatabaseError::HostRestarted)
    ));
    assert_eq!(f.counts(), (1, 1, 1));
}

#[test]
fn command_and_required_audit_reservation_fail_atomically_with_the_graph() {
    let f = Fixture::new();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_command_audit BEFORE INSERT ON control_command_outbox BEGIN SELECT RAISE(ABORT,'test'); END").unwrap();
    assert!(f
        .db
        .execute_workflow_command(&save("Secret script body"), &command("atomic", "0"))
        .is_err());
    assert!(f.db.load_workflow("welcome").unwrap().value.is_none());
    assert_eq!(f.counts(), (0, 0, 0));
    db.execute_batch("DROP TRIGGER fail_command_audit").unwrap();
    f.db.execute_workflow_command(&save("Secret script body"), &command("atomic", "0"))
        .unwrap();
    let audit: String = db.query_row("SELECT event_id||principal_id||session_id||command_id||method||resource_kind||resource_scope||resource_id||before_revision||after_revision||outcome_code FROM control_command_outbox",[],|row|row.get(0)).unwrap();
    assert!(!audit.contains("Secret script body"));
    assert_eq!(f.counts(), (1, 1, 1));
}

#[test]
fn expired_outcomes_keep_deduplication_tombstones_and_capacity_never_evicts_them() {
    let f = Fixture::new();
    let mut request = command("retained-id", "0");
    request.max_ids_per_principal = 1;
    f.db.execute_workflow_command(&save("Retention"), &request)
        .unwrap();
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute(
        "UPDATE control_commands SET committed_at=?1",
        [now() - OUTCOME_TTL - 1000],
    )
    .unwrap();
    f.db.prune_command_outcomes().unwrap();
    let (hash, result, versions, expired): (String, Option<String>, Option<String>, bool) = db
        .query_row(
            "SELECT payload_hash,result_json,versions_json,outcome_expired FROM control_commands",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(hash.len(), 64);
    assert!(result.is_none() && versions.is_none() && expired);
    assert!(matches!(
        f.db.command_outcome("moderator", "retained-id"),
        Err(DatabaseError::CommandExpired)
    ));
    assert!(matches!(
        f.db.execute_workflow_command(&save("Retention"), &request),
        Err(DatabaseError::CommandExpired)
    ));
    assert!(matches!(
        f.db.execute_workflow_command(&save("Changed"), &request),
        Err(DatabaseError::CommandIdReused)
    ));
    request.request.command_id = "another-id".into();
    request.request.expected_revision = "1".into();
    assert!(matches!(
        f.db.execute_workflow_command(&save("No capacity"), &request),
        Err(DatabaseError::CommandCapacity)
    ));
    assert_eq!(
        f.db.load_workflow("welcome").unwrap().value.unwrap()["name"],
        "Retention"
    );
    assert_eq!(f.counts(), (1, 1, 1));
}

#[test]
fn no_op_delete_is_a_queryable_command_and_pre_reservation_failures_are_not() {
    let f = Fixture::new();
    let (result, fresh) =
        f.db.execute_workflow_command(
            &WorkflowMutation::Delete("welcome".into()),
            &command("missing-delete", "0"),
        )
        .unwrap();
    assert!(fresh);
    assert_eq!(result.value, json!({"id":"welcome","deleted":false}));
    assert_eq!(result.resource_versions[0].revision, "0");
    assert_eq!(f.counts(), (1, 1, 0));
    assert!(matches!(
        f.db.execute_workflow_command(&save("Stale"), &command("stale", "1")),
        Err(DatabaseError::RevisionConflict(_))
    ));
    assert!(f
        .db
        .command_outcome("moderator", "stale")
        .unwrap()
        .is_none());
    let mut malformed = command("invalid", "01");
    assert!(matches!(
        f.db.execute_workflow_command(&save("Invalid"), &malformed),
        Err(DatabaseError::Invalid(_))
    ));
    malformed.request.expected_revision = "0".into();
    malformed.request.command_id = "contains a space".into();
    assert!(matches!(
        f.db.execute_workflow_command(&save("Invalid"), &malformed),
        Err(DatabaseError::Invalid(_))
    ));
    assert_eq!(f.counts(), (1, 1, 0));
}

#[test]
#[ignore = "subprocess-only crash fixture"]
fn crash_child() {
    let root = PathBuf::from(std::env::var_os("TIKTOOLS_COMMAND_CRASH_ROOT").expect("child root"));
    let db = DatabaseManager::new(paths(root));
    db.execute_workflow_command(&save("Crash checkpoint"), &command("crash-command", "0"))
        .unwrap();
    panic!("crash failpoint did not terminate");
}

#[test]
fn process_death_before_commit_rolls_back_and_after_commit_replays_without_execution() {
    for (stage, code) in [("before_commit", 86), ("after_commit", 87)] {
        let f = Fixture::new();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "db::sqlite::commands::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("TIKTOOLS_COMMAND_CRASH_ROOT", &f.root)
            .env("TIKTOOLS_COMMAND_CRASH_STAGE", stage)
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        let reopened = DatabaseManager::new(paths(f.root.clone()));
        if stage == "before_commit" {
            assert!(reopened.load_workflow("welcome").unwrap().value.is_none());
            assert!(reopened
                .command_outcome("moderator", "crash-command")
                .unwrap()
                .is_none());
            assert_eq!(f.counts(), (0, 0, 0));
        } else {
            assert_eq!(f.counts(), (1, 1, 1));
            let mut retry = command("crash-command", "0");
            retry.current_instance_id = "instance-b".into();
            let (reply, fresh) = reopened
                .execute_workflow_command(&save("Crash checkpoint"), &retry)
                .unwrap();
            assert!(!fresh);
            assert_eq!(reply.resource_versions[0].revision, "1");
            assert_eq!(f.counts(), (1, 1, 1));
        }
    }
}

#[tokio::test]
async fn core_queries_use_live_identity_grants_and_resource_scope() {
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }
    let f = Fixture::new();
    let core = Arc::new(crate::AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(crate::NoopMediaHost),
        paths(f.root.clone()),
    ));
    let service = core.collaboration().unwrap();
    use crate::collaboration::{
        Capability, ClientKind, Collect, CommandGet, Configuration, Grant, GrantSet, InviteCreate,
        Redeem, ResourceKind,
    };
    service
        .configure(Configuration {
            enabled: true,
            advertised_origin: Some("http://127.0.0.1:8788".into()),
            ..Configuration::default()
        })
        .unwrap();
    let grant = Grant {
        capabilities: [Capability::CommandsRead, Capability::WorkflowsRead].into(),
        resources: [(ResourceKind::Workflow, ["welcome".into()].into())].into(),
        ..Grant::default()
    };
    let invite = service
        .create_invitation(InviteCreate {
            grant: grant.clone(),
        })
        .unwrap();
    let exchange = "b".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Read only".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "test-peer",
        )
        .unwrap();
    service.decide_request(&pending.request_id, true).unwrap();
    let credentials = service
        .collect(
            Collect {
                request_id: pending.request_id,
                exchange_secret: exchange,
            },
            "test-peer",
        )
        .unwrap();
    let authorization = service
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap();
    // A historical command fixture, inserted by trusted store code. The public
    // core still rejects creating remote workflows until effect policy exists.
    let mut tx = command("historical", "0");
    tx.principal_id = credentials.principal_id.clone();
    tx.session_id = credentials.session_id.clone();
    f.db.execute_workflow_command(&save("Private result"), &tx)
        .unwrap();
    let caller = crate::control::CommandCaller::Remote(authorization.clone());
    let query = CommandGet {
        command_id: "historical".into(),
        principal_id: None,
    };
    assert_eq!(
        core.collaboration_command_get(caller.clone(), query.clone())
            .unwrap()
            .result["name"],
        "Private result"
    );
    assert_eq!(
        core.collaboration_command_get(
            caller.clone(),
            CommandGet {
                command_id: "historical".into(),
                principal_id: Some("local-owner".into())
            }
        )
        .unwrap_err()
        .code,
        "forbidden"
    );
    assert_eq!(
        core.workflow_command(
            caller.clone(),
            CommandRequest {
                command_id: "must-not-write".into(),
                host_instance_id: service.instance_id().into(),
                expected_revision: "1".into()
            },
            save("Must not execute")
        )
        .unwrap_err()
        .code,
        "unsupported_remote"
    );
    f.db.open(&f.db.automation_path())
        .unwrap()
        .execute(
            "UPDATE control_commands SET committed_at=?1",
            [now() - OUTCOME_TTL - 1000],
        )
        .unwrap();
    let mut restricted = grant.clone();
    restricted
        .resources
        .insert(ResourceKind::Workflow, ["other-workflow".into()].into());
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "1".into(),
            grant: restricted,
        })
        .unwrap();
    assert_eq!(
        core.collaboration_command_get(caller.clone(), query.clone())
            .unwrap_err()
            .code,
        "forbidden"
    );
    let mut removed = grant.clone();
    removed.capabilities.remove(&Capability::WorkflowsRead);
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "2".into(),
            grant: removed,
        })
        .unwrap();
    assert_eq!(
        core.collaboration_command_get(caller.clone(), query.clone())
            .unwrap_err()
            .code,
        "forbidden"
    );
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "3".into(),
            grant,
        })
        .unwrap();
    let expired = core
        .collaboration_command_get(caller.clone(), query.clone())
        .unwrap_err();
    assert_eq!(expired.code, "command_expired");
    assert_eq!(
        expired.data.as_ref().unwrap()["command"]["resource"]["id"],
        "welcome"
    );
    assert_eq!(expired.data.unwrap()["state"], "unknown");
    service
        .revoke_session(&credentials.session_id, "local-owner")
        .unwrap();
    assert_eq!(
        core.collaboration_command_get(caller, query)
            .unwrap_err()
            .code,
        "unauthorized"
    );
    assert_eq!(f.counts(), (1, 1, 1));
    core.shutdown().await;
    drop(service);
    drop(core);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_drains_command_coordination_and_rejects_new_work_without_opening_identity_storage(
) {
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }
    let f = Fixture::new();
    let core = Arc::new(crate::AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(crate::NoopMediaHost),
        paths(f.root.clone()),
    ));
    core.start_coordination_worker();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let pending = core.clone();
    let gate_holder = std::thread::spawn(move || {
        let _gate = pending.coordination_gate.lock().unwrap();
        entered_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    });
    entered_rx.await.unwrap();
    let stopped = core.clone();
    let mut shutdown = tokio::spawn(async move {
        stopped.shutdown().await;
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !core.is_shutdown() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(25), &mut shutdown)
            .await
            .is_err()
    );
    let late = core.workflow_command(
        crate::control::CommandCaller::LocalOwner,
        CommandRequest {
            command_id: "late".into(),
            host_instance_id: "stopped".into(),
            expected_revision: "0".into(),
        },
        save("Must not commit"),
    );
    assert_eq!(late.unwrap_err().code, "capability_unavailable");
    assert!(!f.root.join("data/collaboration/collaboration.db").exists());
    release_tx.send(()).unwrap();
    gate_holder.join().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), shutdown)
        .await
        .unwrap()
        .unwrap();
    assert!(core.update_shutdown_safe());
    assert_eq!(f.counts(), (0, 0, 0));
    drop(core);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_remote_deactivation_rechecks_the_live_grant_and_session_at_the_writer_gate() {
    use crate::collaboration::{
        Capability, ClientKind, Collect, Configuration, Grant, GrantSet, InviteCreate, Redeem,
        ResourceKind,
    };
    struct Emitter;
    impl crate::HostEmitter for Emitter {
        fn emit(&self, _: crate::ipc::messages::HostMessage) {}
    }
    let f = Fixture::new();
    let core = Arc::new(crate::AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(crate::NoopMediaHost),
        paths(f.root.clone()),
    ));
    let service = core.collaboration().unwrap();
    service
        .configure(Configuration {
            enabled: true,
            advertised_origin: Some("http://127.0.0.1:8788".into()),
            ..Configuration::default()
        })
        .unwrap();
    core.workflow_save(json!({"id":"welcome","name":"Owner definition","enabled":true}))
        .unwrap();
    let grant = Grant {
        capabilities: [
            Capability::WorkflowsRead,
            Capability::WorkflowsToggle,
            Capability::WorkflowsEdit,
        ]
        .into(),
        resources: [(ResourceKind::Workflow, ["welcome".into()].into())].into(),
        ..Grant::default()
    };
    let invite = service
        .create_invitation(InviteCreate {
            grant: grant.clone(),
        })
        .unwrap();
    let exchange = "c".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Queued moderator".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "test-peer",
        )
        .unwrap();
    service.decide_request(&pending.request_id, true).unwrap();
    let credentials = service
        .collect(
            Collect {
                request_id: pending.request_id,
                exchange_secret: exchange,
            },
            "test-peer",
        )
        .unwrap();
    let authorization = service
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap();
    let mut grant_revision = 1;
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_remote_audit BEFORE INSERT ON control_command_outbox BEGIN SELECT RAISE(ABORT,'private-trigger-diagnostic'); END").unwrap();
    for delete in [false, true] {
        let operation = if delete {
            WorkflowMutation::Delete("welcome".into())
        } else {
            WorkflowMutation::SetEnabled {
                id: "welcome".into(),
                enabled: false,
            }
        };
        let failure = core
            .workflow_command(
                crate::control::CommandCaller::Remote(authorization.clone()),
                CommandRequest {
                    command_id: format!("atomic-remote-{delete}"),
                    host_instance_id: service.instance_id().into(),
                    expected_revision: "1".into(),
                },
                operation,
            )
            .unwrap_err();
        assert_eq!(failure.code, "internal");
        assert_eq!(core.workflow_get("welcome").unwrap()["enabled"], true);
        assert_eq!(
            core.workflow_get_versioned("welcome")
                .unwrap()
                .resource_versions[0]
                .revision,
            "1"
        );
        assert_eq!(f.counts(), (0, 0, 1));
    }
    db.execute_batch("DROP TRIGGER fail_remote_audit").unwrap();
    drop(db);
    for (sqlite, revoked) in [(false, false), (true, false), (true, true)] {
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let held = core.clone();
        let database = f.db.clone();
        let holder = std::thread::spawn(move || {
            if sqlite {
                let mut connection = database.open(&database.automation_path()).unwrap();
                let tx = connection
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                    .unwrap();
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                tx.commit().unwrap();
            } else {
                let _gate = held.coordination_gate.lock().unwrap();
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            }
        });
        entered_rx.await.unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let pending = core.clone();
        let caller = crate::control::CommandCaller::Remote(authorization.clone());
        let request = CommandRequest {
            command_id: format!("queued-{sqlite}-{revoked}"),
            host_instance_id: service.instance_id().into(),
            expected_revision: "1".into(),
        };
        let mut task = tokio::task::spawn_blocking(move || {
            started_tx.send(()).unwrap();
            pending.workflow_command(
                caller,
                request,
                WorkflowMutation::SetEnabled {
                    id: "welcome".into(),
                    enabled: false,
                },
            )
        });
        started_rx.await.unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(25), &mut task)
                .await
                .is_err()
        );
        if revoked {
            service
                .revoke_session(&credentials.session_id, "local-owner")
                .unwrap();
        } else {
            service
                .set_grant(GrantSet {
                    principal_id: credentials.principal_id.clone(),
                    expected_revision: grant_revision.to_string(),
                    grant: Grant {
                        resources: [(ResourceKind::Workflow, [].into())].into(),
                        ..grant.clone()
                    },
                })
                .unwrap();
            grant_revision += 1;
        }
        release_tx.send(()).unwrap();
        holder.join().unwrap();
        let failure = task.await.unwrap().unwrap_err();
        assert_eq!(
            failure.code,
            if revoked { "unauthorized" } else { "forbidden" }
        );
        assert_eq!(core.workflow_get("welcome").unwrap()["enabled"], true);
        assert_eq!(f.counts().0, 0);
        if !revoked {
            service
                .set_grant(GrantSet {
                    principal_id: credentials.principal_id.clone(),
                    expected_revision: grant_revision.to_string(),
                    grant: grant.clone(),
                })
                .unwrap();
            grant_revision += 1;
        }
    }
    core.shutdown().await;
    drop(service);
    drop(core);
}
