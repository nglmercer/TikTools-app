//! Real HTTPS/WSS draft-authoring coverage. The host is isolated and never
//! connects to a live or executes the submitted definitions.
use super::{tests::Fixture, ClientKind, RemoteClient};
use crate::{CallResult, ClientError, RequestMetadata, TikToolsClient};
use serde_json::{json, Value};
use std::time::Duration;
use tiktools_collaboration::{Capability, CommandRequest, Grant, GrantSet, ResourceKind};
use tiktools_control_api::modules::automation::{AutomationGetParams, AutomationManifestParams};
use tiktools_core::control::{AutomationKind, AutomationMutation, CommandCaller};

fn grant(ids: &[&str]) -> Grant {
    Grant {
        capabilities: [
            Capability::AutomationRead,
            Capability::AutomationEdit,
            Capability::AutomationToggle,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            ResourceKind::Automation,
            ids.iter().map(|id| (*id).into()).collect(),
        )]
        .into(),
        ..Grant::default()
    }
}

fn metadata(f: &Fixture, command: &str, revision: &str) -> RequestMetadata {
    RequestMetadata {
        command_id: Some(command.into()),
        host_instance_id: Some(f.service.instance_id().into()),
        expected_revision: Some(revision.into()),
    }
}

fn draft(kind: &str, id: &str) -> Value {
    json!({
        "id":id,
        "name":format!("{kind} draft"),
        "enabled":false,
        "typeId":"core.log",
        "config":{"message":"sdk-private-config-marker"},
        "trigger":"tiktok.chat",
        "actionIds":["draft"],
        "source":"sdk-private-source-marker",
        "bodyScript":"sdk-private-script-marker",
        "authority":{"kind":"owner"},
        "provenance":{"authority":{"kind":"owner"},"generation":"999"}
    })
}

fn pending(kind: &str) -> Value {
    json!({"kind":kind,"id":"draft","enabled":false,"pendingReview":true})
}

fn assert_no_source(value: &Value) {
    let encoded = value.to_string();
    for marker in [
        "sdk-private-config-marker",
        "sdk-private-source-marker",
        "sdk-private-script-marker",
        "sdk-hidden-owner-marker",
    ] {
        assert!(!encoded.contains(marker), "definition escaped: {encoded}");
    }
}

fn assert_pending(response: &CallResult<Value>, kind: &str, revision: &str) {
    assert_eq!(response.result, pending(kind));
    let meta = response.meta.as_ref().unwrap();
    assert_eq!(meta.resource_versions.len(), 1);
    let resource = &meta.resource_versions[0];
    assert_eq!(resource.kind, ResourceKind::Automation);
    assert_eq!(resource.scope, kind);
    assert_eq!(resource.id, "draft");
    assert_eq!(resource.revision, revision);
    let receipt_resource = meta.command.as_ref().unwrap().resource.as_ref().unwrap();
    assert_eq!(receipt_resource.kind, ResourceKind::Automation);
    assert_eq!(receipt_resource.scope, kind);
    assert_eq!(receipt_resource.id, "draft");
    assert_no_source(&response.result);
    assert_no_source(&serde_json::to_value(meta).unwrap());
}

fn assert_closed(value: &Value, kind: &str, enabled: bool) {
    assert_eq!(value.as_object().unwrap().len(), 5);
    assert_eq!(value["kind"], kind);
    assert_eq!(value["id"], "draft");
    assert_eq!(value["enabled"], enabled);
    assert_eq!(value["definitionRedacted"], true);
    assert!(value["name"].is_string());
    assert_no_source(value);
}

fn assert_denied(error: ClientError) {
    assert_eq!(error.code, "forbidden", "{error:?}");
    assert!(error
        .data
        .as_ref()
        .is_none_or(|data| data.get("resourceVersions").is_none()));
    assert_no_source(&json!({"message":error.message,"data":error.data}));
}

