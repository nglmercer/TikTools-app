use super::*;
use crate::remote::{profiles::CredentialStore, tests::Fixture};
use std::{collections::BTreeMap, sync::Mutex as StdMutex};
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
        capabilities: [
            Capability::WorkflowsRead,
            Capability::WorkflowsToggle,
            Capability::WorkflowsEdit,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(ResourceKind::Workflow, ["welcome".into()].into())].into(),
        ..Default::default()
    }
}
async fn wait(c: &Controller, p: impl Fn(&ControllerStatus) -> bool) {
    tokio::time::timeout(Duration::from_secs(6), async {
        while !p(&c.status()) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("controller command state did not settle");
}
fn input(c: &Controller, operation: Operation) -> WorkflowCommand {
    let s = c.status();
    let v = c.shared.view.read().unwrap();
    WorkflowCommand {
        epoch: s.epoch,
        view_revision: s.view_revision,
        id: "welcome".into(),
        expected_revision: v.rows["welcome"].version.revision.clone(),
        operation,
    }
}
async fn install(f: &Fixture, m: &ProfileManager, name: &str) -> profiles::RemoteProfile {
    m.install(name, &f.endpoint, f.pair(grant()).await, ClientKind::Agent)
        .await
        .unwrap()
}
fn prepared(profile: &profiles::RemoteProfile, instance: &str, root: &std::path::Path) -> Intent {
    let j = Journal::open(root, profile).unwrap();
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: Operation::Disable,
        resource_id: "welcome".into(),
        host_instance_id: instance.into(),
        expected_revision: Some("1".into()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        created_at: profiles::commands::timestamp(),
    };
    j.begin(intent.clone()).unwrap();
    intent
}
fn hold(j: Arc<Journal>) -> (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let task = std::thread::spawn(move || j.hold_writer_for_test(tx, wait));
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    (release, task)
}

async fn install_points(f: &Fixture, m: &ProfileManager, name: &str) -> profiles::RemoteProfile {
    let grant = tiktools_collaboration::Grant {
        capabilities: [
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(ResourceKind::Viewer, ["alice".into()].into())].into(),
        max_points_delta: Some(10.0),
        ..Default::default()
    };
    m.install(name, &f.endpoint, f.pair(grant).await, ClientKind::Agent)
        .await
        .unwrap()
}
fn points_input(c: &Controller, delta: f64, currency: &str) -> PointsCommand {
    PointsCommand {
        connection_id: None,
        epoch: c.status().epoch,
        unique_id: "alice".into(),
        delta,
        currency_name: currency.into(),
    }
}

// A private fixture credential store, never compiled into a normal client.
// It lets the real native coordinator run in a separately killed process.
struct FixtureFileStore(std::path::PathBuf);
impl CredentialStore for FixtureFileStore {
    fn load(&self, key: &str) -> Result<Option<SecretString>, ClientError> {
        let value: Value = serde_json::from_slice(&std::fs::read(&self.0).unwrap()).unwrap();
        Ok((value["key"].as_str() == Some(key))
            .then(|| SecretString::from(value["secret"].as_str().unwrap().to_owned())))
    }
    fn save(&self, key: &str, secret: &SecretString) -> Result<(), ClientError> {
        use secrecy::ExposeSecret;
        let value = json!({"key":key,"secret":secret.expose_secret()});
        std::fs::write(&self.0, serde_json::to_vec(&value).unwrap()).unwrap();
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<(), ClientError> {
        panic!("fixture must not delete credentials")
    }
}

#[tokio::test]
#[ignore = "private subprocess invoked by native points process-death test"]
async fn native_points_process_fixture() {
    let root =
        std::path::PathBuf::from(std::env::var_os("TIKTOOLS_NATIVE_POINTS_FIXTURE_ROOT").unwrap());
    assert!(root.starts_with(std::env::temp_dir()));
    let manager = ProfileManager::new(
        root.clone(),
        Arc::new(FixtureFileStore(root.join("test-credentials.input"))),
    )
    .unwrap();
    let c = Controller::new(manager);
    c.connect("points").await.unwrap();
    c.points_command(points_input(&c, 4.0, "Points"))
        .await
        .unwrap();
    panic!("native points crash boundary was not reached");
}

#[tokio::test]
async fn native_process_death_before_send_and_after_host_commit_never_replays_an_adjustment() {
    use secrecy::ExposeSecret;
    for stage in [
        "before_intent",
        "after_intent",
        "after_reply",
        "after_record",
    ] {
        let f = Fixture::new().await;
        let home = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::default());
        let m = ProfileManager::new(home.path().join("profiles"), store.clone()).unwrap();
        let profile = install_points(&f, &m, "points").await;
        let root = m.directory().to_path_buf();
        let secret = store.load(&profile.credential_key).unwrap().unwrap();
        let file = root.join("test-credentials.input");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&file).unwrap();
        serde_json::to_writer(
            &mut output,
            &json!({"key":profile.credential_key,"secret":secret.expose_secret()}),
        )
        .unwrap();
        output.sync_all().unwrap();
        drop(output);
        let fixture_root = root.clone();
        let status = tokio::task::spawn_blocking(move || {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "remote::controller::commands::tests::native_points_process_fixture",
                ])
                .env("TIKTOOLS_NATIVE_POINTS_FIXTURE_ROOT", &fixture_root)
                .env("TIKTOOLS_NATIVE_POINTS_CRASH_STAGE", stage)
                .current_dir(&fixture_root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap()
        })
        .await
        .unwrap();
        assert_eq!(status.code(), Some(98), "{stage}");
        let credentials: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        store
            .save(
                credentials["key"].as_str().unwrap(),
                &SecretString::from(credentials["secret"].as_str().unwrap().to_owned()),
            )
            .unwrap();
        let journal = Journal::open(m.directory(), &profile).unwrap().view();
        assert_eq!(
            journal.pending.is_some(),
            matches!(stage, "after_intent" | "after_reply")
        );
        assert_eq!(journal.recent.len(), usize::from(stage == "after_record"));
        let committed = matches!(stage, "after_reply" | "after_record");
        assert_eq!(f.core.points_viewer("alice").is_ok(), committed);
        if committed {
            f.core.points_adjust("alice", 2.0).unwrap();
        }
        let c = Controller::new(m);
        c.connect("points").await.unwrap();
        if committed {
            wait(&c, |s| {
                s.command_journal
                    .as_ref()
                    .is_some_and(|j| j.pending.is_none() && !j.recent.is_empty())
            })
            .await;
            let recovered = c.status().command_journal.unwrap();
            assert_eq!(
                recovered.recent[0]
                    .points_result
                    .as_ref()
                    .unwrap()
                    .total_points,
                4.0
            );
            assert_eq!(f.core.points_viewer("alice").unwrap()["points"], 6.0);
        } else if stage == "after_intent" {
            wait(&c, |s| {
                !s.command_busy && s.command_error_code.as_deref() == Some("forbidden")
            })
            .await;
            assert_eq!(c.status().command_journal.unwrap().pending, journal.pending);
            assert!(f.core.points_viewer("alice").is_err());
        } else {
            assert!(c.status().command_journal.unwrap().recent.is_empty());
        }
        for name in ["data", "plugin-data", "plugins"] {
            assert!(!root.join(name).exists());
        }
        c.disconnect().await;
        f.finish().await;
    }
}

