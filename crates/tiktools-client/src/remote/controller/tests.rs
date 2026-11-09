use super::*;
use crate::remote::{
    profiles::CredentialStore,
    tests::{workflow_grant, Fixture},
};
use std::sync::Mutex as StdMutex;
#[derive(Default)]
struct Store(StdMutex<BTreeMap<String, SecretString>>);
impl CredentialStore for Store {
    fn load(&self, k: &str) -> Result<Option<SecretString>, ClientError> {
        Ok(self.0.lock().unwrap().get(k).cloned())
    }
    fn save(&self, k: &str, s: &SecretString) -> Result<(), ClientError> {
        self.0.lock().unwrap().insert(k.into(), s.clone());
        Ok(())
    }
    fn delete(&self, k: &str) -> Result<(), ClientError> {
        self.0.lock().unwrap().remove(k);
        Ok(())
    }
}
async fn wait(c: &Controller, predicate: impl Fn(&ControllerStatus) -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if predicate(&c.status()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "controller did not reach expected state: {}",
            serde_json::to_value(c.status()).unwrap()
        )
    });
}
fn page(c: &Controller) -> Value {
    let s = c.status();
    c.workflows(&s.epoch, &s.view_revision, 0).unwrap()
}

#[tokio::test]
async fn forget_requires_confirmation_and_fences_active_and_replaced_profiles() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = manager
        .install(
            "desk",
            &f.endpoint,
            f.pair(workflow_grant()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    let controller = Controller::new(manager.clone());
    let request = json!({"profile":"desk","profileId":profile.profile_id,"origin":profile.origin,"hostId":profile.host_id,"confirm":true});
    let mut unconfirmed = request.clone();
    unconfirmed["confirm"] = json!(false);
    assert_eq!(
        controller
            .manage("controller.profile.forget", unconfirmed)
            .await
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    let listed = controller
        .manage("controller.profiles", json!({}))
        .await
        .unwrap();
    assert_eq!(listed[0]["profileId"], request["profileId"]);
    controller.connect("desk").await.unwrap();
    assert_eq!(
        controller
            .manage("controller.profile.forget", request.clone())
            .await
            .unwrap_err()
            .code,
        "profile_in_use"
    );
    let other = Controller::new(manager.clone());
    assert_eq!(
        other
            .manage("controller.profile.forget", request.clone())
            .await
            .unwrap_err()
            .code,
        "command_journal_in_use"
    );
    controller.disconnect().await;
    // Direct SDK connections do not hold the controller journal lease. Local
    // removal cannot revoke an access token already in another process.
    let remote = manager.open("desk").await.unwrap();
    controller
        .manage("controller.profile.forget", request.clone())
        .await
        .unwrap();
    assert!(!remote.is_closed());
    remote
        .call_value_with_meta("collaboration.me", json!({}), None)
        .await
        .unwrap();
    remote.close();
    let replacement = manager
        .install(
            "desk",
            &f.endpoint,
            f.pair(workflow_grant()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    assert_ne!(replacement.profile_id, profile.profile_id);
    assert_eq!(
        controller
            .manage("controller.profile.forget", request)
            .await
            .unwrap_err()
            .code,
        "stale_view"
    );
    assert_eq!(
        manager.get("desk").await.unwrap().profile_id,
        replacement.profile_id
    );
    f.finish().await;
}
#[tokio::test]
async fn controller_projects_only_authorized_host_data_and_recovers_without_local_core() {
    let f = Fixture::new().await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Initial"}))
        .unwrap();
    f.core
        .workflow_save(json!({"id":"hidden","name":"Secret"}))
        .unwrap();
    let credentials = f.pair(workflow_grant()).await;
    let client_home = tempfile::tempdir().unwrap();
    let manager = ProfileManager::new(
        client_home.path().join("profiles"),
        Arc::new(Store::default()),
    )
    .unwrap();
    manager
        .install("desk", &f.endpoint, credentials, ClientKind::Agent)
        .await
        .unwrap();
    let c = Controller::new(manager);
    c.connect("desk").await.unwrap();
    let before = c.status();
    let value = page(&c);
    assert_eq!(value["rows"].as_array().unwrap().len(), 1);
    assert_eq!(value["rows"][0]["graph"]["name"], "Initial");
    assert!(
        !serde_json::to_string(&c.manage("controller.status", json!({})).await.unwrap())
            .unwrap()
            .contains("accessToken")
    );
    assert_eq!(
        c.forward(&before.epoch, "system.shutdown", json!({}), None)
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert_eq!(
        c.manage("local.owner", json!({})).await.unwrap_err().code,
        "method_not_found"
    );
    f.core
        .workflow_save(json!({"id":"welcome","name":"Changed"}))
        .unwrap();
    wait(&c, |s| s.view_revision != before.view_revision).await;
    assert_eq!(page(&c)["rows"][0]["graph"]["name"], "Changed");
    assert_eq!(
        c.workflows(&before.epoch, &before.view_revision, 0)
            .unwrap_err()
            .code,
        "stale_view"
    );
    let remote = c.shared.view.read().unwrap().remote.clone().unwrap();
    remote.close();
    assert_eq!(c.status().phase, "reconnecting");
    let s = c.status();
    assert_eq!(
        c.workflows(&s.epoch, &s.view_revision, 0).unwrap_err().code,
        "not_ready"
    );
    wait(&c, |s| s.phase == "ready").await;
    f.core.workflow_delete("welcome").unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if page(&c)["rows"].as_array().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    c.disconnect().await;
    assert_eq!(
        c.forward(&before.epoch, "live.status", json!({}), None)
            .await
            .unwrap_err()
            .code,
        "not_ready"
    );
    let entries: Vec<_> = std::fs::read_dir(client_home.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("profiles")]);
    f.finish().await;
}
#[tokio::test]
async fn selecting_an_unavailable_profile_closes_previous_host_and_clears_projection() {
    let f = Fixture::new().await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Host A"}))
        .unwrap();
    let credentials = f.pair(workflow_grant()).await;
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    manager
        .install("desk", &f.endpoint, credentials, ClientKind::Agent)
        .await
        .unwrap();
    let c = Controller::new(manager);
    c.connect("desk").await.unwrap();
    let prior = c.status();
    let remote = c.shared.view.read().unwrap().remote.clone().unwrap();
    assert!(c.connect("missing").await.is_err());
    assert!(remote.is_closed());
    assert_eq!(c.status().phase, "failed");
    assert!(c.status().hello.is_none());
    assert_eq!(
        c.workflows(&prior.epoch, &prior.view_revision, 0)
            .unwrap_err()
            .code,
        "stale_view"
    );
    assert!(c.shared.view.read().unwrap().rows.is_empty());
    assert_eq!(
        c.forward(&c.status().epoch, "live.status", json!({}), None)
            .await
            .unwrap_err()
            .code,
        "not_ready"
    );
    f.finish().await;
}
#[tokio::test]
async fn desktop_pairing_consumes_private_invitation_only_after_explicit_trust_and_owner_approval()
{
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let c = Controller::new(
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap(),
    );
    let invite = f
        .service
        .create_invitation(tiktools_collaboration::InviteCreate {
            grant: workflow_grant(),
        })
        .unwrap();
    let mut params = json!({"profile":"new","origin":f.origin,"invitation":serde_json::to_string(&invite).unwrap(),"label":"Desktop","fingerprint":f.pin,"confirmPin":false});
    assert_eq!(
        c.manage("controller.pair", params.clone())
            .await
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    params["confirmPin"] = json!(true);
    let pending = c.manage("controller.pair", params).await.unwrap();
    assert_eq!(c.status().phase, "pairing");
    assert_eq!(
        c.manage("controller.pair.poll", json!({})).await.unwrap()["approved"],
        false
    );
    f.service
        .decide_request(pending["requestId"].as_str().unwrap(), true)
        .unwrap();
    let approved = c.manage("controller.pair.poll", json!({})).await.unwrap();
    assert_eq!(approved, json!({"approved":true,"profile":"new"}));
    c.connect("new").await.unwrap();
    assert_eq!(c.status().phase, "ready");
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn removing_and_restoring_workflow_permission_rebuilds_under_new_grants() {
    let f = Fixture::new().await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Allowed"}))
        .unwrap();
    let credentials = f.pair(workflow_grant()).await;
    let principal = credentials.principal_id().to_string();
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    manager
        .install("desk", &f.endpoint, credentials, ClientKind::Agent)
        .await
        .unwrap();
    let c = Controller::new(manager);
    c.connect("desk").await.unwrap();
    assert_eq!(page(&c)["rows"].as_array().unwrap().len(), 1);
    f.service
        .set_grant(tiktools_collaboration::GrantSet {
            principal_id: principal.clone(),
            expected_revision: "1".into(),
            grant: tiktools_collaboration::Grant {
                capabilities: [Capability::LiveRead].into(),
                ..Default::default()
            },
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "2")
    })
    .await;
    assert!(page(&c)["rows"].as_array().unwrap().is_empty());
    f.service
        .set_grant(tiktools_collaboration::GrantSet {
            principal_id: principal,
            expected_revision: "2".into(),
            grant: workflow_grant(),
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "3")
    })
    .await;
    assert_eq!(page(&c)["rows"][0]["graph"]["name"], "Allowed");
    c.disconnect().await;
    f.finish().await;
}

mod automation;
mod proposals;
