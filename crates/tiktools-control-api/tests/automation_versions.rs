#![cfg(feature = "persistence")]
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{
    ControlApi, RemoteCapability, RemoteSession, RequestContext, RpcResponse,
};
use tiktools_core::{
    control::AutomationKind, paths::AppPaths, AppCore, HostEmitter, NoopMediaHost,
};
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Arc<AppCore>,
    api: ControlApi,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-automation-api-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths,
        ));
        Self {
            api: ControlApi::new(core.clone()),
            core,
            root,
        }
    }
    async fn call(&self, method: &str, params: Value, meta: Value) -> RpcResponse {
        let mut request = json!({"id":"request","method":method,"params":params});
        if !meta.is_null() {
            request["meta"] = meta;
        }
        self.api.execute_value(&request).await
    }
    async fn close(self) {
        self.core.shutdown().await;
        let _ = std::fs::remove_dir_all(self.root);
    }
}
#[tokio::test]
async fn owner_automation_metadata_preserves_shapes_tracks_legacy_writers_and_fences_stale_recreation(
) {
    let f = Fixture::new();
    let action = json!({"id":"same","name":"Action","enabled":true,"typeId":"core.log","config":{"message":"local"}});
    let event = json!({"id":"same","name":"Event","enabled":true,"trigger":"tiktok.chat","actionIds":["same"]});
    for (kind, record) in [("action", action.clone()), ("event", event)] {
        let result = f
            .call(
                "automation.create",
                json!({"kind":kind,"record":record}),
                json!({"expectedRevision":"0"}),
            )
            .await;
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(result.result.unwrap()["id"], "same");
        let meta = result.meta.unwrap();
        assert_eq!(meta.resource_versions[0].scope, kind);
        assert_eq!(meta.resource_versions[0].revision, "1");
        assert!(meta.command.is_none());
    }
    let list = f.call("automation.list", json!({}), Value::Null).await;
    assert!(list.is_ok(), "{list:?}");
    let body = list.result.unwrap();
    assert_eq!(body["actions"].as_array().unwrap().len(), 1);
    assert_eq!(body["events"].as_array().unwrap().len(), 1);
    assert_eq!(list.meta.unwrap().resource_versions.len(), 2);
    assert!(body.get("resourceVersions").is_none());
    f.core
        .automation_set_enabled(AutomationKind::Action, "same", false)
        .unwrap();
    let stale = f
        .call(
            "automation.update",
            json!({"kind":"action","id":"same","record":action}),
            json!({"expectedRevision":"1"}),
        )
        .await
        .error
        .unwrap();
    assert_eq!(stale.code, "revision_conflict");
    assert_eq!(stale.data.unwrap()["resourceVersions"][0]["revision"], "2");
    let updated=f.call("automation.update",json!({"kind":"action","id":"same","record":{"id":"forged-target","name":"New","enabled":true,"typeId":"core.log"}}),json!({"expectedRevision":"2"})).await;
    assert!(updated.is_ok(), "{updated:?}");
    assert_eq!(updated.result.unwrap()["id"], "same");
    assert_eq!(updated.meta.unwrap().resource_versions[0].revision, "3");
    let deleted = f
        .call(
            "automation.delete",
            json!({"kind":"action","id":"same"}),
            json!({"expectedRevision":"3"}),
        )
        .await;
    assert_eq!(deleted.result.unwrap(), json!({"id":"same","deleted":true}));
    let affected = deleted.meta.unwrap().resource_versions;
    assert_eq!(affected[0].revision, "4");
    assert_eq!(affected.len(), 2);
    assert_eq!(affected[1].scope, "event");
    assert_eq!(affected[1].id, "same");
    assert_eq!(affected[1].revision, "2");
    let missing = f
        .call(
            "automation.get",
            json!({"kind":"action","id":"same"}),
            Value::Null,
        )
        .await
        .error
        .unwrap();
    assert_eq!(missing.code, "automation_not_found");
    assert_eq!(
        missing.data.unwrap()["resourceVersions"][0]["revision"],
        "4"
    );
    let event = f
        .call(
            "automation.get",
            json!({"kind":"event","id":"same"}),
            Value::Null,
        )
        .await;
    assert_eq!(event.result.unwrap()["actionIds"], json!([]));
    assert_eq!(event.meta.unwrap().resource_versions[0].revision, "2");
    let old = f
        .call(
            "automation.create",
            json!({"kind":"action","record":action}),
            json!({"expectedRevision":"0"}),
        )
        .await;
    assert_eq!(old.error.unwrap().code, "revision_conflict");
    let recreated = f
        .call(
            "automation.create",
            json!({"kind":"action","record":action}),
            json!({"expectedRevision":"4"}),
        )
        .await;
    assert_eq!(recreated.meta.unwrap().resource_versions[0].revision, "5");
    let bulk=f.call("automation.set_enablement",json!({"updates":[{"kind":"event","id":"same","enabled":false},{"kind":"action","id":"same","enabled":false},{"kind":"action","id":"missing","enabled":true}]}),Value::Null).await;
    assert!(bulk.is_ok(), "{bulk:?}");
    assert_eq!(bulk.result.unwrap()["updated"], 2);
    assert_eq!(bulk.meta.unwrap().resource_versions.len(), 2);
    let snapshot = f.call("automation.snapshot", json!({}), Value::Null).await;
    assert!(snapshot.is_ok(), "{snapshot:?}");
    assert_eq!(snapshot.meta.unwrap().resource_versions.len(), 2);
    let schema = f
        .call(
            "rpc.schema",
            json!({"method":"automation.update"}),
            Value::Null,
        )
        .await
        .result
        .unwrap();
    assert_eq!(schema["concurrency"], "compare-and-set");
    assert_eq!(schema["durableCommands"], true);
    f.close().await;
}

