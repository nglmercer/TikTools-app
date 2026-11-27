//! Real direct/IPC library calls preserve reviewed snapshots and original receipts.
mod support;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tiktools_client::{
    automation_library::{OwnerLibrarySnapshot, OwnerLibrarySubmit},
    TikToolsClient,
};
use tiktools_core::control::AutomationKind;
fn input(snapshot: &OwnerLibrarySnapshot, mutation: Value) -> OwnerLibrarySubmit {
    serde_json::from_value(json!({"mutation":mutation,"expectedRevision":snapshot.revision,"definitionDigest":snapshot.definition_digest,"hostId":snapshot.host_id,"hostInstanceId":snapshot.host_instance_id})).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn library_sdk_preserves_large_owner_transactions_and_metadata_only_recovery_over_ipc() {
    let temporary = tempfile::tempdir().unwrap();
    let support::IpcHost { core, api, server } =
        support::start_ipc_host(temporary.path(), "library-sdk").await;
    let connected = tokio::time::timeout(Duration::from_secs(5), TikToolsClient::connect())
        .await
        .unwrap()
        .unwrap();
    for (index, client) in [TikToolsClient::direct(api), connected]
        .into_iter()
        .enumerate()
    {
        let profile = format!("sdk-{index}");
        let snapshot = client.automation_library_snapshot().await.unwrap();
        let created=client.automation_library_submit(input(&snapshot,json!({"operation":"create-profile","id":profile,"name":"SDK profile","description":""}))).await.unwrap();
        assert!(created.acknowledgement.is_some());
        let snapshot = client.automation_library_snapshot().await.unwrap();
        let actions:Vec<_>=(0..8).map(|n|json!({"id":format!("sdk-{index}-action-{n}"),"name":"Large SDK action","enabled":false,"typeId":"core.log","config":{"message":"s".repeat(170_000),"large":u64::MAX,"float":1.0,"tiny":0.000001,"bigFloat":1e21,"":"bmp","𐀀":"supplementary"}})).collect();
        let event_id = format!("sdk-{index}-event");
        let entry = json!({"actions":actions,"event":{"id":event_id,"name":"SDK event","enabled":false,"trigger":"tiktok.chat","filters":[],"actionIds":actions.iter().map(|a|a["id"].clone()).collect::<Vec<_>>()}});
        let request = input(
            &snapshot,
            json!({"operation":"apply-template","entry":entry,"profileId":profile}),
        );
        assert!(
            serde_json::to_vec(&request).unwrap().len() > tiktools_control_api::MAX_REQUEST_BYTES
        );
        let result = client.automation_library_submit(request).await.unwrap();
        let ack = result.acknowledgement.unwrap();
        assert_eq!(ack.resource_versions[0].scope, "library");
        assert_eq!(ack.resource_versions.len(), 10);
        let original = ack.command.command_id;
        let snapshot = client.automation_library_snapshot().await.unwrap();
        assert!(
            serde_json::to_vec(&snapshot).unwrap().len() > tiktools_control_api::MAX_REQUEST_BYTES
        );
        let source = snapshot.definitions["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == format!("sdk-{index}-action-0"))
            .unwrap();
        assert_eq!(source["config"]["large"], json!(u64::MAX));
        let mut newer = core
            .automation_get(AutomationKind::Event, &event_id)
            .unwrap();
        newer["name"] = json!("Newer Owner edit");
        core.automation_update(AutomationKind::Event, &event_id, newer)
            .unwrap();
        let recovered = client.automation_library_resolve().await.unwrap();
        assert_eq!(recovered.recent[0].intent.command_id, original);
        assert!(!serde_json::to_string(&recovered)
            .unwrap()
            .contains("Large SDK action"));
        assert_eq!(
            core.automation_get(AutomationKind::Event, &event_id)
                .unwrap()["name"],
            "Newer Owner edit"
        );
    }
    // Reading a preexisting Owner body is not a new 256-KiB source write.
    core.automation_create(AutomationKind::Action,json!({"id":"legacy-large","name":"Legacy source","enabled":false,"typeId":"core.log","config":{"message":"x".repeat(300_000)}})).unwrap();
    let snapshot = TikToolsClient::direct(Arc::new(tiktools_client::ControlApi::new(core.clone())))
        .automation_library_snapshot()
        .await
        .unwrap();
    assert!(snapshot.definitions["actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == "legacy-large"));
    core.shutdown().await;
    server.abort();
    let _ = server.await;
}
