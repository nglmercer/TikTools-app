use super::*;

struct Fixture {
    service: AutomationService,
    snapshot: Value,
    versions: Vec<ResourceVersion>,
    authorities: RuntimeAuthorities,
    approval: crate::db::BehaviorApprovalSnapshot,
}
impl Fixture {
    fn new() -> Self {
        let snapshot = json!({"actions":[
            {"id":"a","enabled":true,"typeId":"core.log","config":{"message":"hello"}},
            {"id":"b","enabled":false,"typeId":"core.delay","config":{"ms":1}}
        ],"events":[{"id":"e","enabled":true,"trigger":"tiktok.chat","runMode":"random","actionIds":["a","b","a"]}]});
        let authorities: RuntimeAuthorities = [
            ("event", "e", crate::db::BehaviorAuthority::Owner),
            (
                "action",
                "a",
                crate::db::BehaviorAuthority::Delegated {
                    principal_id: "principal-a".into(),
                    session_id: "session-a".into(),
                },
            ),
            (
                "action",
                "b",
                crate::db::BehaviorAuthority::Delegated {
                    principal_id: "principal-b".into(),
                    session_id: "session-b".into(),
                },
            ),
        ]
        .into_iter()
        .map(|(scope, id, authority)| {
            (
                (scope.into(), id.into()),
                crate::db::BehaviorRuntimeAuthority {
                    authority,
                    generation: 9,
                    revision: 7,
                    deleted: false,
                }
                .into(),
            )
        })
        .collect();
        let versions: Vec<_> = [("event", "e"), ("action", "a"), ("action", "b")]
            .into_iter()
            .map(|(scope, id)| ResourceVersion {
                kind: ResourceKind::Automation,
                scope: scope.into(),
                id: id.into(),
                revision: "7".into(),
            })
            .collect();
        let review = Versioned::new(
            json!({"kind":"event","id":"e","record":snapshot["events"][0],"provenance":provenance(&authorities[&("event".into(),"e".into())]).unwrap(),"dependencies":[
                {"id":"a","record":snapshot["actions"][0],"provenance":provenance(&authorities[&("action".into(),"a".into())]).unwrap()},
                {"id":"b","record":snapshot["actions"][1],"provenance":provenance(&authorities[&("action".into(),"b".into())]).unwrap()}
            ]}),
            versions.clone(),
        );
        let bindings = manifest::definition_bindings(&review).unwrap();
        // A definition-only storage fixture exercises index eligibility. It
        // intentionally cannot become a permit without real current subjects
        // and the complete manifest/policy envelope at admission.
        let approval = crate::db::BehaviorApprovalSnapshot {
            scope: "event".into(),
            id: "e".into(),
            generation: "1".into(),
            policy_version: 1,
            manifest_digest: "a".repeat(64),
            binding: json!({"manifest":{"policyVersion":1,"manifestDigest":"a".repeat(64),"primary":bindings[0],"dependencies":&bindings[1..]},"bindings":{}}),
        };
        let fixture = Self {
            service: AutomationService::default(),
            snapshot,
            versions,
            authorities,
            approval,
        };
        fixture.install(vec![fixture.approval.clone()]);
        fixture
    }
    fn install(&self, approvals: Vec<crate::db::BehaviorApprovalSnapshot>) {
        self.service.replace_snapshot_trusted_with_approvals(
            &self.snapshot,
            &self.versions,
            self.authorities.clone(),
            approvals,
        );
    }
    fn event(&self) -> &Value {
        &self.snapshot["events"][0]
    }
    fn action(&self) -> &Value {
        &self.snapshot["actions"][0]
    }
    fn incoming() -> Value {
        json!({"type":"tiktok.chat","user":{"uniqueId":"viewer"},"data":{"comment":"public"},"private":"secret"})
    }
}

#[test]
fn approved_matching_binds_full_saved_source_including_disabled_random_dependencies() {
    let f = Fixture::new();
    let candidate = f
        .service
        .approval_candidate(f.action(), Some(f.event()))
        .unwrap();
    assert_eq!(candidate.definitions.len(), 3);
    assert_eq!(
        candidate.required_subjects().unwrap(),
        vec![
            ("principal-a".into(), "session-a".into()),
            ("principal-b".into(), "session-b".into())
        ]
    );
    let matched = f.service.manual_record(f.event()).unwrap();
    assert!(matched.generation.is_some());
    assert!(f.service.is_match_live(&matched));
    assert_eq!(matched.actions.len(), 1);
    let mut forged = f.event().clone();
    forged["actionIds"] = json!(["a"]);
    assert!(f.service.manual_record(&forged).is_none());
    assert!(f
        .service
        .approval_candidate(f.action(), Some(&forged))
        .is_none());
    let mut forged = f.action().clone();
    forged["config"]["message"] = json!("changed inline source");
    assert!(f
        .service
        .approval_candidate(&forged, Some(f.event()))
        .is_none());
    assert!(candidate.authorize(&[]).is_err());
}

