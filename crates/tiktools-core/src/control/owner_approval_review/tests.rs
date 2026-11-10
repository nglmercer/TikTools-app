//! Real Owner approval commands and runtime sinks over isolated business and
//! collaboration databases. A preview or editable body never grants execution.
use super::*;
use crate::{
    collaboration::*,
    control::{ApprovalActivateParams, ApprovalEnablement, ApprovalRevokeParams},
    live_events::AutomationEvent,
};
use serde_json::json;

use crate::control::approval_test_support::*;

fn enable(kind: AutomationKind, id: &str) -> ApprovalEnablement {
    ApprovalEnablement {
        kind,
        id: id.into(),
        enabled: true,
    }
}
fn log(id: &str, message: &str) -> Value {
    json!({"id":id,"name":"Log","enabled":false,"typeId":"core.log","config":{"message":message}})
}
fn points(id: &str, viewer: &str, delta: f64) -> Value {
    json!({"id":id,"name":"Points","enabled":false,"typeId":"core.points","config":{"uniqueId":viewer,"delta":delta}})
}
fn event(id: &str, actions: &[&str]) -> Value {
    json!({"id":id,"name":"Live chat","enabled":false,"trigger":"tiktok.chat","runMode":"random","actionIds":actions})
}

#[tokio::test]
async fn review_captures_saved_closure_current_grants_and_retained_approval_without_executing() {
    let mut f = Fixture::new();
    let mut permitted = grant();
    permitted
        .resources
        .insert(ResourceKind::Viewer, ["alice".into()].into());
    permitted.max_points_delta = Some(2.0);
    let auth = f.pair(permitted.clone());
    let identity = auth.snapshot().unwrap();
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "alice", 2.0),
    );
    f.draft(
        &auth,
        AutomationKind::Action,
        log("unused", "unused public literal"),
    );
    f.core()
        .automation_create(AutomationKind::Action, log("owner", "owner"))
        .unwrap();
    f.draft(
        &auth,
        AutomationKind::Event,
        event("event", &["points", "unused", "owner", "points"]),
    );
    let review = f
        .core()
        .owner_approval_review(AutomationKind::Event, "event")
        .unwrap();
    assert!(review.value.eligibility.eligible);
    assert!(review.value.eligibility.reason_code.is_none());
    assert_eq!(
        review.value.source,
        f.core()
            .automation_review(AutomationKind::Event, "event")
            .unwrap()
            .value
    );
    assert_eq!(
        review.value.source["dependencies"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        review.value.source["dependencies"][1]["record"]["enabled"],
        false
    );
    assert_eq!(review.value.subjects.len(), 1);
    assert_eq!(review.value.subjects[0].principal_id, identity.principal_id);
    assert_eq!(review.value.subjects[0].grant, permitted);
    assert_eq!(
        review.value.manifest.as_ref().unwrap()["subjects"][0]["configurationRevision"],
        review.value.subjects[0].configuration_revision
    );
    assert_eq!(
        review.host_instance_id.as_deref(),
        Some(review.value.host_instance_id.as_str())
    );
    assert_eq!(review.value.approval.state, OwnerApprovalStatusKind::None);
    assert!(!review.value.approval.current);
    assert_eq!(f.points("alice"), 0.0);
    let receipt = f.approve(
        AutomationKind::Event,
        "event",
        vec![
            enable(AutomationKind::Event, "event"),
            enable(AutomationKind::Action, "points"),
        ],
    );
    let current = f
        .core()
        .owner_approval_review(AutomationKind::Event, "event")
        .unwrap();
    assert_eq!(
        current.value.approval.state,
        OwnerApprovalStatusKind::Active
    );
    assert!(current.value.approval.current);
    assert_eq!(
        current.value.approval.manifest_digest.as_deref(),
        receipt.value["manifestDigest"].as_str()
    );
    assert_eq!(current.value.source["record"]["enabled"], true);
    assert_eq!(
        current.value.source["dependencies"][0]["record"]["enabled"],
        true
    );
    assert_eq!(
        current.value.source["dependencies"][1]["record"]["enabled"],
        false
    );
    f.assert_delegated(AutomationKind::Event, "event");
    f.core().run_automation_event(AutomationEvent::owner(json!({"id":"live","type":"tiktok.chat","user":{"uniqueId":"alice"},"data":{"comment":"hello"}}))).await;
    assert_eq!(f.points("alice"), 2.0);
    f.core()
        .automation_update(
            AutomationKind::Action,
            "unused",
            log("unused", "changed disabled dependency"),
        )
        .unwrap();
    let changed = f
        .core()
        .owner_approval_review(AutomationKind::Event, "event")
        .unwrap();
    assert_eq!(
        changed.value.approval.state,
        OwnerApprovalStatusKind::Invalidated
    );
    assert!(!changed.value.approval.current);
    assert_eq!(
        changed.value.source["dependencies"][1]["record"],
        f.body(AutomationKind::Action, "unused")
    );
    drop(auth);
    f.reopen();
    let reopened = f
        .core()
        .owner_approval_review(AutomationKind::Event, "event")
        .unwrap();
    assert_eq!(
        reopened.value.approval.state,
        OwnerApprovalStatusKind::Invalidated
    );
    assert!(!reopened.value.approval.current);
}

