//! Host-side projection tests. The only constructors here represent trusted
//! persistence results; candidate JSON never supplies these identities.
use super::*;
use crate::collaboration::{AutomationFamily, ResourceKind, ResourceVersion};
use serde_json::json;

fn snapshot() -> Value {
    json!({"actions":[{"id":"same","enabled":true,"typeId":"core.log","authority":"owner","provenance":{"authority":{"kind":"owner"}}}],"events":[{"id":"same","enabled":true,"trigger":"tiktok.chat","actionIds":["same"]}],"plugins":[],"eventTypes":[]})
}
fn versions() -> Vec<ResourceVersion> {
    ["action", "event"]
        .into_iter()
        .map(|scope| ResourceVersion {
            kind: ResourceKind::Automation,
            scope: scope.into(),
            id: "same".into(),
            revision: "7".into(),
        })
        .collect()
}
fn saved(
    kind: RuntimeAuthorityKind,
    generation: u64,
    revision: &str,
    deleted: bool,
) -> RuntimeAuthority {
    RuntimeAuthority {
        kind,
        generation: Some(generation),
        revision: Some(revision.into()),
        deleted,
    }
}
fn owner() -> RuntimeAuthority {
    saved(RuntimeAuthorityKind::Owner, 11, "7", false)
}
fn delegated(deleted: bool) -> RuntimeAuthority {
    saved(
        RuntimeAuthorityKind::Delegated {
            principal_id: "trusted-principal".into(),
            session_id: "trusted-session".into(),
        },
        19,
        "7",
        deleted,
    )
}
fn authorities() -> RuntimeAuthorities {
    [
        (("action".into(), "same".into()), owner()),
        (("event".into(), "same".into()), owner()),
    ]
    .into()
}
fn input() -> Value {
    json!({"type":"tiktok.chat","user":{"uniqueId":"viewer"},"data":{}})
}

#[test]
fn trusted_projection_retains_exact_subject_generation_and_revision_without_json_authority() {
    let service = AutomationService::default();
    let snapshot = snapshot();
    let mut a = authorities();
    a.insert(("action".into(), "same".into()), delegated(false));
    service.replace_snapshot_trusted(&snapshot, &versions(), a);
    let captured = service
        .runtime_authority(AutomationFamily::Action, "same")
        .unwrap();
    assert!(!captured.is_owner());
    assert_eq!(
        captured.subject(),
        Some(("trusted-principal", "trusted-session"))
    );
    assert_eq!(captured.provenance_generation(), Some(19));
    assert_eq!(captured.resource_revision(), Some("7"));
    assert!(!captured.is_deleted());
    assert!(service.matching_records(&input()).is_empty());
    assert!(!service.owner_action_allowed(&snapshot["actions"][0], None));
    let event = service
        .runtime_authority(AutomationFamily::Event, "same")
        .unwrap();
    assert!(event.is_owner());
    assert_eq!(event.subject(), None);
    assert_eq!(event.provenance_generation(), Some(11));
    // An explicit in-memory mutation is not adoption, even when JSON claims
    // Owner and the new body has the same raw ID as the delegated record.
    service
        .upsert_action(json!({"id":"same","enabled":true,"typeId":"core.log","authority":"owner"}));
    assert_eq!(
        service.runtime_authority(AutomationFamily::Action, "same"),
        Some(captured)
    );
    assert!(service.matching_records(&input()).is_empty());
}

#[test]
fn retained_delegated_tombstones_keep_subject_and_generation_but_never_become_inline_owner() {
    let service = AutomationService::default();
    let mut a = RuntimeAuthorities::new();
    a.insert(("action".into(), "gone".into()), delegated(true));
    service.replace_snapshot_trusted(&json!({"actions":[],"events":[]}), &[], a);
    let captured = service
        .runtime_authority(AutomationFamily::Action, "gone")
        .unwrap();
    assert!(captured.is_deleted());
    assert_eq!(
        captured.subject(),
        Some(("trusted-principal", "trusted-session"))
    );
    assert_eq!(captured.resource_revision(), Some("7"));
    assert_eq!(captured.provenance_generation(), Some(19));
    assert!(!service.owner_action_allowed(&json!({"id":"gone","authority":"owner"}), None));
    assert!(service.owner_action_allowed(
        &json!({"id":"new-owner-inline","authority":"delegated"}),
        None
    ));
    assert!(service
        .runtime_authority(AutomationFamily::Action, "new-owner-inline")
        .is_none());
    service.upsert_action(json!({"id":"new-owner-inline","enabled":true,"typeId":"core.log"}));
    let inline = service
        .runtime_authority(AutomationFamily::Action, "new-owner-inline")
        .unwrap();
    assert!(inline.is_owner());
    assert_eq!(inline.provenance_generation(), None);
    assert_eq!(inline.resource_revision(), None);
}

