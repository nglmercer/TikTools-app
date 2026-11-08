use super::*;
use crate::{
    collaboration::*,
    control::{AutomationMutation, CommandCaller, OwnerAdoptionOutcome, OwnerAdoptionSubmit},
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::sync::Arc;
const SOURCE: &str = "private-native-approval-source";
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}
fn paths(root: &std::path::Path) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.into(),
    }
}
fn open_core(paths: &AppPaths) -> Arc<AppCore> {
    Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths.clone(),
    ))
}
fn author(core: &AppCore) -> Authorization {
    let s = core.collaboration().unwrap();
    let mut c = s.configuration().unwrap();
    c.enabled = true;
    c.advertised_origin = Some("http://127.0.0.1:8788".into());
    s.configure(c).unwrap();
    let i = s
        .create_invitation(InviteCreate {
            grant: Grant {
                capabilities: [
                    Capability::AutomationRead,
                    Capability::AutomationEdit,
                    Capability::LiveRead,
                    Capability::PointsRead,
                    Capability::PointsAdjust,
                ]
                .into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let secret = "e".repeat(64);
    let p = s
        .redeem(
            Redeem {
                invite_id: i.invite_id,
                secret: i.secret,
                client_label: "Native approval fixture".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&secret),
            },
            "fixture",
        )
        .unwrap();
    s.decide_request(&p.request_id, true).unwrap();
    let c = s
        .collect(
            Collect {
                request_id: p.request_id,
                exchange_secret: secret,
            },
            "fixture",
        )
        .unwrap();
    s.authenticate_live(&c.access_token, false, false).unwrap()
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    paths: AppPaths,
    author: Authorization,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-owner-approval-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = paths(&root);
        paths.ensure_directories().unwrap();
        for p in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(p).unwrap();
        }
        let core = open_core(&paths);
        let author = author(&core);
        Self {
            core: Some(core),
            paths,
            author,
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
        self.core = Some(open_core(&self.paths));
    }
    fn graph(&self) {
        for (kind, id, record) in [
            (
                AutomationKind::Action,
                "action",
                json!({"id":"action","name":"Delegated action","enabled":false,"typeId":"core.log","config":{"message":SOURCE}}),
            ),
            (
                AutomationKind::Event,
                "rule",
                json!({"id":"rule","name":"Delegated rule","enabled":false,"trigger":"tiktok.chat","actionIds":["action"]}),
            ),
        ] {
            self.core()
                .automation_command(
                    CommandCaller::Remote(self.author.clone()),
                    CommandRequest {
                        command_id: format!("draft-{id}"),
                        host_instance_id: self.core().collaboration().unwrap().instance_id().into(),
                        expected_revision: "0".into(),
                    },
                    AutomationMutation::Create { kind, record },
                )
                .unwrap();
        }
    }
    fn activate(&self, kind: AutomationKind, id: &str) -> OwnerApprovalSubmit {
        let m = self.core().automation_execution_manifest(kind, id).unwrap();
        let s = self.core().collaboration().unwrap();
        let mut enable_plan = vec![ApprovalEnablement {
            kind,
            id: id.into(),
            enabled: true,
        }];
        if kind == AutomationKind::Event {
            enable_plan.push(ApprovalEnablement {
                kind: AutomationKind::Action,
                id: "action".into(),
                enabled: true,
            });
        }
        OwnerApprovalSubmit::Activate {
            kind,
            id: id.into(),
            expected_revision: m.resource_versions[0].revision.clone(),
            host_id: s.host_id().into(),
            host_instance_id: s.instance_id().into(),
            reviewed_versions: m.resource_versions,
            manifest_digest: m.value["manifestDigest"].as_str().unwrap().into(),
            enable_plan,
            acknowledge_effects: true,
        }
    }
    fn revoke(&self, kind: AutomationKind, id: &str) -> OwnerApprovalSubmit {
        let s = self.core().collaboration().unwrap();
        OwnerApprovalSubmit::Revoke {
            kind,
            id: id.into(),
            expected_revision: self.revision(kind, id),
            host_id: s.host_id().into(),
            host_instance_id: s.instance_id().into(),
            acknowledge_revocation: true,
        }
    }
    fn revision(&self, kind: AutomationKind, id: &str) -> String {
        self.core()
            .automation_get_versioned(kind, id)
            .unwrap()
            .resource_versions[0]
            .revision
            .clone()
    }
    fn count(&self, table: &str) -> i64 {
        self.db()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    fn commands(&self) -> i64 {
        self.db()
            .query_row(
                "SELECT count(*) FROM control_commands WHERE principal_id='local-owner'",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }
    fn adoption(&self) -> OwnerAdoptionSubmit {
        let r = self
            .core()
            .automation_review(AutomationKind::Event, "rule")
            .unwrap();
        let s = self.core().collaboration().unwrap();
        OwnerAdoptionSubmit {
            kind: AutomationKind::Event,
            id: "rule".into(),
            expected_revision: r.resource_versions[0].revision.clone(),
            host_id: s.host_id().into(),
            host_instance_id: s.instance_id().into(),
            reviewed_versions: r.resource_versions,
            acknowledge_authority: true,
        }
    }
    fn pending(&self, input: OwnerApprovalSubmit) -> (OwnerApprovalIntent, ApprovalCommand) {
        let (host, instance, expected, operation) = parts(input);
        let (kind, id) = operation.target();
        let intent = OwnerApprovalIntent {
            command_id: format!("owner-approval-{:032x}", fastrand::u128(..)),
            operation: operation.operation(),
            kind,
            id: id.into(),
            expected_revision: expected,
            host_instance_id: instance,
            created_at: approval_now(),
        };
        self.core()
            .db
            .owner_approval_begin(&host, &intent, &operation)
            .unwrap();
        (intent, operation)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.paths.root);
    }
}
fn parts(input: OwnerApprovalSubmit) -> (String, String, String, ApprovalCommand) {
    match input {
        OwnerApprovalSubmit::Activate {
            kind,
            id,
            expected_revision,
            host_id,
            host_instance_id,
            reviewed_versions,
            manifest_digest,
            enable_plan,
            acknowledge_effects,
        } => (
            host_id,
            host_instance_id,
            expected_revision,
            ApprovalCommand::Activate(ApprovalActivateParams {
                kind,
                id,
                reviewed_versions,
                manifest_digest,
                enable_plan,
                acknowledge_effects,
            }),
        ),
        OwnerApprovalSubmit::Revoke {
            kind,
            id,
            expected_revision,
            host_id,
            host_instance_id,
            acknowledge_revocation,
        } => (
            host_id,
            host_instance_id,
            expected_revision,
            ApprovalCommand::Revoke(ApprovalRevokeParams {
                kind,
                id,
                acknowledge_revocation,
            }),
        ),
    }
}
fn apply(core: &AppCore, intent: &OwnerApprovalIntent, command: ApprovalCommand) {
    let r = CommandRequest {
        command_id: intent.command_id.clone(),
        host_instance_id: intent.host_instance_id.clone(),
        expected_revision: intent.expected_revision.clone(),
    };
    match command {
        ApprovalCommand::Activate(p) => core.automation_approval_activate(r, p),
        ApprovalCommand::Revoke(p) => core.automation_approval_revoke(r, p),
    }
    .unwrap();
}
fn metadata_only(state: &OwnerApprovalState) {
    let encoded = serde_json::to_string(state).unwrap();
    for private in [
        SOURCE,
        "payloadHash",
        "reviewedVersions",
        "enablePlan",
        "activationProof",
        "provenance",
        "\"source\"",
        "\"config\"",
        "\"subjects\"",
        "\"grant\"",
    ] {
        assert!(!encoded.contains(private), "journal disclosed {private}");
    }
}
fn native_id(id: &str) {
    let s = id.strip_prefix("owner-approval-").unwrap();
    assert_eq!(s.len(), 32);
    assert!(s
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
}

#[test]
fn native_approval_and_revocation_record_exact_receipts_without_transferring_authority() {
    let f = Fixture::new();
    f.graph();
    let activated = f
        .core()
        .owner_approval_submit(f.activate(AutomationKind::Event, "rule"))
        .unwrap();
    metadata_only(&activated);
    assert!(activated.pending.is_none());
    let r = &activated.recent[0];
    native_id(&r.intent.command_id);
    assert_eq!(r.outcome, OwnerApprovalOutcome::Succeeded);
    let receipt = r.receipt.as_ref().unwrap();
    assert_eq!(receipt.revision, "2");
    assert_eq!(receipt.policy_version, Some(1));
    assert!(receipt
        .manifest_digest
        .as_ref()
        .is_some_and(|d| d.len() == 64));
    for (kind, id) in [
        (AutomationKind::Action, "action"),
        (AutomationKind::Event, "rule"),
    ] {
        assert_eq!(
            f.core().automation_authority(kind, id).unwrap().value["provenance"]["authority"]
                ["kind"],
            "delegated"
        );
        assert_eq!(f.core().automation_get(kind, id).unwrap()["enabled"], true);
    }
    let raw: String = f
        .db()
        .query_row(
            "SELECT payload_json FROM owner_automation_approvals LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!raw.contains(SOURCE));
    assert!(!raw.contains("\"config\""));
    assert!(raw.contains("activationProof"));
    let revoked = f
        .core()
        .owner_approval_submit(f.revoke(AutomationKind::Event, "rule"))
        .unwrap();
    metadata_only(&revoked);
    assert!(revoked.pending.is_none());
    assert_eq!(revoked.recent.len(), 2);
    let r = &revoked.recent[0];
    assert_eq!(r.intent.operation, OwnerApprovalOperation::Revoke);
    assert_eq!(r.outcome, OwnerApprovalOutcome::Succeeded);
    let receipt = r.receipt.as_ref().unwrap();
    assert_eq!(receipt.revision, "3");
    assert!(receipt.policy_version.is_none());
    assert!(receipt.manifest_digest.is_none());
    assert_eq!(f.commands(), 2);
    assert_eq!(
        f.core().owner_approval_resolve().unwrap().recent,
        revoked.recent
    );
}

#[test]
fn native_submission_rejects_unreviewed_shape_ack_and_host_before_minting_intent() {
    let mut f = Fixture::new();
    f.graph();
    let input = f.activate(AutomationKind::Event, "rule");
    let base = serde_json::to_value(&input).unwrap();
    for (key, value) in [
        ("commandId", json!("caller-owned-id")),
        ("source", json!(SOURCE)),
        ("grant", json!({})),
        ("policy", json!({})),
    ] {
        let mut v = base.clone();
        v[key] = value;
        assert!(serde_json::from_value::<OwnerApprovalSubmit>(v).is_err());
    }
    for (key, value, code) in [
        ("acknowledgeEffects", json!(false), "confirmation_required"),
        ("expectedRevision", json!("01"), "invalid_params"),
        ("id", json!(" rule"), "invalid_params"),
        ("manifestDigest", json!("f".repeat(63)), "invalid_params"),
        ("reviewedVersions", json!([]), "invalid_params"),
        ("enablePlan", json!([]), "invalid_params"),
        ("hostId", json!("foreign"), "host_restarted"),
    ] {
        let mut v = base.clone();
        v[key] = value;
        assert_eq!(
            f.core()
                .owner_approval_submit(serde_json::from_value(v).unwrap())
                .unwrap_err()
                .code,
            code,
            "{key}"
        );
    }
    let mut v = base.clone();
    v["enablePlan"][0]["enabled"] = false.into();
    assert_eq!(
        f.core()
            .owner_approval_submit(serde_json::from_value(v).unwrap())
            .unwrap_err()
            .code,
        "invalid_params"
    );
    f.reopen();
    assert_eq!(
        f.core().owner_approval_submit(input).unwrap_err().code,
        "host_restarted"
    );
    assert_eq!(f.count("owner_automation_approvals"), 0);
    assert_eq!(f.commands(), 0);
}

#[test]
fn stale_review_is_recorded_as_rejected_but_ambiguous_storage_errors_keep_pending() {
    let f = Fixture::new();
    f.graph();
    let input = f.activate(AutomationKind::Event, "rule");
    let mut a = f
        .core()
        .automation_get(AutomationKind::Action, "action")
        .unwrap();
    a["name"] = "Changed after review".into();
    f.core()
        .automation_update(AutomationKind::Action, "action", a)
        .unwrap();
    let state = f.core().owner_approval_submit(input).unwrap();
    assert!(state.pending.is_none());
    assert_eq!(state.recent[0].outcome, OwnerApprovalOutcome::Rejected);
    assert_eq!(
        state.recent[0].error_code.as_deref(),
        Some("revision_conflict")
    );
    assert_eq!(f.commands(), 0);
    assert!(!approval_rejection("overloaded"));
    assert!(!approval_rejection("internal"));
    assert!(!approval_rejection("unavailable"));
    assert!(!approval_rejection("command_id_reused"));
    f.db().execute_batch("CREATE TRIGGER fail_native_approval BEFORE UPDATE ON owner_automation_approvals BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert_eq!(
        f.core()
            .owner_approval_submit(f.activate(AutomationKind::Event, "rule"))
            .unwrap_err()
            .code,
        "approval_journal_unavailable"
    );
    let s = f.core().owner_approval_state().unwrap();
    assert!(s.pending.is_some());
    assert_eq!(f.commands(), 1);
    f.db()
        .execute_batch("DROP TRIGGER fail_native_approval")
        .unwrap();
    let resolved = f.core().owner_approval_resolve().unwrap();
    assert!(resolved.pending.is_none());
    assert_eq!(resolved.recent[0].outcome, OwnerApprovalOutcome::Succeeded);
    assert_eq!(f.commands(), 1);
}

#[test]
fn activation_and_revocation_share_pending_admission_with_native_adoption() {
    let f = Fixture::new();
    f.graph();
    let (pending, _) = f.pending(f.activate(AutomationKind::Event, "rule"));
    assert_eq!(
        f.core()
            .owner_approval_submit(f.activate(AutomationKind::Event, "rule"))
            .unwrap_err()
            .code,
        "command_pending"
    );
    assert_eq!(
        f.core()
            .owner_adoption_submit(f.adoption())
            .unwrap_err()
            .code,
        "command_pending"
    );
    assert_eq!(
        f.core()
            .owner_approval_review_unknown(OwnerApprovalReviewUnknown {
                command_id: "old-stale-id".into(),
                acknowledge_unknown: true
            })
            .unwrap_err()
            .code,
        "stale_view"
    );
    let s = f
        .core()
        .owner_approval_review_unknown(OwnerApprovalReviewUnknown {
            command_id: pending.command_id,
            acknowledge_unknown: true,
        })
        .unwrap();
    assert_eq!(s.recent[0].outcome, OwnerApprovalOutcome::ReviewedUnknown);
    assert_eq!(f.commands(), 0);
    let input = f.adoption();
    let i = crate::control::OwnerAdoptionIntent {
        command_id: format!("owner-adopt-{:032x}", fastrand::u128(..)),
        kind: input.kind,
        id: input.id.clone(),
        expected_revision: input.expected_revision.clone(),
        host_instance_id: input.host_instance_id.clone(),
        created_at: approval_now(),
    };
    f.core()
        .db
        .owner_adoption_begin(
            &input.host_id,
            &i,
            &AutomationMutation::Adopt {
                kind: input.kind,
                id: input.id,
                reviewed_versions: Some(input.reviewed_versions),
            },
        )
        .unwrap();
    assert_eq!(
        f.core()
            .owner_approval_submit(f.activate(AutomationKind::Event, "rule"))
            .unwrap_err()
            .code,
        "command_pending"
    );
    let adopted = f
        .core()
        .owner_adoption_review_unknown(crate::control::OwnerAdoptionReview {
            command_id: i.command_id,
            acknowledge_unknown: true,
        })
        .unwrap();
    assert_eq!(
        adopted.recent[0].outcome,
        OwnerAdoptionOutcome::ReviewedUnknown
    );
    assert_eq!(f.commands(), 0);
}

#[test]
fn journal_insert_failure_prevents_business_dispatch_and_leaves_no_intent() {
    let f = Fixture::new();
    f.graph();
    f.db().execute_batch("CREATE TRIGGER fail_native_begin BEFORE INSERT ON owner_automation_approvals BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
    assert_eq!(
        f.core()
            .owner_approval_submit(f.activate(AutomationKind::Event, "rule"))
            .unwrap_err()
            .code,
        "approval_journal_unavailable"
    );
    assert_eq!(f.commands(), 0);
    assert_eq!(f.count("owner_automation_approvals"), 0);
    assert_eq!(f.revision(AutomationKind::Event, "rule"), "1");
    assert_eq!(
        f.core()
            .automation_get(AutomationKind::Event, "rule")
            .unwrap()["enabled"],
        false
    );
    assert!(f.core().owner_approval_state().unwrap().pending.is_none());
}

#[test]
fn concurrent_owner_clients_get_distinct_native_ids_and_only_one_reviewed_effect() {
    let f = Fixture::new();
    f.graph();
    let input = f.activate(AutomationKind::Event, "rule");
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let core = f.core().clone();
            let input = input.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                core.owner_approval_submit(input).unwrap()
            })
        })
        .collect();
    barrier.wait();
    for worker in workers {
        worker.join().unwrap();
    }
    let state = f.core().owner_approval_state().unwrap();
    assert!(state.pending.is_none());
    assert_eq!(state.recent.len(), 2);
    assert_ne!(
        state.recent[0].intent.command_id,
        state.recent[1].intent.command_id
    );
    assert_eq!(
        state
            .recent
            .iter()
            .filter(|r| r.outcome == OwnerApprovalOutcome::Succeeded)
            .count(),
        1
    );
    assert_eq!(
        state
            .recent
            .iter()
            .filter(|r| r.outcome == OwnerApprovalOutcome::Rejected)
            .count(),
        1
    );
    assert_eq!(f.commands(), 1);
    assert_eq!(f.revision(AutomationKind::Event, "rule"), "2");
}

#[test]
fn restart_resolves_original_receipt_after_revoke_delete_and_author_expiry_without_reinstalling() {
    let mut f = Fixture::new();
    f.graph();
    let (intent, operation) = f.pending(f.activate(AutomationKind::Event, "rule"));
    apply(f.core(), &intent, operation);
    let s = f.core().collaboration().unwrap();
    let auth = f.author.snapshot().unwrap();
    s.revoke_session(&auth.session_id, "local-owner").unwrap();
    drop(s);
    f.core()
        .automation_delete(AutomationKind::Event, "rule")
        .unwrap();
    let version = f.revision(AutomationKind::Event, "rule");
    let commands = f.commands();
    f.reopen();
    assert_ne!(
        intent.host_instance_id,
        f.core().collaboration().unwrap().instance_id()
    );
    let recovered = f
        .core()
        .owner_approval_review_unknown(OwnerApprovalReviewUnknown {
            command_id: intent.command_id.clone(),
            acknowledge_unknown: true,
        })
        .unwrap();
    assert!(recovered.pending.is_none());
    assert_eq!(recovered.recent[0].intent, intent);
    assert_eq!(recovered.recent[0].outcome, OwnerApprovalOutcome::Succeeded);
    assert_eq!(recovered.recent[0].receipt.as_ref().unwrap().revision, "2");
    assert_eq!(f.revision(AutomationKind::Event, "rule"), version);
    assert_eq!(f.commands(), commands);
    assert!(f
        .core()
        .automation_get_versioned(AutomationKind::Event, "rule")
        .unwrap()
        .value
        .is_none());
}

#[test]
fn expired_original_outcomes_are_unknown_until_explicit_review_and_never_resent() {
    let f = Fixture::new();
    f.graph();
    let (intent, operation) = f.pending(f.activate(AutomationKind::Event, "rule"));
    apply(f.core(), &intent, operation);
    f.db().execute("UPDATE control_commands SET outcome_expired=1,result_json=NULL,versions_json=NULL WHERE command_id=?1",[&intent.command_id]).unwrap();
    let before = f.revision(AutomationKind::Event, "rule");
    assert_eq!(
        f.core().owner_approval_resolve().unwrap().pending,
        Some(intent.clone())
    );
    let result = f
        .core()
        .owner_approval_review_unknown(OwnerApprovalReviewUnknown {
            command_id: intent.command_id,
            acknowledge_unknown: true,
        })
        .unwrap();
    assert!(result.pending.is_none());
    assert_eq!(
        result.recent[0].outcome,
        OwnerApprovalOutcome::ReviewedUnknown
    );
    assert_eq!(f.commands(), 1);
    assert_eq!(f.revision(AutomationKind::Event, "rule"), before);
}

#[test]
fn corrupted_journal_and_mismatched_ledger_never_clear_pending_or_dispatch() {
    for corruption in ["proof", "oversized", "hash", "instance", "versions"] {
        let f = Fixture::new();
        f.graph();
        let (intent, operation) = f.pending(f.activate(AutomationKind::Event, "rule"));
        if matches!(corruption, "hash" | "instance" | "versions") {
            apply(f.core(), &intent, operation);
        }
        let db = f.db();
        match corruption {
            "proof" => {
                let raw: String = db
                    .query_row(
                        "SELECT payload_json FROM owner_automation_approvals LIMIT 1",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                let mut value: Value = serde_json::from_str(&raw).unwrap();
                value["activationProof"]["enablePlan"][0]["enabled"] = false.into();
                db.execute(
                    "UPDATE owner_automation_approvals SET payload_json=?1",
                    [value.to_string()],
                )
                .unwrap();
            }
            "oversized" => {
                db.execute(
                    "UPDATE owner_automation_approvals SET payload_json=?1",
                    ["x".repeat(262145)],
                )
                .unwrap();
            }
            "hash" => {
                db.execute(
                    "UPDATE control_commands SET payload_hash=?1 WHERE command_id=?2",
                    params!["a".repeat(64), intent.command_id],
                )
                .unwrap();
            }
            "instance" => {
                db.execute(
                    "UPDATE control_commands SET host_instance_id='foreign' WHERE command_id=?1",
                    [&intent.command_id],
                )
                .unwrap();
            }
            "versions" => {
                db.execute(
                    "UPDATE control_commands SET versions_json='[]' WHERE command_id=?1",
                    [&intent.command_id],
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let commands = f.commands();
        let before = f.revision(AutomationKind::Event, "rule");
        assert_eq!(
            f.core().owner_approval_resolve().unwrap_err().code,
            "approval_journal_invalid",
            "{corruption}"
        );
        assert_eq!(
            f.core()
                .owner_approval_review_unknown(OwnerApprovalReviewUnknown {
                    command_id: intent.command_id,
                    acknowledge_unknown: true
                })
                .unwrap_err()
                .code,
            "approval_journal_invalid",
            "{corruption}"
        );
        assert_eq!(f.commands(), commands);
        assert_eq!(f.revision(AutomationKind::Event, "rule"), before);
    }
}

#[test]
fn journal_history_is_bounded_and_uses_durable_insertion_order_after_clock_rollback() {
    let f = Fixture::new();
    f.graph();
    let mut ids = std::collections::HashSet::new();
    for n in 0..20 {
        let input = if n % 2 == 0 {
            f.activate(AutomationKind::Action, "action")
        } else {
            f.revoke(AutomationKind::Action, "action")
        };
        let state = f.core().owner_approval_submit(input).unwrap();
        assert!(state.pending.is_none());
        assert!(state.recent.len() <= 16);
        let r = &state.recent[0];
        native_id(&r.intent.command_id);
        assert!(ids.insert(r.intent.command_id.clone()));
        metadata_only(&state);
        f.db()
            .execute(
                "UPDATE owner_automation_approvals SET updated_at=9007199254740991",
                [],
            )
            .unwrap();
    }
    assert_eq!(f.count("owner_automation_approvals"), 16);
    assert_eq!(f.commands(), 20);
    let state = f.core().owner_approval_state().unwrap();
    assert!(state
        .recent
        .iter()
        .all(|r| r.intent.created_at < 9_007_199_254_740_991
            && r.recorded_at < 9_007_199_254_740_991));
    assert_eq!(state.recent[0].receipt.as_ref().unwrap().revision, "21");
}

#[test]
#[ignore = "subprocess-only native owner approval coordinator crash fixture"]
fn owner_approval_crash_child() {
    let root =
        std::path::PathBuf::from(std::env::var_os("TIKTOOLS_OWNER_APPROVAL_CRASH_ROOT").unwrap());
    let core = open_core(&paths(&root));
    let kind = AutomationKind::Event;
    let id = "rule";
    let s = core.collaboration().unwrap();
    let input =
        if std::env::var("TIKTOOLS_OWNER_APPROVAL_CRASH_OPERATION").as_deref() == Ok("revoke") {
            OwnerApprovalSubmit::Revoke {
                kind,
                id: id.into(),
                expected_revision: core
                    .automation_get_versioned(kind, id)
                    .unwrap()
                    .resource_versions[0]
                    .revision
                    .clone(),
                host_id: s.host_id().into(),
                host_instance_id: s.instance_id().into(),
                acknowledge_revocation: true,
            }
        } else {
            let m = core.automation_execution_manifest(kind, id).unwrap();
            OwnerApprovalSubmit::Activate {
                kind,
                id: id.into(),
                expected_revision: m.resource_versions[0].revision.clone(),
                host_id: s.host_id().into(),
                host_instance_id: s.instance_id().into(),
                reviewed_versions: m.resource_versions,
                manifest_digest: m.value["manifestDigest"].as_str().unwrap().into(),
                enable_plan: vec![
                    ApprovalEnablement {
                        kind,
                        id: id.into(),
                        enabled: true,
                    },
                    ApprovalEnablement {
                        kind: AutomationKind::Action,
                        id: "action".into(),
                        enabled: true,
                    },
                ],
                acknowledge_effects: true,
            }
        };
    core.owner_approval_submit(input).unwrap();
    panic!("native approval failpoint did not terminate");
}

#[test]
fn real_process_death_around_native_intent_business_commit_and_result_never_replays() {
    for operation in ["activate", "revoke"] {
        for stage in [
            "before_intent",
            "after_intent",
            "before_dispatch",
            "before_business_commit",
            "after_business_commit",
            "after_effect",
            "after_record",
        ] {
            let mut f = Fixture::new();
            f.graph();
            if operation == "revoke" {
                f.core()
                    .owner_approval_submit(f.activate(AutomationKind::Event, "rule"))
                    .unwrap();
            }
            let before = f.revision(AutomationKind::Event, "rule");
            let commands = f.commands();
            drop(f.core.take());
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "control::owner_approval::tests::owner_approval_crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("TIKTOOLS_OWNER_APPROVAL_CRASH_ROOT", &f.paths.root)
                .env("TIKTOOLS_OWNER_APPROVAL_CRASH_OPERATION", operation)
                .env("TIKTOOLS_OWNER_APPROVAL_CRASH", stage)
                .output()
                .unwrap();
            assert_eq!(
                child.status.code(),
                Some(89),
                "{operation}/{stage}: {}",
                String::from_utf8_lossy(&child.stderr)
            );
            f.core = Some(open_core(&f.paths));
            let committed = matches!(
                stage,
                "after_business_commit" | "after_effect" | "after_record"
            );
            assert_eq!(
                f.commands(),
                commands + i64::from(committed),
                "{operation}/{stage}"
            );
            let expected = if committed {
                (before.parse::<u64>().unwrap() + 1).to_string()
            } else {
                before
            };
            assert_eq!(
                f.revision(AutomationKind::Event, "rule"),
                expected,
                "{operation}/{stage}"
            );
            let state = f.core().owner_approval_state().unwrap();
            metadata_only(&state);
            if stage == "before_intent" {
                assert!(state.pending.is_none());
                assert_eq!(state.recent.len(), usize::from(operation == "revoke"));
            } else if stage == "after_record" {
                assert!(state.pending.is_none());
                assert_eq!(state.recent[0].outcome, OwnerApprovalOutcome::Succeeded);
            } else {
                let pending = state.pending.unwrap();
                assert_ne!(pending.host_instance_id, state.host_instance_id);
                assert_eq!(
                    f.core().owner_approval_resolve().unwrap().pending.is_none(),
                    committed
                );
                if !committed {
                    let out = f
                        .core()
                        .owner_approval_review_unknown(OwnerApprovalReviewUnknown {
                            command_id: pending.command_id,
                            acknowledge_unknown: true,
                        })
                        .unwrap();
                    assert_eq!(out.recent[0].outcome, OwnerApprovalOutcome::ReviewedUnknown);
                }
            }
            assert_eq!(f.commands(), commands + i64::from(committed));
            assert_eq!(f.revision(AutomationKind::Event, "rule"), expected);
            let db = f.db();
            let status:Option<String>=db.query_row("SELECT state FROM behavior_execution_approvals WHERE scope='event' AND resource_id='rule'",[],|r|r.get(0)).ok();
            assert_eq!(
                status.as_deref(),
                match (operation, committed) {
                    ("activate", true) => Some("active"),
                    ("activate", false) => None,
                    ("revoke", true) => Some("revoked"),
                    ("revoke", false) => Some("active"),
                    _ => unreachable!(),
                },
                "{operation}/{stage}"
            );
        }
    }
}
