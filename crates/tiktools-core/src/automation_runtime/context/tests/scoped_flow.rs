use super::*;
use crate::control::{
    ApprovalActivateParams, ApprovalEnablement, AutomationKind, AutomationMutation, CommandCaller,
};
use serde_json::Value;

async fn approved_parent(f: &Fixture, restrict_event: bool) -> ExecutionContext {
    let service = f.core().collaboration().unwrap();
    let mut grant = f.identity.grant.clone();
    grant
        .capabilities
        .extend([Capability::LiveRead, Capability::EventsEmit]);
    grant.effects.emit_topics.insert("custom.child".into());
    if restrict_event {
        grant.resources.insert(
            ResourceKind::Automation,
            ["action:parent".into(), "action:child".into()].into(),
        );
    }
    service
        .set_grant(GrantSet {
            principal_id: f.identity.principal_id.clone(),
            expected_revision: f.identity.grant_revision.clone(),
            grant,
        })
        .unwrap();
    let instance = service.instance_id().to_owned();
    f.core().automation_command(
        CommandCaller::Remote(f.authorization.clone()),
        CommandRequest { command_id: "scoped-parent-create".into(), host_instance_id: instance.clone(), expected_revision: "0".into() },
        AutomationMutation::Create { kind: AutomationKind::Action, record: json!({"id":"parent","name":"Parent","enabled":false,"typeId":"core.log","config":{"message":"approved parent"}}) },
    ).unwrap();
    let review = f
        .core()
        .automation_execution_manifest(AutomationKind::Action, "parent")
        .unwrap();
    f.core()
        .automation_approval_activate(
            CommandRequest {
                command_id: "scoped-parent-approve".into(),
                host_instance_id: instance,
                expected_revision: review.resource_versions[0].revision.clone(),
            },
            ApprovalActivateParams {
                kind: AutomationKind::Action,
                id: "parent".into(),
                reviewed_versions: review.resource_versions,
                manifest_digest: review.value["manifestDigest"].as_str().unwrap().into(),
                enable_plan: vec![ApprovalEnablement {
                    kind: AutomationKind::Action,
                    id: "parent".into(),
                    enabled: true,
                }],
                acknowledge_effects: true,
            },
        )
        .unwrap();
    let body = f
        .core()
        .automation_get(AutomationKind::Action, "parent")
        .unwrap();
    f.core()
        .action_execution_context(&body, None, &ExecutionContext::owner())
        .await
        .unwrap()
}

fn owner_child(f: &Fixture, filter: Value) {
    f.core().automation_create(AutomationKind::Action,
        json!({"id":"child","name":"Child","enabled":true,"typeId":"core.log","config":{"message":"child: {{event.data.comment}}"}})
    ).unwrap();
    f.core().automation_create(AutomationKind::Event,
        json!({"id":"child-rule","name":"Child rule","enabled":true,"trigger":"custom.child","filters":filter,"actionIds":["child"]})
    ).unwrap();
}
fn payload() -> Value {
    json!({"id":"caller-cannot-choose-private-identity","type":"custom.child","timestamp":1,"user":{"uniqueId":"alice","token":"hidden"},"data":{"comment":"public","secret":"hidden"},"authority":{"kind":"owner"},"globals":{"secret":"hidden"}})
}
fn emitted(context: &ExecutionContext) -> AutomationEvent {
    AutomationEvent::new(
        payload(),
        context
            .child()
            .unwrap()
            .effect_child_slot("emission", 0)
            .unwrap(),
    )
}

#[tokio::test]
async fn unverified_native_owner_event_runs_owner_rule_but_cannot_promote_delegation() {
    let f = Fixture::new();
    let approved = approved_parent(&f, false).await;
    owner_child(&f, json!([]));
    // Exercise a real saved rule with both an Owner and an approved delegated
    // dependency, not only the low-level intersection helper.
    f.core()
        .automation_create(
            AutomationKind::Event,
            json!({
                "id":"mixed-rule", "name":"Mixed rule", "enabled":true,
                "trigger":"custom.child", "actionIds":["child", "parent"]
            }),
        )
        .unwrap();
    let unverified = ExecutionContext::owner_without_delegation();
    assert!(unverified.is_owner());
    assert!(unverified.intersect(&approved).is_err());
    assert!(approved.intersect(&unverified).is_err());
    let child = unverified.child().unwrap();
    assert!(child.is_owner());
    assert!(child.intersect(&approved).is_err());
    assert!(child
        .intersect(&ExecutionContext::owner())
        .unwrap()
        .intersect(&approved)
        .is_err());

    // A previously approved delegated action cannot acquire authority from
    // the same input, including through an Owner-emitted nested event.
    let action = f
        .core()
        .automation_get(AutomationKind::Action, "parent")
        .unwrap();
    assert!(f
        .core()
        .action_execution_context(&action, None, &unverified)
        .await
        .is_err());
    let nested = f
        .core()
        .make_internal_automation_event(&payload(), "custom.nested", json!({}), &unverified)
        .unwrap();
    assert!(nested.context().intersect(&approved).is_err());

    // The local Owner rule still runs on a payload with no asserted creator.
    let mut input = payload();
    input.as_object_mut().unwrap().remove("authority");
    assert!(input.get("creator").is_none());
    f.core()
        .run_automation_event(AutomationEvent::owner_without_delegation(input))
        .await;
    let runs = f.core().automation_runs();
    assert!(
        runs.iter().any(|run| run["summary"] == "child: public"),
        "{runs:?}"
    );
    assert!(
        !runs.iter().any(|run| run["summary"] == "approved parent"),
        "{runs:?}"
    );
}