#[tokio::test]
async fn tls_drafts_keep_authority_private_and_recover_historical_receipts_after_owner_adoption() {
    let f = Fixture::new().await;
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        f.core
            .automation_create(
                kind,
                json!({"id":"owned","name":"Owner record","enabled":true,"source":"sdk-hidden-owner-marker"}),
            )
            .unwrap();
    }
    f.core
        .automation_create(
            AutomationKind::Action,
            json!({"id":"hidden","name":"sdk-hidden-owner-marker","enabled":true}),
        )
        .unwrap();
    let full_grant = grant(&["action:draft", "event:draft", "action:owned", "event:owned"]);
    let credentials = f.pair(full_grant.clone()).await;
    let other_credentials = f.pair(full_grant).await;
    let remote = RemoteClient::connect(&f.endpoint, &credentials, ClientKind::Agent)
        .await
        .unwrap();
    let other = RemoteClient::connect(&f.endpoint, &other_credentials, ClientKind::Agent)
        .await
        .unwrap();
    let sdk = TikToolsClient::Remote(remote.clone());
    let other_sdk = TikToolsClient::Remote(other.clone());

    // The same raw ID belongs to two independently scoped resource families.
    for (kind, parsed) in [
        ("action", AutomationKind::Action),
        ("event", AutomationKind::Event),
    ] {
        let created = sdk
            .call_with_meta::<_, Value>(
                "automation.create",
                json!({"kind":kind,"record":draft(kind,"draft")}),
                Some(metadata(&f, &format!("create-{kind}"), "0")),
            )
            .await
            .unwrap();
        assert_pending(&created, kind, "1");
        let authority = f.core.automation_authority(parsed, "draft").unwrap();
        assert_eq!(
            authority.value["provenance"]["authority"]["kind"],
            "delegated"
        );
        assert_eq!(
            authority.value["provenance"]["authority"]["principalId"],
            credentials.principal_id()
        );
        assert_eq!(
            authority.value["provenance"]["authority"]["sessionId"],
            credentials.session_id()
        );
        assert_ne!(authority.value["provenance"]["generation"], "999");
        let read = sdk
            .call_value("automation.get", json!({"kind":kind,"id":"draft"}))
            .await
            .unwrap();
        assert_closed(&read, kind, false);
        let outcome = sdk
            .call_value(
                "collaboration.commands.get",
                json!({"commandId":format!("create-{kind}")}),
            )
            .await
            .unwrap();
        assert_eq!(outcome["result"], pending(kind));
        assert_no_source(&outcome);
        assert_denied(
            other_sdk
                .call_with_meta::<_, Value>(
                    "automation.update",
                    json!({"kind":kind,"id":"draft","record":draft(kind,"draft")}),
                    Some(metadata(&f, &format!("steal-{kind}"), "1")),
                )
                .await
                .unwrap_err(),
        );
        for method in ["automation.create", "automation.update"] {
            let params = if method == "automation.create" {
                json!({"kind":kind,"record":draft(kind,"owned")})
            } else {
                json!({"kind":kind,"id":"owned","record":draft(kind,"owned")})
            };
            assert_denied(
                sdk.call_with_meta::<_, Value>(
                    method,
                    params,
                    Some(metadata(
                        &f,
                        &format!("{}-{kind}-owned", method.replace('.', "-")),
                        "1",
                    )),
                )
                .await
                .unwrap_err(),
            );
        }
        assert_denied(
            sdk.call_value(
                "automation.authority.get",
                json!({"kind":kind,"id":"draft"}),
            )
            .await
            .unwrap_err(),
        );
        assert_denied(
            sdk.automation_review(AutomationGetParams {
                kind: Some(kind.into()),
                id: "draft".into(),
            })
            .await
            .unwrap_err(),
        );
        assert_denied(
            sdk.automation_execution_manifest_get(AutomationManifestParams {
                kind: kind.parse().unwrap(),
                id: "draft".into(),
            })
            .await
            .unwrap_err(),
        );
        assert_denied(
            sdk.call_with_meta::<_, Value>(
                "automation.adopt",
                json!({"kind":kind,"id":"draft","acknowledgeAuthority":true}),
                Some(metadata(&f, &format!("remote-adopt-{kind}"), "1")),
            )
            .await
            .unwrap_err(),
        );
    }
    let list = sdk.call_value("automation.list", json!({})).await.unwrap();
    assert_no_source(&list);
    assert_denied(
        sdk.call_value("automation.snapshot", json!({}))
            .await
            .unwrap_err(),
    );

    let mut updated_action = draft("action", "draft");
    updated_action["name"] = "Updated action draft".into();
    let action_update = sdk
        .call_with_meta::<_, Value>(
            "automation.update",
            json!({"kind":"action","id":"draft","record":updated_action}),
            Some(metadata(&f, "update-action", "1")),
        )
        .await
        .unwrap();
    assert_pending(&action_update, "action", "2");

    let mut updated_event = draft("event", "draft");
    updated_event["name"] = "Updated event draft".into();
    let update_params = json!({"kind":"event","id":"draft","record":updated_event});
    let update_meta = metadata(&f, "update-event-lost-ack", "1");
    let in_flight = {
        let sdk = sdk.clone();
        let params = update_params.clone();
        let meta = update_meta.clone();
        tokio::spawn(async move {
            sdk.call_with_meta::<_, Value>("automation.update", params, Some(meta))
                .await
        })
    };
    // Observe durable commit independently of the caller's response. Deliberately
    // discard any racing acknowledgement and close the original TLS connection.
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let current = f
                .core
                .automation_get_versioned(AutomationKind::Event, "draft")
                .unwrap();
            if current.resource_versions[0].revision == "2" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    remote.close();
    drop(sdk);
    let _discarded_ack = in_flight.await.unwrap();
    let reopened = RemoteClient::connect(&f.endpoint, &credentials, ClientKind::Agent)
        .await
        .unwrap();
    let sdk = TikToolsClient::Remote(reopened.clone());
    let recovered = sdk
        .call_value(
            "collaboration.commands.get",
            json!({"commandId":"update-event-lost-ack"}),
        )
        .await
        .unwrap();
    assert_eq!(recovered["result"], pending("event"));
    assert_no_source(&recovered);

    // Adoption changes trusted authority and disables the record atomically.
    // Only a separate owner enable may activate that reviewed definition.
    let adopted = f
        .core
        .automation_command(
            CommandCaller::LocalOwner,
            CommandRequest {
                command_id: "owner-adopt-event".into(),
                host_instance_id: f.service.instance_id().into(),
                expected_revision: "2".into(),
            },
            AutomationMutation::Adopt {
                kind: AutomationKind::Event,
                id: "draft".into(),
                reviewed_versions: None,
            },
        )
        .unwrap();
    assert_eq!(adopted.resource_versions[0].revision, "3");
    assert_eq!(
        f.core
            .automation_authority(AutomationKind::Event, "draft")
            .unwrap()
            .value["provenance"]["authority"]["kind"],
        "owner"
    );
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Event, "draft")
            .unwrap()["enabled"],
        false
    );
    f.core
        .automation_set_enabled(AutomationKind::Event, "draft", true)
        .unwrap();
    let owner_version = f
        .core
        .automation_get_versioned(AutomationKind::Event, "draft")
        .unwrap();
    assert_eq!(owner_version.resource_versions[0].revision, "4");
    assert_closed(
        &sdk.call_value("automation.get", json!({"kind":"event","id":"draft"}))
            .await
            .unwrap(),
        "event",
        true,
    );
    let replay = sdk
        .call_with_meta::<_, Value>("automation.update", update_params, Some(update_meta))
        .await
        .unwrap();
    assert_pending(&replay, "event", "2");
    let replay_create = sdk
        .call_with_meta::<_, Value>(
            "automation.create",
            json!({"kind":"event","record":draft("event","draft")}),
            Some(metadata(&f, "create-event", "0")),
        )
        .await
        .unwrap();
    assert_pending(&replay_create, "event", "1");
    let after_replay = f
        .core
        .automation_get_versioned(AutomationKind::Event, "draft")
        .unwrap();
    assert_eq!(after_replay.value, owner_version.value);
    assert_eq!(
        after_replay.resource_versions,
        owner_version.resource_versions
    );
    assert_denied(
        sdk.call_with_meta::<_, Value>(
            "automation.update",
            json!({"kind":"event","id":"draft","record":draft("event","draft")}),
            Some(metadata(&f, "fresh-update-after-adoption", "4")),
        )
        .await
        .unwrap_err(),
    );
    let historical = sdk
        .call_value(
            "collaboration.commands.get",
            json!({"commandId":"update-event-lost-ack"}),
        )
        .await
        .unwrap();
    assert_eq!(historical, recovered);
    assert_denied(
        other_sdk
            .call_value(
                "collaboration.commands.get",
                json!({"commandId":"update-event-lost-ack","principalId":credentials.principal_id()}),
            )
            .await
            .unwrap_err(),
    );

    // Reconnect with the same credentials after the owner narrows the grant.
    // Historical outcomes receive the same current exact-family check as reads.
    let principal = f.service.principal(credentials.principal_id()).unwrap();
    f.service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id().into(),
            expected_revision: principal.grant_revision,
            grant: grant(&["action:draft"]),
        })
        .unwrap();
    reopened.close();
    let narrowed = RemoteClient::connect(&f.endpoint, &credentials, ClientKind::Agent)
        .await
        .unwrap();
    let sdk = TikToolsClient::Remote(narrowed.clone());
    for (method, params) in [
        ("automation.get", json!({"kind":"event","id":"draft"})),
        (
            "collaboration.commands.get",
            json!({"commandId":"update-event-lost-ack"}),
        ),
        (
            "collaboration.commands.get",
            json!({"commandId":"create-event"}),
        ),
    ] {
        assert_denied(sdk.call_value(method, params).await.unwrap_err());
    }
    assert_denied(
        sdk.call_with_meta::<_, Value>(
            "automation.update",
            json!({"kind":"event","id":"draft","record":draft("event","draft")}),
            Some(metadata(&f, "out-of-scope-update", "4")),
        )
        .await
        .unwrap_err(),
    );
    assert_closed(
        &sdk.call_value("automation.get", json!({"kind":"action","id":"draft"}))
            .await
            .unwrap(),
        "action",
        false,
    );
    let allowed_outcome = sdk
        .call_value(
            "collaboration.commands.get",
            json!({"commandId":"create-action"}),
        )
        .await
        .unwrap();
    assert_eq!(allowed_outcome["result"], pending("action"));
    assert_no_source(&allowed_outcome);
    f.service
        .revoke_session(credentials.session_id(), "owner")
        .unwrap();
    let revoked = sdk
        .call_value(
            "collaboration.commands.get",
            json!({"commandId":"create-action"}),
        )
        .await
        .unwrap_err();
    assert!(revoked
        .data
        .as_ref()
        .is_none_or(|data| data.get("resourceVersions").is_none()));
    assert_no_source(&json!({"message":revoked.message,"data":revoked.data}));
    assert!(
        RemoteClient::connect(&f.endpoint, &credentials, ClientKind::Agent)
            .await
            .is_err()
    );
    narrowed.close();
    other.close();
    f.finish().await;
}
