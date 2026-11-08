use super::*;
use std::{future::Future, task::Poll};

#[tokio::test]
async fn a_private_read_blocked_on_storage_cannot_return_source_after_its_grant_is_removed() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let c = setup(&f, &home, Arc::new(Store::default())).await;
    save(
        &c,
        "action",
        "create",
        "0",
        "0",
        &record("action", "private retained copy").to_string(),
    )
    .await;
    let journal = c.shared.view.read().unwrap().journal.clone().unwrap();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let holding =
        tokio::task::spawn_blocking(move || journal.hold_writer_for_test(entered_tx, release_rx));
    tokio::task::spawn_blocking(move || entered_rx.recv_timeout(Duration::from_secs(5)).unwrap())
        .await
        .unwrap();
    let mut p = params(&c);
    p["kind"] = json!("action");
    p["id"] = json!("same");
    let reading = c.manage("controller.proposals.get", p);
    tokio::pin!(reading);
    std::future::poll_fn(|cx| {
        assert!(reading.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    let principal = c.status().hello.unwrap().principal_id;
    let mut g = grant();
    g.resources
        .insert(ResourceKind::Automation, ["event:same".into()].into());
    f.service
        .set_grant(GrantSet {
            principal_id: principal,
            expected_revision: "1".into(),
            grant: g,
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "2")
    })
    .await;
    release_tx.send(()).unwrap();
    holding.await.unwrap();
    let error = reading.await.unwrap_err();
    assert_eq!(error.code, "stale_view");
    assert!(!format!("{error:?}").contains("local-fixture-secret"));
    c.disconnect().await;
    f.finish().await;
}
use crate::remote::profiles::commands::{Intent, Operation};
use tiktools_collaboration::{Grant, GrantSet};
use tiktools_core::control::AutomationKind;

fn grant() -> Grant {
    Grant {
        capabilities: [
            Capability::AutomationRead,
            Capability::AutomationEdit,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            ResourceKind::Automation,
            ["action:same".into(), "event:same".into()].into(),
        )]
        .into(),
        ..Grant::default()
    }
}
fn params(c: &Controller) -> Value {
    let s = c.status();
    let h = s.hello.unwrap();
    json!({"epoch":s.epoch,"connectionId":h.connection_id,"hostInstanceId":h.host_instance_id,"grantRevision":h.grant_revision})
}
async fn inspect(c: &Controller, kind: &str) -> Value {
    let mut p = params(c);
    p["kind"] = json!(kind);
    p["id"] = json!("same");
    c.manage("controller.proposals.inspect", p).await.unwrap()
}
fn record(kind: &str, name: &str) -> Value {
    if kind == "action" {
        json!({"id":"same","name":name,"enabled":false,"typeId":"core.log","config":{"secret":"local-fixture-secret"}})
    } else {
        json!({"id":"same","name":name,"enabled":false,"trigger":"tiktok.chat","actionIds":["same"]})
    }
}
async fn save(
    c: &Controller,
    kind: &str,
    mode: &str,
    base: &str,
    store: &str,
    text: &str,
) -> Value {
    let mut p = params(c);
    p["kind"] = json!(kind);
    p["id"] = json!("same");
    p["mode"] = json!(mode);
    p["baseRevision"] = json!(base);
    p["expectedStoreRevision"] = json!(store);
    p["text"] = json!(text);
    p["acknowledgeStorage"] = json!(true);
    c.manage("controller.proposals.save", p).await.unwrap()
}
fn submit(c: &Controller, kind: &str, inspection: &Value, store: &str) -> Value {
    let mut p = params(c);
    p["kind"] = json!(kind);
    p["id"] = json!("same");
    p["inspectionId"] = inspection["inspectionId"].clone();
    p["expectedStoreRevision"] = json!(store);
    p["acknowledgeProposal"] = json!(true);
    p
}
async fn setup(f: &Fixture, home: &tempfile::TempDir, store: Arc<Store>) -> Controller {
    let manager = ProfileManager::new(home.path().join("profiles"), store).unwrap();
    manager
        .install(
            "desk",
            &f.endpoint,
            f.pair(grant()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    let c = Controller::new(manager);
    c.connect("desk").await.unwrap();
    c
}

#[tokio::test]
async fn native_saved_proposals_create_and_replace_both_families_without_returning_host_source() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::default());
    let c = setup(&f, &home, store.clone()).await;
    let mut revision = "0".to_string();
    for (kind, mode, base, name) in [
        ("action", "create", "0", "first action"),
        ("action", "update", "1", "changed action"),
        ("event", "create", "0", "first event"),
        ("event", "update", "1", "changed event"),
    ] {
        let inspected = inspect(&c, kind).await;
        assert_eq!(inspected["revision"], base);
        let body = record(kind, name).to_string();
        let saved = save(&c, kind, mode, base, &revision, &body).await;
        revision = saved["storeRevision"].as_str().unwrap().into();
        let result = c
            .manage(
                "controller.proposals.submit",
                submit(&c, kind, &inspected, &revision),
            )
            .await
            .unwrap();
        assert!(result["pending"].is_null());
        assert_eq!(result["recent"][0]["intent"]["proposalRevision"], revision);
        assert!(!result.to_string().contains("local-fixture-secret"));
        let check = inspect(&c, kind).await;
        assert_eq!(check["summary"].as_object().unwrap().len(), 5);
        assert!(!check.to_string().contains("local-fixture-secret"));
        assert_eq!(check["summary"]["enabled"], false);
        let family = if kind == "action" {
            AutomationKind::Action
        } else {
            AutomationKind::Event
        };
        let host = f.core.automation_get(family, "same").unwrap();
        assert_eq!(host["name"], name);
        assert_eq!(host["enabled"], false);
        let authority = f.core.automation_authority(family, "same").unwrap();
        assert!(serde_json::to_value(authority)
            .unwrap()
            .to_string()
            .contains("delegated"));
    }
    assert_eq!(c.status().command_journal.unwrap().recent.len(), 4);
    c.disconnect().await;
    drop(c);
    let c = Controller::new(ProfileManager::new(home.path().join("profiles"), store).unwrap());
    c.connect("desk").await.unwrap();
    let mut p = params(&c);
    p["kind"] = json!("action");
    p["id"] = json!("same");
    let copy = c.manage("controller.proposals.get", p).await.unwrap();
    assert!(copy["draft"]["text"]
        .as_str()
        .unwrap()
        .contains("local-fixture-secret"));
    assert_eq!(copy["storeRevision"], "4");
    let mut list = params(&c);
    list["offset"] = json!(0);
    let list = c.manage("controller.proposals.list", list).await.unwrap();
    assert_eq!(list["rows"].as_array().unwrap().len(), 2);
    assert!(!list.to_string().contains("local-fixture-secret"));
    let entries: Vec<_> = std::fs::read_dir(home.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("profiles")]);
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn native_proposals_reject_unsaved_mismatched_or_changed_copies_and_caller_dispatch_metadata()
{
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let c = setup(&f, &home, Arc::new(Store::default())).await;
    let inspected = inspect(&c, "action").await;
    save(&c, "action", "create", "0", "0", "unfinished private JSON").await;
    assert_eq!(
        c.manage(
            "controller.proposals.submit",
            submit(&c, "action", &inspected, "1")
        )
        .await
        .unwrap_err()
        .code,
        "invalid_params"
    );
    assert!(c.status().command_journal.unwrap().pending.is_none());
    save(
        &c,
        "action",
        "create",
        "0",
        "1",
        &record("action", "ready").to_string(),
    )
    .await;
    assert_eq!(
        c.manage(
            "controller.proposals.submit",
            submit(&c, "action", &inspected, "1")
        )
        .await
        .unwrap_err()
        .code,
        "proposal_conflict"
    );
    for field in ["record", "commandId", "expectedRevision", "method"] {
        let mut p = submit(&c, "action", &inspected, "2");
        p[field] = json!("caller supplied");
        assert_eq!(
            c.manage("controller.proposals.submit", p)
                .await
                .unwrap_err()
                .code,
            "invalid_params"
        );
    }
    let newer = inspect(&c, "event").await;
    assert_ne!(newer["inspectionId"], inspected["inspectionId"]);
    assert_eq!(
        c.manage(
            "controller.proposals.submit",
            submit(&c, "action", &inspected, "2")
        )
        .await
        .unwrap_err()
        .code,
        "stale_view"
    );
    let inspected = inspect(&c, "action").await;
    save(
        &c,
        "action",
        "update",
        "1",
        "2",
        &record("action", "bad base").to_string(),
    )
    .await;
    assert_eq!(
        c.manage(
            "controller.proposals.submit",
            submit(&c, "action", &inspected, "3")
        )
        .await
        .unwrap_err()
        .code,
        "stale_view"
    );
    assert!(c.status().command_journal.unwrap().pending.is_none());
    assert!(f
        .core
        .automation_get(AutomationKind::Action, "same")
        .is_err());
    assert_eq!(
        c.forward(
            &c.status().epoch,
            "automation.create",
            json!({"kind":"action","record":record("action","bypass")}),
            None
        )
        .await
        .unwrap_err()
        .code,
        "forbidden"
    );
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn grant_changes_fence_private_text_and_sending_but_read_only_access_can_keep_local_copies() {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let c = setup(&f, &home, Arc::new(Store::default())).await;
    let old = params(&c);
    save(
        &c,
        "action",
        "create",
        "0",
        "0",
        &record("action", "retained").to_string(),
    )
    .await;
    let principal = c.status().hello.unwrap().principal_id;
    let mut narrowed = grant();
    narrowed
        .resources
        .insert(ResourceKind::Automation, ["event:same".into()].into());
    f.service
        .set_grant(GrantSet {
            principal_id: principal.clone(),
            expected_revision: "1".into(),
            grant: narrowed,
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "2")
    })
    .await;
    let mut p = old;
    p["kind"] = json!("action");
    p["id"] = json!("same");
    assert_eq!(
        c.manage("controller.proposals.get", p)
            .await
            .unwrap_err()
            .code,
        "stale_view"
    );
    let mut p = params(&c);
    p["kind"] = json!("action");
    p["id"] = json!("same");
    assert_eq!(
        c.manage("controller.proposals.get", p)
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    let list = c
        .manage("controller.proposals.list", params(&c))
        .await
        .unwrap();
    assert!(list["rows"].as_array().unwrap().is_empty());
    let mut readonly = grant();
    readonly.capabilities.remove(&Capability::AutomationEdit);
    f.service
        .set_grant(GrantSet {
            principal_id: principal,
            expected_revision: "2".into(),
            grant: readonly,
        })
        .unwrap();
    wait(&c, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "3")
    })
    .await;
    let i = inspect(&c, "action").await;
    save(
        &c,
        "action",
        "create",
        "0",
        "1",
        &record("action", "read only local save").to_string(),
    )
    .await;
    assert_eq!(
        c.manage("controller.proposals.submit", submit(&c, "action", &i, "2"))
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert!(f
        .core
        .automation_get(AutomationKind::Action, "same")
        .is_err());
    c.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn proposal_recovery_checks_original_outcome_after_owner_changes_without_reinstalling_saved_text(
) {
    let f = Fixture::new().await;
    let home = tempfile::tempdir().unwrap();
    let c = setup(&f, &home, Arc::new(Store::default())).await;
    save(
        &c,
        "action",
        "create",
        "0",
        "0",
        &record("action", "saved author copy").to_string(),
    )
    .await;
    let s = c.status();
    let h = s.hello.unwrap();
    let intent = Intent {
        command_id: uuid::Uuid::new_v4().to_string(),
        operation: Operation::CreateActionDraft,
        resource_id: "same".into(),
        host_instance_id: h.host_instance_id,
        expected_revision: Some("0".into()),
        delta: None,
        currency_name: None,
        proposal_revision: Some("1".into()),
        created_at: profiles::commands::timestamp(),
    };
    let (j, remote) = {
        let v = c.shared.view.read().unwrap();
        (v.journal.clone().unwrap(), v.remote.clone().unwrap())
    };
    j.begin(intent.clone()).unwrap();
    remote
        .call_value_with_meta(
            "automation.create",
            json!({"kind":"action","record":record("action","accepted original")}),
            Some(RequestMetadata {
                command_id: Some(intent.command_id.clone()),
                host_instance_id: Some(intent.host_instance_id.clone()),
                expected_revision: Some("0".into()),
            }),
        )
        .await
        .unwrap();
    f.core.automation_update(AutomationKind::Action,"same",json!({"id":"same","enabled":false,"name":"later owner edit","typeId":"core.log","config":{}})).unwrap();
    let result = c
        .manage(
            "controller.command.resolve",
            json!({"epoch":c.status().epoch,"commandId":intent.command_id}),
        )
        .await
        .unwrap();
    assert!(result["pending"].is_null());
    assert_eq!(result["recent"][0]["revision"], "1");
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Action, "same")
            .unwrap()["name"],
        "later owner edit"
    );
    assert_eq!(j.view().recent[0].intent, intent);
    c.disconnect().await;
    f.finish().await;
}