#[test]
fn public_review_digest_binds_host_configuration_and_stale_consent_cannot_activate() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    f.draft(&auth, AutomationKind::Action, log("log", "reviewed"));
    let before = f
        .core()
        .owner_approval_review(AutomationKind::Action, "log")
        .unwrap();
    let service = f.core().collaboration().unwrap();
    let mut config = service.configuration().unwrap();
    config.limits.request_burst -= 1;
    service.configure(config.clone()).unwrap();
    config.limits.request_burst += 1;
    service.configure(config).unwrap();
    let after = f
        .core()
        .owner_approval_review(AutomationKind::Action, "log")
        .unwrap();
    assert_eq!(before.resource_versions, after.resource_versions);
    let digest = before.value.manifest.as_ref().unwrap()["manifestDigest"]
        .as_str()
        .unwrap();
    assert_ne!(
        digest,
        after.value.manifest.as_ref().unwrap()["manifestDigest"]
            .as_str()
            .unwrap()
    );
    let error = f
        .core()
        .automation_approval_activate(
            f.request(&before.resource_versions[0].revision),
            ApprovalActivateParams {
                kind: AutomationKind::Action,
                id: "log".into(),
                reviewed_versions: before.resource_versions,
                manifest_digest: digest.into(),
                enable_plan: vec![enable(AutomationKind::Action, "log")],
                acknowledge_effects: true,
            },
        )
        .unwrap_err();
    assert_eq!(error.code, "invalid_params");
    assert_eq!(f.body(AutomationKind::Action, "log")["enabled"], false);
    assert_eq!(
        f.core()
            .owner_approval_review(AutomationKind::Action, "log")
            .unwrap()
            .value
            .approval
            .state,
        OwnerApprovalStatusKind::None
    );
}

#[test]
fn revoked_authors_missing_definitions_and_unsupported_effects_remain_reviewable_for_revocation() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    let identity = auth.snapshot().unwrap();
    f.draft(&auth, AutomationKind::Action, log("log", "reviewed"));
    f.draft(&auth, AutomationKind::Action, json!({"id":"http","name":"HTTP","enabled":false,"typeId":"core.fetch","config":{"url":"https://example.test"}}));
    let unsupported = f
        .core()
        .owner_approval_review(AutomationKind::Action, "http")
        .unwrap();
    assert_eq!(
        unsupported.value.eligibility.reason_code,
        Some(OwnerApprovalBlockReason::PolicyUnavailable)
    );
    assert!(unsupported.value.manifest.is_some());
    f.approve(
        AutomationKind::Action,
        "log",
        vec![enable(AutomationKind::Action, "log")],
    );
    f.core()
        .collaboration()
        .unwrap()
        .revoke_session(&identity.session_id, "local-owner")
        .unwrap();
    let unavailable = f
        .core()
        .owner_approval_review(AutomationKind::Action, "log")
        .unwrap();
    assert_eq!(
        unavailable.value.eligibility.reason_code,
        Some(OwnerApprovalBlockReason::SubjectsUnavailable)
    );
    assert!(unavailable.value.manifest.is_none());
    assert!(unavailable.value.subjects.is_empty());
    assert_eq!(
        unavailable.value.approval.state,
        OwnerApprovalStatusKind::Active
    );
    assert!(!unavailable.value.approval.current);
    f.core()
        .automation_delete(AutomationKind::Action, "log")
        .unwrap();
    let missing = f
        .core()
        .owner_approval_review(AutomationKind::Action, "log")
        .unwrap();
    assert_eq!(
        missing.value.eligibility.reason_code,
        Some(OwnerApprovalBlockReason::DefinitionMissing)
    );
    assert!(missing.value.source["record"].is_null());
    assert_eq!(
        missing.value.approval.state,
        OwnerApprovalStatusKind::Invalidated
    );
    f.core()
        .automation_approval_revoke(
            f.request(&missing.resource_versions[0].revision),
            ApprovalRevokeParams {
                kind: AutomationKind::Action,
                id: "log".into(),
                acknowledge_revocation: true,
            },
        )
        .unwrap();
    let revoked = f
        .core()
        .owner_approval_review(AutomationKind::Action, "log")
        .unwrap();
    assert_eq!(
        revoked.value.approval.state,
        OwnerApprovalStatusKind::Revoked
    );
    f.core()
        .automation_create(AutomationKind::Action, log("owner", "ordinary owner"))
        .unwrap();
    let owner = f
        .core()
        .owner_approval_review(AutomationKind::Action, "owner")
        .unwrap();
    assert_eq!(
        owner.value.eligibility.reason_code,
        Some(OwnerApprovalBlockReason::NoDelegatedSubjects)
    );
    assert!(owner.value.manifest.is_some());
    assert!(owner.value.subjects.is_empty());
}

