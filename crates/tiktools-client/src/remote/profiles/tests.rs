use super::*;
use crate::remote::tests::{workflow_grant, Fixture};
use secrecy::ExposeSecret;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
    time::Duration,
};
#[derive(Default)]
struct Store {
    entries: Mutex<BTreeMap<String, SecretString>>,
    loads: AtomicUsize,
    saves: AtomicUsize,
    fail: AtomicBool,
    delay_save: AtomicBool,
    save_started: tokio::sync::Notify,
    released: Mutex<bool>,
    wake: std::sync::Condvar,
}
impl CredentialStore for Store {
    fn load(&self, key: &str) -> Result<Option<SecretString>, ClientError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(credential_error());
        }
        Ok(self.entries.lock().unwrap().get(key).cloned())
    }
    fn save(&self, key: &str, value: &SecretString) -> Result<(), ClientError> {
        self.saves.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(credential_error());
        }
        if self.delay_save.load(Ordering::SeqCst) {
            self.save_started.notify_one();
            let released = self.released.lock().unwrap();
            let (released, _) = self
                .wake
                .wait_timeout_while(released, Duration::from_secs(5), |done| !*done)
                .unwrap();
            if !*released {
                return Err(credential_error());
            }
        }
        self.entries
            .lock()
            .unwrap()
            .insert(key.into(), value.clone());
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<(), ClientError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(credential_error());
        }
        self.entries.lock().unwrap().remove(key);
        Ok(())
    }
}

