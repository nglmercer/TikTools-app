use super::*;
use crate::{collaboration::*, paths::AppPaths, services::automation_manifest as manifest};
use std::sync::Arc;

struct Fixture {
    db: DatabaseManager,
    service: Arc<CollaborationService>,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-approvals-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
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
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            drop(Connection::open(path).unwrap());
        }
        let db = DatabaseManager::new(paths);
        let service = Arc::new(CollaborationService::open(&root.join("identity")).unwrap());
        let mut config = service.configuration().unwrap();
        config.enabled = true;
        config.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(config).unwrap();
        Self { db, service, root }
    }
    fn review(&self, table: &str, id: &str) -> Versioned<Value> {
        let db = self.db.open(&self.db.automation_path()).unwrap();
        review::snapshot(&db, table, id).unwrap()
    }
    fn binding(
        &self,
        review: &Versioned<Value>,
        identities: &[ExecutionIdentity],
    ) -> BehaviorApprovalBinding {
        let subjects: Vec<_> = identities
            .iter()
            .map(|i| manifest::ManifestSubject {
                principal_id: i.principal_id(),
                session_id: i.session_id(),
                grant_revision: i.grant_revision(),
                configuration_revision: i.configuration_revision(),
                grant: i.grant(),
            })
            .collect();
        let result = manifest::build(
            review,
            manifest::ManifestSubjects {
                host_id: self.service.host_id(),
                subjects: &subjects,
            },
        )
        .unwrap();
        BehaviorApprovalBinding {
            manifest: serde_json::to_value(result).unwrap(),
            bindings: json!({"policyVersion":1,"allowedEffects":["log","delay"]}),
        }
    }
    fn resolve(&self, review: &Versioned<Value>) -> Result<ExecutionAuthorizations, DatabaseError> {
        self.service
            .execution_authorizations(&manifest::required_subjects(review).unwrap())
            .map_err(DatabaseError::CommandAuthorization)
    }
    fn activate(
        &self,
        table: &str,
        id: &str,
        command_id: &str,
        plan: &[BehaviorApprovalEnableState],
    ) -> Versioned<Value> {
        let review = self.review(table, id);
        let identities = self.resolve(&review).unwrap().snapshot().unwrap();
        let digest = self.binding(&review, &identities).manifest["manifestDigest"]
            .as_str()
            .unwrap()
            .to_owned();
        self.db
            .execute_behavior_approval_activation(
                &BehaviorApprovalActivation {
                    table,
                    id,
                    reviewed_versions: &review.resource_versions,
                    preview_digest: &digest,
                    enable_states: plan,
                    acknowledged: true,
                },
                &command(command_id, &review.resource_versions[0].revision),
                |r| self.resolve(r),
                |r, i| Ok(self.binding(r, i)),
            )
            .unwrap()
            .0
    }
    fn state(&self, scope: &str, id: &str) -> (i64, String) {
        self.db.open(&self.db.automation_path()).unwrap().query_row("SELECT generation,state FROM behavior_execution_approvals WHERE scope=?1 AND resource_id=?2",params![scope,id],|r| Ok((r.get(0)?,r.get(1)?))).unwrap()
    }
    fn author(&self) -> Authorization {
        let invitation = self
            .service
            .create_invitation(InviteCreate {
                grant: Grant {
                    capabilities: [Capability::AutomationRead, Capability::AutomationEdit].into(),
                    ..Grant::default()
                },
            })
            .unwrap();
        let exchange = "a".repeat(64);
        let pending = self
            .service
            .redeem(
                Redeem {
                    invite_id: invitation.invite_id,
                    secret: invitation.secret,
                    client_label: "Author".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "fixture",
            )
            .unwrap();
        self.service
            .decide_request(&pending.request_id, true)
            .unwrap();
        let credentials = self
            .service
            .collect(
                Collect {
                    request_id: pending.request_id,
                    exchange_secret: exchange,
                },
                "fixture",
            )
            .unwrap();
        self.service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap()
    }
    fn delegated(&self, auth: &Authorization, table: &str, record: Value) {
        let id = record["id"].as_str().unwrap();
        self.db
            .behavior_transaction(|tx| {
                let permit = provenance::authorize_authoring(tx, table, id, auth, false, false)?;
                save_record(tx, table, &record, Some("0"), BehaviorSave::Create)?;
                provenance::mark_delegated_new(tx, table, id, &permit)
            })
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn command(id: &str, expected: &str) -> crate::db::CommandTransaction {
    crate::db::CommandTransaction {
        authorization: None,
        principal_id: "owner".into(),
        session_id: "native-owner".into(),
        request: CommandRequest {
            command_id: id.into(),
            host_instance_id: "instance-a".into(),
            expected_revision: expected.into(),
        },
        current_instance_id: "instance-a".into(),
        max_ids_per_principal: 100_000,
    }
}
fn action(id: &str) -> Value {
    json!({"id":id,"name":"Action","enabled":false,"typeId":"core.log","config":{"message":"hello"}})
}
fn event(id: &str) -> Value {
    json!({"id":id,"name":"Event","enabled":false,"trigger":"tiktok.chat","actionIds":["action"]})
}
fn enable(scope: &str, id: &str) -> BehaviorApprovalEnableState {
    BehaviorApprovalEnableState {
        scope: scope.into(),
        id: id.into(),
        enabled: true,
    }
}

#[test]
fn activation_binds_final_state_and_historical_replay_never_resolves_or_reinstalls() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    let before = f.review("behavior_actions", "action");
    let preview = f.binding(&before, &[]).manifest["manifestDigest"]
        .as_str()
        .unwrap()
        .to_owned();
    let states = [enable("action", "action")];
    let request = BehaviorApprovalActivation {
        table: "behavior_actions",
        id: "action",
        reviewed_versions: &before.resource_versions,
        preview_digest: &preview,
        enable_states: &states,
        acknowledged: true,
    };
    let mut metadata = command("approve-1", "1");
    let (result, fresh) =
        f.db.execute_behavior_approval_activation(
            &request,
            &metadata,
            |r| f.resolve(r),
            |r, i| Ok(f.binding(r, i)),
        )
        .unwrap();
    assert!(fresh);
    assert_ne!(result.value["manifestDigest"], preview);
    assert_eq!(result.resource_versions[0].revision, "2");
    assert_eq!(result.value["approvalGeneration"], "1");
    let ((snapshot, authorities), approvals) =
        f.db.load_behavior_runtime_approval_snapshot().unwrap();
    assert_eq!(snapshot.value["actions"][0]["enabled"], true);
    assert_eq!(authorities[0].2.authority, BehaviorAuthority::Owner);
    assert_eq!(authorities[0].2.generation, 2);
    assert_eq!(approvals.len(), 1);
    assert_eq!(
        approvals[0].binding["manifest"]["manifestDigest"],
        result.value["manifestDigest"]
    );
    f.db.delete_behavior_versioned("behavior_actions", "action", Some("2"))
        .unwrap();
    assert_eq!(f.state("action", "action"), (2, "invalidated".into()));
    let mut config = f.service.configuration().unwrap();
    config.enabled = false;
    f.service.configure(config).unwrap();
    metadata.current_instance_id = "instance-after-restart".into();
    let (replayed, fresh) =
        f.db.execute_behavior_approval_activation(
            &request,
            &metadata,
            |_| panic!("historical replay must not resolve subjects"),
            |_, _| panic!("historical replay must not build policy"),
        )
        .unwrap();
    assert!(!fresh);
    assert_eq!(replayed.value, result.value);
    assert!(f
        .db
        .load_behavior_runtime_approval_snapshot()
        .unwrap()
        .1
        .is_empty());
    assert!(f
        .db
        .behavior_get_versioned("behavior_actions", "action")
        .unwrap()
        .value
        .is_none());
}

#[test]
fn dependency_changes_invalidate_retained_identity_and_returning_to_old_body_does_not_revive() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    f.db.save_behavior("behavior_events", &event("event"))
        .unwrap();
    let result = f.activate(
        "behavior_events",
        "event",
        "approve-event",
        &[enable("event", "event")],
    );
    assert_eq!(
        result.resource_versions.len(),
        2,
        "unchanged dependencies remain bound in receipt"
    );
    assert_eq!(result.resource_versions[1].revision, "1");
    assert_eq!(
        f.db.behavior_get_versioned("behavior_actions", "action")
            .unwrap()
            .value
            .unwrap()["enabled"],
        false
    );
    let mut changed = action("action");
    changed["name"] = "changed".into();
    f.db.save_behavior("behavior_actions", &changed).unwrap();
    assert_eq!(f.state("event", "event"), (2, "invalidated".into()));
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    assert!(f
        .db
        .load_behavior_runtime_approval_snapshot()
        .unwrap()
        .1
        .is_empty());
    let result = f.activate(
        "behavior_events",
        "event",
        "approve-again",
        &[enable("event", "event")],
    );
    assert_eq!(result.value["approvalGeneration"], "3");
    assert_eq!(
        result.resource_versions[0].revision, "3",
        "already-enabled primary advances exactly once"
    );
    let reopened = DatabaseManager::new(f.db.paths().clone());
    assert_eq!(
        reopened
            .load_behavior_runtime_approval_snapshot()
            .unwrap()
            .1[0]
            .generation,
        "3"
    );
}

