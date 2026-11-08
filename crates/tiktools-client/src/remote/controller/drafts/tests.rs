use super::*;
use crate::remote::{profiles::CredentialStore, tests::Fixture};
use std::sync::Mutex as StdMutex;
#[derive(Default)]
struct Store(StdMutex<BTreeMap<String, SecretString>>);
impl CredentialStore for Store {
    fn load(&self, k: &str) -> Result<Option<SecretString>, ClientError> {
        Ok(self.0.lock().unwrap().get(k).cloned())
    }
    fn save(&self, k: &str, v: &SecretString) -> Result<(), ClientError> {
        self.0.lock().unwrap().insert(k.into(), v.clone());
        Ok(())
    }
    fn delete(&self, k: &str) -> Result<(), ClientError> {
        self.0.lock().unwrap().remove(k);
        Ok(())
    }
}
fn grant() -> tiktools_collaboration::Grant {
    tiktools_collaboration::Grant {
        capabilities: [Capability::WorkflowsRead].into(),
        resources: [(ResourceKind::Workflow, ["welcome".into()].into())].into(),
        ..Default::default()
    }
}
async fn call(c: &Controller, method: &str, mut p: Value) -> Result<Value, ClientError> {
    let s = c.status();
    p["epoch"] = s.epoch.into();
    p["viewRevision"] = s.view_revision.into();
    c.manage(method, p).await
}
async fn wait(c: &Controller, p: impl Fn(&ControllerStatus) -> bool) {
    tokio::time::timeout(Duration::from_secs(7), async {
        while !p(&c.status()) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn explicit_native_drafts_survive_reopen_stay_local_and_follow_live_read_grants() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("profiles");
    f.core
        .workflow_save(json!({"id":"welcome","name":"Host original","enabled":true}))
        .unwrap();
    let m = ProfileManager::new(root.clone(), Arc::new(Store::default())).unwrap();
    let profile = m
        .install(
            "desk",
            &f.endpoint,
            f.pair(grant()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    let path = root
        .join("commands")
        .join(format!("{}.drafts", profile.profile_id));
    let c = Controller::new(m.clone());
    c.connect("desk").await.unwrap();
    for (method, params) in [
        ("controller.drafts.get", json!({})),
        ("controller.drafts.list", json!({"id":"welcome"})),
        ("controller.drafts.list", json!({"offset":u64::MAX})),
    ] {
        assert_eq!(
            call(&c, method, params).await.unwrap_err().code,
            "invalid_params"
        );
    }
    assert_eq!(
        call(&c, "controller.drafts.get", json!({"id":"hidden"}))
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert!(
        call(&c, "controller.drafts.list", json!({})).await.unwrap()["rows"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!path.exists());
    let mut save = json!({"id":"welcome","expectedStoreRevision":"0","baseRevision":"1","text":"unfinished private local text"});
    assert_eq!(
        call(&c, "controller.drafts.save", save.clone())
            .await
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    assert!(!path.exists());
    save["acknowledgeStorage"] = true.into();
    assert_eq!(
        call(&c, "controller.drafts.save", save).await.unwrap()["storeRevision"],
        "1"
    );
    c.disconnect().await;
    drop(c);
    let c = Controller::new(m.clone());
    c.connect("desk").await.unwrap();
    let d = call(&c, "controller.drafts.get", json!({"id":"welcome"}))
        .await
        .unwrap();
    assert_eq!(d["draft"]["text"], "unfinished private local text");
    assert_eq!(d["draft"]["baseRevision"], "1");
    assert_eq!(
        f.core.workflow_get("welcome").unwrap()["name"],
        "Host original"
    );
    assert!(c.status().command_journal.unwrap().recent.is_empty());
    f.core.workflow_delete("welcome").unwrap();
    wait(&c, |s| {
        c.workflows(&s.epoch, &s.view_revision, 0)
            .is_ok_and(|v| v["rows"].as_array().is_some_and(Vec::is_empty))
    })
    .await;
    let list = call(&c, "controller.drafts.list", json!({})).await.unwrap();
    assert_eq!(list["rows"][0]["id"], "welcome");
    assert!(list["rows"][0].get("text").is_none());
    let mut narrowed = grant();
    narrowed
        .resources
        .insert(ResourceKind::Workflow, Default::default());
    f.service
        .set_grant(tiktools_collaboration::GrantSet {
            principal_id: profile.principal_id.clone(),
            expected_revision: "1".into(),
            grant: narrowed,
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "2")
    })
    .await;
    assert!(
        call(&c, "controller.drafts.list", json!({})).await.unwrap()["rows"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        call(&c, "controller.drafts.get", json!({"id":"welcome"}))
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert!(path.exists());
    f.service
        .set_grant(tiktools_collaboration::GrantSet {
            principal_id: profile.principal_id,
            expected_revision: "2".into(),
            grant: grant(),
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "3")
    })
    .await;
    assert_eq!(
        call(&c, "controller.drafts.get", json!({"id":"welcome"}))
            .await
            .unwrap()["draft"]["text"],
        "unfinished private local text"
    );
    c.disconnect().await;
    drop(c);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(m.forget("desk").await.is_err());
        assert!(
            m.get("desk").await.is_ok(),
            "a failed draft cleanup must leave the profile available for retry"
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    m.forget("desk").await.unwrap();
    assert!(!path.exists());
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 1);
    f.finish().await;
}

#[tokio::test]
async fn blocked_draft_read_cannot_return_text_after_profile_selection_changes() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let m = ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    m.install(
        "desk",
        &f.endpoint,
        f.pair(grant()).await,
        ClientKind::Agent,
    )
    .await
    .unwrap();
    let c = Arc::new(Controller::new(m));
    c.connect("desk").await.unwrap();
    call(&c,"controller.drafts.save",json!({"id":"welcome","expectedStoreRevision":"0","baseRevision":"1","text":"private saved edit","acknowledgeStorage":true})).await.unwrap();
    let j = c.shared.view.read().unwrap().journal.clone().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let (release, waiter) = std::sync::mpsc::channel();
    let held = j.clone();
    let thread = std::thread::spawn(move || held.hold_writer_for_test(tx, waiter));
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let count = Arc::strong_count(&j);
    let reader = c.clone();
    let task = tokio::spawn(async move {
        call(&reader, "controller.drafts.get", json!({"id":"welcome"})).await
    });
    tokio::time::timeout(Duration::from_secs(2), async {
        while Arc::strong_count(&j) <= count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    c.disconnect().await;
    release.send(()).unwrap();
    thread.join().unwrap();
    assert_eq!(task.await.unwrap().unwrap_err().code, "stale_view");
    f.finish().await;
}