#[tokio::test]
async fn wrong_host_or_bulk_preconditions_reject_before_writes_and_remote_execution_stays_denied() {
    let f = Fixture::new();
    let action = json!({"id":"never-created","name":"Action","enabled":true,"typeId":"core.log"});
    let refused=f.call("automation.create",json!({"kind":"action","record":action}),json!({"commandId":"must-not-be-ignored","hostInstanceId":"instance","expectedRevision":"0"})).await;
    assert_eq!(refused.error.unwrap().code, "host_restarted");
    let missing = f
        .call(
            "automation.get",
            json!({"kind":"action","id":"never-created"}),
            Value::Null,
        )
        .await
        .error
        .unwrap();
    assert_eq!(
        missing.data.unwrap()["resourceVersions"][0]["revision"],
        "0"
    );
    assert_eq!(
        f.call(
            "automation.set_enablement",
            json!({"updates":[]}),
            json!({"expectedRevision":"0"})
        )
        .await
        .error
        .unwrap()
        .code,
        "invalid_params"
    );
    let remote = RequestContext::Remote(
        RemoteSession::new(
            "moderator",
            "device",
            [
                RemoteCapability::AutomationRead,
                RemoteCapability::AutomationEdit,
                RemoteCapability::AutomationToggle,
                RemoteCapability::WorkflowsRead,
                RemoteCapability::WorkflowsEdit,
                RemoteCapability::WorkflowsToggle,
            ],
            std::time::Duration::from_secs(60),
        )
        .unwrap(),
    );
    for method in [
        "automation.snapshot",
        "automation.create",
        "automation.update",
        "automation.enable",
        "automation.disable",
        "automation.delete",
        "automation.set_enablement",
        "automation.fire",
        "workflows.save",
        "workflows.enable",
    ] {
        let result = f
            .api
            .execute_value_with_context(
                &remote,
                &json!({"id":1,"method":method,"params":{"kind":"action","id":"never-created","record":action}}),
            )
            .await;
        assert_eq!(result.error.unwrap().code, "forbidden", "{method}");
    }
    f.close().await;
}

