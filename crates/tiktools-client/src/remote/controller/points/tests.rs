use super::*;
use crate::remote::{profiles::CredentialStore, tests::Fixture};
use std::sync::Mutex;
#[derive(Default)]
struct Store(Mutex<BTreeMap<String, SecretString>>);
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
fn page(c: &Controller) -> Option<Value> {
    let s = c.status();
    c.points(&s.epoch, &s.points_revision).ok()
}
async fn wait(c: &Controller, p: impl Fn(&Value) -> bool) -> Value {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if let Some(v) = page(c) {
                if !v["loading"].as_bool().unwrap_or(true) && p(&v) {
                    return v;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "points view did not settle: {}",
            serde_json::to_value(c.status()).unwrap()
        )
    })
}
fn grant() -> tiktools_collaboration::Grant {
    tiktools_collaboration::Grant {
        capabilities: [
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::CommandsRead,
        ]
        .into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn native_points_view_coalesces_awards_preserves_lookup_on_resume_and_fences_old_confirmations(
) {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let m = ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    m.install(
        "points",
        &f.endpoint,
        f.pair(grant()).await,
        ClientKind::Agent,
    )
    .await
    .unwrap();
    for n in 0..30 {
        f.core
            .points_adjust(&format!("v-{n:02}"), f64::from(n))
            .unwrap();
    }
    f.core.points_adjust("alice", 1.0).unwrap();
    let c = Controller::new(m);
    c.connect("points").await.unwrap();
    wait(&c, |v| v["rows"].as_array().is_some_and(|r| r.len() == 20)).await;
    c.select_points(&c.status().epoch, Some("alice".into()))
        .unwrap();
    wait(&c, |v| v["selected"]["viewer"]["points"] == 1.0).await;
    let revision = c.status().view_revision;
    let point_revision = c.status().points_revision.parse::<u64>().unwrap();
    let start = Instant::now();
    for _ in 0..100 {
        f.core.points_adjust("alice", 1.0).unwrap();
    }
    wait(&c, |v| v["selected"]["viewer"]["points"] == 101.0).await;
    assert_eq!(
        c.status().view_revision,
        revision,
        "points must not invalidate workflow confirmation revisions"
    );
    assert!(
        c.status().points_revision.parse::<u64>().unwrap() - point_revision
            <= start.elapsed().as_millis() as u64 / 250 + 2
    );
    let remote = c.shared.view.read().unwrap().remote.clone().unwrap();
    let old_connection = remote.hello().connection_id.clone();
    remote.close();
    f.core.points_adjust("alice", 2.0).unwrap();
    let view = wait(&c, |v| v["selected"]["viewer"]["points"] == 103.0).await;
    assert_eq!(view["selectedId"], "alice");
    assert_ne!(c.status().hello.unwrap().connection_id, old_connection);
    assert_eq!(
        c.points_command(PointsCommand {
            connection_id: Some(old_connection),
            epoch: c.status().epoch,
            unique_id: "alice".into(),
            delta: 1.0,
            currency_name: "Points".into()
        })
        .await
        .unwrap_err()
        .code,
        "stale_view"
    );
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn mixed_baselines_scope_missing_viewers_and_grant_changes_never_mix_resource_kinds() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    f.core
        .workflow_save(json!({"id":"alice","name":"Workflow"}))
        .unwrap();
    f.core.points_adjust("alice", 5.0).unwrap();
    f.core.points_adjust("hidden", 100.0).unwrap();
    let mut grant = grant();
    grant.capabilities.insert(Capability::WorkflowsRead);
    grant
        .resources
        .insert(ResourceKind::Viewer, ["alice".into(), "new".into()].into());
    let m = ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let p = m
        .install(
            "mixed",
            &f.endpoint,
            f.pair(grant.clone()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    let c = Controller::new(m);
    c.connect("mixed").await.unwrap();
    let view = wait(&c, |v| v["rows"].as_array().is_some_and(|r| r.len() == 1)).await;
    assert_eq!(view["rows"][0]["version"]["kind"], "viewer");
    let status = c.status();
    assert_eq!(
        c.workflows(&status.epoch, &status.view_revision, 0)
            .unwrap()["rows"][0]["version"]["kind"],
        "workflow"
    );
    assert_eq!(
        c.select_points(&status.epoch, Some("hidden".into()))
            .unwrap_err()
            .code,
        "forbidden"
    );
    c.select_points(&status.epoch, Some("new".into())).unwrap();
    wait(&c, |v| v["selectedVersion"]["revision"] == "0").await;
    c.points_command(PointsCommand {
        connection_id: None,
        epoch: c.status().epoch,
        unique_id: "new".into(),
        delta: 2.0,
        currency_name: "Points".into(),
    })
    .await
    .unwrap();
    wait(&c, |v| v["selected"]["viewer"]["points"] == 2.0).await;
    grant
        .resources
        .insert(ResourceKind::Viewer, Default::default());
    f.service
        .set_grant(tiktools_collaboration::GrantSet {
            principal_id: p.principal_id,
            expected_revision: "1".into(),
            grant,
        })
        .unwrap();
    let view = wait(&c, |v| {
        v["rows"].as_array().is_some_and(Vec::is_empty) && v["selectedId"].is_null()
    })
    .await;
    assert!(view["selected"].is_null());
    c.disconnect().await;
    f.finish().await;
}