#[test]
fn aggregate_owner_review_budget_includes_current_grants_as_well_as_source() {
    let f = Fixture::new();
    let mut permitted = grant();
    for group in 0..64 {
        permitted.settings_fields.insert(
            format!("fixture-{group}"),
            (0..24)
                .map(|field| format!("{field:03}-{}", "a".repeat(196)))
                .collect(),
        );
    }
    let auth = f.pair(permitted);
    let ids: Vec<_> = (0..16).map(|index| format!("action-{index}")).collect();
    for id in &ids {
        f.draft(&auth, AutomationKind::Action, log(id, &"x".repeat(247_000)));
    }
    f.draft(
        &auth,
        AutomationKind::Event,
        event("event", &ids.iter().map(String::as_str).collect::<Vec<_>>()),
    );
    // Each component is legal, but a source+grant response must not silently
    // exceed the one bounded local review/frame budget.
    assert!(f
        .core()
        .automation_review(AutomationKind::Event, "event")
        .is_ok());
    assert_eq!(
        f.core()
            .owner_approval_review(AutomationKind::Event, "event")
            .unwrap_err()
            .code,
        "too_large"
    );
}
#[test]
fn active_review_rejects_missing_malformed_or_extraneous_reverse_dependencies() {
    for corruption in ["missing", "position", "oversized-id", "extra", "digest"] {
        let f = Fixture::new();
        let auth = f.pair(grant());
        f.draft(
            &auth,
            AutomationKind::Action,
            log("action", "reviewed literal"),
        );
        f.draft(&auth, AutomationKind::Event, event("event", &["action"]));
        f.approve(
            AutomationKind::Event,
            "event",
            vec![
                enable(AutomationKind::Event, "event"),
                enable(AutomationKind::Action, "action"),
            ],
        );
        assert!(
            f.core()
                .owner_approval_review(AutomationKind::Event, "event")
                .unwrap()
                .value
                .approval
                .current
        );
        let db = rusqlite::Connection::open(f.paths.automation_database()).unwrap();
        match corruption {
            "missing" => {
                db.execute("DELETE FROM behavior_execution_approval_dependencies WHERE approval_scope='event' AND approval_id='event' AND position=1",[]).unwrap();
            }
            "position" => {
                db.execute("UPDATE behavior_execution_approval_dependencies SET position=2 WHERE approval_scope='event' AND approval_id='event' AND position=1",[]).unwrap();
            }
            "oversized-id" => {
                db.execute("UPDATE behavior_execution_approval_dependencies SET definition_id=?1 WHERE approval_scope='event' AND approval_id='event' AND position=1",["x".repeat(129)]).unwrap();
            }
            "extra" => {
                db.execute("INSERT INTO behavior_execution_approval_dependencies(approval_scope,approval_id,position,definition_scope,definition_id,revision,provenance_generation,present,content_digest) VALUES ('event','event',2,'action','extra',1,1,1,?1)",["a".repeat(64)]).unwrap();
            }
            "digest" => {
                db.execute("UPDATE behavior_execution_approval_dependencies SET content_digest=?1 WHERE approval_scope='event' AND approval_id='event' AND position=1",["a".repeat(64)]).unwrap();
            }
            _ => unreachable!(),
        }
        assert_eq!(
            f.core()
                .owner_approval_review(AutomationKind::Event, "event")
                .unwrap_err()
                .code,
            "internal",
            "{corruption}"
        );
        assert!(
            f.core()
                .db
                .load_behavior_runtime_approval_snapshot()
                .is_err(),
            "{corruption}"
        );
    }
}
