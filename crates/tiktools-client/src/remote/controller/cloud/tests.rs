use super::network;
use super::*;
use crate::remote::{
    profiles::{CredentialStore, ProfileManager},
    SecretString,
};
struct NoBearer;
impl CredentialStore for NoBearer {
    fn load(&self, _: &str) -> Result<Option<SecretString>, ClientError> {
        panic!("Cloud controller status must not load host bearer credentials")
    }
    fn save(&self, _: &str, _: &SecretString) -> Result<(), ClientError> {
        panic!("Cloud controller metadata must not save host bearer credentials")
    }
    fn delete(&self, _: &str) -> Result<(), ClientError> {
        panic!("Cloud controller metadata must not delete host bearer credentials")
    }
}
fn native() -> (tempfile::TempDir, Arc<Controller>) {
    let home = tempfile::tempdir().unwrap();
    let profiles = ProfileManager::new(home.path().join("profiles"), Arc::new(NoBearer)).unwrap();
    let controller = Arc::new(Controller::with_cloud_environment(profiles).unwrap());
    (home, controller)
}
#[tokio::test]
async fn cloud_management_is_nonsecret_and_never_constructs_a_local_host() {
    let (home, controller) = native();
    assert!(controller.status().cloud_supported);
    let status = controller
        .manage("controller.cloud.status", json!({}))
        .await
        .unwrap();
    assert!(status.get("account").is_some());
    assert!(status["deviceId"].is_null());
    let raw = serde_json::to_string(&status).unwrap();
    assert!(!raw.contains("accessToken") && !raw.contains("refreshToken"));
    assert!(controller
        .manage(
            "controller.cloud.login.open",
            json!({"operationId":"not-current-login"})
        )
        .await
        .is_err());
    assert_eq!(
        controller
            .manage(
                "controller.cloud.join",
                json!({"profile":"../bad","label":"Desktop","token":"private-marker"})
            )
            .await
            .unwrap_err()
            .code,
        "invalid_params"
    );
    assert_eq!(
        controller
            .manage("controller.cloud.unknown", json!({}))
            .await
            .unwrap_err()
            .code,
        "method_not_found"
    );
    for path in ["data", "plugins", "plugin-data"] {
        assert!(!home.path().join(path).exists());
    }
}
#[tokio::test]
async fn disconnect_cancels_cloud_network_work_before_waiting_for_lifecycle_serialization() {
    let (_home, controller) = native();
    let lifecycle = controller.lifecycle.lock().await;
    let pending = network::begin(
        &controller.cloud_network,
        network::generation(&controller.cloud_network).unwrap(),
    )
    .unwrap();
    let other = controller.clone();
    let disconnect = tokio::spawn(async move {
        other.disconnect().await;
    });
    tokio::time::timeout(Duration::from_secs(2), pending.token.cancelled())
        .await
        .unwrap();
    assert!(!disconnect.is_finished());
    drop(lifecycle);
    disconnect.await.unwrap();
    assert_eq!(controller.status().phase, "idle");
    assert!(controller.status().cloud_pairing.is_none());
}