#[test]
fn explicit_plan_can_enable_delegated_dependency_without_transferring_authority() {
    let f = Fixture::new();
    let auth = f.author();
    f.delegated(&auth, "behavior_actions", action("action"));
    f.delegated(&auth, "behavior_events", event("event"));
    let result = f.activate(
        "behavior_events",
        "event",
        "delegated-approve",
        &[enable("event", "event"), enable("action", "action")],
    );
    assert_eq!(
        result
            .resource_versions
            .iter()
            .map(|v| v.revision.as_str())
            .collect::<Vec<_>>(),
        ["2", "2"]
    );
    let ((_, authorities), approvals) = f.db.load_behavior_runtime_approval_snapshot().unwrap();
    assert_eq!(approvals.len(), 1);
    for (_, _, authority) in authorities {
        assert!(matches!(
            authority.authority,
            BehaviorAuthority::Delegated { .. }
        ));
    }
}

#[test]
fn invalid_plan_or_final_policy_failure_rolls_back_enablement_versions_ledger_and_outboxes() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    f.db.save_behavior("behavior_events", &event("event"))
        .unwrap();
    let review = f.review("behavior_events", "event");
    let preview = f.binding(&review, &[]).manifest["manifestDigest"]
        .as_str()
        .unwrap()
        .to_owned();
    for states in [
        vec![enable("event", "event"), enable("action", "action")],
        vec![enable("event", "event"), enable("action", "missing")],
        vec![],
    ] {
        let request = BehaviorApprovalActivation {
            table: "behavior_events",
            id: "event",
            reviewed_versions: &review.resource_versions,
            preview_digest: &preview,
            enable_states: &states,
            acknowledged: true,
        };
        assert!(f
            .db
            .execute_behavior_approval_activation(
                &request,
                &command("bad-plan", "1"),
                |r| f.resolve(r),
                |r, i| Ok(f.binding(r, i))
            )
            .is_err());
        assert_eq!(f.review("behavior_events", "event"), review);
    }
    let states = [enable("event", "event")];
    let request = BehaviorApprovalActivation {
        table: "behavior_events",
        id: "event",
        reviewed_versions: &review.resource_versions,
        preview_digest: &preview,
        enable_states: &states,
        acknowledged: true,
    };
    let mut called = 0;
    assert!(f
        .db
        .execute_behavior_approval_activation(
            &request,
            &command("bad-final", "1"),
            |r| f.resolve(r),
            |r, i| {
                called += 1;
                if called == 2 {
                    Err(DatabaseError::Invalid("final policy failed".into()))
                } else {
                    Ok(f.binding(r, i))
                }
            }
        )
        .is_err());
    assert_eq!(f.review("behavior_events", "event"), review);
    let db = f.db.open(&f.db.automation_path()).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM control_commands", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM control_resource_outbox", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM behavior_execution_approvals",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn revoke_is_durable_fenced_and_replayed_without_reinstating_approval() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    f.activate(
        "behavior_actions",
        "action",
        "approve",
        &[enable("action", "action")],
    );
    let command = command("revoke", "2");
    let (result, fresh) =
        f.db.execute_behavior_approval_revocation("behavior_actions", "action", true, &command)
            .unwrap();
    assert!(fresh);
    assert_eq!(result.value["approvalGeneration"], "2");
    assert_eq!(result.resource_versions[0].revision, "3");
    assert_eq!(f.state("action", "action"), (2, "revoked".into()));
    f.db.delete_behavior_versioned("behavior_actions", "action", Some("3"))
        .unwrap();
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    let (again, fresh) =
        f.db.execute_behavior_approval_revocation("behavior_actions", "action", true, &command)
            .unwrap();
    assert!(!fresh);
    assert_eq!(again.value, result.value);
    assert_eq!(f.state("action", "action"), (2, "revoked".into()));
    assert!(f
        .db
        .load_behavior_runtime_approval_snapshot()
        .unwrap()
        .1
        .is_empty());
    assert_eq!(
        f.activate(
            "behavior_actions",
            "action",
            "reapprove",
            &[enable("action", "action")]
        )
        .value["approvalGeneration"],
        "3"
    );
}

