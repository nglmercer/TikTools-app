//! Restricted approved outcomes remain visible to presentation consumers while
//! arbitrary subscribed plugin code cannot observe or amplify them.
use super::*;
use crate::{
    collaboration::{
        hash_secret, Capability, ClientKind, Collect, CommandRequest, Grant, InviteCreate, Redeem,
    },
    control::{
        ApprovalActivateParams, ApprovalEnablement, AutomationKind, AutomationMutation,
        CommandCaller,
    },
    services::{AwardOptions, PointAction},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approved_points_and_run_outcomes_never_invoke_plugins_but_owner_events_still_do() {
    let state = Arc::new(FakePluginState::default());
    let (core, _, root) = core_with_fake_hotkeys(state.clone());
    let manifest =
        FAKE_HOTKEY_MANIFEST.replace("\"points.changed\"", "\"points.changed\", \"automation.*\"");
    std::fs::write(root.join("hotkeys/plugin.json"), manifest).unwrap();
    core.plugins.scan().unwrap();
    core.plugins.start("hotkeys").unwrap();
    core.rebuild_processor_index();
    core.points
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();

    let service = core.collaboration().unwrap();
    let mut config = service.configuration().unwrap();
    config.enabled = true;
    config.advertised_origin = Some("http://127.0.0.1:8788".into());
    service.configure(config).unwrap();
    let invite = service
        .create_invitation(InviteCreate {
            grant: Grant {
                capabilities: [
                    Capability::AutomationRead,
                    Capability::AutomationEdit,
                    Capability::LiveRead,
                    Capability::PointsRead,
                    Capability::PointsAdjust,
                ]
                .into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let exchange = "a".repeat(64);
    let pending = service
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Restricted observer author".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "fixture",
        )
        .unwrap();
    service.decide_request(&pending.request_id, true).unwrap();
    let credentials = service
        .collect(
            Collect {
                request_id: pending.request_id,
                exchange_secret: exchange,
            },
            "fixture",
        )
        .unwrap();
    let author = service
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap();
    let request = |base: &str| CommandRequest {
        command_id: format!("observer-proof-{}", fastrand::u64(..)),
        host_instance_id: service.instance_id().into(),
        expected_revision: base.into(),
    };
    for (id, type_id, config) in [
        (
            "approved-log",
            "core.log",
            json!({"message":"restricted {{event.user.uniqueId}}"}),
        ),
        (
            "approved-points",
            "core.points",
            json!({"uniqueId":"alice","delta":2.0}),
        ),
    ] {
        core.automation_command(
            CommandCaller::Remote(author.clone()),
            request("0"),
            AutomationMutation::Create {
                kind: AutomationKind::Action,
                record: json!({"id":id,"enabled":false,"typeId":type_id,"config":config}),
            },
        )
        .unwrap();
        let review = core
            .automation_execution_manifest(AutomationKind::Action, id)
            .unwrap();
        core.automation_approval_activate(
            request(&review.resource_versions[0].revision),
            ApprovalActivateParams {
                kind: AutomationKind::Action,
                id: id.into(),
                reviewed_versions: review.resource_versions,
                manifest_digest: review.value["manifestDigest"].as_str().unwrap().into(),
                enable_plan: vec![ApprovalEnablement {
                    kind: AutomationKind::Action,
                    id: id.into(),
                    enabled: true,
                }],
                acknowledge_effects: true,
            },
        )
        .unwrap();
    }

    core.spawn_plugin_event_observer(&tokio::runtime::Handle::current());
    core.events
        .publish_domain(crate::events::DomainEvent::PointsChanged {
            unique_id: "owner-before".into(),
            delta: 1.0,
            total_points: 1.0,
            level: 1,
        });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while state.event_calls.lock().unwrap().len() != 1 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let mut ui = core.events.subscribe_domain();
    for id in ["approved-log", "approved-points"] {
        let body = core.automation_get(AutomationKind::Action, id).unwrap();
        let outcome = core
            .execute_action(
                &body,
                &json!({"type":"tiktok.chat","user":{"uniqueId":"alice"}}),
                None,
                false,
            )
            .await;
        assert_eq!(outcome["status"], "ok", "{outcome}");
    }
    assert_eq!(core.points_viewer("alice").unwrap()["points"], 3.0);
    let mut points = false;
    let mut runs = 0;
    let mut history = false;
    while let Ok(event) = ui.try_recv() {
        match event {
            crate::events::DomainEvent::PointsChanged { unique_id, .. } => {
                points |= unique_id == "alice"
            }
            crate::events::DomainEvent::AutomationRunCompleted { .. } => runs += 1,
            crate::events::DomainEvent::AutomationRunsChanged { .. } => history = true,
            _ => {}
        }
    }
    assert!(
        points && runs == 2 && history,
        "presentation retained points/runs/history"
    );
    let owner_outcome = core
        .execute_action(
            &json!({"id":"owner-inline-log","typeId":"core.log","config":{"message":"Owner positive control"}}),
            &json!({"type":"tiktok.chat"}),
            None,
            false,
        )
        .await;
    assert_eq!(owner_outcome["status"], "ok");
    let mut owner_history = false;
    while let Ok(event) = ui.try_recv() {
        if let crate::events::DomainEvent::AutomationRunsChanged { runs } = event {
            owner_history |= runs.len() == 3;
        }
    }
    assert!(
        owner_history,
        "later Owner completion still presents complete mixed history"
    );
    core.events
        .publish_domain(crate::events::DomainEvent::PointsChanged {
            unique_id: "owner-after".into(),
            delta: 1.0,
            total_points: 1.0,
            level: 1,
        });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while state.event_calls.lock().unwrap().len() < 3 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    let calls = state.event_calls.lock().unwrap().clone();
    assert_eq!(
        calls.len(),
        3,
        "approved results must never enter plugin queues: {calls:?}"
    );
    assert_eq!(calls[0]["event"]["data"]["uniqueId"], "owner-before");
    assert_eq!(calls[1]["event"]["topic"], "automation.run.completed");
    assert_eq!(
        calls[1]["event"]["data"]["run"]["actionId"],
        "owner-inline-log"
    );
    assert_eq!(calls[2]["event"]["data"]["uniqueId"], "owner-after");
    assert!(
        calls
            .iter()
            .all(|call| call["event"]["topic"] != "automation.runs.changed"),
        "later Owner completion cannot expose the earlier restricted aggregate history"
    );
    core.shutdown().await;
    drop(core);
    std::fs::remove_dir_all(root).unwrap();
}