#[tokio::test]
async fn approved_emit_runs_owner_child_under_inherited_fences_without_owner_observers_or_memory() {
    let f = Fixture::new();
    let context = approved_parent(&f, false).await.mark_script_flow();
    owner_child(
        &f,
        json!([{"path":"data.comment","operator":"eq","value":"public"}]),
    );
    let mut presentation = f.core().events.subscribe_projection();
    let mut observer = f.core().events.subscribe_plugin_domain();
    assert!(f.core().execution_context_guard(&context).is_err());
    f.core()
        .publish_scoped_automation_event(emitted(&context))
        .await
        .unwrap();
    let runs = f.core().automation_runs();
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!(runs[0]["summary"], "child: public");
    assert_eq!(runs[0]["status"], "ok");
    // The bus drains reliable action outcomes before its lossy LIVE lane, so
    // publication order does not imply receive order across those two lanes.
    let expected = crate::services::automation_approval_policy::project_event(&payload());
    let mut saw_scoped_event = false;
    loop {
        match presentation.try_recv() {
            Ok(presented) => {
                assert!(!serde_json::to_string(&presented)
                    .unwrap()
                    .contains("hidden"));
                if let crate::events::DomainEvent::LiveEvent { event_type, event } = presented {
                    assert_eq!(event_type, "custom.child");
                    assert_eq!(event, expected);
                    assert!(event.get("authority").is_none());
                    saw_scoped_event = true;
                }
            }
            Err(crate::events::DomainTryRecvError::Empty) => break,
            Err(error) => panic!("scoped presentation lane failed: {error:?}"),
        }
    }
    assert!(saw_scoped_event);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(25), observer.recv_shared())
            .await
            .is_err()
    );
    assert!(f.core().automation_context().0.is_none());
    assert!(f.core().last_event_for("custom.child").is_none());
    let matched = f
        .core()
        .automation
        .matching_records_scoped(&payload())
        .pop()
        .unwrap();
    let child = f
        .core()
        .record_execution_context(&matched, &context)
        .await
        .unwrap();
    assert!(child.has_script_flow());
    assert!(child.child().unwrap().has_script_flow());
    let body = f
        .core()
        .automation_get(AutomationKind::Action, "child")
        .unwrap();
    assert!(child
        .with_current_subjects(&body, &f.core().automation, |_| Ok(()))
        .is_ok());
    let mut changed = body;
    changed["config"]["message"] = json!("new Owner source");
    f.core()
        .automation_update_versioned(AutomationKind::Action, "child", changed, None)
        .unwrap();
    assert!(f.core().automation_dispatch_guard(&child).is_err());
    f.core().shutdown().await;
}

#[tokio::test]
async fn scoped_matching_rejects_private_negative_filters_and_unauthorized_owner_rule_resources() {
    for restricted in [false, true] {
        let f = Fixture::new();
        let context = approved_parent(&f, restricted).await;
        owner_child(
            &f,
            if restricted {
                json!([])
            } else {
                json!([{"path":"data.secret","operator":"neq","value":"denied"}])
            },
        );
        f.core()
            .publish_scoped_automation_event(emitted(&context))
            .await
            .unwrap();
        assert!(
            f.core().automation_runs().is_empty(),
            "resource/private filter must not admit an Owner child"
        );
        f.core().shutdown().await;
    }
}

#[tokio::test]
async fn revoked_parent_cannot_publish_and_script_flow_never_clears_on_intersection() {
    let f = Fixture::new();
    let context = approved_parent(&f, false).await.mark_script_flow();
    assert!(context
        .intersect(&ExecutionContext::owner())
        .unwrap()
        .has_script_flow());
    assert!(ExecutionContext::owner()
        .intersect(&context)
        .unwrap()
        .has_script_flow());
    assert!(context
        .intersect(&ExecutionContext::denied())
        .unwrap()
        .has_script_flow());
    let service = f.core().collaboration().unwrap();
    service
        .revoke_session(&f.identity.session_id, "fixture")
        .unwrap();
    let mut presentation = f.core().events.subscribe_projection();
    assert!(f
        .core()
        .publish_scoped_automation_event(emitted(&context))
        .await
        .is_err());
    assert!(presentation.try_recv().is_err());
    assert!(f.core().automation_runs().is_empty());
    f.core().shutdown().await;
}

