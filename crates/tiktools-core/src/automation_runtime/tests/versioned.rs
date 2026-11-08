use super::*;
use crate::{
    control::AutomationKind,
    paths::AppPaths,
    services::{AwardOptions, PointAction},
};
use std::future::{poll_fn, Future};
use std::task::Poll;

fn fixture() -> (Arc<AppCore>, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "tt-execution-fence-{}-{}",
        std::process::id(),
        fastrand::u64(..)
    ));
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent-builtin"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.clone(),
    };
    paths.ensure_directories().unwrap();
    for path in [
        paths.points_database(),
        paths.automation_database(),
        paths.analytics_database(),
    ] {
        std::fs::File::create(path).unwrap();
    }
    let core = Arc::new(AppCore::with_paths(
        Arc::new(NoopEmitter),
        Arc::new(crate::NoopMediaHost),
        paths,
    ));
    (core, root)
}

#[tokio::test(start_paused = true)]
async fn changed_definitions_and_recreated_ids_stop_unstarted_effects_after_a_real_delay() {
    for change in [
        "action",
        "event",
        "recreate",
        "unrelated",
        #[cfg(feature = "persistence")]
        "remote-disable",
        #[cfg(feature = "persistence")]
        "remote-delete",
    ] {
        let (core, root) = fixture();
        core.points
            .award_points(
                "fenced-viewer",
                PointAction::Manual,
                AwardOptions {
                    custom_amount: Some(10.0),
                    ..Default::default()
                },
            )
            .unwrap();
        core.automation_create(AutomationKind::Action,json!({"id":"delay","name":"Delay","enabled":true,"typeId":"core.delay","config":{"ms":1000}})).unwrap();
        let penalty = json!({"id":"penalty","name":"Penalty","enabled":true,"typeId":"core.points.subtract","config":{"uniqueId":"fenced-viewer","amount":2}});
        core.automation_create(AutomationKind::Action, penalty.clone())
            .unwrap();
        let event = json!({"id":"event","name":"Event","enabled":true,"trigger":"tiktok.chat","actionIds":["delay","penalty"]});
        core.automation_create(AutomationKind::Event, event.clone())
            .unwrap();
        let input = json!({"type":"tiktok.chat","user":{"uniqueId":"fenced-viewer"},"data":{}});
        let records = core.automation.matching_records(&input);
        assert_eq!(records.len(), 1);
        let incoming = crate::automation_runtime::context::ExecutionContext::owner();
        let mut run = std::pin::pin!(core.run_matched_records(records, &input, &incoming));
        poll_fn(|cx| {
            assert!(
                run.as_mut().poll(cx).is_pending(),
                "the delay must begin before the edit"
            );
            Poll::Ready(())
        })
        .await;
        match change {
            "action" => {
                let mut changed = penalty;
                changed["config"]["amount"] = 999.into();
                core.automation_update(AutomationKind::Action, "penalty", changed)
                    .unwrap();
            }
            "event" => {
                core.automation_set_enabled(AutomationKind::Event, "event", false)
                    .unwrap();
                core.automation_set_enabled(AutomationKind::Event, "event", true)
                    .unwrap();
            }
            "recreate" => {
                core.automation_delete(AutomationKind::Event, "event")
                    .unwrap();
                core.automation_create(AutomationKind::Event, event)
                    .unwrap();
            }
            #[cfg(feature = "persistence")]
            "remote-disable" | "remote-delete" => {
                use crate::{
                    collaboration::*,
                    control::{AutomationMutation, CommandCaller},
                };
                let auth = remote_event_auth(&core);
                let request = CommandRequest {
                    command_id: "remote-stop".into(),
                    host_instance_id: core.collaboration().unwrap().instance_id().into(),
                    expected_revision: "1".into(),
                };
                let operation = || {
                    if change == "remote-disable" {
                        AutomationMutation::SetEnabled {
                            kind: AutomationKind::Event,
                            id: "event".into(),
                            enabled: false,
                        }
                    } else {
                        AutomationMutation::Delete {
                            kind: AutomationKind::Event,
                            id: "event".into(),
                        }
                    }
                };
                core.automation_command(
                    CommandCaller::Remote(auth.clone()),
                    request.clone(),
                    operation(),
                )
                .unwrap();
                if change == "remote-disable" {
                    core.automation_set_enabled(AutomationKind::Event, "event", true)
                        .unwrap();
                } else {
                    core.automation_create(AutomationKind::Event, event)
                        .unwrap();
                }
                // An original-ID retry cannot undo the owner's later decision
                // or revive the stale in-flight event generation.
                core.automation_command(CommandCaller::Remote(auth), request, operation())
                    .unwrap();
                assert_eq!(
                    core.automation_get(AutomationKind::Event, "event").unwrap()["enabled"],
                    true
                );
            }
            "unrelated" => {
                core.automation_create(
                    AutomationKind::Action,
                    json!({"id":"other","name":"Other","enabled":true,"typeId":"core.log"}),
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        tokio::time::advance(std::time::Duration::from_secs(2)).await;
        run.await;
        assert_eq!(
            core.points.viewer("fenced-viewer").unwrap()["points"],
            if change == "unrelated" { 8.0 } else { 10.0 },
            "{change}"
        );
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }
}

#[tokio::test]
async fn admission_yields_during_a_definition_write_and_does_not_block_on_audit_delivery() {
    let (core, root) = fixture();
    core.automation_create(
        AutomationKind::Action,
        json!({"id":"action","name":"Action","enabled":true,"typeId":"core.log"}),
    )
    .unwrap();
    core.automation_create(AutomationKind::Event,json!({"id":"event","name":"Event","enabled":true,"trigger":"tiktok.chat","actionIds":["action"]})).unwrap();
    let input = json!({"type":"tiktok.chat"});
    let records = core.automation.matching_records(&input);
    let (entered, ready) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let writer = core.clone();
    let thread = std::thread::spawn(move || {
        let _guard = writer.automation_gate().unwrap();
        entered.send(()).unwrap();
        let _ = wait.recv_timeout(std::time::Duration::from_secs(3));
        // Simulate the index-install half of a committed operator write.
        writer.automation.upsert_action(
            json!({"id":"action","name":"Changed","enabled":true,"typeId":"core.log"}),
        );
    });
    ready
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap();
    let incoming = crate::automation_runtime::context::ExecutionContext::owner();
    let mut run = std::pin::pin!(core.run_matched_records(records, &input, &incoming));
    let started = std::time::Instant::now();
    poll_fn(|cx| {
        assert!(run.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "configuration admission must yield instead of blocking a Tokio worker"
    );
    release.send(()).unwrap();
    thread.join().unwrap();
    run.await;
    assert!(
        core.automation.recent_runs().is_empty(),
        "the stale action must never start"
    );
    #[cfg(feature = "persistence")]
    {
        let (entered, ready) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let owner = core.clone();
        let thread = std::thread::spawn(move || {
            let _guard = owner.coordination_gate.lock().unwrap();
            entered.send(()).unwrap();
            let _ = wait.recv_timeout(std::time::Duration::from_secs(3));
        });
        ready
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        let record = core.automation.matching_records(&input).pop().unwrap();
        let started = std::time::Instant::now();
        assert!(core.action_still_current(&record, &record.actions[0]).await);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "unrelated audit delivery must not hold execution admission"
        );
        release.send(()).unwrap();
        thread.join().unwrap();
    }
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn shutdown_wakes_admission_waiting_for_a_definition_write() {
    let (core, root) = fixture();
    core.automation_create(
        AutomationKind::Action,
        json!({"id":"action","name":"Action","enabled":true,"typeId":"core.log"}),
    )
    .unwrap();
    core.automation_create(AutomationKind::Event,json!({"id":"event","name":"Event","enabled":true,"trigger":"tiktok.chat","actionIds":["action"]})).unwrap();
    let input = json!({"type":"tiktok.chat"});
    let record = core.automation.matching_records(&input).pop().unwrap();
    let (entered, ready) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let writer = core.clone();
    let thread = std::thread::spawn(move || {
        let _guard = writer.automation_gate().unwrap();
        entered.send(()).unwrap();
        let _ = wait.recv_timeout(std::time::Duration::from_secs(3));
    });
    ready
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap();
    let mut admission = std::pin::pin!(core.action_still_current(&record, &record.actions[0]));
    poll_fn(|cx| {
        assert!(admission.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    core.shutdown().await;
    assert!(
        !tokio::time::timeout(std::time::Duration::from_secs(1), admission)
            .await
            .unwrap()
    );
    release.send(()).unwrap();
    thread.join().unwrap();
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn behavior_metadata_delivery_imports_audit_once_without_dispatching_live_effects() {
    use crate::collaboration::{AuditQuery, CoordinationEvent, ResourceKind};
    let (core, root) = fixture();
    core.automation_create(AutomationKind::Action, json!({"id":"effect","name":"Effect","enabled":true,"typeId":"core.points.add","config":{"uniqueId":"never-awarded","amount":100}})).unwrap();
    core.automation_create(AutomationKind::Event, json!({"id":"event","name":"Event","enabled":true,"trigger":"tiktok.chat","actionIds":["effect"]})).unwrap();
    core.automation_set_enabled(AutomationKind::Event, "event", false)
        .unwrap();
    core.automation_set_enabled(AutomationKind::Event, "event", true)
        .unwrap();
    core.automation_delete(AutomationKind::Action, "effect")
        .unwrap();
    let mut notifications = core.subscribe_coordination().unwrap();
    assert_eq!(core.deliver_coordination().unwrap(), 6);
    assert_eq!(core.deliver_coordination().unwrap(), 0);
    let mut changes = Vec::new();
    while let Ok(event) = notifications.try_recv() {
        let CoordinationEvent::Invalidated {
            resource, change, ..
        } = event.as_ref()
        else {
            panic!("legacy owner writes must not invent command receipts");
        };
        assert_eq!(resource.kind, ResourceKind::Automation);
        changes.push(change.clone());
    }
    assert_eq!(changes.len(), 6);
    assert!(changes.iter().any(|s| s == "references-updated"));
    let service = core.collaboration().unwrap();
    let audit = service
        .audit(AuditQuery {
            after: 0,
            limit: Some(100),
        })
        .unwrap();
    let records: Vec<_> = audit.iter().filter_map(|a| a.details.clone()).collect();
    assert_eq!(records.len(), 6);
    assert!(records
        .iter()
        .any(|r| r.method == "automation.references.update" && r.resource.scope == "event"));
    assert!(records
        .iter()
        .all(|r| r.principal_id == "local-owner" && r.command_id.is_none()));
    assert_eq!(service.import_audit(&records).unwrap(), 0);
    assert_eq!(
        service
            .audit(AuditQuery {
                after: 0,
                limit: Some(100)
            })
            .unwrap()
            .len(),
        audit.len()
    );
    assert!(core.automation.recent_runs().is_empty());
    assert!(core.points.viewer("never-awarded").is_none());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn exhausted_or_panicked_definition_writers_reject_admission_and_further_mutation() {
    use std::sync::atomic::Ordering;
    for panic_writer in [false, true] {
        let (core, root) = fixture();
        core.automation_create(
            AutomationKind::Action,
            json!({"id":"action","name":"Action","enabled":true,"typeId":"core.log"}),
        )
        .unwrap();
        core.automation_create(AutomationKind::Event, json!({"id":"event","name":"Event","enabled":true,"trigger":"tiktok.chat","actionIds":["action"]})).unwrap();
        let input = json!({"type":"tiktok.chat"});
        let record = core.automation.matching_records(&input).pop().unwrap();
        if panic_writer {
            let writer = core.clone();
            assert!(std::thread::spawn(move || {
                let _guard = writer.automation_gate().unwrap();
                panic!("simulated definition writer failure");
            })
            .join()
            .is_err());
        } else {
            core.automation_write_epoch
                .store(u64::MAX - 1, Ordering::Release);
        }
        assert!(!tokio::time::timeout(
            std::time::Duration::from_secs(1),
            core.action_still_current(&record, &record.actions[0])
        )
        .await
        .unwrap());
        assert!(core
            .automation_set_enabled(AutomationKind::Event, "event", false)
            .is_err());
        assert!(core.automation.recent_runs().is_empty());
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(feature = "persistence")]
fn remote_event_auth(core: &AppCore) -> crate::collaboration::Authorization {
    use crate::collaboration::*;
    let s = core.collaboration().unwrap();
    let mut config = s.configuration().unwrap();
    config.enabled = true;
    config.advertised_origin = Some("http://127.0.0.1:8788".into());
    s.configure(config).unwrap();
    let invite = s
        .create_invitation(InviteCreate {
            grant: Grant {
                capabilities: [
                    Capability::AutomationRead,
                    Capability::AutomationToggle,
                    Capability::AutomationEdit,
                ]
                .into(),
                resources: [(ResourceKind::Automation, ["event:event".into()].into())].into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let exchange = "a".repeat(64);
    let request = s
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Moderation test".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "fixture",
        )
        .unwrap();
    s.decide_request(&request.request_id, true).unwrap();
    let credentials = s
        .collect(
            Collect {
                request_id: request.request_id,
                exchange_secret: exchange,
            },
            "fixture",
        )
        .unwrap();
    s.authenticate_live(&credentials.access_token, false, false)
        .unwrap()
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn remote_behavior_commands_recheck_revocation_after_sqlite_writer_admission() {
    use crate::{
        collaboration::*,
        control::{AutomationMutation, CommandCaller},
    };
    for replay in [false, true] {
        let (core, root) = fixture();
        core.automation_create(
            AutomationKind::Event,
            json!({"id":"event","name":"Event","enabled":true}),
        )
        .unwrap();
        let auth = remote_event_auth(&core);
        let identity = auth.snapshot().unwrap();
        let request = CommandRequest {
            command_id: "revoked".into(),
            host_instance_id: core.collaboration().unwrap().instance_id().into(),
            expected_revision: "1".into(),
        };
        let operation = || AutomationMutation::SetEnabled {
            kind: AutomationKind::Event,
            id: "event".into(),
            enabled: false,
        };
        if replay {
            core.automation_command(
                CommandCaller::Remote(auth.clone()),
                request.clone(),
                operation(),
            )
            .unwrap();
            core.automation_set_enabled(AutomationKind::Event, "event", true)
                .unwrap();
        }
        // Capture a valid admission identity before queueing. Invoke the private
        // transaction directly so an earlier core check cannot accidentally make
        // this test pass if the post-SQLite authorization check is removed.
        operation().authorize_remote(&auth).unwrap();
        let command = crate::db::CommandTransaction {
            principal_id: identity.principal_id.clone(),
            session_id: identity.session_id.clone(),
            authorization: Some(auth),
            request: request.clone(),
            current_instance_id: request.host_instance_id.clone(),
            max_ids_per_principal: 100,
        };
        let mut db = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        let database = core.db.clone();
        let (entered, ready) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            entered.send(()).unwrap();
            database.execute_behavior_command(&operation(), &command)
        });
        ready
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap();
        // Independent identity storage remains writable while the behavior
        // transaction waits for SQLite. Both a fresh mutation and replay must
        // authenticate after that writer admission, before reading any outcome.
        core.collaboration()
            .unwrap()
            .revoke_session(&identity.session_id, "owner")
            .unwrap();
        tx.commit().unwrap();
        assert!(
            matches!(
                thread.join().unwrap(),
                Err(crate::db::DatabaseError::CommandAuthorization(_))
            ),
            "replay={replay}"
        );
        assert_eq!(
            core.automation_get(AutomationKind::Event, "event").unwrap()["enabled"],
            true
        );
        assert_eq!(
            core.db
                .command_resource(&identity.principal_id, "revoked")
                .unwrap()
                .is_some(),
            replay
        );
        core.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }
}