#[test]
fn points_receipts_bind_the_exact_viewer_unit_and_effective_amount_without_a_fake_cas() {
    let i = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: Operation::AdjustPoints,
        resource_id: "alice".into(),
        host_instance_id: uuid::Uuid::new_v4().to_string(),
        expected_revision: None,
        delta: Some(2.0),
        currency_name: Some("Points".into()),
        proposal_revision: None,
        created_at: 0,
    };
    let receipt = CommandReceipt {
        command_id: i.command_id.clone(),
        state: CommandState::Succeeded,
        resource: Some(tiktools_collaboration::CommandResource {
            kind: ResourceKind::Viewer,
            scope: "host".into(),
            id: "alice".into(),
        }),
        committed_at: Some(1),
    };
    let version = ResourceVersion {
        kind: ResourceKind::Viewer,
        scope: "host".into(),
        id: "alice".into(),
        revision: "1234".into(),
    };
    let body =
        json!({"uniqueId":"alice","delta":2.5,"totalPoints":7.5,"level":1,"currencyName":"Points"});
    assert_eq!(
        validate_success(&i, &receipt, std::slice::from_ref(&version), &body).unwrap(),
        "1234"
    );
    for (field, value) in [
        ("currencyName", json!("Gems")),
        ("uniqueId", json!("hidden")),
        ("delta", json!(-2)),
        ("delta", json!(203)),
        ("totalPoints", json!(-1)),
    ] {
        let mut bad = body.clone();
        bad[field] = value;
        assert!(validate_success(&i, &receipt, std::slice::from_ref(&version), &bad).is_err());
    }
    let wrong = ResourceVersion {
        kind: ResourceKind::Workflow,
        ..version
    };
    assert!(validate_success(&i, &receipt, &[wrong], &body).is_err());
}

