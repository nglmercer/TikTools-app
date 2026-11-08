#![cfg(feature = "persistence")]
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext};
use tiktools_core::{
    collaboration::{
        hash_secret, Capability, ClientKind, Collect, Grant, GrantSet, InviteCreate, Redeem,
        ResourceKind,
    },
    ipc::messages::HostMessage,
    AppCore, HostEmitter,
};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

fn pair(
    core: &AppCore,
    grant: Grant,
) -> (RequestContext, tiktools_core::collaboration::Credentials) {
    let service = core.collaboration().unwrap();
    let invitation = service.create_invitation(InviteCreate { grant }).unwrap();
    let exchange = "a".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invitation.invite_id,
                secret: invitation.secret,
                client_label: "Workflow moderator".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "test-peer",
        )
        .unwrap();
    service.decide_request(&pending.request_id, true).unwrap();
    let credentials = service
        .collect(
            Collect {
                request_id: pending.request_id,
                exchange_secret: exchange,
            },
            "test-peer",
        )
        .unwrap();
    let session = RemoteSession::from_authorization(
        service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap(),
    )
    .unwrap();
    (RequestContext::Remote(session), credentials)
}

fn command(method: &str, id: &str, command: &str, revision: &str, instance: &str) -> Value {
    json!({"id":"wire","method":method,"params":{"id":id},"meta":{"commandId":command,"hostInstanceId":instance,"expectedRevision":revision}})
}