#[test]
fn identical_reload_keeps_fences_but_approval_removal_cannot_be_reversed_by_stale_reload() {
    let mut f = Fixture::new();
    let matched = f
        .service
        .matching_records(&Fixture::incoming())
        .pop()
        .unwrap();
    f.install(vec![f.approval.clone()]);
    assert!(f.service.is_match_live(&matched));
    f.install(vec![]);
    assert!(!f.service.is_match_live(&matched));
    assert!(f.service.matching_records(&Fixture::incoming()).is_empty());
    f.install(vec![f.approval.clone()]);
    assert!(f.service.matching_records(&Fixture::incoming()).is_empty());
    f.approval.generation = "2".into();
    f.install(vec![f.approval.clone()]);
    assert_eq!(f.service.matching_records(&Fixture::incoming()).len(), 1);
    assert!(!f.service.is_match_live(&matched));
}

#[test]
fn copied_saved_state_in_another_runtime_cannot_reuse_a_captured_candidate() {
    let first = Fixture::new();
    let second = Fixture::new();
    let candidate = first
        .service
        .approval_candidate(first.action(), Some(first.event()))
        .unwrap();
    let first_index = recover_rwlock_read(&first.service.index, "automation index");
    let second_index = recover_rwlock_read(&second.service.index, "automation index");
    assert!(candidate_current(&first_index, &candidate));
    assert!(!candidate_current(&second_index, &candidate));
}

#[test]
fn any_full_closure_body_change_or_in_memory_mutation_invalidates_old_approval() {
    let mut f = Fixture::new();
    let matched = f
        .service
        .matching_records(&Fixture::incoming())
        .pop()
        .unwrap();
    // The disabled unselected branch is still part of the approval.
    f.snapshot["actions"][1]["config"]["ms"] = json!(2);
    f.install(vec![f.approval.clone()]);
    assert!(!f.service.is_match_live(&matched));
    assert!(f.service.matching_records(&Fixture::incoming()).is_empty());
    f.snapshot["actions"][1]["config"]["ms"] = json!(1);
    f.install(vec![f.approval.clone()]);
    assert!(f.service.matching_records(&Fixture::incoming()).is_empty());
    f.approval.generation = "2".into();
    f.install(vec![f.approval.clone()]);
    assert_eq!(f.service.matching_records(&Fixture::incoming()).len(), 1);
    f.service.upsert_action(f.action().clone());
    assert!(f.service.matching_records(&Fixture::incoming()).is_empty());
    f.install(vec![f.approval.clone()]);
    assert!(f.service.matching_records(&Fixture::incoming()).is_empty());
}

#[test]
fn borrowed_definition_fence_rejects_stale_body_generation_revision_and_authority() {
    let f = Fixture::new();
    let mut index = recover_rwlock_write(&f.service.index, "automation index");
    let fence = capture(&index, AutomationFamily::Action, "a").unwrap();
    let original_record = index.actions["a"].clone();
    let original_authorities = index.authorities.clone();
    assert!(fence_current(&index, &fence));
    for change in [
        "body",
        "generation",
        "missing_generation",
        "revision",
        "missing_revision",
        "consistent_new_revision",
        "deleted",
        "owner",
        "principal",
        "session",
        "provenance_generation",
        "sidecar_revision",
        "missing_authority",
        "missing_record",
    ] {
        let mut authority = crate::db::BehaviorRuntimeAuthority {
            authority: crate::db::BehaviorAuthority::Delegated {
                principal_id: "principal-a".into(),
                session_id: "session-a".into(),
            },
            generation: 9,
            revision: 7,
            deleted: false,
        };
        match change {
            "body" => {
                index.actions.get_mut("a").unwrap().definition["config"]["message"] =
                    json!("stale source")
            }
            "generation" => {
                index.actions.get_mut("a").unwrap().generation = Some(fence.generation + 1)
            }
            "missing_generation" => index.actions.get_mut("a").unwrap().generation = None,
            "revision" => index.actions.get_mut("a").unwrap().persisted_revision = Some("8".into()),
            "missing_revision" => index.actions.get_mut("a").unwrap().persisted_revision = None,
            "consistent_new_revision" => {
                index.actions.get_mut("a").unwrap().persisted_revision = Some("8".into());
                authority.revision = 8;
            }
            "deleted" => authority.deleted = true,
            "owner" => authority.authority = crate::db::BehaviorAuthority::Owner,
            "principal" => {
                authority.authority = crate::db::BehaviorAuthority::Delegated {
                    principal_id: "different-principal".into(),
                    session_id: "session-a".into(),
                }
            }
            "session" => {
                authority.authority = crate::db::BehaviorAuthority::Delegated {
                    principal_id: "principal-a".into(),
                    session_id: "different-session".into(),
                }
            }
            "provenance_generation" => authority.generation += 1,
            "sidecar_revision" => authority.revision += 1,
            "missing_authority" | "missing_record" => {}
            _ => unreachable!(),
        }
        index
            .authorities
            .as_mut()
            .unwrap()
            .insert(fence.authority_key.clone(), authority.into());
        if change == "missing_authority" {
            index
                .authorities
                .as_mut()
                .unwrap()
                .remove(&fence.authority_key);
        }
        if change == "missing_record" {
            index.actions.remove("a");
        }
        assert!(!fence_current(&index, &fence), "{change}");
        index.actions.insert("a".into(), original_record.clone());
        index.authorities = original_authorities.clone();
        assert!(fence_current(&index, &fence), "restored after {change}");
    }
    let mut wrong_key = fence.clone();
    wrong_key.authority_key = ("event".into(), "e".into());
    assert!(!fence_current(&index, &wrong_key));
    let mut wrong_family = fence;
    wrong_family.family = AutomationFamily::Event;
    assert!(!fence_current(&index, &wrong_family));
}

