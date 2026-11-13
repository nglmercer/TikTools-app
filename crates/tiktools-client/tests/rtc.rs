#![cfg(feature = "peer")]
//! Business RPC uses the same engine after signed native WebRTC admission.
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tiktools_client::{remote::RemoteClient, RequestMetadata};
use tiktools_collaboration::{
    Capability, ClientKind, CloudPeerGrant, CloudPeerSource, Configuration, Grant,
    PeerConfiguration, ResourceKind,
};
use tiktools_control_api::ControlApi;
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};
use tiktools_rtc::{
    negotiation::{AuthenticatedChannel, Event, Negotiation},
    signaling::{Context, ForwardedSignal},
    Config,
};
use uuid::Uuid;
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
fn forwarded(frame: Value, context: &Context, endpoint: &str) -> ForwardedSignal {
    let mut value = serde_json::to_value(context).unwrap();
    for (key, item) in frame.as_object().unwrap() {
        value[key] = item.clone();
    }
    value["fromEndpointId"] = json!(endpoint);
    value["expiresAt"] = json!(9999999999999_i64);
    serde_json::from_value(value).unwrap()
}
async fn channels(
    host_key: &tiktools_peer::SecretKey,
    client_key: &tiktools_peer::SecretKey,
    context: Context,
) -> (AuthenticatedChannel, AuthenticatedChannel) {
    let mut host_context = context.clone();
    host_context.from_device_id = context.host_device_id;
    let config = || Config {
        bind_addresses: vec!["127.0.0.1:0".into()],
        ..Default::default()
    };
    let mut client = Negotiation::new(
        config(),
        context.clone(),
        &client_key.to_bytes(),
        context.host_device_id,
        *host_key.public().as_bytes(),
    )
    .await
    .unwrap();
    let mut host = Negotiation::new(
        config(),
        host_context.clone(),
        &host_key.to_bytes(),
        context.from_device_id,
        *client_key.public().as_bytes(),
    )
    .await
    .unwrap();
    let offer = client.offer().await.unwrap();
    let answer = host
        .apply(forwarded(offer, &context, &client_key.public().to_string()))
        .await
        .unwrap()
        .unwrap();
    client
        .apply(forwarded(
            answer,
            &host_context,
            &host_key.public().to_string(),
        ))
        .await
        .unwrap();
    let (mut a, mut b) = (None, None);
    tokio::time::timeout(Duration::from_secs(20),async {
        while a.is_none()||b.is_none(){
            tokio::select! {
                event=client.next(),if a.is_none()=>match event.unwrap(){
                    Event::Signal(frame)=>{host.apply(forwarded(frame,&context,&client_key.public().to_string())).await.unwrap();},
                    Event::Connected(channel)=>a=Some(*channel),
                },
                event=host.next(),if b.is_none()=>match event.unwrap(){
                    Event::Signal(frame)=>{client.apply(forwarded(frame,&host_context,&host_key.public().to_string())).await.unwrap();},
                    Event::Connected(channel)=>b=Some(*channel),
                },
            }
        }
    }).await.expect("signed WebRTC negotiation");
    (a.unwrap(), b.unwrap())
}
#[tokio::test]
async fn rtc_rpc_preserves_local_scopes_receipts_heartbeat_and_revocation() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().to_path_buf();
        let paths = tiktools_core::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root,
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
            Arc::new(tiktools_core::NoopMediaHost),
            paths,
        ));
        let service = core.collaboration().unwrap();
        service
            .configure(Configuration {
                enabled: true,
                peer: Some(PeerConfiguration::default()),
                ..Default::default()
            })
            .unwrap();
        let host_id = Uuid::new_v4();
        let host_device = Uuid::new_v4();
        let client_device = Uuid::new_v4();
        let source = CloudPeerSource {
            origin: "https://cloud.example".into(),
            account_id: "owner".into(),
            host_id: host_id.to_string(),
        };
        let client_key = tiktools_peer::SecretKey::generate();
        let identity = service
            .apply_cloud_peer_grant(CloudPeerGrant {
                origin: source.origin.clone(),
                account_id: source.account_id.clone(),
                host_id: source.host_id.clone(),
                device_id: client_device.to_string(),
                grant_id: Uuid::new_v4().to_string(),
                cloud_revision: 1,
                endpoint_id: client_key.public().to_string(),
                label: "Headless RTC moderator".into(),
                client_kind: ClientKind::Agent,
                grant: Grant {
                    capabilities: [
                        Capability::PointsRead,
                        Capability::PointsAdjust,
                        Capability::CommandsRead,
                    ]
                    .into(),
                    resources: [(ResourceKind::Viewer, ["alice".into()].into())].into(),
                    max_points_delta: Some(10.0),
                    ..Default::default()
                },
            })
            .unwrap();
        let api = Arc::new(ControlApi::new(core.clone()));
        assert!(tiktools_remote::RtcGateway::start(
            api.clone(),
            source.clone(),
            host_device.to_string()
        )
        .await
        .is_err());
        assert!(
            !home
                .path()
                .join("data/collaboration-peer/identity.key")
                .exists(),
            "admission must never replace a missing registered identity"
        );
        let host_key = tiktools_peer::load_or_create_key(
            &home.path().join("data/collaboration-peer/identity.key"),
        )
        .unwrap();
        let gateway = Arc::new(
            tiktools_remote::RtcGateway::start(
                api.clone(),
                source.clone(),
                host_device.to_string(),
            )
            .await
            .unwrap(),
        );

        assert_eq!(gateway.endpoint_id(), host_key.public().to_string());
        let context = Context {
            host_id,
            host_device_id: host_device,
            from_device_id: client_device,
            session_id: Uuid::new_v4(),
            negotiation_id: Uuid::new_v4(),
            grant_revision: 1,
        };
        let unknown = tiktools_peer::SecretKey::generate();
        let (unapproved, unapproved_host) = channels(&host_key, &unknown, context.clone()).await;
        assert!(
            gateway.serve(unapproved_host).await.is_err(),
            "signed transport identity must not create a grant"
        );
        drop(unapproved);
        let mut stale = context.clone();
        stale.grant_revision = 2;
        stale.negotiation_id = Uuid::new_v4();
        let (mismatch, mismatch_host) = channels(&host_key, &client_key, stale).await;
        assert!(
            gateway.serve(mismatch_host).await.is_err(),
            "signed revision must match the local import"
        );
        drop(mismatch);
        let (idle_client, idle_host) = channels(&host_key, &client_key, context.clone()).await;
        let idle_server = {
            let gateway = gateway.clone();
            tokio::spawn(async move { gateway.serve(idle_host).await })
        };
        tokio::time::timeout(Duration::from_secs(3), async {
            while service.peer_gateway_status().unwrap().gateway.connections == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        gateway.stop();
        tokio::time::timeout(Duration::from_secs(1), idle_server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(!service.peer_gateway_status().unwrap().gateway.running);
        drop(idle_client);
        drop(gateway);
        let gateway = Arc::new(
            tiktools_remote::RtcGateway::start(api, source, host_device.to_string())
                .await
                .unwrap(),
        );
        let (client, host) = channels(&host_key, &client_key, context.clone()).await;
        let server = {
            let gateway = gateway.clone();
            tokio::spawn(async move { gateway.serve(host).await })
        };
        let remote = RemoteClient::connect_rtc(
            client,
            tiktools_client::remote::ClientKind::Agent,
            Some(service.host_id()),
        )
        .await
        .unwrap();
        assert_eq!(remote.hello().principal_id, identity.principal_id);
        let meta = RequestMetadata {
            command_id: Some("rtc-once".into()),
            host_instance_id: Some(remote.hello().host_instance_id.clone()),
            ..Default::default()
        };
        let first = remote
            .call_value_with_meta(
                "points.adjust",
                json!({"uniqueId":"alice","delta":3}),
                Some(meta.clone()),
            )
            .await
            .unwrap();
        let retry = remote
            .call_value_with_meta(
                "points.adjust",
                json!({"uniqueId":"alice","delta":3}),
                Some(meta),
            )
            .await
            .unwrap();
        assert_eq!(first, retry);
        assert_eq!(core.points_viewer("alice").unwrap()["points"], 3.0);
        let receipt = remote
            .call_value_with_meta(
                "collaboration.commands.get",
                json!({"commandId":"rtc-once"}),
                None,
            )
            .await
            .unwrap();
        assert_eq!(receipt.result["principalId"], identity.principal_id);
        assert_eq!(receipt.result["result"]["totalPoints"], 3.0);
        let denied = remote
            .call_value_with_meta("points.viewer.get", json!({"uniqueId":"bob"}), None)
            .await
            .unwrap_err();
        assert_eq!(denied.code, "forbidden");
        tokio::time::sleep(Duration::from_secs(11)).await;
        assert!(!remote.is_closed());
        remote
            .call_value_with_meta("points.viewer.get", json!({"uniqueId":"alice"}), None)
            .await
            .unwrap();
        service.revoke_principal(&identity.principal_id).unwrap();
        assert!(remote
            .call_value_with_meta("points.adjust", json!({"uniqueId":"alice","delta":3}), None)
            .await
            .is_err());
        assert_eq!(core.points_viewer("alice").unwrap()["points"], 3.0);
        remote.close();
        gateway.stop();
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        core.shutdown().await;
    })
    .await
    .unwrap();
}
