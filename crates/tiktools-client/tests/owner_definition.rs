//! The strict native definition SDK validates real direct and IPC replies.
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tiktools_client::{automation_definition::OwnerDefinitionSubmit, TikToolsClient};
use tiktools_core::{
    control::AutomationKind, ipc::messages::HostMessage, paths::AppPaths, AppCore, HostEmitter,
    NoopMediaHost,
};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_definition_sdk_keeps_fresh_acknowledgements_and_historical_recovery_distinct_over_ipc(
) {
    let temporary = tempfile::tempdir().unwrap();
    let home = temporary.path().join("home");
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var(
        "TIKTOOLS_IPC_NAME",
        format!(
            "tiktools-definition-sdk-{}-{}",
            std::process::id(),
            temporary.path().file_name().unwrap().to_string_lossy()
        ),
    );
    let paths = AppPaths {
        data: home.join("data"),
        plugins: home.join("plugins"),
        plugin_data: home.join("plugin-data"),
        builtin_plugins: home.join("builtins"),
        development_plugins: None,
        logs: home.join("logs"),
        temp: home.join("temp"),
        root: home,
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
    let api = Arc::new(tiktools_client::ControlApi::new(core.clone()));
    let (ready, started) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(tiktools_control_api::run_ipc_shared_with_ready(
        api.clone(),
        move || {
            let _ = ready.send(());
        },
    ));
    tokio::time::timeout(Duration::from_secs(5), started)
        .await
        .unwrap()
        .unwrap();
    let direct = TikToolsClient::direct(api.clone());
    let connected = tokio::time::timeout(Duration::from_secs(5), TikToolsClient::connect())
        .await
        .unwrap()
        .unwrap();
    for (index, client) in [direct, connected].into_iter().enumerate() {
        let initial = client.automation_definition_get().await.unwrap();
        let id = format!("native-sdk-{index}");
        let missing = format!("missing-sdk-{index}");
        let remove_missing: OwnerDefinitionSubmit = serde_json::from_value(json!({"operation":"delete","kind":"action","id":missing,"expectedRevision":"0","hostId":initial.host_id,"hostInstanceId":initial.host_instance_id})).unwrap();
        let absent = client
            .automation_definition_submit(remove_missing)
            .await
            .unwrap();
        assert_eq!(
            absent.acknowledgement.as_ref().unwrap().value,
            json!({"id":missing,"deleted":false})
        );
        assert_eq!(
            absent.acknowledgement.unwrap().resource_versions[0].revision,
            "0"
        );
        let missing_update: OwnerDefinitionSubmit = serde_json::from_value(json!({"operation":"update","kind":"action","id":missing,"record":{"id":missing,"name":"Missing owner action","typeId":"core.log"},"expectedRevision":"0","hostId":initial.host_id,"hostInstanceId":initial.host_instance_id})).unwrap();
        let rejected = client
            .automation_definition_submit(missing_update)
            .await
            .unwrap();
        assert_eq!(
            rejected.state.recent[0].error_code.as_deref(),
            Some("not_found")
        );
        assert!(rejected.acknowledgement.is_none());
        // Native Rust callers preserve exact integers and optional legacy
        // fields; they do not round source through a browser Number.
        let source = json!({"id":id,"name":"  ","typeId":"core.log","config":{"message":"private-sdk-definition-source","large":u64::MAX}});
        let create:OwnerDefinitionSubmit=serde_json::from_value(json!({"operation":"create","kind":"action","record":source,"expectedRevision":"0","hostId":initial.host_id,"hostInstanceId":initial.host_instance_id})).unwrap();
        let created = client.automation_definition_submit(create).await.unwrap();
        let ack = created.acknowledgement.unwrap();
        assert_eq!(ack.value["name"], "Untitled action");
        assert_eq!(ack.value["config"]["large"], json!(u64::MAX));
        assert!(ack.value.get("enabled").is_none());
        assert_eq!(ack.resource_versions[0].revision, "1");
        let original_command = ack.command.command_id;
        let mut newer = ack.value;
        newer["name"] = json!("Newer owner edit");
        core.automation_update(AutomationKind::Action, &id, newer)
            .unwrap();
        let recovered = client.automation_definition_resolve().await.unwrap();
        assert_eq!(recovered.recent[0].intent.command_id, original_command);
        assert_eq!(recovered.recent[0].receipt.as_ref().unwrap().revision, "1");
        assert!(!serde_json::to_string(&recovered)
            .unwrap()
            .contains("private-sdk-definition-source"));
        assert_eq!(
            core.automation_get(AutomationKind::Action, &id).unwrap()["name"],
            "Newer owner edit"
        );

        let enable:OwnerDefinitionSubmit=serde_json::from_value(json!({"operation":"enable","kind":"action","id":id,"expectedRevision":"2","hostId":initial.host_id,"hostInstanceId":initial.host_instance_id})).unwrap();
        let enabled = client.automation_definition_submit(enable).await.unwrap();
        assert_eq!(enabled.acknowledgement.unwrap().value["enabled"], true);
        let delete:OwnerDefinitionSubmit=serde_json::from_value(json!({"operation":"delete","kind":"action","id":id,"expectedRevision":"3","hostId":initial.host_id,"hostInstanceId":initial.host_instance_id})).unwrap();
        let deleted = client.automation_definition_submit(delete).await.unwrap();
        assert_eq!(
            deleted.acknowledgement.unwrap().value,
            json!({"id":id,"deleted":true})
        );
        let recreate:OwnerDefinitionSubmit=serde_json::from_value(json!({"operation":"create","kind":"action","record":source,"expectedRevision":"4","hostId":initial.host_id,"hostInstanceId":initial.host_instance_id})).unwrap();
        assert_eq!(
            client
                .automation_definition_submit(recreate)
                .await
                .unwrap()
                .acknowledgement
                .unwrap()
                .resource_versions[0]
                .revision,
            "5"
        );
    }
    core.shutdown().await;
    server.abort();
    let _ = server.await;
}