#[test]
fn incomplete_or_inconsistent_trusted_installations_quarantine_the_entire_index() {
    for defect in [
        "missing",
        "revision",
        "generation",
        "deleted-live",
        "orphan-live",
        "malformed-subject",
        "version-duplicate",
    ] {
        let service = AutomationService::default();
        let mut a = authorities();
        let mut v = versions();
        service.replace_snapshot_trusted(&snapshot(), &v, a.clone());
        assert_eq!(service.matching_records(&input()).len(), 1);
        match defect {
            "missing" => {
                a.remove(&("action".into(), "same".into()));
            }
            "revision" => {
                a.get_mut(&("action".into(), "same".into()))
                    .unwrap()
                    .revision = Some("8".into())
            }
            "generation" => {
                a.get_mut(&("action".into(), "same".into()))
                    .unwrap()
                    .generation = Some(0)
            }
            "deleted-live" => {
                a.get_mut(&("action".into(), "same".into()))
                    .unwrap()
                    .deleted = true
            }
            "orphan-live" => {
                a.insert(("action".into(), "orphan".into()), owner());
            }
            "malformed-subject" => {
                let mut d = delegated(false);
                d.kind = RuntimeAuthorityKind::Delegated {
                    principal_id: " bad ".into(),
                    session_id: "session".into(),
                };
                a.insert(("action".into(), "same".into()), d);
            }
            "version-duplicate" => v.push(v[0].clone()),
            _ => unreachable!(),
        }
        service.replace_snapshot_trusted(&snapshot(), &v, a);
        assert!(
            service
                .runtime_authority(AutomationFamily::Action, "same")
                .is_none(),
            "{defect}"
        );
        assert!(service.matching_records(&input()).is_empty(), "{defect}");
        assert!(
            !service.owner_event_allowed(&json!({"id":"fresh-inline"})),
            "{defect}"
        );
    }
}

#[test]
fn authority_only_changes_cancel_queued_generations_even_when_body_and_resource_version_are_unchanged(
) {
    let service = AutomationService::default();
    let s = snapshot();
    let v = versions();
    let mut a = authorities();
    service.replace_snapshot_trusted(&s, &v, a.clone());
    let original = service.matching_records(&input()).pop().unwrap();
    service.replace_snapshot_trusted(&s, &v, a.clone());
    assert!(service.is_match_live(&original));
    a.get_mut(&("action".into(), "same".into()))
        .unwrap()
        .generation = Some(12);
    service.replace_snapshot_trusted(&s, &v, a.clone());
    assert!(!service.is_match_live(&original));
    let changed_action = service.matching_records(&input()).pop().unwrap();
    assert!(service.is_match_live(&changed_action));
    a.get_mut(&("event".into(), "same".into()))
        .unwrap()
        .generation = Some(13);
    service.replace_snapshot_trusted(&s, &v, a.clone());
    assert!(!service.is_match_live(&changed_action));
    let changed_event = service.matching_records(&input()).pop().unwrap();
    a.insert(("action".into(), "same".into()), delegated(false));
    service.replace_snapshot_trusted(&s, &v, a.clone());
    assert!(!service.is_match_live(&changed_event));
    assert!(service.matching_records(&input()).is_empty());
    a.insert(("action".into(), "same".into()), owner());
    service.replace_snapshot_trusted(&s, &v, a);
    assert!(!service.is_match_live(&original));
    assert!(!service.is_match_live(&changed_event));
    assert_eq!(service.matching_records(&input()).len(), 1);
}

#[test]
fn disabled_or_absent_dependencies_still_invalidate_the_event_authority_closure() {
    let service = AutomationService::default();
    let mut s = snapshot();
    s["actions"][0]["enabled"] = json!(false);
    s["events"][0]["runMode"] = json!("random");
    s["events"][0]["actionIds"] = json!(["same", "absent"]);
    let v = versions();
    let mut a = authorities();
    service.replace_snapshot_trusted(&s, &v, a.clone());
    let queued = service.matching_records(&input()).pop().unwrap();
    assert!(queued.actions.is_empty());
    a.get_mut(&("action".into(), "same".into()))
        .unwrap()
        .generation = Some(12);
    service.replace_snapshot_trusted(&s, &v, a.clone());
    assert!(!service.is_match_live(&queued));
    let queued = service.matching_records(&input()).pop().unwrap();
    a.insert(("action".into(), "absent".into()), delegated(true));
    service.replace_snapshot_trusted(&s, &v, a);
    assert!(!service.is_match_live(&queued));
    assert!(service.matching_records(&input()).is_empty());
    service.quarantine();
    assert!(service
        .runtime_authority(AutomationFamily::Action, "absent")
        .is_none());
}
