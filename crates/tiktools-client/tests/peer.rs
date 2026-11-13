#![cfg(feature = "peer")]
//! Real native RPC over encrypted P2P, using the existing permission/receipt engine.
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tiktools_client::{remote::RemoteClient, RequestMetadata};
use tiktools_collaboration::{
    Capability, ClientKind, CloudPeerGrant, Configuration, Grant, PeerConfiguration, ResourceKind,
};
use tiktools_control_api::ControlApi;
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};
use tiktools_peer::{Config, PeerEndpoint, SecretKey};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

#[cfg(all(feature = "remote-profiles", not(feature = "cloud-peer")))]
struct NoBearerStore;
#[cfg(all(feature = "remote-profiles", not(feature = "cloud-peer")))]
impl tiktools_client::remote::profiles::CredentialStore for NoBearerStore {
    fn load(
        &self,
        _: &str,
    ) -> Result<Option<tiktools_client::remote::SecretString>, tiktools_client::ClientError> {
        panic!("P2P must not load bearer tokens")
    }
    fn save(
        &self,
        _: &str,
        _: &tiktools_client::remote::SecretString,
    ) -> Result<(), tiktools_client::ClientError> {
        panic!("P2P must not store bearer tokens")
    }
    fn delete(&self, _: &str) -> Result<(), tiktools_client::ClientError> {
        panic!("P2P has no bearer token to delete")
    }
}

#[tokio::test]
async fn peer_rpc_preserves_scopes_receipts_heartbeat_and_immediate_host_revocation() {
    tokio::time::timeout(Duration::from_secs(40), async {
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
        for db in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(db).unwrap();
        }
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(tiktools_core::NoopMediaHost),
            paths,
        ));
        let api = Arc::new(ControlApi::new(core.clone()));
        let service = core.collaboration().unwrap();
        service
            .configure(Configuration {
                enabled: true,
                peer: Some(PeerConfiguration::default()),
                ..Configuration::default()
            })
            .unwrap();
        let key_path = home.path().join("client/identity.key");
        let client = PeerEndpoint::open(&key_path, &Config::default())
            .await
            .unwrap();
        let grant = CloudPeerGrant {
            origin: "https://cloud.example".into(),
            account_id: "owner".into(),
            host_id: uuid::Uuid::new_v4().to_string(),
            device_id: uuid::Uuid::new_v4().to_string(),
            grant_id: uuid::Uuid::new_v4().to_string(),
            cloud_revision: 1,
            endpoint_id: client.id().to_string(),
            label: "Headless moderator".into(),
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
                ..Grant::default()
            },
        };
        let identity = service.apply_cloud_peer_grant(grant).unwrap();
        let gateway = tiktools_remote::start_peer(api).await.unwrap();
        assert!(service
            .status()
            .unwrap()
            .configuration
            .advertised_origin
            .is_none());
        let stranger = PeerEndpoint::bind(SecretKey::generate(), &Config::default())
            .await
            .unwrap();
        assert!(RemoteClient::connect_peer(
            stranger.clone(),
            gateway.address(),
            ClientKind::Agent,
            None
        )
        .await
        .is_err());
        stranger.close().await;
        let remote = RemoteClient::connect_peer(
            client.clone(),
            gateway.address(),
            ClientKind::Agent,
            Some(service.host_id()),
        )
        .await
        .unwrap();
        #[cfg(all(feature = "remote-profiles", not(feature = "cloud-peer")))]
        let mut remote = remote;
        let meta = RequestMetadata {
            command_id: Some("peer-adjust-once".into()),
            host_instance_id: Some(remote.hello().host_instance_id.clone()),
            ..RequestMetadata::default()
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
                json!({"commandId":"peer-adjust-once"}),
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
        // Cross the host's 10-second authority heartbeat. P2P must use its local
        // endpoint binding, not an empty bearer or a Cloud network request.
        tokio::time::sleep(Duration::from_secs(11)).await;
        assert!(!remote.is_closed());
        let read = remote
            .call_value_with_meta("points.viewer.get", json!({"uniqueId":"alice"}), None)
            .await
            .unwrap();
        assert_eq!(read.result["viewer"]["points"], 3.0);
        #[cfg(all(feature = "remote-profiles", not(feature = "cloud-peer")))]
        {
            use tiktools_client::remote::profiles::{PeerProfile, ProfileManager};
            let profiles =
                ProfileManager::new(home.path().join("profiles"), Arc::new(NoBearerStore)).unwrap();
            let metadata = PeerProfile {
                key_path: key_path.clone(),
                local_endpoint_id: client.id().to_string(),
                remote_address: gateway.address(),
                relay_urls: Vec::new(),
                cloud_host_id: uuid::Uuid::new_v4().to_string(),
                host_device_id: None,
            };
            let saved = profiles
                .install_peer(
                    "peer",
                    "https://cloud.example",
                    metadata,
                    &remote,
                    ClientKind::Agent,
                )
                .await
                .unwrap();
            assert_eq!(saved.version, 2);
            remote.close();
            client.close().await;
            remote = profiles.open("peer").await.unwrap();
            let recovered = remote
                .call_value_with_meta(
                    "collaboration.commands.get",
                    json!({"commandId":"peer-adjust-once"}),
                    None,
                )
                .await
                .unwrap();
            assert_eq!(recovered.result["result"]["totalPoints"], 3.0);
            let replacement = remote.reopen().await.unwrap();
            remote.close();
            remote = replacement;
            assert_eq!(remote.hello().principal_id, identity.principal_id);
            let json = std::fs::read_to_string(profiles.directory().join("peer.json")).unwrap();
            assert!(!json.contains("accessToken") && !json.contains("refreshToken"));
            remote.close();
            let controller = tiktools_client::remote::controller::Controller::new(profiles.clone());
            controller.connect("peer").await.unwrap();
            assert_eq!(
                controller.status().transport_route.as_deref(),
                Some("direct")
            );
            controller.disconnect().await;
            assert_eq!(controller.status().transport_route, None);
            remote = profiles.open("peer").await.unwrap();
            profiles.forget("peer").await.unwrap();
        }
        service.revoke_principal(&identity.principal_id).unwrap();
        assert!(remote
            .call_value_with_meta("points.viewer.get", json!({"uniqueId":"alice"}), None)
            .await
            .is_err());
        remote.close();
        client.close().await;
        gateway.shutdown().await;
        assert!(!service.peer_gateway_status().unwrap().gateway.running);
        core.shutdown().await;
    })
    .await
    .unwrap();
}
