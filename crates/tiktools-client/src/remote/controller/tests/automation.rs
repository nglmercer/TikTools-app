//! Actual native controller → TLS host coverage. Client homes contain profiles
//! and intent journals only; executable owner definitions stay on the host.
use super::*;
use crate::remote::profiles::commands::{Intent, Journal, Operation, Outcome};
use tiktools_collaboration::{Grant, GrantSet};
use tiktools_core::control::AutomationKind;

fn grant() -> Grant {
    Grant {
        capabilities: [
            Capability::AutomationRead,
            Capability::AutomationToggle,
            Capability::AutomationEdit,
            Capability::CommandsRead,
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::WorkflowsRead,
            Capability::WorkflowsToggle,
        ]
        .into(),
        resources: [
            (
                ResourceKind::Automation,
                ["event:same".into(), "action:same".into()].into(),
            ),
            (ResourceKind::Viewer, ["alice".into()].into()),
            (ResourceKind::Workflow, ["welcome".into()].into()),
        ]
        .into(),
        ..Grant::default()
    }
}
fn seed(f: &Fixture) {
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        f.core.automation_create(kind, json!({"id":"same","name":"Allowed status","enabled":true,"typeId":"core.log","trigger":"tiktok.chat","actionIds":["private-action"],"config":{"secret":"owner-only-secret"},"source":"owner-only-script"})).unwrap();
        f.core
            .automation_create(
                kind,
                json!({"id":"hidden","name":"Hidden private label","enabled":true}),
            )
            .unwrap();
    }
    f.core
        .workflow_save(json!({"id":"welcome","name":"Workflow","enabled":true}))
        .unwrap();
    f.core.points_adjust("alice", 3.0).unwrap();
}
async fn install(f: &Fixture, manager: &ProfileManager, grant: Grant) -> profiles::RemoteProfile {
    manager
        .install("desk", &f.endpoint, f.pair(grant).await, ClientKind::Agent)
        .await
        .unwrap()
}
async fn page(c: &Controller) -> Value {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let s = c.status();
            match c
                .manage(
                    "controller.automations",
                    json!({"epoch":s.epoch,"automationRevision":s.automation_revision,"offset":0}),
                )
                .await
            {
                Ok(value) => break value,
                Err(error) if matches!(error.code.as_str(), "stale_view" | "not_ready") => {
                    tokio::time::sleep(Duration::from_millis(10)).await
                }
                Err(error) => panic!("automation page failed: {error:?}"),
            }
        }
    })
    .await
    .expect("automation page did not become ready")
}
async fn wait_rows(c: &Controller, predicate: impl Fn(&[Value]) -> bool) -> Value {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let value = page(c).await;
            if predicate(value["rows"].as_array().unwrap()) {
                return value;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("automation rows did not settle")
}
fn command(c: &Controller, revision: &str, operation: &str) -> Value {
    let s = c.status();
    let h = s.hello.unwrap();
    json!({"epoch":s.epoch,"automationRevision":s.automation_revision,"connectionId":h.connection_id,"hostInstanceId":h.host_instance_id,"grantRevision":h.grant_revision,"id":"same","expectedRevision":revision,"operation":operation})
}
fn assert_private_home(home: &std::path::Path) {
    let entries: Vec<_> = std::fs::read_dir(home)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("profiles")]);
    fn check(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                check(&entry.path());
            } else {
                assert_ne!(
                    entry.path().extension().and_then(|s| s.to_str()),
                    Some("db"),
                    "a controller must not create host business databases"
                );
            }
        }
    }
    check(home);
}

