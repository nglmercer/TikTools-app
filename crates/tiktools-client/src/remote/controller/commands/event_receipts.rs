use super::*;
#[test]
fn proposal_receipts_require_a_closed_disabled_body_exact_family_and_next_revision() {
    for operation in [
        Operation::CreateEventDraft,
        Operation::UpdateEventDraft,
        Operation::CreateActionDraft,
        Operation::UpdateActionDraft,
    ] {
        let intent = Intent {
            command_id: "11111111-1111-4111-8111-111111111111".into(),
            operation,
            resource_id: "same".into(),
            host_instance_id: "22222222-2222-4222-8222-222222222222".into(),
            expected_revision: Some("1".into()),
            delta: None,
            currency_name: None,
            proposal_revision: Some("7".into()),
            created_at: 1,
        };
        let receipt = CommandReceipt {
            command_id: intent.command_id.clone(),
            state: CommandState::Succeeded,
            resource: Some(tiktools_collaboration::CommandResource {
                kind: ResourceKind::Automation,
                scope: operation.scope().into(),
                id: "same".into(),
            }),
            committed_at: Some(2),
        };
        let version = ResourceVersion {
            kind: ResourceKind::Automation,
            scope: operation.scope().into(),
            id: "same".into(),
            revision: "2".into(),
        };
        let body =
            json!({"kind":operation.scope(),"id":"same","enabled":false,"pendingReview":true});
        assert_eq!(
            validate_success(&intent, &receipt, std::slice::from_ref(&version), &body).unwrap(),
            "2"
        );
        for field in [
            "kind",
            "id",
            "enabled",
            "pendingReview",
            "source",
            "revision",
            "scope",
        ] {
            let mut b = body.clone();
            let mut v = version.clone();
            match field {
                "kind" => {
                    b["kind"] = json!(if operation.scope() == "event" {
                        "action"
                    } else {
                        "event"
                    })
                }
                "id" => b["id"] = json!("other"),
                "enabled" => b["enabled"] = json!(true),
                "pendingReview" => b["pendingReview"] = json!(false),
                "source" => b["source"] = json!("private"),
                "revision" => v.revision = "1".into(),
                "scope" => v.scope = "host".into(),
                _ => unreachable!(),
            }
            assert!(
                validate_success(&intent, &receipt, &[v], &b).is_err(),
                "{operation:?}/{field}"
            );
        }
    }
}
#[test]
fn event_receipts_require_the_exact_family_target_revision_and_closed_body() {
    let intent = Intent {
        command_id: "11111111-1111-4111-8111-111111111111".into(),
        operation: Operation::DisableEvent,
        resource_id: "same".into(),
        host_instance_id: "22222222-2222-4222-8222-222222222222".into(),
        expected_revision: Some("1".into()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        created_at: 1,
    };
    let receipt = CommandReceipt {
        command_id: "11111111-1111-4111-8111-111111111111".into(),
        state: CommandState::Succeeded,
        resource: Some(tiktools_collaboration::CommandResource {
            kind: ResourceKind::Automation,
            scope: "event".into(),
            id: "same".into(),
        }),
        committed_at: Some(2),
    };
    let version = ResourceVersion {
        kind: ResourceKind::Automation,
        scope: "event".into(),
        id: "same".into(),
        revision: "2".into(),
    };
    let body = json!({"id":"same","enabled":false});
    assert_eq!(
        validate_success(&intent, &receipt, std::slice::from_ref(&version), &body).unwrap(),
        "2"
    );
    for change in [
        "scope", "kind", "target", "revision", "payload", "enabled", "command",
    ] {
        let mut r = receipt.clone();
        let mut v = version.clone();
        let mut b = body.clone();
        match change {
            "scope" => {
                r.resource.as_mut().unwrap().scope = "action".into();
                v.scope = "action".into();
            }
            "kind" => {
                r.resource.as_mut().unwrap().kind = ResourceKind::Workflow;
                v.kind = ResourceKind::Workflow;
            }
            "target" => {
                r.resource.as_mut().unwrap().id = "other".into();
                v.id = "other".into();
                b["id"] = "other".into();
            }
            "revision" => v.revision = "9".into(),
            "payload" => b["source"] = "owner definition must not enter receipt".into(),
            "enabled" => b["enabled"] = true.into(),
            "command" => r.command_id = "another-command".into(),
            _ => unreachable!(),
        }
        assert!(validate_success(&intent, &r, &[v], &b).is_err(), "{change}");
    }
    let deleted = Intent {
        operation: Operation::DeleteEvent,
        ..intent
    };
    assert!(validate_success(
        &deleted,
        &receipt,
        std::slice::from_ref(&version),
        &json!({"id":"same","deleted":true})
    )
    .is_ok());
    let unchanged = ResourceVersion {
        revision: "1".into(),
        ..version
    };
    assert!(validate_success(
        &deleted,
        &receipt,
        &[unchanged],
        &json!({"id":"same","deleted":false})
    )
    .is_ok());
}

#[test]
fn activation_receipts_bind_family_id_and_next_revision() {
    for operation in [Operation::EnableEvent, Operation::EnableAction] {
        let intent = Intent {
            command_id: "11111111-1111-4111-8111-111111111111".into(),
            operation,
            resource_id: "same".into(),
            host_instance_id: "22222222-2222-4222-8222-222222222222".into(),
            expected_revision: Some("1".into()),
            delta: None,
            currency_name: None,
            proposal_revision: None,
            created_at: 1,
        };
        let receipt = CommandReceipt {
            command_id: intent.command_id.clone(),
            state: CommandState::Succeeded,
            resource: Some(tiktools_collaboration::CommandResource {
                kind: ResourceKind::Automation,
                scope: operation.scope().into(),
                id: "same".into(),
            }),
            committed_at: Some(2),
        };
        let version = ResourceVersion {
            kind: ResourceKind::Automation,
            scope: operation.scope().into(),
            id: "same".into(),
            revision: "2".into(),
        };
        let body = json!({"kind":operation.scope(),"id":"same","enabled":true});
        assert!(validate_success(&intent, &receipt, std::slice::from_ref(&version), &body).is_ok());
        for altered in [
            json!({"kind":"wrong","id":"same","enabled":true}),
            json!({"kind":operation.scope(),"id":"same","enabled":false}),
            json!({"kind":operation.scope(),"id":"same","enabled":true,"extra":"secret"}),
        ] {
            assert!(
                validate_success(&intent, &receipt, std::slice::from_ref(&version), &altered)
                    .is_err()
            );
        }
    }
}
