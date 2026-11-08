//! The metadata-aware SDK behaves identically in-process and over local IPC.
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tiktools_client::{RequestMetadata, TikToolsClient};
use tiktools_control_api::modules::automation::{
    AutomationGetParams, AutomationManifestParams, AutomationReviewAuthority,
};
use tiktools_core::{
    control::AutomationKind, ipc::messages::HostMessage, paths::AppPaths, AppCore, HostEmitter,
    NoopMediaHost,
};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

#[tokio::test]
async fn sdk_preserves_versions_for_direct_and_connected_backends() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let home = std::env::temp_dir().join(format!("ttm-{}-{nonce:x}", std::process::id()));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var(
        "TIKTOOLS_IPC_NAME",
        format!("tiktools-metadata-{}-{nonce:x}", std::process::id()),
    );
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.join("builtins"));
    std::env::set_var("TIKTOOLS_DEV_PLUGINS_DIR", home.join("development"));
    std::env::set_var("TIKTOOLS_PLUGIN_DATA_DIR", home.join("plugin-data"));
    let paths = AppPaths {
        data: home.join("data"),
        plugins: home.join("plugins"),
        plugin_data: home.join("plugin-data"),
        builtin_plugins: home.join("builtins"),
        development_plugins: None,
        logs: home.join("logs"),
        temp: home.join("temp"),
        root: home.clone(),
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
    let server = tokio::spawn(tiktools_control_api::run_ipc_shared(api.clone()));
    let direct = TikToolsClient::direct(api.clone());
    let connected = tokio::time::timeout(Duration::from_secs(5), TikToolsClient::connect())
        .await
        .unwrap()
        .unwrap();
    for (index, client) in [direct, connected].into_iter().enumerate() {
        let id = format!("sdk-{index}");
        let params = json!({"graph":{"id":id,"name":"SDK"}});
        let created = client
            .call_with_meta::<_, Value>(
                "workflows.save",
                params,
                Some(RequestMetadata {
                    expected_revision: Some("0".into()),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
        assert_eq!(created.result["id"], id);
        assert_eq!(created.meta.unwrap().resource_versions[0].revision, "1");
        let read = client
            .call_with_meta::<_, Value>("workflows.get", json!({"id":id}), None)
            .await
            .unwrap();
        assert_eq!(read.meta.unwrap().resource_versions[0].revision, "1");
        core.workflow_set_enabled(&id, true).unwrap();
        let error = client
            .call_value_with_meta(
                "workflows.delete",
                json!({"id":id}),
                Some(RequestMetadata {
                    expected_revision: Some("1".into()),
                    ..Default::default()
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, "revision_conflict");
        assert_eq!(error.data.unwrap()["resourceVersions"][0]["revision"], "2");
        let instance = core.collaboration().unwrap().instance_id().to_owned();
        let metadata = RequestMetadata {
            command_id: Some(format!("sdk-command-{index}")),
            host_instance_id: Some(instance),
            expected_revision: Some("2".into()),
        };
        let deleted = client
            .call_value_with_meta("workflows.delete", json!({"id":id}), Some(metadata.clone()))
            .await
            .unwrap();
        assert_eq!(deleted.result["deleted"], true);
        let receipt = deleted.meta.as_ref().unwrap().command.clone().unwrap();
        assert_eq!(receipt.command_id, format!("sdk-command-{index}"));
        let retry = client
            .call_value_with_meta("workflows.delete", json!({"id":id}), Some(metadata))
            .await
            .unwrap();
        assert_eq!(retry, deleted);
        let outcome = client
            .collaboration_commands_get(tiktools_control_api::modules::collaboration::CommandGet {
                command_id: receipt.command_id,
                principal_id: None,
            })
            .await
            .unwrap();
        assert_eq!(outcome.result["deleted"], true);
        let restore = core
            .workflow_save(serde_json::json!({"id":id,"name":"SDK","enabled":true}))
            .unwrap();
        assert_eq!(restore["id"], id);
        let legacy: Value = client
            .call("workflows.get", json!({"id":id}))
            .await
            .unwrap();
        assert_eq!(legacy["enabled"], true);
        assert!(legacy.get("meta").is_none());
        let viewer = format!("sdk-points-{index}");
        let metadata = RequestMetadata {
            command_id: Some(format!("sdk-points-command-{index}")),
            host_instance_id: Some(core.collaboration().unwrap().instance_id().into()),
            expected_revision: None,
        };
        let adjusted = client
            .call_value_with_meta(
                "points.adjust",
                json!({"uniqueId":viewer,"delta":2}),
                Some(metadata.clone()),
            )
            .await
            .unwrap();
        assert_eq!(adjusted.result["totalPoints"], 2.0);
        assert_eq!(
            adjusted.meta.as_ref().unwrap().resource_versions[0].kind,
            tiktools_core::collaboration::ResourceKind::Viewer
        );
        assert_eq!(
            client
                .call_value_with_meta(
                    "points.adjust",
                    json!({"uniqueId":viewer,"delta":2}),
                    Some(metadata)
                )
                .await
                .unwrap(),
            adjusted
        );
        let points_outcome = client
            .collaboration_commands_get(tiktools_control_api::modules::collaboration::CommandGet {
                command_id: format!("sdk-points-command-{index}"),
                principal_id: None,
            })
            .await
            .unwrap();
        assert_eq!(points_outcome.result["totalPoints"], 2.0);
        let reused = client
            .call_value_with_meta(
                "points.adjust",
                json!({"uniqueId":viewer,"delta":1}),
                Some(RequestMetadata {
                    command_id: Some(format!("sdk-command-{index}")),
                    host_instance_id: Some(core.collaboration().unwrap().instance_id().into()),
                    expected_revision: None,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(reused.code, "command_id_reused");

        // The same generic metadata transport covers actual behavior records,
        // with family-scoped versions and no durable-command advertisement.
        let action = json!({"id":id,"name":"SDK action","enabled":true,"typeId":"core.log"});
        let created = client
            .call_value_with_meta(
                "automation.create",
                json!({"kind":"action","record":action}),
                Some(RequestMetadata {
                    expected_revision: Some("0".into()),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
        let meta = created.meta.unwrap();
        assert_eq!(meta.resource_versions[0].scope, "action");
        assert_eq!(meta.resource_versions[0].revision, "1");
        assert!(meta.command.is_none());
        let read = client
            .call_value_with_meta("automation.get", json!({"kind":"action","id":id}), None)
            .await
            .unwrap();
        assert_eq!(read.result["id"], id);
        assert_eq!(read.meta.unwrap().resource_versions[0].revision, "1");
        core.automation_set_enabled(AutomationKind::Action, &id, false)
            .unwrap();
        let stale = client
            .call_value_with_meta(
                "automation.delete",
                json!({"kind":"action","id":id}),
                Some(RequestMetadata {
                    expected_revision: Some("1".into()),
                    ..Default::default()
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(stale.code, "revision_conflict");
        assert_eq!(stale.data.unwrap()["resourceVersions"][0]["revision"], "2");
        let missing_id = format!("missing-{index}");
        core.automation_create(
            AutomationKind::Event,
            json!({"id":id,"name":"SDK review event","enabled":false,"actionIds":[id,missing_id,id],"provenance":{"authority":{"kind":"delegated","principalId":"forged","sessionId":"forged"},"generation":"999"}}),
        )
        .unwrap();
        let reviewed = client
            .automation_review(AutomationGetParams {
                kind: Some("events".into()),
                id: format!(" {id} "),
            })
            .await
            .unwrap();
        assert_eq!(reviewed.result.id, id);
        assert_eq!(reviewed.result.record.as_ref().unwrap()["enabled"], false);
        let provenance = reviewed.result.provenance.as_ref().unwrap();
        assert!(matches!(
            provenance.authority,
            AutomationReviewAuthority::Owner
        ));
        assert_eq!(provenance.generation, "1");
        assert_eq!(reviewed.result.dependencies.len(), 2);
        assert_eq!(reviewed.result.dependencies[0].id, id);
        assert_eq!(
            reviewed.result.dependencies[0].record.as_ref().unwrap()["enabled"],
            false
        );
        assert_eq!(reviewed.result.dependencies[1].id, missing_id);
        assert!(reviewed.result.dependencies[1].record.is_none());
        assert!(reviewed.result.dependencies[1].provenance.is_none());
        assert_eq!(
            serde_json::to_value(&reviewed.meta.unwrap().resource_versions).unwrap(),
            json!([
                {"kind":"automation","scope":"event","id":id,"revision":"1"},
                {"kind":"automation","scope":"action","id":id,"revision":"2"},
                {"kind":"automation","scope":"action","id":missing_id,"revision":"0"}
            ])
        );
        let manifest = client
            .automation_execution_manifest_get(AutomationManifestParams {
                kind: AutomationKind::Event,
                id: id.clone(),
            })
            .await
            .unwrap();
        assert_eq!(manifest.result["executionAuthorized"], false);
        assert_eq!(manifest.result["subjects"], json!([]));
        assert_eq!(manifest.result["dependencies"].as_array().unwrap().len(), 2);
        assert_eq!(
            serde_json::to_value(&manifest.meta.unwrap().resource_versions).unwrap(),
            json!([
                {"kind":"automation","scope":"event","id":id,"revision":"1"},
                {"kind":"automation","scope":"action","id":id,"revision":"2"},
                {"kind":"automation","scope":"action","id":missing_id,"revision":"0"}
            ])
        );
        let deleted = client
            .call_value_with_meta(
                "automation.delete",
                json!({"kind":"action","id":id}),
                Some(RequestMetadata {
                    expected_revision: Some("2".into()),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
        assert_eq!(deleted.result["deleted"], true);
        assert_eq!(deleted.meta.unwrap().resource_versions[0].revision, "3");
        let missing = client
            .call_value_with_meta("automation.get", json!({"kind":"action","id":id}), None)
            .await
            .unwrap_err();
        assert_eq!(missing.code, "automation_not_found");
        assert_eq!(
            missing.data.unwrap()["resourceVersions"][0]["revision"],
            "3"
        );
        let tombstone = client
            .automation_review(AutomationGetParams {
                kind: Some("action".into()),
                id: id.clone(),
            })
            .await
            .unwrap();
        assert!(tombstone.result.record.is_none());
        assert!(tombstone.result.dependencies.is_empty());
        assert!(matches!(
            tombstone.result.provenance.as_ref().unwrap().authority,
            AutomationReviewAuthority::Owner
        ));
        assert_eq!(tombstone.meta.unwrap().resource_versions[0].revision, "3");
    }
    core.shutdown().await;
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    drop(api);
    drop(core);
    std::fs::remove_dir_all(home).unwrap();
}