#[tokio::test]
async fn custom_trigger_approval_requires_scoped_input_and_conjoins_parent_and_child_permits() {
    let f = Fixture::new();
    let context = approved_parent(&f, false).await;
    let instance = f.core().collaboration().unwrap().instance_id().to_owned();
    for (kind, id, record) in [
        (
            AutomationKind::Action,
            "approved-child",
            json!({"id":"approved-child","name":"Approved child","enabled":false,"typeId":"core.log","config":{"message":"approved child"}}),
        ),
        (
            AutomationKind::Event,
            "approved-child-rule",
            json!({"id":"approved-child-rule","name":"Approved child rule","enabled":false,"trigger":"custom.child","actionIds":["approved-child"]}),
        ),
    ] {
        f.core()
            .automation_command(
                CommandCaller::Remote(f.authorization.clone()),
                CommandRequest {
                    command_id: format!("scoped-create-{id}"),
                    host_instance_id: instance.clone(),
                    expected_revision: "0".into(),
                },
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    let review = f
        .core()
        .automation_execution_manifest(AutomationKind::Event, "approved-child-rule")
        .unwrap();
    f.core()
        .automation_approval_activate(
            CommandRequest {
                command_id: "scoped-child-approve".into(),
                host_instance_id: instance,
                expected_revision: review.resource_versions[0].revision.clone(),
            },
            ApprovalActivateParams {
                kind: AutomationKind::Event,
                id: "approved-child-rule".into(),
                reviewed_versions: review.resource_versions,
                manifest_digest: review.value["manifestDigest"].as_str().unwrap().into(),
                acknowledge_effects: true,
                enable_plan: vec![
                    ApprovalEnablement {
                        kind: AutomationKind::Event,
                        id: "approved-child-rule".into(),
                        enabled: true,
                    },
                    ApprovalEnablement {
                        kind: AutomationKind::Action,
                        id: "approved-child".into(),
                        enabled: true,
                    },
                ],
            },
        )
        .unwrap();
    assert!(
        f.core().automation.matching_records(&payload()).is_empty(),
        "Owner/plugin/manual input cannot borrow a custom scoped approval"
    );
    f.core()
        .run_automation_event(AutomationEvent::owner(payload()))
        .await;
    assert!(f.core().automation_runs().is_empty());
    let matched = f
        .core()
        .automation
        .matching_records_scoped(&payload())
        .pop()
        .unwrap();
    let child = f
        .core()
        .record_execution_context(&matched, &context)
        .await
        .unwrap();
    let action = f
        .core()
        .automation_get(AutomationKind::Action, "approved-child")
        .unwrap();
    assert!(child.with_current_subjects(&action, &f.core().automation, |_| Ok(())).is_ok(), "a child permit covers its action while the unrelated inherited parent permit remains current");
    f.core()
        .publish_scoped_automation_event(emitted(&context))
        .await
        .unwrap();
    assert_eq!(f.core().automation_runs()[0]["summary"], "approved child");
    f.core().shutdown().await;
}

async fn child_with_cooldown(
    f: &Fixture,
    parent: &ExecutionContext,
) -> (crate::services::MatchedRecord, ExecutionContext) {
    owner_child(f, json!([]));
    let mut rule = f
        .core()
        .automation_get(AutomationKind::Event, "child-rule")
        .unwrap();
    rule["cooldownMs"] = json!(60_000);
    rule["cooldownScope"] = json!("global");
    f.core()
        .automation_update_versioned(AutomationKind::Event, "child-rule", rule, None)
        .unwrap();
    let record = f
        .core()
        .automation
        .matching_records_scoped(&payload())
        .pop()
        .unwrap();
    let context = f
        .core()
        .record_execution_context(&record, parent)
        .await
        .unwrap();
    (record, context)
}

#[tokio::test]
async fn revoked_after_context_resolution_cannot_consume_owner_child_cooldown() {
    let f = Fixture::new();
    let parent = approved_parent(&f, false).await;
    let (record, context) = child_with_cooldown(&f, &parent).await;
    // Pin the race boundary with synchronous revocation: the saved rule is
    // still live, but the previously resolved incoming subject is revoked.
    f.core()
        .collaboration()
        .unwrap()
        .revoke_session(&f.identity.session_id, "fixture")
        .unwrap();
    assert!(f.core().automation.is_match_live(&record));
    assert!(
        !f.core()
            .claim_matched_record(&record, &payload(), &context)
            .await
    );
    // A subsequent real Owner claim must still get its first admission. This
    // fails if the rejected scoped claim touched the global cooldown entry.
    assert!(f
        .core()
        .automation
        .claim_event(&record.definition, &payload(), crate::now_millis()));
    assert!(!f
        .core()
        .automation
        .claim_event(&record.definition, &payload(), crate::now_millis()));
    f.core().shutdown().await;
}

#[tokio::test]
async fn current_scoped_cooldown_claim_completes_under_the_held_subject_and_index_fence() {
    let f = Fixture::new();
    let parent = approved_parent(&f, false).await;
    let (record, context) = child_with_cooldown(&f, &parent).await;
    assert!(
        f.core()
            .claim_matched_record(&record, &payload(), &context)
            .await
    );
    assert!(
        !f.core()
            .claim_matched_record(&record, &payload(), &context)
            .await
    );
    assert!(!f
        .core()
        .automation
        .claim_event(&record.definition, &payload(), crate::now_millis()));
    f.core().shutdown().await;
}