#[test]
fn malformed_active_storage_and_missing_reverse_dependency_fail_closed() {
    let f = Fixture::new();
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    f.activate(
        "behavior_actions",
        "action",
        "approve",
        &[enable("action", "action")],
    );
    let db = f.db.open(&f.db.automation_path()).unwrap();
    db.execute("DELETE FROM behavior_execution_approval_dependencies WHERE approval_scope='action' AND approval_id='action'",[]).unwrap();
    assert!(f.db.load_behavior_runtime_approval_snapshot().is_err());
    // Even a writer unable to find the corrupt reverse row cannot revive it:
    // the exact current definition binding is independently checked on load.
    f.db.save_behavior("behavior_actions", &action("action"))
        .unwrap();
    assert!(f.db.load_behavior_runtime_approval_snapshot().is_err());
    db.execute(
        "UPDATE behavior_execution_approvals SET binding_json='{}'",
        [],
    )
    .unwrap();
    assert!(f.db.load_behavior_runtime_approval_snapshot().is_err());
}

#[test]
fn aggregate_approval_budget_rejects_before_committing_enablement_or_ledger() {
    let f = Fixture::new();
    let padding = "x".repeat(APPROVAL_BYTES / 2);
    for (index, id) in ["first", "second"].into_iter().enumerate() {
        f.db.save_behavior("behavior_actions", &action(id)).unwrap();
        let before = f.review("behavior_actions", id);
        let preview = f.binding(&before, &[]).manifest["manifestDigest"]
            .as_str()
            .unwrap()
            .to_owned();
        let plan = [enable("action", id)];
        let request = BehaviorApprovalActivation {
            table: "behavior_actions",
            id,
            reviewed_versions: &before.resource_versions,
            preview_digest: &preview,
            enable_states: &plan,
            acknowledged: true,
        };
        let result = f.db.execute_behavior_approval_activation(
            &request,
            &command(id, "1"),
            |r| f.resolve(r),
            |r, i| {
                let mut binding = f.binding(r, i);
                binding.bindings["padding"] = padding.clone().into();
                Ok(binding)
            },
        );
        if index == 0 {
            assert!(result.is_ok());
        } else {
            assert!(matches!(result, Err(DatabaseError::SnapshotTooLarge)));
            assert_eq!(f.review("behavior_actions", id), before);
        }
    }
    assert_eq!(
        f.db.load_behavior_runtime_approval_snapshot()
            .unwrap()
            .1
            .len(),
        1
    );
    let db = f.db.open(&f.db.automation_path()).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM control_commands", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM behavior_execution_approvals",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}
