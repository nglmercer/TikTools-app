use super::*;
use crate::{
    collaboration::*,
    control::{AutomationMutation, CommandCaller, OwnerAdoptionSubmit, OwnerApprovalSubmit},
    paths::AppPaths,
    HostEmitter, NoopMediaHost,
};
use rusqlite::Connection;
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

const SOURCE: &str = "private-owner-definition-source";
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}
fn paths(root: PathBuf) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    }
}
fn open(paths: &AppPaths) -> Arc<AppCore> {
    Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths.clone(),
    ))
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    paths: AppPaths,
}
impl Fixture {
    fn new() -> Self {
        let paths = paths(std::env::temp_dir().join(format!(
            "tt-owner-definition-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        )));
        paths.ensure_directories().unwrap();
        for file in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(file).unwrap();
        }
        Self {
            core: Some(open(&paths)),
            paths,
        }
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    fn db(&self) -> Connection {
        Connection::open(self.paths.automation_database()).unwrap()
    }
    fn reopen(&mut self) {
        drop(self.core.take());
        self.core = Some(open(&self.paths));
    }
    fn seed(&self, kind: AutomationKind) {
        self.core()
            .automation_create_versioned(kind, source(kind, SOURCE), Some("0"))
            .unwrap();
    }
    fn submit(
        &self,
        kind: AutomationKind,
        operation: OwnerDefinitionOperation,
        revision: &str,
    ) -> OwnerDefinitionSubmit {
        input(self.core(), kind, operation, revision)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.paths.root);
    }
}
fn source(kind: AutomationKind, name: &str) -> Value {
    match kind {
        AutomationKind::Action => {
            json!({"id":"target","name":name,"enabled":false,"typeId":"core.log","config":{"message":SOURCE}})
        }
        AutomationKind::Event => {
            json!({"id":"target","name":name,"enabled":false,"trigger":"tiktok.chat","actionIds":[]})
        }
    }
}
fn input(
    core: &AppCore,
    kind: AutomationKind,
    operation: OwnerDefinitionOperation,
    revision: &str,
) -> OwnerDefinitionSubmit {
    let service = core.collaboration().unwrap();
    let host_id = service.host_id().to_owned();
    let host_instance_id = service.instance_id().to_owned();
    let expected_revision = revision.to_owned();
    let id = "target".to_owned();
    match operation {
        OwnerDefinitionOperation::Create => OwnerDefinitionSubmit::Create {
            kind,
            record: source(kind, SOURCE),
            expected_revision,
            host_id,
            host_instance_id,
        },
        OwnerDefinitionOperation::Update => OwnerDefinitionSubmit::Update {
            kind,
            id,
            record: source(kind, "updated source"),
            expected_revision,
            host_id,
            host_instance_id,
        },
        OwnerDefinitionOperation::Delete => OwnerDefinitionSubmit::Delete {
            kind,
            id,
            expected_revision,
            host_id,
            host_instance_id,
        },
        OwnerDefinitionOperation::Enable => OwnerDefinitionSubmit::Enable {
            kind,
            id,
            expected_revision,
            host_id,
            host_instance_id,
        },
        OwnerDefinitionOperation::Disable => OwnerDefinitionSubmit::Disable {
            kind,
            id,
            expected_revision,
            host_id,
            host_instance_id,
        },
    }
}
fn persisted_revision(f: &Fixture, kind: AutomationKind) -> String {
    f.db().query_row("SELECT revision FROM control_resource_versions WHERE kind='automation' AND scope=?1 AND resource_id='target'",[match kind{AutomationKind::Action=>"action",AutomationKind::Event=>"event"}],|r|r.get::<_,i64>(0)).unwrap().to_string()
}

#[test]
fn every_single_record_operation_returns_only_its_fresh_verified_acknowledgement() {
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        let mut f = Fixture::new();
        let created = f
            .core()
            .owner_definition_submit(f.submit(kind, OwnerDefinitionOperation::Create, "0"))
            .unwrap();
        assert!(created.state.pending.is_none());
        let ack = created.acknowledgement.unwrap();
        assert_eq!(ack.value, source(kind, SOURCE));
        assert_eq!(ack.resource_versions[0].revision, "1");
        assert!(ack.command.command_id.starts_with("owner-definition-"));
        let mut previous = ack.command.command_id;
        for (operation, before, after) in [
            (OwnerDefinitionOperation::Update, "1", "2"),
            (OwnerDefinitionOperation::Enable, "2", "3"),
            (OwnerDefinitionOperation::Disable, "3", "4"),
            (OwnerDefinitionOperation::Delete, "4", "5"),
        ] {
            let result = f
                .core()
                .owner_definition_submit(f.submit(kind, operation, before))
                .unwrap();
            let ack = result.acknowledgement.unwrap();
            assert_ne!(ack.command.command_id, previous);
            previous = ack.command.command_id;
            assert_eq!(ack.resource_versions[0].revision, after);
            assert_eq!(result.state.recent[0].intent.operation, operation);
            assert_eq!(
                result.state.recent[0].outcome,
                OwnerDefinitionOutcome::Succeeded
            );
            assert_eq!(persisted_revision(&f, kind), after);
        }
        let state = f.core().owner_definition_state().unwrap();
        let serialized = serde_json::to_string(&state).unwrap();
        assert!(!serialized.contains(SOURCE));
        assert!(!serialized.contains("updated source"));
        let native: String = f
            .db()
            .query_row(
                "SELECT group_concat(payload_json) FROM owner_automation_definitions",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!native.contains(SOURCE));
        f.reopen();
        let recovered = f.core().owner_definition_resolve().unwrap();
        assert_eq!(recovered.recent, state.recent);
        assert_eq!(persisted_revision(&f, kind), "5");
        let absent_update = f
            .core()
            .owner_definition_submit(f.submit(kind, OwnerDefinitionOperation::Update, "5"))
            .unwrap();
        assert!(absent_update.acknowledgement.is_none());
        assert_eq!(
            absent_update.state.recent[0].error_code.as_deref(),
            Some("not_found")
        );
        // Recreation uses the reviewed retained tombstone, never an implicit
        // update or a reset to revision zero.
        let recreated = f
            .core()
            .owner_definition_submit(f.submit(kind, OwnerDefinitionOperation::Create, "5"))
            .unwrap();
        assert_eq!(
            recreated.acknowledgement.unwrap().resource_versions[0].revision,
            "6"
        );
    }
}

#[test]
fn normalization_missing_update_conflict_and_host_fences_never_mint_unchecked_writes() {
    let f = Fixture::new();
    let mut create = f.submit(
        AutomationKind::Action,
        OwnerDefinitionOperation::Create,
        "0",
    );
    if let OwnerDefinitionSubmit::Create { record, .. } = &mut create {
        record["name"] = json!("");
    }
    let result = f.core().owner_definition_submit(create).unwrap();
    let ack = result.acknowledgement.unwrap();
    assert_eq!(ack.value["name"], "Untitled action");
    assert_eq!(
        result.state.recent[0]
            .receipt
            .as_ref()
            .unwrap()
            .record_digest,
        Some(definition_record_digest(&ack.value))
    );
    let conflict = f
        .core()
        .owner_definition_submit(f.submit(
            AutomationKind::Action,
            OwnerDefinitionOperation::Update,
            "0",
        ))
        .unwrap();
    assert!(conflict.acknowledgement.is_none());
    assert_eq!(
        conflict.state.recent[0].error_code.as_deref(),
        Some("revision_conflict")
    );
    let rejected = f
        .core()
        .owner_definition_submit(f.submit(
            AutomationKind::Action,
            OwnerDefinitionOperation::Update,
            "2",
        ))
        .unwrap();
    assert!(rejected.acknowledgement.is_none());
    assert_eq!(
        rejected.state.recent[0].outcome,
        OwnerDefinitionOutcome::Rejected
    );
    assert_eq!(
        rejected.state.recent[0].error_code.as_deref(),
        Some("revision_conflict")
    );
    let mut wrong = f.submit(
        AutomationKind::Action,
        OwnerDefinitionOperation::Update,
        "1",
    );
    if let OwnerDefinitionSubmit::Update { record, .. } = &mut wrong {
        record["id"] = json!("other");
    }
    assert!(f.core().owner_definition_submit(wrong).is_err());
    let mut stale = f.submit(
        AutomationKind::Action,
        OwnerDefinitionOperation::Enable,
        "1",
    );
    if let OwnerDefinitionSubmit::Enable {
        host_instance_id, ..
    } = &mut stale
    {
        *host_instance_id = "stale-instance".into();
    }
    assert_eq!(
        f.core().owner_definition_submit(stale).unwrap_err().code,
        "host_restarted"
    );
    let state = f.core().owner_definition_state().unwrap();
    assert!(state.pending.is_none());
    assert_eq!(state.recent.len(), 3);
    assert_eq!(persisted_revision(&f, AutomationKind::Action), "1");
    let missing = Fixture::new();
    let rejected = missing
        .core()
        .owner_definition_submit(missing.submit(
            AutomationKind::Event,
            OwnerDefinitionOperation::Update,
            "1",
        ))
        .unwrap();
    assert!(rejected.acknowledgement.is_none());
    assert_eq!(
        rejected.state.recent[0].outcome,
        OwnerDefinitionOutcome::Rejected
    );
    assert_eq!(
        missing
            .core()
            .automation_get(AutomationKind::Event, "target")
            .unwrap_err()
            .code,
        "not_found"
    );
    let noop = missing
        .core()
        .owner_definition_submit(missing.submit(
            AutomationKind::Event,
            OwnerDefinitionOperation::Delete,
            "0",
        ))
        .unwrap();
    let ack = noop.acknowledgement.unwrap();
    assert_eq!(ack.value, json!({"id":"target","deleted":false}));
    assert_eq!(ack.resource_versions[0].revision, "0");
    assert_eq!(
        noop.state.recent[0].receipt.as_ref().unwrap().deleted,
        Some(false)
    );
}

#[test]
fn action_deletion_acknowledges_all_cascades_and_recovery_never_reinstalls_old_source() {
    let f = Fixture::new();
    f.seed(AutomationKind::Action);
    for id in ["a-rule", "z-rule"] {
        f.core().automation_create_versioned(AutomationKind::Event,json!({"id":id,"name":id,"enabled":false,"trigger":"tiktok.chat","actionIds":["target"]}),Some("0")).unwrap();
    }
    let deleted = f
        .core()
        .owner_definition_submit(f.submit(
            AutomationKind::Action,
            OwnerDefinitionOperation::Delete,
            "1",
        ))
        .unwrap();
    let ack = deleted.acknowledgement.unwrap();
    assert_eq!(ack.resource_versions.len(), 3);
    assert_eq!(ack.resource_versions[1].id, "a-rule");
    assert_eq!(ack.resource_versions[2].id, "z-rule");
    assert_eq!(
        deleted.state.recent[0]
            .receipt
            .as_ref()
            .unwrap()
            .affected_resources,
        3
    );
    f.core()
        .automation_create_versioned(
            AutomationKind::Action,
            source(AutomationKind::Action, "newer recreated source"),
            Some("2"),
        )
        .unwrap();
    let state = f.core().owner_definition_resolve().unwrap();
    assert_eq!(state.recent, deleted.state.recent);
    assert_eq!(
        f.core()
            .automation_get(AutomationKind::Action, "target")
            .unwrap()["name"],
        "newer recreated source"
    );
    assert_eq!(persisted_revision(&f, AutomationKind::Action), "3");
    for id in ["a-rule", "z-rule"] {
        assert_eq!(
            f.core().automation_get(AutomationKind::Event, id).unwrap()["actionIds"],
            json!([])
        );
    }
}

#[test]
fn pending_definition_blocks_both_existing_authority_journals_and_requires_exact_unknown_review() {
    let f = Fixture::new();
    f.seed(AutomationKind::Action);
    let core = f.core();
    let service = core.collaboration().unwrap();
    let (_, _, expected, operation) = prepare(f.submit(
        AutomationKind::Action,
        OwnerDefinitionOperation::Enable,
        "1",
    ))
    .unwrap();
    let intent = OwnerDefinitionIntent {
        command_id: format!("owner-definition-{}", "a".repeat(32)),
        operation: OwnerDefinitionOperation::Enable,
        kind: AutomationKind::Action,
        id: "target".into(),
        expected_revision: expected,
        host_instance_id: service.instance_id().into(),
        record_digest: None,
        created_at: definition_now(),
    };
    core.db
        .owner_definition_begin(service.host_id(), &intent, &operation)
        .unwrap();
    assert_eq!(
        core.owner_definition_submit(f.submit(
            AutomationKind::Action,
            OwnerDefinitionOperation::Disable,
            "1"
        ))
        .unwrap_err()
        .code,
        "command_pending"
    );
    let revoke = OwnerApprovalSubmit::Revoke {
        kind: AutomationKind::Action,
        id: "target".into(),
        expected_revision: "1".into(),
        host_id: service.host_id().into(),
        host_instance_id: service.instance_id().into(),
        acknowledge_revocation: true,
    };
    assert_eq!(
        core.owner_approval_submit(revoke).unwrap_err().code,
        "command_pending"
    );
    let reviewed = core
        .automation_review(AutomationKind::Action, "target")
        .unwrap();
    assert_eq!(
        core.owner_adoption_submit(OwnerAdoptionSubmit {
            kind: AutomationKind::Action,
            id: "target".into(),
            expected_revision: "1".into(),
            host_id: service.host_id().into(),
            host_instance_id: service.instance_id().into(),
            reviewed_versions: reviewed.resource_versions,
            acknowledge_authority: true,
        })
        .unwrap_err()
        .code,
        "command_pending"
    );
    assert_eq!(
        core.owner_definition_resolve().unwrap().pending,
        Some(intent.clone())
    );
    assert_eq!(
        core.owner_definition_review_unknown(OwnerDefinitionReviewUnknown {
            command_id: "stale".into(),
            acknowledge_unknown: true
        })
        .unwrap_err()
        .code,
        "stale_view"
    );
    assert_eq!(
        core.owner_definition_review_unknown(OwnerDefinitionReviewUnknown {
            command_id: intent.command_id.clone(),
            acknowledge_unknown: false
        })
        .unwrap_err()
        .code,
        "confirmation_required"
    );
    let cleared = core
        .owner_definition_review_unknown(OwnerDefinitionReviewUnknown {
            command_id: intent.command_id,
            acknowledge_unknown: true,
        })
        .unwrap();
    assert!(cleared.pending.is_none());
    assert_eq!(
        cleared.recent[0].outcome,
        OwnerDefinitionOutcome::ReviewedUnknown
    );
    assert_eq!(persisted_revision(&f, AutomationKind::Action), "1");
}

#[test]
fn owned_journal_failures_preserve_pending_and_source_authority() {
    let f = Fixture::new();
    f.seed(AutomationKind::Action);
    f.db().execute_batch("CREATE TRIGGER fail_definition_history BEFORE UPDATE ON owner_automation_definitions WHEN NEW.pending=0 BEGIN SELECT RAISE(ABORT,'private journal fixture'); END;").unwrap();
    assert!(f
        .core()
        .owner_definition_submit(f.submit(
            AutomationKind::Action,
            OwnerDefinitionOperation::Update,
            "1"
        ))
        .is_err());
    assert_eq!(persisted_revision(&f, AutomationKind::Action), "2");
    assert!(f.core().owner_definition_state().unwrap().pending.is_some());
    f.db()
        .execute_batch("DROP TRIGGER fail_definition_history")
        .unwrap();
    let state = f.core().owner_definition_resolve().unwrap();
    assert!(state.pending.is_none());
    assert_eq!(state.recent[0].outcome, OwnerDefinitionOutcome::Succeeded);
    assert_eq!(persisted_revision(&f, AutomationKind::Action), "2");
}

#[tokio::test]
async fn native_owner_edits_and_enablement_preserve_delegated_provenance_and_quarantine() {
    let f = Fixture::new();
    let core = f.core();
    let service = core.collaboration().unwrap();
    let mut config = service.configuration().unwrap();
    config.enabled = true;
    config.advertised_origin = Some("http://127.0.0.1:8788".into());
    service.configure(config).unwrap();
    let invitation = service
        .create_invitation(InviteCreate {
            grant: Grant {
                capabilities: [Capability::AutomationRead, Capability::AutomationEdit].into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let exchange = "d".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invitation.invite_id,
                secret: invitation.secret,
                client_label: "Definition fixture author".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "fixture",
        )
        .unwrap();
    service.decide_request(&pending.request_id, true).unwrap();
    let credentials = service
        .collect(
            Collect {
                request_id: pending.request_id,
                exchange_secret: exchange,
            },
            "fixture",
        )
        .unwrap();
    let author = service
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap();
    core.automation_command(
        CommandCaller::Remote(author),
        CommandRequest {
            command_id: "disabled-author-source".into(),
            host_instance_id: service.instance_id().into(),
            expected_revision: "0".into(),
        },
        AutomationMutation::Create {
            kind: AutomationKind::Action,
            record: source(AutomationKind::Action, SOURCE),
        },
    )
    .unwrap();
    let before = core
        .automation_authority(AutomationKind::Action, "target")
        .unwrap()
        .value["provenance"]["authority"]
        .clone();
    assert_eq!(before["kind"], "delegated");
    core.owner_definition_submit(f.submit(
        AutomationKind::Action,
        OwnerDefinitionOperation::Update,
        "1",
    ))
    .unwrap();
    core.owner_definition_submit(f.submit(
        AutomationKind::Action,
        OwnerDefinitionOperation::Enable,
        "2",
    ))
    .unwrap();
    assert_eq!(
        core.automation_authority(AutomationKind::Action, "target")
            .unwrap()
            .value["provenance"]["authority"],
        before
    );
    let action = core
        .automation_get(AutomationKind::Action, "target")
        .unwrap();
    assert_eq!(action["enabled"], true);
    let run = core
        .execute_action(&action, &json!({"type":"tiktok.chat"}), None, false)
        .await;
    assert_eq!(run["error"], "authority_required");
    core.shutdown().await;
}

#[test]
#[ignore = "subprocess-only native definition crash fixture"]
fn crash_child() {
    let root = PathBuf::from(std::env::var_os("TIKTOOLS_OWNER_DEFINITION_ROOT").unwrap());
    let paths = paths(root);
    let core = open(&paths);
    let kind = if std::env::var("TIKTOOLS_OWNER_DEFINITION_KIND").unwrap() == "action" {
        AutomationKind::Action
    } else {
        AutomationKind::Event
    };
    let operation: OwnerDefinitionOperation = serde_json::from_str(&format!(
        "\"{}\"",
        std::env::var("TIKTOOLS_OWNER_DEFINITION_OPERATION").unwrap()
    ))
    .unwrap();
    let before = if operation == OwnerDefinitionOperation::Create {
        "0"
    } else {
        "1"
    };
    core.owner_definition_submit(input(&core, kind, operation, before))
        .unwrap();
    panic!("native definition crash hook did not exit");
}

#[test]
fn real_process_death_at_every_native_and_business_boundary_recovers_original_outcome_only() {
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        for operation in [
            OwnerDefinitionOperation::Create,
            OwnerDefinitionOperation::Update,
            OwnerDefinitionOperation::Delete,
            OwnerDefinitionOperation::Enable,
            OwnerDefinitionOperation::Disable,
        ] {
            for (stage, committed, code) in [
                ("before_intent", false, 90),
                ("after_intent", false, 90),
                ("before_dispatch", false, 90),
                ("business_before", false, 91),
                ("business_after", true, 92),
                ("after_effect", true, 90),
                ("after_record", true, 90),
            ] {
                let mut f = Fixture::new();
                if operation != OwnerDefinitionOperation::Create {
                    f.seed(kind);
                }
                drop(f.core.take());
                let mut child = std::process::Command::new(std::env::current_exe().unwrap());
                child
                    .args([
                        "--exact",
                        "control::owner_definition::tests::crash_child",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env("TIKTOOLS_OWNER_DEFINITION_ROOT", &f.paths.root)
                    .env(
                        "TIKTOOLS_OWNER_DEFINITION_KIND",
                        match kind {
                            AutomationKind::Action => "action",
                            AutomationKind::Event => "event",
                        },
                    )
                    .env(
                        "TIKTOOLS_OWNER_DEFINITION_OPERATION",
                        serde_json::to_value(operation).unwrap().as_str().unwrap(),
                    );
                if let Some(boundary) = stage.strip_prefix("business_") {
                    child.env("TIKTOOLS_BEHAVIOR_CRASH", boundary);
                } else {
                    child.env("TIKTOOLS_OWNER_DEFINITION_CRASH", stage);
                }
                let output = child.output().unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(code),
                    "{kind:?}/{operation:?}/{stage}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                f.core = Some(open(&f.paths));
                let before = if operation == OwnerDefinitionOperation::Create {
                    0
                } else {
                    1
                };
                let expected = before + i32::from(committed);
                let resolved = f.core().owner_definition_resolve().unwrap();
                if committed {
                    assert!(resolved.pending.is_none());
                    assert_eq!(resolved.recent.len(), 1);
                    assert_eq!(
                        resolved.recent[0].outcome,
                        OwnerDefinitionOutcome::Succeeded
                    );
                    assert_eq!(
                        resolved.recent[0].receipt.as_ref().unwrap().revision,
                        expected.to_string()
                    );
                    assert_eq!(persisted_revision(&f, kind), expected.to_string());
                } else if stage == "before_intent" {
                    assert!(resolved.pending.is_none());
                    assert!(resolved.recent.is_empty());
                } else {
                    let pending = resolved.pending.unwrap();
                    let cleared = f
                        .core()
                        .owner_definition_review_unknown(OwnerDefinitionReviewUnknown {
                            command_id: pending.command_id,
                            acknowledge_unknown: true,
                        })
                        .unwrap();
                    assert_eq!(
                        cleared.recent[0].outcome,
                        OwnerDefinitionOutcome::ReviewedUnknown
                    );
                }
                let ledger: i64 = f
                    .db()
                    .query_row(
                        "SELECT count(*) FROM control_commands WHERE principal_id='local-owner'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert_eq!(ledger, i64::from(committed));
                if committed {
                    // A later independent Owner write must remain installed
                    // after another original-result lookup.
                    if operation == OwnerDefinitionOperation::Delete {
                        f.core()
                            .automation_create_versioned(
                                kind,
                                source(kind, "newer owner source"),
                                Some(&expected.to_string()),
                            )
                            .unwrap();
                    } else {
                        f.core()
                            .automation_update_versioned(
                                kind,
                                "target",
                                source(kind, "newer owner source"),
                                Some(&expected.to_string()),
                            )
                            .unwrap();
                    }
                    let state = f.core().owner_definition_resolve().unwrap();
                    assert_eq!(state.recent, resolved.recent);
                    assert_eq!(
                        f.core().automation_get(kind, "target").unwrap()["name"],
                        "newer owner source"
                    );
                    assert_eq!(persisted_revision(&f, kind), (expected + 1).to_string());
                }
            }
        }
    }
}