#[tokio::test]
async fn cancelling_an_open_caller_does_not_abandon_known_refresh_persistence() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    let store = Arc::new(Store::default());
    let manager = manager(&f, store.clone());
    let p = manager
        .install("cancel", &f.endpoint, creds.clone(), ClientKind::Agent)
        .await
        .unwrap();
    let mut expired = creds;
    expired.access_expires_at = now() - 1000;
    store
        .save(
            &p.credential_key,
            &sealed::encode(&p, &expired, None).unwrap(),
        )
        .unwrap();
    store.delay_save.store(true, Ordering::SeqCst);
    let copy = manager.clone();
    let caller = tokio::spawn(async move { copy.open("cancel").await });
    tokio::time::timeout(Duration::from_secs(4), store.save_started.notified())
        .await
        .unwrap();
    caller.abort();
    *store.released.lock().unwrap() = true;
    store.wake.notify_all();
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if manager
                .get("cancel")
                .await
                .is_ok_and(|p| p.refresh_attempt.is_none())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let client = manager.open("cancel").await.unwrap();
    assert_eq!(client.hello().session_id, p.session_id);
    client.close();
    f.finish().await;
}
fn manager(f: &Fixture, store: Arc<Store>) -> ProfileManager {
    ProfileManager::new(f.home.path().join("client/remote-profiles"), store).unwrap()
}
async fn disconnected(f: &Fixture) {
    tokio::time::timeout(Duration::from_secs(4), async {
        while f.service.gateway_status().unwrap().connections != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
async fn delivered(f: &Fixture, revision: &str) {
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if f.service
                .audit(tiktools_collaboration::AuditQuery {
                    after: 0,
                    limit: Some(100),
                })
                .unwrap()
                .iter()
                .filter_map(|e| e.details.as_ref())
                .any(|r| r.resource.id == "welcome" && r.after_revision == revision)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn profiles_store_only_metadata_and_connect_without_client_databases_or_local_fallback() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    let store = Arc::new(Store::default());
    let manager = manager(&f, store.clone());
    let profile = manager
        .install("live-desk", &f.endpoint, creds.clone(), ClientKind::Agent)
        .await
        .unwrap();
    let body = std::fs::read_to_string(manager.directory().join("live-desk.json")).unwrap();
    assert_eq!(
        manager
            .install("alias", &f.endpoint, creds.clone(), ClientKind::Agent)
            .await
            .err()
            .unwrap()
            .code,
        "profile_session_exists"
    );
    assert_eq!(store.entries.lock().unwrap().len(), 1);
    assert!(!body.contains(creds.access.expose_secret()));
    assert!(!body.contains(creds.refresh.expose_secret()));
    assert!(!body.contains("accessToken"));
    assert!(!body.contains("refreshToken"));
    assert_eq!(manager.list().await.unwrap().len(), 1);
    assert!(manager.ensure_new("live-desk").await.is_err());
    let client = manager.open("live-desk").await.unwrap();
    assert_eq!(client.hello().host_id, profile.host_id);
    assert!(client
        .call_value_with_meta("live.status", json!({}), None)
        .await
        .is_ok());
    assert!(!f.home.path().join("client/data").exists());
    assert!(!f.home.path().join("client/plugins").exists());
    assert!(f.core.db.automation_path().exists());
    client.close();
    disconnected(&f).await;
    manager.forget("live-desk").await.unwrap();
    assert!(store.entries.lock().unwrap().is_empty());
    assert!(manager.list().await.unwrap().is_empty());
    // Forgetting a local profile does not falsely claim host-side revocation.
    assert!(f
        .service
        .authenticate_live(creds.access.expose_secret(), false, false)
        .is_ok());
    f.finish().await;
}
#[tokio::test]
async fn refresh_commits_new_keychain_generation_and_fences_unknown_prior_attempts() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    let store = Arc::new(Store::default());
    let manager = manager(&f, store.clone());
    let p = manager
        .install("refresh", &f.endpoint, creds.clone(), ClientKind::Agent)
        .await
        .unwrap();
    let mut expired = creds.clone();
    expired.access_expires_at = now() - 1000;
    store
        .save(
            &p.credential_key,
            &sealed::encode(&p, &expired, None).unwrap(),
        )
        .unwrap();
    let client = manager.open("refresh").await.unwrap();
    client.close();
    disconnected(&f).await;
    let (current, marker) =
        sealed::decode(&p, &store.load(&p.credential_key).unwrap().unwrap()).unwrap();
    assert!(marker.is_some());
    assert!(manager
        .get("refresh")
        .await
        .unwrap()
        .refresh_attempt
        .is_none());
    assert!(f
        .service
        .authenticate_live(creds.access.expose_secret(), false, false)
        .is_err());
    assert!(f
        .service
        .authenticate_live(current.access.expose_secret(), false, false)
        .is_ok());
    // Simulate process death after the host rotated but before keychain save.
    let lease = manager.lease("refresh", false).await.unwrap();
    let mut pending = lease.profile.clone().unwrap();
    pending.refresh_attempt = Some(Uuid::new_v4().to_string());
    lease.write(&pending).unwrap();
    drop(lease);
    let fresh = f.endpoint.rotate_credentials(&current).await.unwrap();
    assert_eq!(
        manager.open("refresh").await.err().unwrap().code,
        "reauthentication_required"
    );
    // A replayed old refresh would revoke this new token; it stays valid.
    assert!(f
        .service
        .authenticate_live(fresh.access.expose_secret(), false, false)
        .is_ok());
    f.finish().await;
}
#[tokio::test]
async fn successful_keychain_save_repairs_a_crash_before_refresh_intent_is_cleared() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    let store = Arc::new(Store::default());
    let manager = manager(&f, store.clone());
    let mut p = manager
        .install("repair", &f.endpoint, creds.clone(), ClientKind::Agent)
        .await
        .unwrap();
    let attempt = Uuid::new_v4().to_string();
    p.refresh_attempt = Some(attempt.clone());
    let lease = manager.lease("repair", false).await.unwrap();
    lease.write(&p).unwrap();
    drop(lease);
    let fresh = f.endpoint.rotate_credentials(&creds).await.unwrap();
    store
        .save(
            &p.credential_key,
            &sealed::encode(&p, &fresh, Some(attempt)).unwrap(),
        )
        .unwrap();
    let saves = store.saves.load(Ordering::SeqCst);
    let client = manager.open("repair").await.unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), saves);
    assert!(manager
        .get("repair")
        .await
        .unwrap()
        .refresh_attempt
        .is_none());
    client.close();
    f.finish().await;
}
#[tokio::test]
async fn host_mismatch_is_detected_before_loading_a_credential_and_bundles_cannot_cross_profiles() {
    let first = Fixture::new().await;
    let second = Fixture::new().await;
    let creds = first.pair(workflow_grant()).await;
    let store = Arc::new(Store::default());
    let manager = manager(&first, store.clone());
    let mut p = manager
        .install("bound", &first.endpoint, creds, ClientKind::Agent)
        .await
        .unwrap();
    let raw = store.load(&p.credential_key).unwrap().unwrap();
    let mut another = p.clone();
    another.profile_id = Uuid::new_v4().to_string();
    assert!(sealed::decode(&another, &raw).is_err());
    p.origin = second.origin.clone();
    p.trust = second.endpoint.trust().clone();
    p.credential_key = credential_key(&p);
    let lease = manager.lease("bound", false).await.unwrap();
    lease.write(&p).unwrap();
    drop(lease);
    let reads = store.loads.load(Ordering::SeqCst);
    assert_eq!(
        manager.open("bound").await.err().unwrap().code,
        "host_changed"
    );
    assert_eq!(store.loads.load(Ordering::SeqCst), reads);
    second.finish().await;
    first.finish().await;
}
#[tokio::test]
async fn keychain_failure_never_falls_back_to_a_file_and_cross_process_leases_are_exclusive() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    let store = Arc::new(Store::default());
    let manager = manager(&f, store.clone());
    store.fail.store(true, Ordering::SeqCst);
    assert_eq!(
        manager.check_store().await.err().unwrap().code,
        "credential_store_unavailable"
    );
    assert_eq!(
        manager
            .install("failed", &f.endpoint, creds, ClientKind::Agent)
            .await
            .err()
            .unwrap()
            .code,
        "credential_store_unavailable"
    );
    assert!(!manager.directory().join("failed.json").exists());
    let lease = manager.lease("held", true).await.unwrap();
    assert_eq!(
        manager.lease("held", true).await.err().unwrap().code,
        "profile_in_use"
    );
    drop(lease);
    assert!(manager.lease("held", true).await.is_ok());
    f.finish().await;
}
#[tokio::test]
async fn recovery_resumes_unacknowledged_events_and_replaces_the_baseline_after_grant_change() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    let principal = creds.principal_id().to_owned();
    let store = Arc::new(Store::default());
    let manager = manager(&f, store);
    manager
        .install("stream", &f.endpoint, creds, ClientKind::Agent)
        .await
        .unwrap();
    f.core
        .workflow_save(json!({"id":"welcome","name":"One"}))
        .unwrap();
    delivered(&f, "1").await;
    let remote = manager.open("stream").await.unwrap();
    let mut stream = RecoveryStream::start(remote.clone(), [StreamDomain::Workflows].into())
        .await
        .unwrap();
    let first_snapshot = stream.baseline().unwrap().snapshot_id.clone();
    f.core.workflow_set_enabled("welcome", true).unwrap();
    let event = tokio::time::timeout(Duration::from_secs(4), stream.next_event())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.resource.revision, "2");
    remote.close();
    disconnected(&f).await;
    f.core.workflow_set_enabled("welcome", false).unwrap();
    delivered(&f, "3").await;
    assert_eq!(stream.reconnect().await.unwrap(), RecoveryMode::Resumed);
    assert_eq!(stream.baseline().unwrap().snapshot_id, first_snapshot);
    let replay = tokio::time::timeout(Duration::from_secs(4), stream.next_event())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay, event);
    stream.acknowledge(&replay).unwrap();
    let third = stream.next_event().await.unwrap();
    assert_eq!(third.resource.revision, "3");
    stream.acknowledge(&third).unwrap();
    f.service
        .set_grant(tiktools_collaboration::GrantSet {
            principal_id: principal,
            expected_revision: "1".into(),
            grant: tiktools_collaboration::Grant {
                resources: [(
                    tiktools_collaboration::ResourceKind::Workflow,
                    std::collections::BTreeSet::new(),
                )]
                .into(),
                ..workflow_grant()
            },
        })
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(4), stream.next_event())
            .await
            .unwrap()
            .unwrap_err()
            .code,
        "resync_required"
    );
    assert!(stream.baseline().is_none());
    assert!(stream.cursor().is_none());
    disconnected(&f).await;
    assert_eq!(stream.reconnect().await.unwrap(), RecoveryMode::Snapshot);
    assert!(stream.baseline().unwrap().workflows.is_empty());
    assert_ne!(stream.baseline().unwrap().snapshot_id, first_snapshot);
    stream.connection().close();
    f.finish().await;
}