#[tokio::test]
async fn two_native_controllers_keep_family_status_private_and_stop_delete_only_complete_events() {
    let f = Fixture::new().await;
    seed(&f);
    let a_home = tempfile::tempdir().unwrap();
    let b_home = tempfile::tempdir().unwrap();
    let a_manager =
        ProfileManager::new(a_home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let b_manager =
        ProfileManager::new(b_home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    install(&f, &a_manager, grant()).await;
    install(&f, &b_manager, grant()).await;
    let a = Controller::new(a_manager);
    let b = Controller::new(b_manager);
    a.connect("desk").await.unwrap();
    b.connect("desk").await.unwrap();
    let baseline = page(&a).await;
    assert_eq!(baseline["rows"].as_array().unwrap().len(), 2);
    let mut families = std::collections::BTreeSet::new();
    for row in baseline["rows"].as_array().unwrap() {
        let summary = &row["summary"];
        assert_eq!(summary.as_object().unwrap().len(), 5);
        assert_eq!(summary["definitionRedacted"], true);
        assert_eq!(summary["id"], "same");
        assert_eq!(row["version"]["id"], "same");
        assert_eq!(summary["kind"], row["version"]["scope"]);
        families.insert(summary["kind"].as_str().unwrap());
    }
    assert_eq!(families, ["action", "event"].into());
    let text = baseline.to_string();
    for secret in [
        "owner-only-secret",
        "owner-only-script",
        "private-action",
        "Hidden private label",
    ] {
        assert!(!text.contains(secret));
    }
    let captured = command(&a, "1", "disable-event");
    let before = a.status();
    f.core.points_adjust("alice", 2.0).unwrap();
    wait(&a, |s| s.points_revision != before.points_revision).await;
    assert_eq!(
        a.status().automation_revision,
        before.automation_revision,
        "points updates must not stale an event confirmation"
    );
    let stopped = a
        .manage("controller.automation.command", captured)
        .await
        .unwrap();
    assert_eq!(stopped["pending"], Value::Null);
    assert_eq!(stopped["recent"][0]["intent"]["operation"], "disable-event");
    assert_eq!(stopped["recent"][0]["outcome"], "succeeded");
    assert_eq!(stopped["recent"][0]["revision"], "2");
    wait_rows(&b, |rows| {
        rows.iter().any(|r| {
            r["summary"]["kind"] == "event"
                && r["summary"]["enabled"] == false
                && r["version"]["revision"] == "2"
        })
    })
    .await;
    assert!(
        b.status().command_journal.unwrap().recent.is_empty(),
        "another principal's receipt must not enter this local journal"
    );
    let deleted = b
        .manage(
            "controller.automation.command",
            command(&b, "2", "delete-event"),
        )
        .await
        .unwrap();
    assert_eq!(deleted["recent"][0]["intent"]["operation"], "delete-event");
    assert_eq!(deleted["recent"][0]["revision"], "3");
    for c in [&a, &b] {
        let remaining = wait_rows(c, |rows| {
            rows.len() == 1 && rows[0]["summary"]["kind"] == "action"
        })
        .await;
        assert_eq!(remaining["rows"][0]["version"]["revision"], "1");
        assert_eq!(remaining["rows"][0]["summary"]["enabled"], true);
        assert_eq!(
            c.forward(&c.status().epoch, "automation.snapshot", json!({}), None)
                .await
                .unwrap_err()
                .code,
            "forbidden"
        );
    }
    assert!(f
        .core
        .automation_get(AutomationKind::Event, "same")
        .is_err());
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Action, "same")
            .unwrap()["enabled"],
        true
    );
    a.disconnect().await;
    b.disconnect().await;
    assert_private_home(a_home.path());
    assert_private_home(b_home.path());
    f.finish().await;
}