#[tokio::test]
async fn native_points_command_is_durable_without_workflow_access_or_javascript_command_ids() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let m = ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = install_points(&f, &m, "points").await;
    let c = Controller::new(m.clone());
    c.connect("points").await.unwrap();
    assert_eq!(
        c.forward(&c.status().epoch, "points.info", json!({}), None)
            .await
            .unwrap()
            .result,
        json!({"currencyName":"Points"})
    );
    assert_eq!(c.manage("controller.points.command",json!({"epoch":c.status().epoch,"uniqueId":"alice","delta":2,"currencyName":"Points","commandId":"page-chosen"})).await.unwrap_err().code,"invalid_params");
    assert!(c
        .status()
        .command_journal
        .as_ref()
        .unwrap()
        .recent
        .is_empty());
    c.points_command(points_input(&c, 2.0, "Points"))
        .await
        .unwrap();
    let j = c.status().command_journal.unwrap();
    let r = &j.recent[0];
    assert_eq!(r.outcome, Outcome::Succeeded);
    assert_eq!(r.intent.operation, Operation::AdjustPoints);
    assert!(r.intent.expected_revision.is_none());
    assert_eq!(r.intent.delta, Some(2.0));
    assert_eq!(r.points_result.as_ref().unwrap().total_points, 2.0);
    assert_eq!(f.core.points_viewer("alice").unwrap()["points"], 2.0);
    assert_eq!(
        c.forward(
            &c.status().epoch,
            "points.adjust",
            json!({"uniqueId":"alice","delta":1}),
            None
        )
        .await
        .unwrap_err()
        .code,
        "forbidden"
    );
    for (revision, allowed) in [("1", false), ("2", true)] {
        f.service
            .set_grant(tiktools_collaboration::GrantSet {
                principal_id: profile.principal_id.clone(),
                expected_revision: revision.into(),
                grant: tiktools_collaboration::Grant {
                    capabilities: [
                        Capability::PointsRead,
                        Capability::PointsAdjust,
                        Capability::CommandsRead,
                    ]
                    .into(),
                    resources: [(
                        ResourceKind::Viewer,
                        if allowed {
                            ["alice".into()].into()
                        } else {
                            Default::default()
                        },
                    )]
                    .into(),
                    ..Default::default()
                },
            })
            .unwrap();
        let expected = if allowed { "3" } else { "2" };
        wait(&c, |s| {
            s.phase == "ready"
                && s.hello
                    .as_ref()
                    .is_some_and(|h| h.grant_revision == expected)
        })
        .await;
        assert_eq!(
            c.status().command_journal.unwrap().recent[0]
                .points_result
                .is_some(),
            allowed
        );
    }
    f.service
        .set_grant(tiktools_collaboration::GrantSet {
            principal_id: profile.principal_id.clone(),
            expected_revision: "3".into(),
            grant: tiktools_collaboration::Grant {
                capabilities: [Capability::CommandsRead].into(),
                ..Default::default()
            },
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready"
            && s.hello
                .as_ref()
                .is_some_and(|h| !h.capabilities.contains(&Capability::PointsRead))
    })
    .await;
    assert!(c.status().command_journal.unwrap().recent[0]
        .points_result
        .is_none());
    c.disconnect().await;
    // The private durable outcome survives; only its page projection is hidden.
    assert!(Journal::open(m.directory(), &profile)
        .unwrap()
        .view()
        .recent[0]
        .points_result
        .is_some());
    f.finish().await;
}