#[test]
fn borrowed_candidate_checks_keep_runtime_catalog_approval_and_full_closure_fences() {
    let f = Fixture::new();
    let candidate = f
        .service
        .approval_candidate(f.action(), Some(f.event()))
        .unwrap();
    let mut index = recover_rwlock_write(&f.service.index, "automation index");
    assert!(candidate_current(&index, &candidate));
    let identity = index.identity.clone();
    index.identity = std::sync::Arc::new(());
    assert!(!candidate_current(&index, &candidate));
    index.identity = identity;
    let catalog = index.catalog_epoch;
    index.catalog_epoch = None;
    assert!(!candidate_current(&index, &candidate));
    index.catalog_epoch = Some(candidate.catalog_epoch.saturating_add(1));
    assert!(!candidate_current(&index, &candidate));
    index.catalog_epoch = catalog;
    index.authority_available = false;
    assert!(!candidate_current(&index, &candidate));
    index.authority_available = true;
    let approval = index.approvals.remove(&candidate.approval_key).unwrap();
    assert!(!candidate_current(&index, &candidate));
    let mut new_approval = approval.clone();
    new_approval.generation = "2".into();
    index
        .approvals
        .insert(candidate.approval_key.clone(), new_approval);
    assert!(!candidate_current(&index, &candidate));
    index
        .approvals
        .insert(candidate.approval_key.clone(), approval);
    assert!(candidate_current(&index, &candidate));
    // The unselected, disabled branch remains part of every current check.
    index.actions.get_mut("b").unwrap().definition["config"]["ms"] = json!(2);
    assert!(!candidate_current(&index, &candidate));
}

#[test]
fn approved_filters_only_see_the_public_scalar_event_projection() {
    let mut f = Fixture::new();
    // Rebuild the bound digest to represent a trusted saved filter definition.
    f.snapshot["events"][0]["filters"] =
        json!([{ "path":"event.private","operator":"eq","value":"secret"}]);
    let mut index = recover_rwlock_write(&f.service.index, "automation index");
    index.events.get_mut("e").unwrap().definition = f.event().clone();
    let fence = capture(&index, AutomationFamily::Event, "e").unwrap();
    let mut review = Versioned::new(
        json!({"kind":"event","id":"e","record":f.event(),"provenance":provenance(&fence.authority).unwrap(),"dependencies":[
            {"id":"a","record":f.snapshot["actions"][0],"provenance":provenance(&f.authorities[&("action".into(),"a".into())]).unwrap()},
            {"id":"b","record":f.snapshot["actions"][1],"provenance":provenance(&f.authorities[&("action".into(),"b".into())]).unwrap()}
        ]}),
        f.versions.clone(),
    );
    let bindings = manifest::definition_bindings(&review).unwrap();
    f.approval.binding["manifest"]["primary"] = serde_json::to_value(&bindings[0]).unwrap();
    index
        .approvals
        .insert(("event".into(), "e".into()), f.approval.clone());
    drop(index);
    assert!(f
        .service
        .event_record_matches(f.event(), &Fixture::incoming()));
    assert!(f.service.matching_records(&Fixture::incoming()).is_empty());
    // Absence matches an empty-string filter. The private full event would
    // reject it; cooldown admission must re-evaluate the same projected view.
    f.snapshot["events"][0]["filters"][0]["value"] = json!("");
    review.value["record"] = f.event().clone();
    let bindings = manifest::definition_bindings(&review).unwrap();
    f.approval.binding["manifest"]["primary"] = serde_json::to_value(&bindings[0]).unwrap();
    let mut index = recover_rwlock_write(&f.service.index, "automation index");
    index.events.get_mut("e").unwrap().definition = f.event().clone();
    index
        .approvals
        .insert(("event".into(), "e".into()), f.approval.clone());
    drop(index);
    assert!(!f
        .service
        .event_record_matches(f.event(), &Fixture::incoming()));
    assert_eq!(f.service.matching_records(&Fixture::incoming()).len(), 1);
    assert!(f.service.claim_event(f.event(), &Fixture::incoming(), 100));
}