#[tokio::test]
async fn native_automation_scope_recovery_hides_removed_families_and_rejects_old_confirmations() {
    let f = Fixture::new().await;
    seed(&f);
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = install(&f, &manager, grant()).await;
    let c = Controller::new(manager);
    c.connect("desk").await.unwrap();
    assert_eq!(page(&c).await["rows"].as_array().unwrap().len(), 2);
    let captured = command(&c, "1", "disable-event");
    let mut action_only = grant();
    action_only
        .resources
        .insert(ResourceKind::Automation, ["action:same".into()].into());
    f.service
        .set_grant(GrantSet {
            principal_id: profile.principal_id.clone(),
            expected_revision: "1".into(),
            grant: action_only,
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "2")
    })
    .await;
    let narrowed = page(&c).await;
    assert_eq!(narrowed["rows"].as_array().unwrap().len(), 1);
    assert_eq!(narrowed["rows"][0]["summary"]["kind"], "action");
    let rejected = c
        .manage("controller.automation.command", captured)
        .await
        .unwrap_err();
    assert!(matches!(
        rejected.code.as_str(),
        "stale_view" | "forbidden" | "not_ready"
    ));
    let rejected = c
        .manage(
            "controller.automation.command",
            command(&c, "1", "disable-event"),
        )
        .await
        .unwrap_err();
    assert!(matches!(rejected.code.as_str(), "stale_view" | "forbidden"));
    assert_eq!(
        f.core
            .automation_get_versioned(AutomationKind::Event, "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    f.service
        .set_grant(GrantSet {
            principal_id: profile.principal_id.clone(),
            expected_revision: "2".into(),
            grant: Grant::viewer(),
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "3")
    })
    .await;
    let s = c.status();
    let denied = c
        .manage(
            "controller.automations",
            json!({"epoch":s.epoch,"automationRevision":s.automation_revision,"offset":0}),
        )
        .await
        .unwrap_err();
    assert!(matches!(denied.code.as_str(), "forbidden" | "not_ready"));
    let mut event_only = grant();
    event_only
        .resources
        .insert(ResourceKind::Automation, ["event:same".into()].into());
    f.service
        .set_grant(GrantSet {
            principal_id: profile.principal_id,
            expected_revision: "3".into(),
            grant: event_only,
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "4")
    })
    .await;
    assert_eq!(page(&c).await["rows"][0]["summary"]["kind"], "event");
    let mut forged = command(&c, "1", "disable-event");
    forged["kind"] = "action".into();
    assert_eq!(
        c.manage("controller.automation.command", forged)
            .await
            .unwrap_err()
            .code,
        "invalid_params"
    );
    for operation in ["disable", "delete", "enable-event", "adjust-points"] {
        let rejected = c
            .manage("controller.automation.command", command(&c, "1", operation))
            .await
            .unwrap_err();
        assert!(
            matches!(rejected.code.as_str(), "invalid_params" | "forbidden"),
            "{operation}"
        );
    }
    let mut forged = command(&c, "1", "disable-event");
    forged["connectionId"] = "a-previous-connection".into();
    let rejected = c
        .manage("controller.automation.command", forged)
        .await
        .unwrap_err();
    assert!(matches!(rejected.code.as_str(), "stale_view" | "not_ready"));
    assert_eq!(
        f.core
            .automation_get_versioned(AutomationKind::Event, "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    c.disconnect().await;
    assert_private_home(home.path());
    f.finish().await;
}

fn prepare_event(
    profile: &profiles::RemoteProfile,
    manager: &ProfileManager,
    instance: &str,
) -> Intent {
    let journal = Journal::open(manager.directory(), profile).unwrap();
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: Operation::DisableEvent,
        resource_id: "same".into(),
        host_instance_id: instance.into(),
        expected_revision: Some("1".into()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        expected_generation: None,
        created_at: profiles::commands::timestamp(),
    };
    journal.begin(intent.clone()).unwrap();
    intent
}

#[tokio::test]
async fn native_event_pending_intent_blocks_other_domains_and_never_replays_without_a_result() {
    let f = Fixture::new().await;
    seed(&f);
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = install(&f, &manager, grant()).await;
    let original = prepare_event(&profile, &manager, f.service.instance_id());
    assert_eq!(
        manager.forget("desk").await.unwrap_err().code,
        "command_pending"
    );
    let c = Controller::new(manager.clone());
    c.connect("desk").await.unwrap();
    wait(&c, |s| !s.command_busy && s.command_error_code.is_some()).await;
    assert_eq!(
        c.status().command_journal.unwrap().pending,
        Some(original.clone())
    );
    assert_eq!(
        f.core
            .automation_get_versioned(AutomationKind::Event, "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    assert_eq!(
        c.manage(
            "controller.automation.command",
            command(&c, "1", "delete-event")
        )
        .await
        .unwrap_err()
        .code,
        "command_pending"
    );
    let s = c.status();
    assert_eq!(c.manage("controller.workflow.command",json!({"epoch":s.epoch,"viewRevision":s.view_revision,"id":"welcome","expectedRevision":"1","operation":"disable"})).await.unwrap_err().code,"command_pending");
    let s = c.status();
    let points = c
        .manage(
            "controller.points",
            json!({"epoch":s.epoch,"pointsRevision":s.points_revision}),
        )
        .await
        .unwrap();
    assert_eq!(c.manage("controller.points.command",json!({"epoch":s.epoch,"connectionId":s.hello.unwrap().connection_id,"uniqueId":"alice","delta":1,"currencyName":points["currencyName"]})).await.unwrap_err().code,"command_pending");
    let review = json!({"epoch":c.status().epoch,"commandId":original.command_id,"acknowledgeUnknown":false});
    assert_eq!(
        c.manage("controller.command.review", review.clone())
            .await
            .unwrap_err()
            .code,
        "confirmation_required"
    );
    let mut review = review;
    review["acknowledgeUnknown"] = true.into();
    c.manage("controller.command.review", review).await.unwrap();
    let history = c.status().command_journal.unwrap();
    assert!(history.pending.is_none());
    assert_eq!(history.recent[0].outcome, Outcome::ReviewedUnknown);
    assert_eq!(history.recent[0].intent, original);
    let result = c
        .manage(
            "controller.automation.command",
            command(&c, "1", "disable-event"),
        )
        .await
        .unwrap();
    assert_ne!(
        result["recent"][0]["intent"]["commandId"],
        original.command_id
    );
    assert_eq!(result["recent"][0]["revision"], "2");
    c.disconnect().await;
    drop(c);
    assert_private_home(home.path());
    f.finish().await;
}

#[tokio::test]
async fn native_event_receipt_recovery_after_restart_does_not_undo_a_later_owner_change() {
    let f = Fixture::new().await;
    seed(&f);
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = install(&f, &manager, grant()).await;
    let original = prepare_event(&profile, &manager, f.service.instance_id());
    let remote = manager.open("desk").await.unwrap();
    let accepted = remote
        .call_value_with_meta(
            "automation.disable",
            json!({"kind":"event","id":"same"}),
            Some(RequestMetadata {
                command_id: Some(original.command_id.clone()),
                host_instance_id: Some(original.host_instance_id.clone()),
                expected_revision: Some("1".into()),
            }),
        )
        .await
        .unwrap();
    assert_eq!(accepted.result, json!({"id":"same","enabled":false}));
    remote.close();
    drop(remote);
    // The host committed, but the native journal did not record the reply.
    f.core
        .automation_set_enabled(AutomationKind::Event, "same", true)
        .unwrap();
    let c = Controller::new(manager.clone());
    c.connect("desk").await.unwrap();
    wait(&c, |s| {
        !s.command_busy
            && s.command_journal
                .as_ref()
                .is_some_and(|j| j.pending.is_none() && !j.recent.is_empty())
    })
    .await;
    let history = c.status().command_journal.unwrap();
    assert_eq!(history.recent[0].intent, original);
    assert_eq!(history.recent[0].revision.as_deref(), Some("2"));
    assert_eq!(history.recent[0].outcome, Outcome::Succeeded);
    let host = f
        .core
        .automation_get_versioned(AutomationKind::Event, "same")
        .unwrap();
    assert_eq!(host.value.unwrap()["enabled"], true);
    assert_eq!(host.resource_versions[0].revision, "3");
    let current = page(&c).await;
    assert!(current["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["summary"]["kind"] == "event"
            && r["summary"]["enabled"] == true
            && r["version"]["revision"] == "3"));
    assert_eq!(
        c.forward(
            &c.status().epoch,
            "automation.disable",
            json!({"id":"same"}),
            None
        )
        .await
        .unwrap_err()
        .code,
        "forbidden"
    );
    c.disconnect().await;
    drop(c);
    let restarted = Controller::new(manager);
    restarted.connect("desk").await.unwrap();
    assert!(restarted
        .status()
        .command_journal
        .unwrap()
        .pending
        .is_none());
    assert_eq!(
        f.core
            .automation_get_versioned(AutomationKind::Event, "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "3"
    );
    restarted.disconnect().await;
    assert_private_home(home.path());
    f.finish().await;
}

#[tokio::test]
async fn native_automation_pages_bind_one_projection_revision_and_independent_family_keys() {
    let f = Fixture::new().await;
    seed(&f);
    for index in 0..40 {
        f.core
            .automation_create(
                AutomationKind::Event,
                json!({"id":format!("extra-{index:02}"),"name":"Bounded status","enabled":true}),
            )
            .unwrap();
    }
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let mut all = grant();
    all.resources.remove(&ResourceKind::Automation);
    install(&f, &manager, all).await;
    let c = Controller::new(manager);
    c.connect("desk").await.unwrap();
    let initial = page(&c).await;
    assert_eq!(initial["rows"].as_array().unwrap().len(), 32);
    assert_eq!(initial["next"], 32);
    let mut collected = initial["rows"].as_array().unwrap().clone();
    let s = c.status();
    assert_eq!(initial["automationRevision"], s.automation_revision);
    let tail=c.manage("controller.automations",json!({"epoch":s.epoch,"automationRevision":s.automation_revision,"offset":initial["next"]})).await.unwrap();
    collected.extend(tail["rows"].as_array().unwrap().iter().cloned());
    assert!(tail["next"].is_null());
    assert_eq!(collected.len(), 44);
    let keys: std::collections::BTreeSet<_> = collected
        .iter()
        .map(|r| {
            (
                r["version"]["scope"].as_str().unwrap(),
                r["version"]["id"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(keys.len(), 44);
    f.core
        .automation_set_enabled(AutomationKind::Event, "same", false)
        .unwrap();
    wait(&c, |current| {
        current.automation_revision != s.automation_revision
    })
    .await;
    assert_eq!(
        c.manage(
            "controller.automations",
            json!({"epoch":s.epoch,"automationRevision":s.automation_revision,"offset":32})
        )
        .await
        .unwrap_err()
        .code,
        "stale_view"
    );
    c.disconnect().await;
    assert_private_home(home.path());
    f.finish().await;
}

#[tokio::test]
async fn adding_automation_to_a_surviving_workflow_points_stream_rebuilds_its_baseline_and_updates()
{
    let f = Fixture::new().await;
    seed(&f);
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let mut initial = grant();
    initial.capabilities.remove(&Capability::AutomationRead);
    let profile = install(&f, &manager, initial).await;
    let c = Controller::new(manager);
    c.connect("desk").await.unwrap();
    let before = c.status();
    assert!(before
        .hello
        .as_ref()
        .unwrap()
        .capabilities
        .contains(&Capability::WorkflowsRead));
    assert!(before
        .hello
        .as_ref()
        .unwrap()
        .capabilities
        .contains(&Capability::PointsRead));
    assert_eq!(super::page(&c)["rows"].as_array().unwrap().len(), 1);
    assert!(c
        .manage(
            "controller.automations",
            json!({"epoch":before.epoch,"automationRevision":before.automation_revision,"offset":0})
        )
        .await
        .is_err());
    f.service
        .set_grant(GrantSet {
            principal_id: profile.principal_id,
            expected_revision: "1".into(),
            grant: grant(),
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "2")
    })
    .await;
    let added = page(&c).await;
    assert_eq!(added["rows"].as_array().unwrap().len(), 2);
    assert_eq!(super::page(&c)["rows"].as_array().unwrap().len(), 1);
    // Seeing the baseline alone would not prove the stream subscribed to the
    // newly permitted domain. A later owner write must arrive without polling.
    f.core
        .automation_set_enabled(AutomationKind::Event, "same", false)
        .unwrap();
    let changed = wait_rows(&c, |rows| {
        rows.iter().any(|row| {
            row["summary"]["kind"] == "event"
                && row["summary"]["enabled"] == false
                && row["version"]["revision"] == "2"
        })
    })
    .await;
    assert_ne!(changed["automationRevision"], added["automationRevision"]);
    c.disconnect().await;
    assert_private_home(home.path());
    f.finish().await;
}

fn hold_journal_writer(
    journal: Arc<Journal>,
) -> (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>) {
    let (entered, ready) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || journal.hold_writer_for_test(entered, released));
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    (release, thread)
}

#[tokio::test]
async fn switching_profiles_while_syncing_an_event_intent_never_dispatches_and_preserves_its_original_journal(
) {
    let f = Fixture::new().await;
    seed(&f);
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    install(&f, &manager, grant()).await;
    manager
        .install(
            "other",
            &f.endpoint,
            f.pair(grant()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    let c = Arc::new(Controller::new(manager));
    c.connect("desk").await.unwrap();
    let original = command(&c, "1", "disable-event");
    let journal = c.shared.view.read().unwrap().journal.clone().unwrap();
    let (release, writer) = hold_journal_writer(journal);
    let caller = c.clone();
    let task = tokio::spawn(async move {
        caller
            .manage("controller.automation.command", original)
            .await
    });
    wait(&c, |s| s.command_busy).await;
    c.connect("other").await.unwrap();
    release.send(()).unwrap();
    writer.join().unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(error.code.as_str(), "stale_view" | "not_ready"));
    assert_eq!(c.status().profile.as_deref(), Some("other"));
    let other = c.status().command_journal.unwrap();
    assert!(other.pending.is_none());
    assert!(other.recent.is_empty());
    assert_eq!(
        f.core
            .automation_get_versioned(AutomationKind::Event, "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    c.connect("desk").await.unwrap();
    let origin = c.status().command_journal.unwrap();
    assert!(origin.pending.is_none());
    assert_eq!(origin.recent.len(), 1);
    assert_eq!(origin.recent[0].intent.operation, Operation::DisableEvent);
    assert_eq!(origin.recent[0].outcome, Outcome::Rejected);
    assert!(origin.recent[0].revision.is_none());
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Event, "same")
            .unwrap()["enabled"],
        true
    );
    c.disconnect().await;
    assert_private_home(home.path());
    f.finish().await;
}

#[tokio::test]
async fn reusable_grant_activates_authored_action_and_event_through_native_controller() {
    use secrecy::ExposeSecret;
    use tiktools_collaboration::{AutomationDelegation, CommandRequest, DelegatedAutomationEffect};
    use tiktools_core::control::{AutomationMutation, CommandCaller};
    let f = Fixture::new().await;
    let mut delegated = grant();
    delegated.capabilities.insert(Capability::LiveRead);
    delegated.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Log].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
    });
    let credentials = f.pair(delegated).await;
    let auth = f
        .service
        .authenticate_live(credentials.access.expose_secret(), false, false)
        .unwrap();
    let instance = f.service.instance_id().to_owned();
    for (kind, record) in [
        (
            AutomationKind::Action,
            json!({"id":"same","name":"Log","enabled":false,"typeId":"core.log","config":{"message":"hello"}}),
        ),
        (
            AutomationKind::Event,
            json!({"id":"same","name":"Chat","enabled":false,"trigger":"tiktok.chat","actionIds":["same"]}),
        ),
    ] {
        f.core
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                CommandRequest {
                    command_id: uuid::Uuid::new_v4().to_string(),
                    host_instance_id: instance.clone(),
                    expected_revision: "0".into(),
                },
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    manager
        .install("desk", &f.endpoint, credentials, ClientKind::Agent)
        .await
        .unwrap();
    let controller = Controller::new(manager);
    controller.connect("desk").await.unwrap();
    for (family, operation) in [("action", "enable-action"), ("event", "enable-event")] {
        let page = wait_rows(&controller, |rows| {
            rows.iter()
                .any(|r| r["summary"]["kind"] == family && r["summary"]["enabled"] == false)
        })
        .await;
        let row = page["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["summary"]["kind"] == family)
            .unwrap();
        let s = controller.status();
        let hello = s.hello.as_ref().unwrap();
        assert_eq!(
            hello.features.get("automationReusableActivation"),
            Some(&true)
        );
        let request = json!({"epoch":s.epoch,"automationRevision":s.automation_revision,"connectionId":hello.connection_id,
            "hostInstanceId":hello.host_instance_id,"grantRevision":hello.grant_revision,"id":"same",
            "expectedRevision":row["version"]["revision"],"operation":operation});
        controller
            .manage("controller.automation.command", request)
            .await
            .unwrap();
        wait_rows(&controller, |rows| {
            rows.iter()
                .any(|r| r["summary"]["kind"] == family && r["summary"]["enabled"] == true)
        })
        .await;
        assert_eq!(
            controller.status().command_journal.as_ref().unwrap().recent[0].outcome,
            Outcome::Succeeded
        );
    }
    controller.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn native_activation_receipt_recovers_original_id_without_reapplying() {
    use secrecy::ExposeSecret;
    use tiktools_collaboration::{AutomationDelegation, CommandRequest, DelegatedAutomationEffect};
    use tiktools_core::control::{AutomationMutation, CommandCaller};
    let f = Fixture::new().await;
    let mut permitted = grant();
    permitted.capabilities.insert(Capability::LiveRead);
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Log].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
    });
    let credentials = f.pair(permitted).await;
    let auth = f
        .service
        .authenticate_live(credentials.access.expose_secret(), false, false)
        .unwrap();
    f.core.automation_command(CommandCaller::Remote(auth), CommandRequest { command_id: uuid::Uuid::new_v4().to_string(), host_instance_id: f.service.instance_id().into(), expected_revision: "0".into() }, AutomationMutation::Create { kind: AutomationKind::Action, record: json!({"id":"same","name":"Log","enabled":false,"typeId":"core.log","config":{"message":"hello"}}) }).unwrap();
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = manager
        .install("desk", &f.endpoint, credentials, ClientKind::Agent)
        .await
        .unwrap();
    let original = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: Operation::EnableAction,
        resource_id: "same".into(),
        host_instance_id: f.service.instance_id().into(),
        expected_revision: Some("1".into()),
        delta: None,
        currency_name: None,
        proposal_revision: None,
        expected_generation: None,
        created_at: profiles::commands::timestamp(),
    };
    let journal = Journal::open(manager.directory(), &profile).unwrap();
    journal.begin(original.clone()).unwrap();
    drop(journal);
    let remote = manager.open("desk").await.unwrap();
    let reply = remote
        .call_value_with_meta(
            "automation.enable",
            json!({"kind":"action","id":"same"}),
            Some(RequestMetadata {
                command_id: Some(original.command_id.clone()),
                host_instance_id: Some(original.host_instance_id.clone()),
                expected_revision: original.expected_revision.clone(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(
        reply.result,
        json!({"kind":"action","id":"same","enabled":true})
    );
    remote.close();
    drop(remote);
    f.core
        .automation_set_enabled(AutomationKind::Action, "same", false)
        .unwrap();
    let c = Controller::new(manager);
    c.connect("desk").await.unwrap();
    wait(&c, |s| {
        !s.command_busy
            && s.command_journal
                .as_ref()
                .is_some_and(|j| j.pending.is_none() && !j.recent.is_empty())
    })
    .await;
    let history = c.status().command_journal.unwrap();
    assert_eq!(history.recent[0].intent, original);
    assert_eq!(history.recent[0].revision.as_deref(), Some("2"));
    assert_eq!(history.recent[0].outcome, Outcome::Succeeded);
    let after = f
        .core
        .automation_get_versioned(AutomationKind::Action, "same")
        .unwrap();
    assert_eq!(after.value.unwrap()["enabled"], false);
    assert_eq!(after.resource_versions[0].revision, "3");
    c.disconnect().await;
    f.finish().await;
}