#[tokio::test]
async fn points_currency_scope_and_malformed_amounts_cannot_cross_native_admission() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let m = ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    install_points(&f, &m, "points").await;
    let c = Controller::new(m);
    c.connect("points").await.unwrap();
    for delta in [f64::NAN, f64::INFINITY, 0.0, 1_000_000_001.0] {
        assert_eq!(
            c.points_command(points_input(&c, delta, "Points"))
                .await
                .unwrap_err()
                .code,
            "invalid_params"
        );
    }
    let mut other = points_input(&c, 1.0, "Points");
    other.unique_id = "hidden".into();
    assert_eq!(c.points_command(other).await.unwrap_err().code, "forbidden");
    assert!(c.status().command_journal.unwrap().recent.is_empty());
    f.core
        .points_update_config(serde_json::from_value(json!({"currencyName":"Gems"})).unwrap());
    assert_eq!(
        c.points_command(points_input(&c, 1.0, "Points"))
            .await
            .unwrap_err()
            .code,
        "currency_changed"
    );
    let j = c.status().command_journal.unwrap();
    assert!(j.pending.is_none());
    assert_eq!(j.recent[0].outcome, Outcome::Rejected);
    assert_eq!(j.recent[0].error_code.as_deref(), Some("currency_changed"));
    assert!(f.core.points_viewer("alice").is_err());
    c.points_command(points_input(&c, -3.0, "Gems"))
        .await
        .unwrap();
    assert_eq!(
        c.status().command_journal.unwrap().recent[0]
            .points_result
            .as_ref()
            .unwrap()
            .total_points,
        0.0
    );
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn points_recovery_confirms_the_original_result_after_later_awards_and_unit_rename() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let m = ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = install_points(&f, &m, "points").await;
    let original = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: Operation::AdjustPoints,
        resource_id: "alice".into(),
        host_instance_id: f.service.instance_id().into(),
        expected_revision: None,
        delta: Some(4.0),
        currency_name: Some("Points".into()),
        proposal_revision: None,
        created_at: profiles::commands::timestamp(),
    };
    {
        let j = Journal::open(m.directory(), &profile).unwrap();
        j.begin(original.clone()).unwrap();
    }
    let remote = m.open("points").await.unwrap();
    remote
        .call_value_with_meta(
            "points.adjust",
            json!({"uniqueId":"alice","delta":4,"expectedCurrency":"Points"}),
            Some(RequestMetadata {
                command_id: Some(original.command_id.clone()),
                host_instance_id: Some(original.host_instance_id.clone()),
                expected_revision: None,
            }),
        )
        .await
        .unwrap();
    remote.close();
    drop(remote);
    f.core.points_adjust("alice", 2.0).unwrap();
    f.core
        .points_update_config(serde_json::from_value(json!({"currencyName":"Gems"})).unwrap());
    let c = Controller::new(m);
    c.connect("points").await.unwrap();
    wait(&c, |s| {
        s.command_journal
            .as_ref()
            .is_some_and(|j| j.pending.is_none() && !j.recent.is_empty())
    })
    .await;
    let j = c.status().command_journal.unwrap();
    assert_eq!(j.recent[0].intent, original);
    assert_eq!(
        j.recent[0].points_result.as_ref().unwrap().total_points,
        4.0
    );
    assert_eq!(f.core.points_viewer("alice").unwrap()["points"], 6.0);
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn cancelled_points_page_is_owned_and_profile_switch_during_sync_cannot_dispatch() {
    let a = Fixture::new().await;
    let b = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let m = ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    install_points(&a, &m, "first").await;
    install_points(&b, &m, "second").await;
    let c = Arc::new(Controller::new(m));
    c.connect("first").await.unwrap();
    let j = c.shared.view.read().unwrap().journal.clone().unwrap();
    let (release, writer) = hold(j);
    let p = points_input(&c, 3.0, "Points");
    let own = c.clone();
    let caller = tokio::spawn(async move { own.points_command(p).await });
    wait(&c, |s| s.command_busy).await;
    caller.abort();
    let _ = caller.await;
    c.connect("second").await.unwrap();
    release.send(()).unwrap();
    writer.join().unwrap();
    // The finalizer owns the first journal even after its page/profile leaves.
    c.connect("first").await.unwrap();
    wait(&c, |s| {
        s.command_journal
            .as_ref()
            .is_some_and(|j| j.pending.is_none() && !j.recent.is_empty())
    })
    .await;
    let j = c.status().command_journal.unwrap();
    assert_eq!(j.recent[0].outcome, Outcome::Rejected);
    assert!(a.core.points_viewer("alice").is_err());
    assert!(b.core.points_viewer("alice").is_err());
    c.disconnect().await;
    a.finish().await;
    b.finish().await;
}