impl Fixture {
    fn pair(&self, grant: tiktools_core::collaboration::Grant) -> (RequestContext, String) {
        use tiktools_core::collaboration::*;
        let service = self.core.collaboration().unwrap();
        let mut config = service.configuration().unwrap();
        config.enabled = true;
        config.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(config).unwrap();
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "a".repeat(64);
        let request = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Automation moderator".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "fixture",
            )
            .unwrap();
        service.decide_request(&request.request_id, true).unwrap();
        let credentials = service
            .collect(
                Collect {
                    request_id: request.request_id,
                    exchange_secret: exchange,
                },
                "fixture",
            )
            .unwrap();
        let auth = service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap();
        let principal = auth.snapshot().unwrap().principal_id;
        (
            RequestContext::Remote(RemoteSession::from_authorization(auth).unwrap()),
            principal,
        )
    }
    async fn remote(
        &self,
        context: &RequestContext,
        method: &str,
        params: Value,
        meta: Value,
    ) -> RpcResponse {
        self.api
            .execute_value_with_context(
                context,
                &json!({"id":1,"method":method,"params":params,"meta":meta}),
            )
            .await
    }
}
fn event_command(instance: &str, id: &str, revision: &str) -> Value {
    json!({"commandId":id,"hostInstanceId":instance,"expectedRevision":revision})
}