#[tokio::test]
async fn points_only_recovery_replays_then_resnapshots_after_grant_change() {
    use tiktools_collaboration::{Capability, Grant, ResourceKind};
    let f = Fixture::new().await;
    let grant = Grant {
        capabilities: [Capability::PointsRead].into(),
        resources: [(ResourceKind::Viewer, ["alice".into()].into())].into(),
        ..Grant::default()
    };
    let creds = f.pair(grant.clone()).await;
    let principal = creds.principal_id().to_owned();
    let manager = manager(&f, Arc::new(Store::default()));
    manager
        .install("points-stream", &f.endpoint, creds, ClientKind::Agent)
        .await
        .unwrap();
    let remote = manager.open("points-stream").await.unwrap();
    let mut stream = RecoveryStream::start(remote.clone(), [StreamDomain::Points].into())
        .await
        .unwrap();
    assert!(stream.baseline().unwrap().points.is_empty());
    f.core.points_adjust("alice", 2.0).unwrap();
    let event = tokio::time::timeout(Duration::from_secs(4), stream.next_event())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.topic, "points.changed");
    assert_eq!(event.resource.kind, ResourceKind::Viewer);
    remote.close();
    disconnected(&f).await;
    f.core.points_adjust("alice", 3.0).unwrap();
    assert_eq!(stream.reconnect().await.unwrap(), RecoveryMode::Resumed);
    let replay = tokio::time::timeout(Duration::from_secs(4), stream.next_event())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay, event);
    stream.acknowledge(&replay).unwrap();
    let next = tokio::time::timeout(Duration::from_secs(4), stream.next_event())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(next.resource.revision, "2");
    stream.acknowledge(&next).unwrap();
    f.service
        .set_grant(tiktools_collaboration::GrantSet {
            principal_id: principal,
            expected_revision: "1".into(),
            grant: Grant {
                resources: [(ResourceKind::Viewer, Default::default())].into(),
                ..grant
            },
        })
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(4), stream.next_event())
            .await
            .unwrap()
            .unwrap_err()
            .code,
        "resync_required"
    );
    assert!(stream.baseline().is_none());
    disconnected(&f).await;
    assert_eq!(stream.reconnect().await.unwrap(), RecoveryMode::Snapshot);
    assert!(stream.baseline().unwrap().points.is_empty());
    stream.connection().close();
    f.finish().await;
}
#[cfg(unix)]
#[tokio::test]
async fn profile_files_reject_public_permissions_and_symlink_paths() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let home = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::default());
    let directory = home.path().join("profiles");
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
    let manager = ProfileManager::new(directory.clone(), store).unwrap();
    assert!(manager.ensure_new("public").await.is_err());
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    symlink(home.path().join("missing"), directory.join("escape.lock")).unwrap();
    assert!(manager.ensure_new("escape").await.is_err());
    symlink(&directory, home.path().join("symlink")).unwrap();
    assert!(
        ProfileManager::new(home.path().join("symlink"), Arc::new(Store::default()))
            .unwrap()
            .ensure_new("name")
            .await
            .is_err()
    );
}

