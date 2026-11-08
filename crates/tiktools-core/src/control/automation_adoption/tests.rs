use super::*;
use crate::{
    collaboration::*,
    control::{AutomationMutation, CommandCaller},
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::sync::Arc;

const SOURCE: &str = "private-proposal-source-marker";

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
    let service = core.collaboration().unwrap();
    let mut config = service.configuration().unwrap();
    config.enabled = true;
    config.advertised_origin = Some("http://127.0.0.1:8788".into());
    service.configure(config).unwrap();
    let invite = service
        .create_invitation(InviteCreate {
            grant: Grant {
                capabilities: [Capability::AutomationRead, Capability::AutomationEdit].into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let exchange = "a".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Proposal author".into(),
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
    service
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap()
}

struct Fixture {
    core: Option<Arc<AppCore>>,
    paths: AppPaths,
    author: Authorization,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-owner-adoption-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = paths(&root);
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
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
    fn delegated(&self, kind: AutomationKind, id: &str, action_ids: &[&str]) {
        self.core().automation_command(CommandCaller::Remote(self.author.clone()),CommandRequest {
            command_id:format!("draft-{}-{id}",match kind {AutomationKind::Action=>"action",AutomationKind::Event=>"event"}),
            host_instance_id:self.core().collaboration().unwrap().instance_id().into(), expected_revision:"0".into(),
        }, AutomationMutation::Create {kind,record:json!({"id":id,"name":"Delegated proposal","enabled":false,"source":SOURCE,"authority":{"kind":"owner"},"trigger":"tiktok.chat","actionIds":action_ids,"typeId":"core.log","config":{"message":SOURCE}})}).unwrap();
    }
    fn graph(&self) {
        self.delegated(AutomationKind::Action, "action", &[]);
        self.delegated(AutomationKind::Event, "rule", &["action"]);
    }
    fn input(&self, kind: AutomationKind, id: &str) -> OwnerAdoptionSubmit {
        let reviewed = self.core().automation_review(kind, id).unwrap();
        let service = self.core().collaboration().unwrap();
        OwnerAdoptionSubmit {
            kind,
            id: id.into(),
            expected_revision: reviewed.resource_versions[0].revision.clone(),
            host_id: service.host_id().into(),
            host_instance_id: service.instance_id().into(),
            reviewed_versions: reviewed.resource_versions,
            acknowledge_authority: true,
        }
    }
    fn authority(&self, kind: AutomationKind, id: &str) -> Value {
        self.core().automation_authority(kind, id).unwrap().value["provenance"].clone()
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
    fn local_commands(&self) -> i64 {
        self.db()
            .query_row(
                "SELECT count(*) FROM control_commands WHERE principal_id='local-owner'",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }
    fn seed_pending(
        &self,
        kind: AutomationKind,
        id: &str,
    ) -> (OwnerAdoptionIntent, AutomationMutation) {
        let input = self.input(kind, id);
        let intent = OwnerAdoptionIntent {
            command_id: format!("owner-adopt-{:032x}", fastrand::u128(..)),
            kind,
            id: id.into(),
            expected_revision: input.expected_revision,
            host_instance_id: input.host_instance_id,
            created_at: adoption_now(),
        };
        let operation = AutomationMutation::Adopt {
            kind,
            id: id.into(),
            reviewed_versions: Some(input.reviewed_versions),
        };
        self.core()
            .db
            .owner_adoption_begin(&input.host_id, &intent, &operation)
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

fn server_id(id: &str) {
    let suffix = id.strip_prefix("owner-adopt-").unwrap();
    assert_eq!(suffix.len(), 32);
    assert!(suffix
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
}
fn metadata_only(state: &OwnerAdoptionState) {
    let text = serde_json::to_string(state).unwrap();
    for private in [
        SOURCE,
        "payloadHash",
        "reviewedVersions",
        "provenance",
        "\"record\"",
        "\"source\"",
    ] {
        assert!(
            !text.contains(private),
            "journal response disclosed {private}"
        );
    }
}

#[test]
fn owner_coordinator_records_exact_receipts_disables_transfers_and_requires_separate_enablement() {
    let f = Fixture::new();
    f.graph();
    let mut ids = std::collections::HashSet::new();
    for (kind, id) in [
        (AutomationKind::Action, "action"),
        (AutomationKind::Event, "rule"),
    ] {
        f.core().automation_set_enabled(kind, id, true).unwrap();
        assert_eq!(f.authority(kind, id)["authority"]["kind"], "delegated");
        let input = f.input(kind, id);
        let expected = input.expected_revision.parse::<u64>().unwrap();
        let state = f.core().owner_adoption_submit(input).unwrap();
        assert!(state.pending.is_none());
        metadata_only(&state);
        let record = state
            .recent
            .iter()
            .find(|r| r.intent.kind == kind && r.intent.id == id)
            .unwrap();
        server_id(&record.intent.command_id);
        assert!(ids.insert(record.intent.command_id.clone()));
        assert_eq!(record.outcome, OwnerAdoptionOutcome::Succeeded);
        assert_eq!(
            record.revision.as_deref(),
            Some((expected + 1).to_string().as_str())
        );
        assert!(record.error_code.is_none());
        assert_eq!(f.core().automation_get(kind, id).unwrap()["enabled"], false);
        assert_eq!(f.authority(kind, id)["authority"]["kind"], "owner");
        let receipt = f
            .core()
            .db
            .command_outcome("local-owner", &record.intent.command_id)
            .unwrap()
            .unwrap();
        assert_eq!(receipt.method, "automation.adopt");
        assert_eq!(receipt.command.command_id, record.intent.command_id);
        assert_eq!(receipt.host_instance_id, record.intent.host_instance_id);
        assert_eq!(receipt.result, json!({"id":id,"adopted":true}));
        assert_eq!(receipt.resource_versions.len(), 1);
        assert_eq!(
            Some(receipt.resource_versions[0].revision.as_str()),
            record.revision.as_deref()
        );
        f.core().automation_set_enabled(kind, id, true).unwrap();
        assert_eq!(f.core().automation_get(kind, id).unwrap()["enabled"], true);
        let resolved = f.core().owner_adoption_resolve().unwrap();
        assert_eq!(
            resolved
                .recent
                .iter()
                .find(|r| r.intent.command_id == record.intent.command_id)
                .unwrap(),
            record
        );
        assert_eq!(f.revision(kind, id), (expected + 2).to_string());
    }
    let raw: String = f
        .db()
        .query_row(
            "SELECT payload_json FROM owner_automation_adoptions LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!raw.contains(SOURCE));
    assert!(!raw.contains("reviewedVersions"));
    assert_eq!(f.local_commands(), 2);
}

#[test]
fn owner_coordinator_records_dependency_rejection_without_authority_or_command_side_effects() {
    let f = Fixture::new();
    f.graph();
    let input = f.input(AutomationKind::Event, "rule");
    let authority = f.authority(AutomationKind::Event, "rule");
    let mut action = f
        .core()
        .automation_get(AutomationKind::Action, "action")
        .unwrap();
    action["name"] = "Changed after review".into();
    f.core()
        .automation_update(AutomationKind::Action, "action", action)
        .unwrap();
    let outbox = f.count("control_resource_outbox");
    let state = f.core().owner_adoption_submit(input).unwrap();
    assert!(state.pending.is_none());
    assert_eq!(state.recent.len(), 1);
    let record = &state.recent[0];
    server_id(&record.intent.command_id);
    assert_eq!(record.outcome, OwnerAdoptionOutcome::Rejected);
    assert_eq!(record.error_code.as_deref(), Some("revision_conflict"));
    assert!(record.revision.is_none());
    metadata_only(&state);
    assert_eq!(f.authority(AutomationKind::Event, "rule"), authority);
    assert_eq!(f.revision(AutomationKind::Event, "rule"), "1");
    assert_eq!(f.local_commands(), 0);
    assert_eq!(f.count("control_resource_outbox"), outbox);
    assert_eq!(
        f.core().owner_adoption_resolve().unwrap().recent,
        state.recent
    );
}

#[test]
fn owner_journal_bounds_history_and_uses_a_new_server_identity_for_every_explicit_submission() {
    let f = Fixture::new();
    f.delegated(AutomationKind::Action, "action", &[]);
    let mut ids = std::collections::HashSet::new();
    for _ in 0..20 {
        let state = f
            .core()
            .owner_adoption_submit(f.input(AutomationKind::Action, "action"))
            .unwrap();
        metadata_only(&state);
        assert!(state.pending.is_none());
        assert!(state.recent.len() <= 16);
        for record in &state.recent {
            server_id(&record.intent.command_id);
        }
        let current = state
            .recent
            .iter()
            .find(|r| !ids.contains(&r.intent.command_id))
            .unwrap();
        assert!(ids.insert(current.intent.command_id.clone()));
    }
    assert_eq!(ids.len(), 20);
    assert_eq!(f.core().owner_adoption_state().unwrap().recent.len(), 16);
    assert_eq!(f.count("owner_automation_adoptions"), 16);
    assert_eq!(f.local_commands(), 20);
    assert_eq!(
        f.revision(AutomationKind::Action, "action"),
        "2",
        "Owner-to-Owner review is a checked no-op"
    );
}

#[test]
fn owner_journal_keeps_the_latest_completed_intent_after_clock_rollback_without_rewriting_audit_times(
) {
    let f = Fixture::new();
    f.delegated(AutomationKind::Action, "action", &[]);
    let mut previous = Vec::<OwnerAdoptionRecord>::new();
    for _ in 0..16 {
        let state = f
            .core()
            .owner_adoption_submit(f.input(AutomationKind::Action, "action"))
            .unwrap();
        assert!(state.pending.is_none());
        let new = state
            .recent
            .iter()
            .find(|record| {
                previous
                    .iter()
                    .all(|old| old.intent.command_id != record.intent.command_id)
            })
            .unwrap();
        previous.push(new.clone());
    }
    const FUTURE: i64 = 9_007_199_254_740_991;
    assert_eq!(
        f.db()
            .execute(
                "UPDATE owner_automation_adoptions SET updated_at=?1",
                [FUTURE]
            )
            .unwrap(),
        16
    );

    let state = f
        .core()
        .owner_adoption_submit(f.input(AutomationKind::Action, "action"))
        .unwrap();
    assert!(state.pending.is_none());
    assert_eq!(state.recent.len(), 16);
    let latest = &state.recent[0];
    server_id(&latest.intent.command_id);
    assert!(previous
        .iter()
        .all(|old| old.intent.command_id != latest.intent.command_id));
    assert_eq!(latest.outcome, OwnerAdoptionOutcome::Succeeded);
    assert!(latest.intent.created_at < FUTURE);
    assert!(latest.recorded_at < FUTURE);
    let retained = previous[1..].iter().rev().cloned().collect::<Vec<_>>();
    assert_eq!(
        &state.recent[1..],
        retained.as_slice(),
        "prune the oldest completed intent using insertion order"
    );
    let c = f.db();
    assert_eq!(f.count("owner_automation_adoptions"), 16);
    assert_eq!(f.local_commands(), 17);
    assert_eq!(f.revision(AutomationKind::Action, "action"), "2");
    for record in &state.recent[1..] {
        let (payload, updated_at): (String, i64) = c
            .query_row(
                "SELECT payload_json,updated_at FROM owner_automation_adoptions WHERE command_id=?1",
                [&record.intent.command_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let stored: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(stored["record"], serde_json::to_value(record).unwrap());
        assert_eq!(
            updated_at, FUTURE,
            "retained audit metadata is never rebased"
        );
    }
    let updated_at: i64 = c
        .query_row(
            "SELECT updated_at FROM owner_automation_adoptions WHERE command_id=?1",
            [&latest.intent.command_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(updated_at, latest.recorded_at);
    assert_eq!(
        f.core().owner_adoption_state().unwrap().recent,
        state.recent
    );
}

#[test]
fn owner_coordinator_requires_acknowledgement_and_exact_current_host_before_persisting_an_intent() {
    let mut f = Fixture::new();
    f.graph();
    let input = f.input(AutomationKind::Event, "rule");
    let mut unacknowledged = input.clone();
    unacknowledged.acknowledge_authority = false;
    assert_eq!(
        f.core()
            .owner_adoption_submit(unacknowledged)
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    let mut foreign = input.clone();
    foreign.host_id = "another-host".into();
    assert_eq!(
        f.core().owner_adoption_submit(foreign).unwrap_err().code,
        "host_restarted"
    );
    f.reopen();
    assert_eq!(
        f.core().owner_adoption_submit(input).unwrap_err().code,
        "host_restarted"
    );
    assert!(f.core().owner_adoption_state().unwrap().pending.is_none());
    assert_eq!(f.count("owner_automation_adoptions"), 0);
    assert_eq!(f.local_commands(), 0);
    assert_eq!(
        f.authority(AutomationKind::Event, "rule")["authority"]["kind"],
        "delegated"
    );
}

#[test]
fn owner_journal_failure_before_begin_prevents_adoption_and_failure_after_effect_retains_pending() {
    for failure in ["begin", "complete"] {
        let f = Fixture::new();
        f.graph();
        let c = f.db();
        let sql = if failure == "begin" {
            "CREATE TRIGGER reject_owner_journal BEFORE INSERT ON owner_automation_adoptions BEGIN SELECT RAISE(ABORT,'injected'); END"
        } else {
            "CREATE TRIGGER reject_owner_journal BEFORE UPDATE ON owner_automation_adoptions BEGIN SELECT RAISE(ABORT,'injected'); END"
        };
        c.execute_batch(sql).unwrap();
        let outbox = f.count("control_resource_outbox");
        assert_eq!(
            f.core()
                .owner_adoption_submit(f.input(AutomationKind::Event, "rule"))
                .unwrap_err()
                .code,
            "adoption_journal_unavailable"
        );
        let state = f.core().owner_adoption_state().unwrap();
        metadata_only(&state);
        if failure == "begin" {
            assert!(state.pending.is_none());
            assert!(state.recent.is_empty());
            assert_eq!(
                f.authority(AutomationKind::Event, "rule")["authority"]["kind"],
                "delegated"
            );
            assert_eq!(f.local_commands(), 0);
            assert_eq!(f.count("control_resource_outbox"), outbox);
        } else {
            assert!(state.pending.is_some());
            assert!(state.recent.is_empty());
            assert_eq!(
                f.authority(AutomationKind::Event, "rule")["authority"]["kind"],
                "owner"
            );
            assert_eq!(
                f.core()
                    .automation_get(AutomationKind::Event, "rule")
                    .unwrap()["enabled"],
                false
            );
            assert_eq!(f.local_commands(), 1);
            assert_eq!(f.count("control_resource_outbox"), outbox + 1);
            assert_eq!(
                f.core()
                    .owner_adoption_submit(f.input(AutomationKind::Event, "rule"))
                    .unwrap_err()
                    .code,
                "command_pending"
            );
        }
        c.execute_batch("DROP TRIGGER reject_owner_journal;")
            .unwrap();
        let recovered = f.core().owner_adoption_resolve().unwrap();
        assert!(recovered.pending.is_none());
        if failure == "complete" {
            assert_eq!(recovered.recent[0].outcome, OwnerAdoptionOutcome::Succeeded);
        }
    }
}

#[test]
fn owner_unknown_resolution_never_dispatches_and_explicit_review_only_clears_the_pending_gate() {
    let mut f = Fixture::new();
    f.graph();
    let (intent, _) = f.seed_pending(AutomationKind::Event, "rule");
    let outbox = f.count("control_resource_outbox");
    f.reopen();
    let state = f.core().owner_adoption_state().unwrap();
    assert_eq!(state.pending, Some(intent.clone()));
    assert_ne!(state.host_instance_id, intent.host_instance_id);
    for _ in 0..2 {
        assert_eq!(
            f.core().owner_adoption_resolve().unwrap().pending,
            Some(intent.clone())
        );
        assert_eq!(f.local_commands(), 0);
        assert_eq!(
            f.authority(AutomationKind::Event, "rule")["authority"]["kind"],
            "delegated"
        );
    }
    assert_eq!(
        f.core()
            .owner_adoption_submit(f.input(AutomationKind::Event, "rule"))
            .unwrap_err()
            .code,
        "command_pending"
    );
    assert_eq!(
        f.core()
            .owner_adoption_review_unknown(OwnerAdoptionReview {
                command_id: intent.command_id.clone(),
                acknowledge_unknown: false
            })
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    assert_eq!(
        f.core()
            .owner_adoption_review_unknown(OwnerAdoptionReview {
                command_id: "different-command".into(),
                acknowledge_unknown: true
            })
            .unwrap_err()
            .code,
        "stale_view"
    );
    let reviewed = f
        .core()
        .owner_adoption_review_unknown(OwnerAdoptionReview {
            command_id: intent.command_id.clone(),
            acknowledge_unknown: true,
        })
        .unwrap();
    assert!(reviewed.pending.is_none());
    assert_eq!(reviewed.recent.len(), 1);
    assert_eq!(reviewed.recent[0].intent, intent);
    assert_eq!(
        reviewed.recent[0].outcome,
        OwnerAdoptionOutcome::ReviewedUnknown
    );
    assert!(reviewed.recent[0].revision.is_none());
    assert!(reviewed.recent[0].error_code.is_none());
    assert_eq!(f.local_commands(), 0);
    assert_eq!(f.count("control_resource_outbox"), outbox);
    assert_eq!(
        f.authority(AutomationKind::Event, "rule")["authority"]["kind"],
        "delegated"
    );
    let next = f
        .core()
        .owner_adoption_submit(f.input(AutomationKind::Event, "rule"))
        .unwrap();
    assert!(next.pending.is_none());
    assert_eq!(next.recent.len(), 2);
    assert!(next
        .recent
        .iter()
        .any(|r| r.outcome == OwnerAdoptionOutcome::Succeeded
            && r.intent.command_id != intent.command_id));
}

#[test]
fn owner_journal_rejects_cross_host_and_corrupt_private_bindings_without_dispatch() {
    for corruption in [
        "host",
        "hash",
        "intent",
        "command-prefix",
        "command-case",
        "json",
        "oversized",
        "oversized-table-id",
    ] {
        let mut f = Fixture::new();
        f.graph();
        let (intent, _) = f.seed_pending(AutomationKind::Event, "rule");
        let c = f.db();
        let raw: String = c
            .query_row(
                "SELECT payload_json FROM owner_automation_adoptions",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let mut stored: Value = serde_json::from_str(&raw).unwrap();
        let raw = match corruption {
            "host" => {
                stored["hostId"] = "foreign-host".into();
                stored.to_string()
            }
            "hash" => {
                stored["payloadHash"] = "invalid-hash".into();
                stored.to_string()
            }
            "intent" => {
                stored["intent"]["commandId"] = "foreign-intent".into();
                stored.to_string()
            }
            "command-prefix" => {
                stored["intent"]["commandId"] = "untrusted-caller-command".into();
                stored.to_string()
            }
            "command-case" => {
                stored["intent"]["commandId"] = format!("owner-adopt-{}", "A".repeat(32)).into();
                stored.to_string()
            }
            "json" => "malformed journal".into(),
            "oversized" => "x".repeat(8193),
            "oversized-table-id" => stored.to_string(),
            _ => unreachable!(),
        };
        c.execute(
            "UPDATE owner_automation_adoptions SET payload_json=?1 WHERE command_id=?2",
            params![raw, intent.command_id],
        )
        .unwrap();
        if matches!(corruption, "command-prefix" | "command-case") {
            c.execute(
                "UPDATE owner_automation_adoptions SET command_id=?1 WHERE command_id=?2",
                params![
                    stored["intent"]["commandId"].as_str().unwrap(),
                    intent.command_id
                ],
            )
            .unwrap();
        }
        if corruption == "oversized-table-id" {
            c.execute(
                "UPDATE owner_automation_adoptions SET command_id=?1 WHERE command_id=?2",
                params!["x".repeat(1024 * 1024), intent.command_id],
            )
            .unwrap();
        }
        drop(c);
        f.reopen();
        assert_eq!(
            f.core().owner_adoption_state().unwrap_err().code,
            "adoption_journal_invalid",
            "{corruption}"
        );
        assert_eq!(
            f.core().owner_adoption_resolve().unwrap_err().code,
            "adoption_journal_invalid",
            "{corruption}"
        );
        assert_eq!(
            f.core()
                .owner_adoption_submit(f.input(AutomationKind::Event, "rule"))
                .unwrap_err()
                .code,
            "adoption_journal_invalid",
            "{corruption}"
        );
        let manifest = f
            .core()
            .automation_execution_manifest(AutomationKind::Event, "rule")
            .unwrap();
        let service = f.core().collaboration().unwrap();
        assert_eq!(
            f.core()
                .owner_approval_submit(crate::control::OwnerApprovalSubmit::Activate {
                    kind: AutomationKind::Event,
                    id: "rule".into(),
                    expected_revision: manifest.resource_versions[0].revision.clone(),
                    host_id: service.host_id().into(),
                    host_instance_id: service.instance_id().into(),
                    reviewed_versions: manifest.resource_versions,
                    manifest_digest: manifest.value["manifestDigest"].as_str().unwrap().into(),
                    enable_plan: vec![crate::control::ApprovalEnablement {
                        kind: AutomationKind::Event,
                        id: "rule".into(),
                        enabled: true
                    }],
                    acknowledge_effects: true,
                })
                .unwrap_err()
                .code,
            "approval_journal_invalid",
            "{corruption}"
        );
        assert_eq!(f.local_commands(), 0);
        assert_eq!(
            f.authority(AutomationKind::Event, "rule")["authority"]["kind"],
            "delegated"
        );
    }
}

#[test]
fn owner_resolution_rejects_a_ledger_outcome_with_a_different_semantic_or_host_binding() {
    for corruption in ["hash", "instance"] {
        let f = Fixture::new();
        f.graph();
        let (intent, operation) = f.seed_pending(AutomationKind::Event, "rule");
        let operation = if corruption == "hash" {
            AutomationMutation::Adopt {
                kind: intent.kind,
                id: intent.id.clone(),
                reviewed_versions: None,
            }
        } else {
            operation
        };
        f.core()
            .automation_command(
                CommandCaller::LocalOwner,
                CommandRequest {
                    command_id: intent.command_id.clone(),
                    host_instance_id: intent.host_instance_id.clone(),
                    expected_revision: intent.expected_revision.clone(),
                },
                operation,
            )
            .unwrap();
        if corruption == "instance" {
            f.db().execute("UPDATE control_commands SET host_instance_id='foreign-host-instance' WHERE principal_id='local-owner' AND command_id=?1",[&intent.command_id]).unwrap();
        }
        let before = f
            .core()
            .automation_get(AutomationKind::Event, "rule")
            .unwrap();
        let outbox = f.count("control_resource_outbox");
        assert_eq!(
            f.core().owner_adoption_resolve().unwrap_err().code,
            "adoption_journal_invalid",
            "{corruption}"
        );
        assert_eq!(
            f.core().owner_adoption_state().unwrap().pending,
            Some(intent.clone())
        );
        assert_eq!(
            f.core()
                .owner_adoption_review_unknown(OwnerAdoptionReview {
                    command_id: intent.command_id,
                    acknowledge_unknown: true
                })
                .unwrap_err()
                .code,
            "adoption_journal_invalid"
        );
        assert_eq!(
            f.core()
                .automation_get(AutomationKind::Event, "rule")
                .unwrap(),
            before
        );
        assert_eq!(f.count("control_resource_outbox"), outbox);
        assert_eq!(f.local_commands(), 1);
    }
}

#[test]
#[ignore = "subprocess-only owner adoption coordinator crash fixture"]
fn owner_adoption_crash_child() {
    let root =
        std::path::PathBuf::from(std::env::var_os("TIKTOOLS_OWNER_ADOPTION_CRASH_ROOT").unwrap());
    let core = open_core(&paths(&root));
    let review = core
        .automation_review(AutomationKind::Event, "rule")
        .unwrap();
    let service = core.collaboration().unwrap();
    core.owner_adoption_submit(OwnerAdoptionSubmit {
        kind: AutomationKind::Event,
        id: "rule".into(),
        expected_revision: review.resource_versions[0].revision.clone(),
        host_id: service.host_id().into(),
        host_instance_id: service.instance_id().into(),
        reviewed_versions: review.resource_versions,
        acknowledge_authority: true,
    })
    .unwrap();
    panic!("owner adoption failpoint did not terminate");
}

#[test]
fn owner_adoption_process_death_at_every_dispatch_boundary_preserves_intent_and_never_replays() {
    for stage in [
        "before_intent",
        "after_intent",
        "before_dispatch",
        "after_effect",
        "after_record",
    ] {
        let mut f = Fixture::new();
        f.graph();
        f.core()
            .automation_set_enabled(AutomationKind::Event, "rule", true)
            .unwrap();
        drop(f.core.take());
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "control::automation_adoption::tests::owner_adoption_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("TIKTOOLS_OWNER_ADOPTION_CRASH_ROOT", &f.paths.root)
            .env("TIKTOOLS_OWNER_ADOPTION_CRASH", stage)
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(88),
            "{stage}: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        f.core = Some(open_core(&f.paths));
        let state = f.core().owner_adoption_state().unwrap();
        metadata_only(&state);
        let committed = matches!(stage, "after_effect" | "after_record");
        assert_eq!(f.local_commands(), i64::from(committed), "{stage}");
        assert_eq!(
            f.authority(AutomationKind::Event, "rule")["authority"]["kind"],
            if committed { "owner" } else { "delegated" },
            "{stage}"
        );
        assert_eq!(
            f.revision(AutomationKind::Event, "rule"),
            if committed { "3" } else { "2" },
            "{stage}"
        );
        assert_eq!(
            f.core()
                .automation_get(AutomationKind::Event, "rule")
                .unwrap()["enabled"],
            !committed,
            "{stage}"
        );
        match stage {
            "before_intent" => {
                assert!(state.pending.is_none());
                assert!(state.recent.is_empty());
            }
            "after_intent" | "before_dispatch" => {
                let pending = state.pending.unwrap();
                server_id(&pending.command_id);
                assert!(state.recent.is_empty());
                assert_ne!(pending.host_instance_id, state.host_instance_id);
                let outbox = f.count("control_resource_outbox");
                let mut events = f.core().events.subscribe_domain();
                assert_eq!(
                    f.core().owner_adoption_resolve().unwrap().pending,
                    Some(pending.clone())
                );
                assert_eq!(
                    f.core()
                        .owner_adoption_submit(f.input(AutomationKind::Event, "rule"))
                        .unwrap_err()
                        .code,
                    "command_pending"
                );
                let reviewed = f
                    .core()
                    .owner_adoption_review_unknown(OwnerAdoptionReview {
                        command_id: pending.command_id,
                        acknowledge_unknown: true,
                    })
                    .unwrap();
                assert!(reviewed.pending.is_none());
                assert_eq!(
                    reviewed.recent[0].outcome,
                    OwnerAdoptionOutcome::ReviewedUnknown
                );
                assert_eq!(f.local_commands(), 0);
                assert_eq!(f.count("control_resource_outbox"), outbox);
                assert_eq!(
                    f.authority(AutomationKind::Event, "rule")["authority"]["kind"],
                    "delegated"
                );
                assert!(matches!(
                    events.try_recv(),
                    Err(crate::events::DomainTryRecvError::Empty)
                ));
            }
            "after_effect" => {
                let pending = state.pending.unwrap();
                server_id(&pending.command_id);
                assert!(state.recent.is_empty());
                assert_ne!(pending.host_instance_id, state.host_instance_id);
                f.core()
                    .automation_set_enabled(AutomationKind::Event, "rule", true)
                    .unwrap();
                let mut changed = f
                    .core()
                    .automation_get(AutomationKind::Event, "rule")
                    .unwrap();
                changed["name"] = "Later owner edit".into();
                f.core()
                    .automation_update(AutomationKind::Event, "rule", changed.clone())
                    .unwrap();
                let outbox = f.count("control_resource_outbox");
                let mut events = f.core().events.subscribe_domain();
                // A known exact outcome wins over a stale 'unknown' assertion.
                let recovered = f
                    .core()
                    .owner_adoption_review_unknown(OwnerAdoptionReview {
                        command_id: pending.command_id.clone(),
                        acknowledge_unknown: true,
                    })
                    .unwrap();
                assert!(recovered.pending.is_none());
                assert_eq!(recovered.recent[0].intent, pending);
                assert_eq!(recovered.recent[0].outcome, OwnerAdoptionOutcome::Succeeded);
                assert_eq!(recovered.recent[0].revision.as_deref(), Some("3"));
                assert_eq!(f.revision(AutomationKind::Event, "rule"), "5");
                assert_eq!(
                    f.core()
                        .automation_get(AutomationKind::Event, "rule")
                        .unwrap(),
                    changed
                );
                assert_eq!(f.count("control_resource_outbox"), outbox);
                assert_eq!(f.local_commands(), 1);
                assert!(matches!(
                    events.try_recv(),
                    Err(crate::events::DomainTryRecvError::Empty)
                ));
            }
            "after_record" => {
                assert!(state.pending.is_none());
                assert_eq!(state.recent[0].outcome, OwnerAdoptionOutcome::Succeeded);
                assert_eq!(state.recent[0].revision.as_deref(), Some("3"));
            }
            _ => unreachable!(),
        }
        let before = f
            .core()
            .automation_get(AutomationKind::Event, "rule")
            .unwrap();
        let outbox = f.count("control_resource_outbox");
        f.core().owner_adoption_resolve().unwrap();
        assert_eq!(
            f.core()
                .automation_get(AutomationKind::Event, "rule")
                .unwrap(),
            before
        );
        assert_eq!(f.count("control_resource_outbox"), outbox);
    }
}