#[tokio::test]
async fn scoped_deactivation_and_deletion_are_durable_without_enabling_definition_execution() {
    let home = std::env::temp_dir().join(format!("tt-remote-workflow-{}", std::process::id()));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.join("builtins"));
    std::env::set_var("TIKTOOLS_DEV_PLUGINS_DIR", home.join("dev"));
    std::env::set_var("TIKTOOLS_PLUGIN_DATA_DIR", home.join("plugin-data"));
    std::env::set_var("TIKTOOLS_CLOUD_ORIGIN", "");
    let core = Arc::new(AppCore::new(Arc::new(Emitter)));
    let api = ControlApi::new(core.clone());
    let service = core.collaboration().unwrap();
    let mut configuration = service.configuration().unwrap();
    configuration.enabled = true;
    configuration.advertised_origin = Some("http://127.0.0.1:8788".into());
    service.configure(configuration).unwrap();
    let instance = service.instance_id().to_owned();
    for id in ["welcome", "remove", "private"] {
        core.workflow_save(
            json!({"id":id,"name":id,"enabled":true,"source":"owner-only-definition-body"}),
        )
        .unwrap();
    }
    let grant = Grant {
        capabilities: [
            Capability::WorkflowsRead,
            Capability::WorkflowsToggle,
            Capability::WorkflowsEdit,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            ResourceKind::Workflow,
            ["welcome".into(), "remove".into(), "missing".into()].into(),
        )]
        .into(),
        ..Grant::default()
    };
    let (context, credentials) = pair(&core, grant.clone());
    let (other, _) = pair(&core, grant.clone());
    let (no_read, _) = pair(
        &core,
        Grant {
            capabilities: [Capability::WorkflowsToggle, Capability::WorkflowsEdit].into(),
            ..Grant::default()
        },
    );

    let discovery = api
        .execute_value_with_context(&context, &json!({"id":1,"method":"rpc.discover"}))
        .await
        .result
        .unwrap();
    let methods: Vec<_> = discovery["methods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert!(methods.contains(&"workflows.disable"));
    assert!(methods.contains(&"workflows.delete"));
    assert!(!methods.contains(&"workflows.save"));
    assert!(!methods.contains(&"workflows.enable"));
    for method in ["workflows.disable", "workflows.delete"] {
        let metadata = api
            .execute_value_with_context(
                &context,
                &json!({"id":2,"method":"rpc.schema","params":{"method":method}}),
            )
            .await
            .result
            .unwrap();
        assert_eq!(metadata["durableCommands"], true);
        assert_eq!(metadata["concurrency"], "compare-and-set");
        assert_eq!(
            api.execute_value_with_context(
                &no_read,
                &command(method, "welcome", "no-read", "1", &instance)
            )
            .await
            .error
            .unwrap()
            .code,
            "forbidden"
        );
        assert_eq!(
            api.execute_value_with_context(
                &context,
                &json!({"id":3,"method":method,"params":{"id":"welcome"}})
            )
            .await
            .error
            .unwrap()
            .code,
            "forbidden"
        );
        let denied = api
            .execute_value_with_context(
                &context,
                &command(method, "private", "private", "1", &instance),
            )
            .await
            .error
            .unwrap();
        assert_eq!(denied.code, "forbidden");
        assert!(denied.data.is_none());
    }
    let mut request = command(
        "workflows.disable",
        "welcome",
        "disable-once",
        "0",
        &instance,
    );
    let conflict = api
        .execute_value_with_context(&context, &request)
        .await
        .error
        .unwrap();
    assert_eq!(conflict.code, "revision_conflict");
    assert_eq!(
        conflict.data.unwrap()["resourceVersions"][0]["revision"],
        "1"
    );
    assert_eq!(core.workflow_get("welcome").unwrap()["enabled"], true);
    request["meta"]["expectedRevision"] = json!("1");
    let first = api.execute_value_with_context(&context, &request).await;
    assert!(first.is_ok(), "{:?}", first.error);
    assert_eq!(first.result, Some(json!({"id":"welcome","enabled":false})));
    assert_eq!(
        first.meta.as_ref().unwrap().resource_versions[0].revision,
        "2"
    );
    core.workflow_set_enabled("welcome", true).unwrap();
    request["id"] = json!("lost-ack-retry");
    let retry = api.execute_value_with_context(&context, &request).await;
    assert_eq!(retry.result, first.result);
    assert_eq!(retry.meta, first.meta);
    assert_eq!(core.workflow_get("welcome").unwrap()["enabled"], true);
    assert_eq!(
        core.workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "3"
    );
    assert_eq!(
        api.execute_value_with_context(&other, &request)
            .await
            .error
            .unwrap()
            .code,
        "revision_conflict"
    );
    let outcome = api.execute_value_with_context(&context, &json!({"id":4,"method":"collaboration.commands.get","params":{"commandId":"disable-once"}})).await.result.unwrap();
    assert_eq!(outcome["principalId"], credentials.principal_id);
    assert_eq!(outcome["result"], first.result.unwrap());
    assert!(!outcome.to_string().contains("owner-only-definition-body"));
    assert_eq!(api.execute_value_with_context(&other, &json!({"id":5,"method":"collaboration.commands.get","params":{"commandId":"disable-once","principalId":credentials.principal_id}})).await.error.unwrap().code,"forbidden");
    let reused = command("workflows.delete", "remove", "disable-once", "1", &instance);
    assert_eq!(
        api.execute_value_with_context(&context, &reused)
            .await
            .error
            .unwrap()
            .code,
        "command_id_reused"
    );

    let remove = command("workflows.delete", "remove", "delete-once", "1", &instance);
    let deleted = api.execute_value_with_context(&context, &remove).await;
    assert_eq!(deleted.result, Some(json!({"id":"remove","deleted":true})));
    core.workflow_save(json!({"id":"remove","name":"Recreated by owner","enabled":true}))
        .unwrap();
    let retry = api.execute_value_with_context(&context, &remove).await;
    assert_eq!(retry.result, deleted.result);
    assert_eq!(retry.meta, deleted.meta);
    assert_eq!(
        core.workflow_get("remove").unwrap()["name"],
        "Recreated by owner"
    );
    assert_eq!(
        api.execute_value_with_context(
            &context,
            &command(
                "workflows.delete",
                "missing",
                "missing-once",
                "0",
                &instance
            )
        )
        .await
        .result,
        Some(json!({"id":"missing","deleted":false}))
    );
    for method in ["workflows.save", "workflows.enable"] {
        assert_eq!(
            api.execute_value_with_context(
                &context,
                &command(method, "welcome", "must-not-activate", "3", &instance)
            )
            .await
            .error
            .unwrap()
            .code,
            "forbidden"
        );
    }
    let RequestContext::Remote(remote) = &context else {
        unreachable!()
    };
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "1".into(),
            grant: Grant {
                resources: [(ResourceKind::Workflow, [].into())].into(),
                ..grant
            },
        })
        .unwrap();
    assert_eq!(
        api.execute_value_with_context(&context, &request)
            .await
            .error
            .unwrap()
            .code,
        "forbidden"
    );
    assert_eq!(
        api.authorize_remote_response(
            remote,
            "workflows.disable",
            &json!({"id":"welcome","enabled":false})
        )
        .unwrap_err()
        .code,
        "forbidden"
    );
    assert_eq!(api.authorize_remote_failure(remote,"workflows.delete",Some(&json!({"resourceVersions":[{"kind":"workflow","scope":"host","id":"welcome","revision":"3"}]}))).unwrap_err().code,"forbidden");
    assert_eq!(api.execute_value_with_context(&context, &json!({"id":6,"method":"collaboration.commands.get","params":{"commandId":"disable-once"}})).await.error.unwrap().code,"forbidden");
    assert_eq!(core.workflow_get("private").unwrap()["enabled"], true);
    service
        .revoke_session(&credentials.session_id, "local-owner")
        .unwrap();
    assert_eq!(
        api.execute_value_with_context(&context, &request)
            .await
            .error
            .unwrap()
            .code,
        "unauthorized"
    );
    core.shutdown().await;
    drop(api);
    drop(service);
    drop(core);
    std::fs::remove_dir_all(home).unwrap();
}