#[test]
#[ignore = "subprocess-only profile lease crash fixture"]
fn lease_crash_child() {
    let directory =
        PathBuf::from(std::env::var_os("TIKTOOLS_CLIENT_LEASE_TEST_DIR").expect("child directory"));
    let _lease = directory::Lease::open(directory.clone(), "lease", true).unwrap();
    std::fs::write(directory.join("ready"), "ready").unwrap();
    let mut input = Vec::new();
    use std::io::Read;
    let _ = std::io::stdin().read_to_end(&mut input);
}
#[tokio::test]
async fn profile_lease_excludes_another_process_and_releases_after_process_death() {
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("profiles");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "remote::profiles::tests::lease_crash_child",
            "--ignored",
            "--nocapture",
        ])
        .env("TIKTOOLS_CLIENT_LEASE_TEST_DIR", &directory)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut child = Child(child);
    tokio::time::timeout(Duration::from_secs(4), async {
        while !directory.join("ready").exists() {
            assert!(child.0.try_wait().unwrap().is_none());
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        directory::Lease::open(directory.clone(), "lease", true)
            .err()
            .unwrap()
            .code,
        "profile_in_use"
    );
    child.0.kill().unwrap();
    assert!(!child.0.wait().unwrap().success());
    assert!(directory::Lease::open(directory, "lease", true).is_ok());
}