#[tokio::test]
async fn controller_recovers_a_committed_id_after_restart_without_reapplying_the_operation() {
    let f = Fixture::new().await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Owner graph","enabled":true}))
        .unwrap();
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = install(&f, &manager, "desk").await;
    let original = prepared(&profile, f.service.instance_id(), manager.directory());
    let remote = manager.open("desk").await.unwrap();
    let result = remote
        .call_value_with_meta(
            "workflows.disable",
            json!({"id":"welcome"}),
            Some(RequestMetadata {
                command_id: Some(original.command_id.clone()),
                host_instance_id: Some(original.host_instance_id.clone()),
                expected_revision: Some("1".into()),
            }),
        )
        .await
        .unwrap();
    assert_eq!(result.meta.unwrap().resource_versions[0].revision, "2");
    remote.close();
    drop(remote);
    f.core.workflow_set_enabled("welcome", true).unwrap();
    let c = Controller::new(manager.clone());
    c.connect("desk").await.unwrap();
    wait(&c, |s| {
        s.command_journal
            .as_ref()
            .is_some_and(|j| j.pending.is_none() && !j.recent.is_empty())
    })
    .await;
    let recovered = c.status().command_journal.unwrap();
    assert_eq!(recovered.recent[0].intent, original);
    assert_eq!(recovered.recent[0].outcome, Outcome::Succeeded);
    assert_eq!(recovered.recent[0].revision.as_deref(), Some("2"));
    assert_eq!(f.core.workflow_get("welcome").unwrap()["enabled"], true);
    assert_eq!(
        c.forward(
            &c.status().epoch,
            "workflows.disable",
            json!({"id":"welcome"}),
            None
        )
        .await
        .unwrap_err()
        .code,
        "forbidden"
    );
    let other = Controller::new(manager.clone());
    assert_eq!(
        other.connect("desk").await.unwrap_err().code,
        "command_journal_in_use"
    );
    c.workflow_command(input(&c, Operation::Delete))
        .await
        .unwrap();
    wait(&c, |s| {
        !s.command_busy
            && s.command_journal
                .as_ref()
                .is_some_and(|j| j.recent.len() == 2)
    })
    .await;
    assert!(f.core.workflow_get("welcome").is_err());
    let latest = c.status().command_journal.unwrap();
    assert_ne!(latest.recent[0].intent.command_id, original.command_id);
    assert_eq!(latest.recent[0].revision.as_deref(), Some("4"));
    c.disconnect().await;
    drop(c);
    other.connect("desk").await.unwrap();
    assert!(other.status().command_journal.unwrap().pending.is_none());
    other.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn an_unaccepted_intent_is_never_replayed_or_forgotten_without_explicit_review() {
    let f = Fixture::new().await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Owner graph","enabled":true}))
        .unwrap();
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let p = install(&f, &manager, "desk").await;
    let original = prepared(&p, f.service.instance_id(), manager.directory());
    assert_eq!(
        manager.forget("desk").await.unwrap_err().code,
        "command_pending"
    );
    let c = Controller::new(manager.clone());
    c.connect("desk").await.unwrap();
    wait(&c, |s| {
        !s.command_busy && s.command_error_code.as_deref() == Some("forbidden")
    })
    .await;
    assert_eq!(
        c.status().command_journal.unwrap().pending,
        Some(original.clone())
    );
    assert_eq!(
        c.workflow_command(input(&c, Operation::Disable))
            .await
            .unwrap_err()
            .code,
        "command_pending"
    );
    assert_eq!(
        c.resolve_command(c.status().epoch, "another-id".into())
            .await
            .unwrap_err()
            .code,
        "stale_view"
    );
    let review = json!({"epoch":c.status().epoch,"commandId":original.command_id,"acknowledgeUnknown":false});
    assert_eq!(
        c.manage("controller.command.review", review.clone())
            .await
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    assert_eq!(f.core.workflow_get("welcome").unwrap()["enabled"], true);
    let mut review = review;
    review["acknowledgeUnknown"] = json!(true);
    c.manage("controller.command.review", review).await.unwrap();
    assert_eq!(
        c.status().command_journal.unwrap().recent[0].outcome,
        Outcome::ReviewedUnknown
    );
    c.workflow_command(input(&c, Operation::Disable))
        .await
        .unwrap();
    assert_eq!(
        f.core
            .workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "2"
    );
    assert_ne!(
        c.status().command_journal.unwrap().recent[0]
            .intent
            .command_id,
        original.command_id
    );
    c.disconnect().await;
    drop(c);
    manager.forget("desk").await.unwrap();
    assert!(manager.list().await.unwrap().is_empty());
    f.finish().await;
}

#[tokio::test]
async fn journal_write_failure_prevents_remote_dispatch_and_keeps_the_intent_visible() {
    let f = Fixture::new().await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Owner graph","enabled":true}))
        .unwrap();
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let p = install(&f, &manager, "desk").await;
    let c = Controller::new(manager.clone());
    c.connect("desk").await.unwrap();
    std::fs::create_dir(
        manager
            .directory()
            .join("commands")
            .join(format!("{}.journal", p.profile_id)),
    )
    .unwrap();
    assert!(c
        .workflow_command(input(&c, Operation::Disable))
        .await
        .is_err());
    let state = c.status();
    assert!(!state.command_busy);
    let j = state.command_journal.unwrap();
    assert!(j.unavailable);
    assert!(j.pending.is_some());
    assert_eq!(f.core.workflow_get("welcome").unwrap()["enabled"], true);
    assert_eq!(
        f.core
            .workflow_get_versioned("welcome")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn closing_a_page_request_does_not_abandon_the_native_command_or_its_result() {
    let f = Fixture::new().await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Owner graph","enabled":true}))
        .unwrap();
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    install(&f, &manager, "desk").await;
    let c = Arc::new(Controller::new(manager));
    c.connect("desk").await.unwrap();
    let j = c.shared.view.read().unwrap().journal.clone().unwrap();
    let (release, writer) = hold(j);
    let pending = c.clone();
    let p = input(&c, Operation::Disable);
    let caller = tokio::spawn(async move { pending.workflow_command(p).await });
    wait(&c, |s| s.command_busy).await;
    let s = c.status();
    assert!(c.workflows(&s.epoch, &s.view_revision, 0).is_ok());
    caller.abort();
    let _ = caller.await;
    release.send(()).unwrap();
    writer.join().unwrap();
    wait(&c, |s| {
        !s.command_busy
            && s.command_journal
                .as_ref()
                .is_some_and(|j| j.pending.is_none() && !j.recent.is_empty())
    })
    .await;
    assert_eq!(
        c.status().command_journal.unwrap().recent[0].outcome,
        Outcome::Succeeded
    );
    assert_eq!(f.core.workflow_get("welcome").unwrap()["enabled"], false);
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn switching_profiles_while_persisting_never_sends_the_old_intent_to_either_host() {
    let a = Fixture::new().await;
    let b = Fixture::new().await;
    for f in [&a, &b] {
        f.core
            .workflow_save(json!({"id":"welcome","name":"Owner graph","enabled":true}))
            .unwrap();
    }
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    install(&a, &manager, "first").await;
    install(&b, &manager, "second").await;
    let c = Arc::new(Controller::new(manager));
    c.connect("first").await.unwrap();
    let j = c.shared.view.read().unwrap().journal.clone().unwrap();
    let (release, writer) = hold(j);
    let pending = c.clone();
    let p = input(&c, Operation::Disable);
    let caller = tokio::spawn(async move { pending.workflow_command(p).await });
    wait(&c, |s| s.command_busy).await;
    c.connect("second").await.unwrap();
    assert_eq!(c.status().origin.as_deref(), Some(b.origin.as_str()));
    release.send(()).unwrap();
    writer.join().unwrap();
    assert_eq!(caller.await.unwrap().unwrap_err().code, "stale_view");
    assert!(c.status().command_journal.unwrap().recent.is_empty());
    for f in [&a, &b] {
        assert_eq!(f.core.workflow_get("welcome").unwrap()["enabled"], true);
    }
    c.disconnect().await;
    c.connect("first").await.unwrap();
    assert_eq!(
        c.status().command_journal.unwrap().recent[0].outcome,
        Outcome::Rejected
    );
    c.disconnect().await;
    a.finish().await;
    b.finish().await;
}

#[test]
fn receipt_validation_requires_the_exact_id_resource_and_revision_transition() {
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: Operation::Disable,
        resource_id: "welcome".into(),
        host_instance_id: uuid::Uuid::new_v4().to_string(),
        expected_revision: Some("7".into()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        created_at: 0,
    };
    let mut receipt = CommandReceipt {
        command_id: intent.command_id.clone(),
        state: CommandState::Succeeded,
        resource: Some(tiktools_collaboration::CommandResource {
            kind: ResourceKind::Workflow,
            scope: "host".into(),
            id: "welcome".into(),
        }),
        committed_at: Some(1),
    };
    let mut version = ResourceVersion {
        kind: ResourceKind::Workflow,
        scope: "host".into(),
        id: "welcome".into(),
        revision: "8".into(),
    };
    let result = json!({"id":"welcome","enabled":false});
    assert_eq!(
        validate_success(&intent, &receipt, std::slice::from_ref(&version), &result).unwrap(),
        "8"
    );
    receipt.command_id = uuid::Uuid::new_v4().to_string();
    assert!(validate_success(&intent, &receipt, std::slice::from_ref(&version), &result).is_err());
    receipt.command_id = intent.command_id.clone();
    version.revision = "9".into();
    assert!(validate_success(&intent, &receipt, std::slice::from_ref(&version), &result).is_err());
    version.revision = "8".into();
    receipt.resource.as_mut().unwrap().id = "private".into();
    assert!(validate_success(&intent, &receipt, &[version], &result).is_err());
}

mod event_process_tests;
