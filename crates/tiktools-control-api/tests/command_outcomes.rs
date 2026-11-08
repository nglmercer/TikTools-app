#![cfg(feature = "persistence")]
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
fn boot() -> (Arc<AppCore>, Arc<ControlApi>) {
    let core = Arc::new(AppCore::new(Arc::new(Emitter)));
    let api = Arc::new(ControlApi::new(core.clone()));
    (core, api)
}

#[tokio::test]
async fn commands_survive_lost_ack_restart_and_metadata_transport_without_widening_authority() {
    let home = std::env::temp_dir().join(format!("tt-cmd-api-{}", std::process::id()));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.join("builtins"));
    std::env::set_var("TIKTOOLS_DEV_PLUGINS_DIR", home.join("dev"));
    std::env::set_var("TIKTOOLS_PLUGIN_DATA_DIR", home.join("plugin-data"));
    let (core, api) = boot();
    let service = core.collaboration().unwrap();
    let instance = service.instance_id().to_owned();
    let graph = json!({"id":"welcome","name":"Original","enabled":false,"nodes":[],"edges":[]});
    let mut request = json!({"jsonrpc":"2.0","id":"wire-1","method":"workflows.save","params":{"graph":graph},"meta":{"commandId":"save-once","hostInstanceId":instance,"expectedRevision":"0"}});
    let first = api.execute_value(&request).await;
    assert!(first.is_ok());
    let receipt = first.meta.as_ref().unwrap().command.clone().unwrap();
    assert_eq!(
        receipt.state,
        tiktools_core::collaboration::CommandState::Succeeded
    );
    assert_eq!(receipt.resource.as_ref().unwrap().id, "welcome");
    core.workflow_set_enabled("welcome", true).unwrap();
    request["id"] = json!("wire-retry");
    let duplicate = api.execute_value(&request).await;
    assert_eq!(duplicate.result, first.result);
    assert_eq!(
        duplicate.meta.as_ref().unwrap().command.as_ref(),
        Some(&receipt)
    );
    assert_eq!(duplicate.meta.unwrap().resource_versions[0].revision, "1");
    assert_eq!(
        core.workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "2"
    );
    // The normal host worker imports both durable-command and legacy-writer
    // metadata into the existing owner API without exposing graph bodies.
    let audit = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let response = api.execute_value(&json!({"id":"audit","method":"collaboration.audit.list","params":{"after":0,"limit":100}})).await;
            let rows = response.result.unwrap();
            let details: Vec<_> = rows["entries"].as_array().unwrap().iter().filter_map(|row| row.get("details")).cloned().collect();
            if details.len() >= 2 { break details; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    assert_eq!(
        audit
            .iter()
            .filter(|entry| entry["commandId"] == "save-once")
            .count(),
        1
    );
    assert!(audit
        .iter()
        .any(|entry| entry["method"] == "workflows.enable"
            && entry["beforeRevision"] == "1"
            && entry["afterRevision"] == "2"));
    assert!(!serde_json::to_string(&audit).unwrap().contains("Original"));
    let outcome=api.execute_value(&json!({"id":2,"method":"collaboration.commands.get","params":{"commandId":"save-once"}})).await.result.unwrap();
    assert_eq!(outcome["result"]["enabled"], false);
    assert_eq!(outcome["resourceVersions"][0]["revision"], "1");
    let mut reused = request.clone();
    reused["params"]["graph"]["name"] = json!("Different");
    assert_eq!(
        api.execute_value(&reused).await.error.unwrap().code,
        "command_id_reused"
    );
    let mut stale = request.clone();
    stale["meta"]["commandId"] = json!("uncommitted");
    stale["meta"]["hostInstanceId"] = json!("stale-instance");
    stale["meta"]["expectedRevision"] = json!("2");
    assert_eq!(
        api.execute_value(&stale).await.error.unwrap().code,
        "host_restarted"
    );
    assert_eq!(api.execute_value(&json!({"id":3,"method":"collaboration.commands.get","params":{"commandId":"uncommitted"}})).await.error.unwrap().code,"not_found");
    let mut config = service.configuration().unwrap();
    config.limits.command_ids_per_principal = 1;
    service.configure(config.clone()).unwrap();
    let mut capacity = request.clone();
    capacity["meta"]["commandId"] = json!("capacity");
    capacity["meta"]["expectedRevision"] = json!("2");
    assert_eq!(
        api.execute_value(&capacity).await.error.unwrap().code,
        "overloaded"
    );
    assert!(api.execute_value(&request).await.is_ok());
    config.limits.command_ids_per_principal = 100_000;
    service.configure(config).unwrap();
    let shutdown=api.execute_value(&json!({"id":4,"method":"system.shutdown","meta":{"commandId":"must-reject","hostInstanceId":instance,"expectedRevision":"0"}})).await;
    assert_eq!(shutdown.error.unwrap().code, "unsupported_remote");
    assert!(!core.is_shutdown());
    core.shutdown().await;
    drop(service);
    drop(api);
    drop(core);

    let (core, api) = boot();
    let service = core.collaboration().unwrap();
    let current = service.instance_id().to_owned();
    assert_ne!(current, instance);
    let resumed = api.execute_value(&request).await;
    assert_eq!(resumed.result, first.result);
    assert_eq!(
        resumed.meta.as_ref().unwrap().host_instance_id.as_deref(),
        Some(current.as_str())
    );
    assert_eq!(resumed.meta.unwrap().command, Some(receipt));
    capacity["meta"]["hostInstanceId"] = json!(instance);
    assert_eq!(
        api.execute_value(&capacity).await.error.unwrap().code,
        "host_restarted"
    );
    capacity["meta"]["hostInstanceId"] = json!(current);
    assert_eq!(
        api.execute_value(&capacity)
            .await
            .meta
            .unwrap()
            .resource_versions[0]
            .revision,
        "3"
    );

    // An authenticated controller with read permissions still cannot select
    // the local owner's principal, including when it knows a valid command ID.
    use tiktools_core::collaboration::{
        hash_secret, Capability, ClientKind, Collect, Grant, InviteCreate, Redeem,
    };
    let mut enabled = service.configuration().unwrap();
    enabled.enabled = true;
    enabled.advertised_origin = Some("http://127.0.0.1:8788".into());
    service.configure(enabled).unwrap();
    let invite = service
        .create_invitation(InviteCreate {
            grant: Grant {
                capabilities: [Capability::CommandsRead, Capability::WorkflowsRead].into(),
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
                client_label: "Moderator".into(),
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
    let remote = RemoteSession::from_authorization(
        service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap(),
    )
    .unwrap();
    let context = RequestContext::Remote(remote.clone());
    assert_eq!(
        api.execute_value_with_context(
            &context,
            &json!({"id":"forbidden-audit","method":"collaboration.audit.list","params":{}})
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    for params in [
        json!({"commandId":"save-once","principalId":"local-owner"}),
        json!({"commandId":"save-once"}),
    ] {
        assert_eq!(
            api.execute_value_with_context(
                &context,
                &json!({"id":5,"method":"collaboration.commands.get","params":params})
            )
            .await
            .error
            .unwrap()
            .code,
            "forbidden"
        );
    }
    assert_eq!(
        api.authorize_remote_response(&remote, "collaboration.commands.get", &outcome)
            .unwrap_err()
            .code,
        "forbidden"
    );
    let own_failure = json!({"principalId":credentials.principal_id,"command":{"resource":{"kind":"workflow","scope":"host","id":"welcome"}},"commandId":"historical","state":"unknown"});
    assert!(api
        .authorize_remote_failure(&remote, "collaboration.commands.get", Some(&own_failure))
        .is_ok());
    service
        .set_grant(tiktools_core::collaboration::GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "1".into(),
            grant: Grant {
                capabilities: [Capability::CommandsRead, Capability::WorkflowsRead].into(),
                resources: [(
                    tiktools_core::collaboration::ResourceKind::Workflow,
                    ["another-workflow".into()].into(),
                )]
                .into(),
                ..Grant::default()
            },
        })
        .unwrap();
    assert_eq!(
        api.authorize_remote_failure(&remote, "collaboration.commands.get", Some(&own_failure))
            .unwrap_err()
            .code,
        "forbidden"
    );
    request["meta"]["hostInstanceId"] = json!(current);
    assert_eq!(
        api.execute_value_with_context(&context, &request)
            .await
            .error
            .unwrap()
            .code,
        "forbidden"
    );
    let schema = api
        .execute_value(&json!({"id":6,"method":"rpc.schema","params":{"method":"workflows.save"}}))
        .await
        .result
        .unwrap();
    assert_eq!(schema["durableCommands"], true);
    let empty: Value = api
        .execute_value(&json!({"id":7,"method":"workflows.list"}))
        .await
        .result
        .unwrap();
    assert_eq!(empty["workflows"].as_array().unwrap().len(), 1);
    core.shutdown().await;
    drop(service);
    drop(api);
    drop(core);
    std::fs::remove_dir_all(home).unwrap();
}
