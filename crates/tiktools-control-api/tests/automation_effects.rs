#![cfg(feature = "persistence")]
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext};
use tiktools_core::{
    collaboration::{hash_secret, Capability, ClientKind, Collect, Grant, InviteCreate, Redeem},
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
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
            "tt-effect-api-{}-{}",
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
        for file in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(file).unwrap();
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
    fn all_grants(&self) -> RequestContext {
        let service = self.core.collaboration().unwrap();
        let mut configuration = service.configuration().unwrap();
        configuration.enabled = true;
        configuration.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(configuration).unwrap();
        let grant = Grant {
            capabilities: [
                Capability::LiveRead,
                Capability::PointsRead,
                Capability::PointsAdjust,
                Capability::RankingsRead,
                Capability::RankingsManage,
                Capability::CompetitionsRead,
                Capability::CompetitionsManage,
                Capability::WorkflowsRead,
                Capability::WorkflowsEdit,
                Capability::WorkflowsToggle,
                Capability::AutomationRead,
                Capability::AutomationEdit,
                Capability::AutomationToggle,
                Capability::PluginsRead,
                Capability::PluginsToggle,
                Capability::PluginSettingsRead,
                Capability::PluginSettingsWrite,
                Capability::PluginActionExecute,
                Capability::WidgetsRead,
                Capability::WidgetsConfigure,
                Capability::CommandsRead,
                Capability::HttpRequest,
                Capability::AudioPlay,
                Capability::EventsEmit,
                Capability::ScriptsExecute,
            ]
            .into(),
            ..Grant::default()
        };
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "e".repeat(64);
        let request = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Effect history denial".into(),
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
        let authorization = service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap();
        RequestContext::Remote(RemoteSession::from_authorization(authorization).unwrap())
    }
    async fn close(self) {
        self.core.shutdown().await;
        std::fs::remove_dir_all(self.root).unwrap();
    }
}

#[tokio::test]
async fn owner_reads_metadata_only_receipts_with_closed_params_and_no_replay_surface() {
    let f = Fixture::new();
    for (method, params, expected) in [
        (
            "automation.effects.get",
            json!({"effectId":"a".repeat(64)}),
            Value::Null,
        ),
        (
            "automation.effects.history",
            json!({"before":null,"limit":10}),
            json!({"items":[],"nextBefore":null}),
        ),
    ] {
        let response = f
            .api
            .execute_value(&json!({"id":1,"method":method,"params":params}))
            .await;
        assert!(response.is_ok(), "{response:?}");
        assert_eq!(response.result, Some(expected));
        let metadata = response.meta.unwrap();
        assert!(metadata.resource_versions.is_empty());
        assert!(metadata.command.is_none());
        assert!(metadata.host_instance_id.is_none());
        for extra in [
            json!({"replay":true}),
            json!({"source":"private-body"}),
            json!({"commandId":"forged"}),
        ] {
            let mut invalid = params.clone();
            invalid
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let response = f
                .api
                .execute_value(&json!({"id":1,"method":method,"params":invalid}))
                .await;
            assert_eq!(response.error.unwrap().code, "invalid_params");
        }
        for metadata in [
            json!({"commandId":"forged"}),
            json!({"hostInstanceId":"forged"}),
            json!({"expectedRevision":"1"}),
        ] {
            assert!(!f
                .api
                .execute_value(&json!({"id":1,"method":method,"params":params,"meta":metadata}))
                .await
                .is_ok());
        }
    }
    for params in [
        json!({"before":null,"limit":0}),
        json!({"before":null,"limit":101}),
        json!({"before":"A".repeat(64),"limit":10}),
    ] {
        assert_eq!(
            f.api
                .execute_value(
                    &json!({"id":1,"method":"automation.effects.history","params":params})
                )
                .await
                .error
                .unwrap()
                .code,
            "invalid_params"
        );
    }
    let methods = f.api.router().metadata();
    let effects = methods
        .iter()
        .filter(|method| method.name.starts_with("automation.effects."))
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), 2);
    assert!(effects
        .iter()
        .all(|method| !method.side_effect && !method.durable_commands));
    assert!(effects.iter().all(|method| matches!(
        method.name.as_str(),
        "automation.effects.get" | "automation.effects.history"
    )));
    f.close().await;
}

#[tokio::test]
async fn all_remote_grants_deny_effect_history_ingress_and_result_or_error_metadata_egress() {
    let f = Fixture::new();
    let remote = f.all_grants();
    let RequestContext::Remote(session) = &remote else {
        unreachable!()
    };
    for (method, params) in [
        ("automation.effects.get", json!({"effectId":"a".repeat(64)})),
        (
            "automation.effects.history",
            json!({"before":null,"limit":10}),
        ),
    ] {
        let denied = f
            .api
            .execute_value_with_context(&remote, &json!({"id":1,"method":method,"params":params}))
            .await;
        assert_eq!(denied.error.unwrap().code, "forbidden");
        assert!(denied.result.is_none());
        assert!(denied.meta.is_none());
        for result in [
            Value::Null,
            json!({"items":[],"nextBefore":null}),
            json!({"effectId":"a".repeat(64),"kind":"http","state":"unknown","acceptedAt":1,"updatedAt":2,"outcomeExpired":false}),
        ] {
            assert!(f
                .api
                .authorize_remote_response(session, method, &result)
                .is_err());
        }
        for metadata in [
            None,
            Some(
                json!({"resourceVersions":[],"command":{"commandId":"opaque"},"hostInstanceId":"owner-only"}),
            ),
        ] {
            assert!(f
                .api
                .authorize_remote_failure(session, method, metadata.as_ref())
                .is_err());
        }
    }
    f.close().await;
}