#[tokio::test]
async fn remote_summaries_and_event_commands_are_family_scoped_recoverable_and_reauthorized() {
    use tiktools_core::collaboration::*;
    let f = Fixture::new();
    for (kind, label) in [
        (AutomationKind::Action, "action"),
        (AutomationKind::Event, "event"),
    ] {
        for id in ["same", "private"] {
            f.core.automation_create(kind,json!({"id":id,"name":label,"enabled":true,"trigger":"tiktok.chat","actionIds":["private-action"],"config":{"secret":"owner-only"},"source":"secret-script"})).unwrap();
        }
    }
    let grant = Grant {
        capabilities: [
            Capability::AutomationRead,
            Capability::AutomationEdit,
            Capability::AutomationToggle,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            ResourceKind::Automation,
            ["event:same".into(), "event:missing".into()].into(),
        )]
        .into(),
        ..Grant::default()
    };
    let (context, principal) = f.pair(grant.clone());
    let (other, _) = f.pair(grant.clone());
    let service = f.core.collaboration().unwrap();
    let instance = service.instance_id();
    let read = f
        .remote(&context, "automation.list", json!({}), Value::Null)
        .await;
    assert!(read.is_ok(), "{read:?}");
    assert_eq!(read.meta.unwrap().resource_versions.len(), 1);
    assert_eq!(
        read.result.unwrap(),
        json!({"events":[{"id":"same","kind":"event","name":"event","enabled":true,"definitionRedacted":true}],"actions":[]})
    );
    let read = f
        .remote(
            &context,
            "automation.get",
            json!({"id":"same","kind":"event"}),
            Value::Null,
        )
        .await;
    assert!(read.is_ok(), "{read:?}");
    for (method, params) in [
        ("automation.get", json!({"kind":"action","id":"same"})),
        ("automation.get", json!({"id":"private"})),
        ("automation.snapshot", json!({})),
        (
            "automation.create",
            json!({"record":{"id":"new","name":"New"}}),
        ),
        (
            "automation.update",
            json!({"id":"same","record":{"name":"forged"}}),
        ),
        ("automation.enable", json!({"id":"same"})),
        ("automation.fire", json!({"trigger":"tiktok.chat"})),
    ] {
        let response = f
            .remote(
                &context,
                method,
                params,
                if method == "automation.get" {
                    Value::Null
                } else {
                    event_command(instance, "forbidden", "1")
                },
            )
            .await;
        assert_eq!(response.error.unwrap().code, "forbidden", "{method}");
    }
    for method in ["automation.disable", "automation.delete"] {
        let refused = f
            .remote(
                &context,
                method,
                json!({"id":"same","kind":"action"}),
                event_command(instance, "no-action", "1"),
            )
            .await;
        assert_eq!(refused.error.unwrap().code, "forbidden");
        let refused = f
            .remote(&context, method, json!({"id":"same"}), Value::Null)
            .await;
        assert_eq!(refused.error.unwrap().code, "forbidden");
    }
    let missing = f
        .remote(
            &context,
            "automation.disable",
            json!({"id":"missing"}),
            event_command(instance, "missing", "0"),
        )
        .await
        .error
        .unwrap();
    assert_eq!(missing.code, "automation_not_found");
    assert!(!missing.message.contains("workflow"));
    let stale = f
        .remote(
            &context,
            "automation.disable",
            json!({"id":"same"}),
            event_command(instance, "once", "0"),
        )
        .await;
    assert_eq!(stale.error.unwrap().code, "revision_conflict");
    let meta = event_command(instance, "once", "1");
    let success = f
        .remote(
            &context,
            "automation.disable",
            json!({"id":"same"}),
            meta.clone(),
        )
        .await;
    assert!(success.is_ok(), "{success:?}");
    assert_eq!(
        success.result.as_ref().unwrap(),
        &json!({"id":"same","enabled":false})
    );
    let receipt = success.meta.unwrap();
    assert_eq!(receipt.resource_versions[0].scope, "event");
    assert_eq!(receipt.resource_versions[0].revision, "2");
    // Owner changes after the command. Replay and outcome queries are receipts,
    // never a new definition/index write or a second disable.
    f.core
        .automation_set_enabled(AutomationKind::Event, "same", true)
        .unwrap();
    let replay = f
        .remote(&context, "automation.disable", json!({"id":"same"}), meta)
        .await;
    assert_eq!(replay.result, success.result);
    assert_eq!(replay.meta.unwrap().resource_versions[0].revision, "2");
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Event, "same")
            .unwrap()["enabled"],
        true
    );
    assert_eq!(
        f.core
            .automation_get_versioned(AutomationKind::Action, "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    let outcome = f
        .remote(
            &context,
            "collaboration.commands.get",
            json!({"commandId":"once"}),
            Value::Null,
        )
        .await;
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(
        outcome.result.as_ref().unwrap()["result"],
        json!({"id":"same","enabled":false})
    );
    assert!(!outcome
        .result
        .as_ref()
        .unwrap()
        .to_string()
        .contains("owner-only"));
    assert_eq!(
        f.remote(
            &other,
            "collaboration.commands.get",
            json!({"commandId":"once","principalId":principal}),
            Value::Null
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    let reused = f
        .remote(
            &context,
            "automation.delete",
            json!({"id":"same"}),
            event_command(instance, "once", "3"),
        )
        .await;
    assert_eq!(reused.error.unwrap().code, "command_id_reused");
    let deleted = f
        .remote(
            &context,
            "automation.delete",
            json!({"id":"same"}),
            event_command(instance, "remove", "3"),
        )
        .await;
    assert!(deleted.is_ok(), "{deleted:?}");
    assert_eq!(deleted.meta.unwrap().resource_versions[0].revision, "4");
    let missing = f
        .remote(
            &context,
            "automation.get",
            json!({"id":"same"}),
            Value::Null,
        )
        .await;
    assert_eq!(
        missing.error.unwrap().data.unwrap()["resourceVersions"][0]["revision"],
        "4"
    );
    let RequestContext::Remote(session) = &context else {
        panic!()
    };
    assert!(f
        .api
        .authorize_remote_response(
            session,
            "automation.disable",
            &json!({"id":"same","enabled":false,"config":{"secret":"must-not-egress"}})
        )
        .is_err());
    let mut forged = outcome.result.clone().unwrap();
    forged["result"]["config"] = json!({"secret":"must-not-egress"});
    assert!(f
        .api
        .authorize_remote_response(session, "collaboration.commands.get", &forged)
        .is_err());
    let (no_read, _) = f.pair(Grant {
        capabilities: [Capability::AutomationToggle].into(),
        ..Grant::default()
    });
    assert_eq!(
        f.remote(
            &no_read,
            "automation.disable",
            json!({"id":"same"}),
            event_command(instance, "no-read", "4")
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    let (creator_restricted, _) = f.pair(Grant {
        resources: [(ResourceKind::Live, ["creator".into()].into())].into(),
        ..grant.clone()
    });
    assert_eq!(
        f.remote(
            &creator_restricted,
            "automation.list",
            json!({}),
            Value::Null
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    // Grant narrowing fences queued successes, errors, and retained outcomes.
    let current = service.principal(&principal).unwrap();
    service
        .set_grant(GrantSet {
            principal_id: principal.clone(),
            expected_revision: current.grant_revision,
            grant: Grant {
                resources: [(ResourceKind::Automation, ["action:same".into()].into())].into(),
                ..grant
            },
        })
        .unwrap();
    assert!(f
        .api
        .authorize_remote_response(
            session,
            "automation.disable",
            &json!({"id":"same","enabled":false})
        )
        .is_err());
    assert!(f
        .api
        .authorize_remote_response(
            session,
            "collaboration.commands.get",
            outcome.result.as_ref().unwrap()
        )
        .is_err());
    assert!(f.api.authorize_remote_failure(session,"automation.get",Some(&json!({"resourceVersions":[{"kind":"automation","scope":"event","id":"same","revision":"4"}]}))).is_err());
    assert_eq!(
        f.remote(
            &context,
            "collaboration.commands.get",
            json!({"commandId":"once"}),
            Value::Null
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    f.close().await;
}

#[tokio::test]
async fn owner_durable_behavior_crud_has_explicit_ids_shared_namespace_and_cascade_versions() {
    let f = Fixture::new();
    let service = f.core.collaboration().unwrap();
    let instance = service.instance_id();
    let invalid = f
        .call(
            "automation.create",
            json!({"record":{"name":"Missing ID"}}),
            event_command(instance, "create", "0"),
        )
        .await;
    assert!(invalid.error.is_some());
    let action = json!({"id":"same","name":"","enabled":true,"typeId":"core.log"});
    let created = f
        .call(
            "automation.create",
            json!({"kind":"action","record":action}),
            event_command(instance, "create", "0"),
        )
        .await;
    assert!(created.is_ok(), "{created:?}");
    assert_eq!(created.result.unwrap()["name"], "Untitled action");
    let reused = f
        .call(
            "workflows.save",
            json!({"graph":{"id":"same","name":"WF"}}),
            event_command(instance, "create", "0"),
        )
        .await;
    assert_eq!(reused.error.unwrap().code, "command_id_reused");
    f.core
        .automation_create(
            AutomationKind::Event,
            json!({"id":"same","name":"Event","actionIds":["same"]}),
        )
        .unwrap();
    let updated=f.call("automation.update",json!({"kind":"action","id":"same","record":{"id":"different","name":"Updated","enabled":false}}),event_command(instance,"update","1")).await;
    assert!(updated.is_ok(), "{updated:?}");
    assert_eq!(updated.result.unwrap()["id"], "same");
    let deleted = f
        .call(
            "automation.delete",
            json!({"kind":"action","id":"same"}),
            event_command(instance, "delete", "2"),
        )
        .await;
    assert!(deleted.is_ok(), "{deleted:?}");
    assert_eq!(deleted.meta.as_ref().unwrap().resource_versions.len(), 2);
    assert_eq!(
        deleted.meta.as_ref().unwrap().resource_versions[0].scope,
        "action"
    );
    assert_eq!(
        deleted.meta.as_ref().unwrap().resource_versions[1].scope,
        "event"
    );
    let outcome = f
        .call(
            "collaboration.commands.get",
            json!({"commandId":"delete"}),
            Value::Null,
        )
        .await;
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(
        outcome.result.unwrap()["resourceVersions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let replay = f
        .call(
            "automation.delete",
            json!({"kind":"action","id":"same"}),
            event_command(instance, "delete", "2"),
        )
        .await;
    assert_eq!(replay.result, deleted.result);
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Event, "same")
            .unwrap()["actionIds"],
        json!([])
    );
    f.close().await;
}
