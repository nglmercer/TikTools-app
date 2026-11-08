//! Direct approved points consume the native root and saved action slot in the
//! points transaction, independently of the bounded in-memory LIVE replay cache.
use super::*;
use crate::{
    control::{
        ApprovalActivateParams, ApprovalEnablement, AutomationKind, AutomationMutation,
        CommandCaller,
    },
    events::{DomainEvent, DomainSubscription, DomainTryRecvError},
    services::{effects::EffectExecution, AwardOptions, PointAction},
};
use tiktools_tiktok::events::{CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser};

fn seed(f: &Fixture, id: &str, amount: f64) {
    f.core()
        .points
        .award_points(
            id,
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(amount),
                ..AwardOptions::default()
            },
        )
        .unwrap();
}

fn balance(f: &Fixture, id: &str) -> f64 {
    f.core().points_viewer(id).unwrap()["points"]
        .as_f64()
        .unwrap()
}

fn approved_points_rule(f: &Fixture, type_id: &str, config: serde_json::Value) {
    let service = f.core().collaboration().unwrap();
    let mut grant = f.identity.grant.clone();
    grant.capabilities.insert(Capability::LiveRead);
    service
        .set_grant(GrantSet {
            principal_id: f.identity.principal_id.clone(),
            expected_revision: f.identity.grant_revision.clone(),
            grant,
        })
        .unwrap();
    let instance = service.instance_id().to_owned();
    for (kind, id, record) in [
        (
            AutomationKind::Action,
            "direct-points",
            json!({"id":"direct-points","name":"Direct points","enabled":false,"typeId":type_id,"config":config}),
        ),
        (
            AutomationKind::Event,
            "native-points-rule",
            json!({"id":"native-points-rule","name":"Native points rule","enabled":false,"trigger":"tiktok.chat","cooldownMs":0,"filters":[],"actionIds":["direct-points"]}),
        ),
    ] {
        f.core()
            .automation_command(
                CommandCaller::Remote(f.authorization.clone()),
                CommandRequest {
                    command_id: format!("create-{id}"),
                    host_instance_id: instance.clone(),
                    expected_revision: "0".into(),
                },
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    let review = f
        .core()
        .automation_execution_manifest(AutomationKind::Event, "native-points-rule")
        .unwrap();
    f.core()
        .automation_approval_activate(
            CommandRequest {
                command_id: "approve-native-points-rule".into(),
                host_instance_id: instance,
                expected_revision: review.resource_versions[0].revision.clone(),
            },
            ApprovalActivateParams {
                kind: AutomationKind::Event,
                id: "native-points-rule".into(),
                reviewed_versions: review.resource_versions,
                manifest_digest: review.value["manifestDigest"].as_str().unwrap().into(),
                enable_plan: vec![
                    ApprovalEnablement {
                        kind: AutomationKind::Event,
                        id: "native-points-rule".into(),
                        enabled: true,
                    },
                    ApprovalEnablement {
                        kind: AutomationKind::Action,
                        id: "direct-points".into(),
                        enabled: true,
                    },
                ],
                acknowledge_effects: true,
            },
        )
        .unwrap();
}

fn native_input(msg_id: u64, connection: &str, payload_id: &str, viewer: &str) -> AutomationEvent {
    let native = crate::NativeLiveEvent {
        base: CanonicalLiveEvent::Chat(ChatEvent::default()),
        metadata: EventMetadata {
            method: "WebcastChatMessage".into(),
            msg_id,
            is_history: connection != "initial-connection",
        },
        gift: None,
    };
    let live = crate::LiveContext {
        unique_id: "native-creator".into(),
        room_id: "native-live-room".into(),
        connection_id: connection.into(),
    };
    AutomationEvent::new(
        json!({"id":payload_id,"type":"tiktok.chat","timestamp":123,"user":{"uniqueId":viewer},"data":{"comment":"native chat","depth":0},"authority":"owner","executionId":payload_id}),
        ExecutionContext::owner().with_execution(EffectExecution::native(&native, &live).unwrap()),
    )
}

async fn run(f: &Fixture, input: AutomationEvent) -> serde_json::Value {
    f.core().automation.clear_runs();
    f.core().run_automation_event(input).await;
    let mut runs = f.core().automation_runs();
    assert_eq!(runs.len(), 1, "one saved action must be admitted: {runs:?}");
    runs.remove(0)
}

fn drain_points(subscription: &mut DomainSubscription) -> Vec<(String, f64, f64)> {
    let mut points = Vec::new();
    loop {
        match subscription.try_recv() {
            Ok(DomainEvent::PointsChanged {
                unique_id,
                delta,
                total_points,
                ..
            }) => points.push((unique_id, delta, total_points)),
            Ok(_) => {}
            Err(DomainTryRecvError::Empty) => return points,
            Err(error) => panic!("unexpected presentation gap: {error:?}"),
        }
    }
}

fn assert_retained(run: &serde_json::Value) {
    assert_eq!(run["status"], "ok", "{run}");
    assert_eq!(
        run["summary"],
        "approved points adjustment was already committed; it was not repeated"
    );
}

async fn prepare_actual_native_chat(
    f: &Fixture,
    msg_id: u64,
    connection: &str,
    is_history: bool,
) -> serde_json::Value {
    let core = f.core();
    *core.connection_context.write().unwrap() = Some(crate::LiveContext {
        unique_id: "native-creator".into(),
        room_id: "native-live-room".into(),
        connection_id: connection.into(),
    });
    core.live_dedup
        .set_scope("native-creator", "native-live-room");
    core.automation.clear_runs();
    let slots = core.automation_state.slots.clone();
    let available = slots.available_permits();
    assert!(available > 0);
    let effect = core.prepare_live_event(crate::NativeLiveEvent {
        base: CanonicalLiveEvent::Chat(ChatEvent {
            user: EventUser {
                id: 1,
                unique_id: "alice".into(),
                nickname: "Alice".into(),
                ..Default::default()
            },
            comment: "real adapter chat".into(),
            ..Default::default()
        }),
        metadata: EventMetadata {
            method: "WebcastChatMessage".into(),
            msg_id,
            is_history,
        },
        gift: None,
    });
    assert!(effect.is_some(), "the native replay cache must admit input");
    // prepare_live_event has already queued the real normalized automation
    // envelope. Do not apply its separate ordinary chat-point persistence
    // effect: this fixture measures only the approved saved action.
    drop(effect);
    let completed = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        slots.acquire_many_owned(u32::try_from(available).unwrap()),
    )
    .await
    .expect("the actual native automation task must release its slot")
    .unwrap();
    let mut runs = core.automation_runs();
    assert_eq!(runs.len(), 1, "{runs:?}");
    drop(completed);
    runs.remove(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_native_adapter_preserves_approved_points_identity_on_historical_restart_replay() {
    let mut f = Fixture::new();
    seed(&f, "alice", 0.0);
    approved_points_rule(
        &f,
        "core.points",
        json!({"uniqueId":"{{event.user.uniqueId}}","delta":2.0}),
    );
    let mut projection = f.core().events.subscribe_projection();
    let first = prepare_actual_native_chat(&f, 42, "initial-connection", false).await;
    assert_eq!(first["status"], "ok", "{first}");
    assert_eq!(balance(&f, "alice"), 2.0);
    assert_eq!(drain_points(&mut projection), [("alice".into(), 2.0, 2.0)]);
    drop(projection);
    let previous = f.core.take().unwrap();
    previous.shutdown().await;
    drop(previous);
    f.core = Some(Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        f.paths.clone(),
    )));
    let mut projection = f.core().events.subscribe_projection();
    let retained = prepare_actual_native_chat(&f, 42, "restarted-connection", true).await;
    assert_retained(&retained);
    assert_eq!(balance(&f, "alice"), 2.0);
    assert!(drain_points(&mut projection).is_empty());
    let fresh = prepare_actual_native_chat(&f, 43, "restarted-connection", false).await;
    assert_eq!(fresh["status"], "ok", "{fresh}");
    assert_eq!(balance(&f, "alice"), 4.0);
    assert_eq!(drain_points(&mut projection), [("alice".into(), 2.0, 4.0)]);
    f.core().shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approved_direct_points_replay_survives_restart_without_trusting_payload_identity() {
    let mut f = Fixture::new();
    seed(&f, "alice", 0.0);
    approved_points_rule(
        &f,
        "core.points",
        json!({"uniqueId":"{{event.user.uniqueId}}","delta":2.0}),
    );
    let mut projection = f.core().events.subscribe_projection();
    let first = run(
        &f,
        native_input(42, "initial-connection", "caller-payload-id", "alice"),
    )
    .await;
    assert_eq!(first["status"], "ok", "{first}");
    assert_eq!(balance(&f, "alice"), 2.0);
    assert_eq!(drain_points(&mut projection), [("alice".into(), 2.0, 2.0)]);
    let retained = run(
        &f,
        native_input(42, "reconnected", "different-payload-id", "alice"),
    )
    .await;
    assert_retained(&retained);
    assert!(drain_points(&mut projection).is_empty());
    drop(projection);

    let original_instance = f.core().collaboration().unwrap().instance_id().to_owned();
    let previous = f.core.take().unwrap();
    previous.shutdown().await;
    drop(previous);
    f.core = Some(Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        f.paths.clone(),
    )));
    assert_ne!(
        original_instance,
        f.core().collaboration().unwrap().instance_id()
    );
    let mut projection = f.core().events.subscribe_projection();
    let replay = run(
        &f,
        native_input(42, "after-restart", "new-forged-identity", "alice"),
    )
    .await;
    assert_retained(&replay);
    assert_eq!(balance(&f, "alice"), 2.0);
    assert!(drain_points(&mut projection).is_empty());
    // Reusing the exact caller JSON identity cannot suppress a different real
    // upstream message; only the host-created native root determines identity.
    let fresh = run(
        &f,
        native_input(43, "after-restart", "new-forged-identity", "alice"),
    )
    .await;
    assert_eq!(fresh["status"], "ok", "{fresh}");
    assert_eq!(balance(&f, "alice"), 4.0);
    assert_eq!(drain_points(&mut projection), [("alice".into(), 2.0, 4.0)]);
    f.core().shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approved_points_same_native_root_cannot_move_an_adjustment_to_a_new_viewer() {
    let f = Fixture::new();
    seed(&f, "alice", 0.0);
    seed(&f, "bob", 0.0);
    approved_points_rule(
        &f,
        "core.points",
        json!({"uniqueId":"{{event.user.uniqueId}}","delta":2.0}),
    );
    assert_eq!(
        run(&f, native_input(42, "initial-connection", "first", "alice")).await["status"],
        "ok"
    );
    let mut projection = f.core().events.subscribe_projection();
    let changed = run(&f, native_input(42, "reconnected", "changed", "bob")).await;
    assert_eq!(changed["status"], "error", "{changed}");
    assert_ne!(changed["error"], "authority_required", "{changed}");
    assert_eq!(balance(&f, "alice"), 2.0);
    assert_eq!(balance(&f, "bob"), 0.0);
    assert!(drain_points(&mut projection).is_empty());
    assert_retained(&run(&f, native_input(42, "reconnected", "again", "alice")).await);
    f.core().shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approved_subtract_consumes_a_clamped_zero_identity_before_later_funds_arrive() {
    let f = Fixture::new();
    seed(&f, "alice", 0.0);
    approved_points_rule(
        &f,
        "core.points.subtract",
        json!({"uniqueId":"alice","amount":5.0}),
    );
    assert_eq!(
        run(&f, native_input(42, "initial-connection", "zero", "alice")).await["status"],
        "ok"
    );
    assert_eq!(balance(&f, "alice"), 0.0);
    seed(&f, "alice", 10.0);
    let mut projection = f.core().events.subscribe_projection();
    assert_retained(&run(&f, native_input(42, "reconnected", "later", "alice")).await);
    assert_eq!(balance(&f, "alice"), 10.0);
    assert!(drain_points(&mut projection).is_empty());
    assert_eq!(
        run(&f, native_input(43, "reconnected", "later", "alice")).await["status"],
        "ok"
    );
    assert_eq!(balance(&f, "alice"), 5.0);
    assert_eq!(drain_points(&mut projection), [("alice".into(), -5.0, 5.0)]);
    f.core().shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approved_points_retained_result_neither_overwrites_reset_cache_nor_bypasses_revocation() {
    let f = Fixture::new();
    seed(&f, "alice", 0.0);
    approved_points_rule(&f, "core.points", json!({"uniqueId":"alice","delta":2.0}));
    assert_eq!(
        run(&f, native_input(42, "initial-connection", "first", "alice")).await["status"],
        "ok"
    );
    f.core().points_reset(Some("alice"));
    seed(&f, "alice", 7.0);
    let mut projection = f.core().events.subscribe_projection();
    assert_retained(&run(&f, native_input(42, "reconnected", "again", "alice")).await);
    assert_eq!(balance(&f, "alice"), 7.0);
    assert_eq!(
        f.core().db.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        7.0
    );
    assert!(drain_points(&mut projection).is_empty());
    f.core()
        .collaboration()
        .unwrap()
        .revoke_session(&f.identity.session_id, "runtime replay test")
        .unwrap();
    f.core().automation.clear_runs();
    f.core()
        .run_automation_event(native_input(42, "after-revoke", "again", "alice"))
        .await;
    assert!(f.core().automation_runs().is_empty());
    assert_eq!(balance(&f, "alice"), 7.0);
    assert!(drain_points(&mut projection).is_empty());
    f.core().shutdown().await;
}

struct HeldCoordination {
    release: Option<std::sync::mpsc::SyncSender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for HeldCoordination {
    fn drop(&mut self) {
        // Release on assertion failures too, so the blocked runtime worker
        // cannot keep the host alive or poison the coordination mutex.
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn hold_shared_coordination(core: Arc<AppCore>) -> HeldCoordination {
    let (held, ready) = tokio::sync::oneshot::channel();
    let (release, resume) = std::sync::mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        let _guard = core.coordination_gate.lock().unwrap();
        let _ = held.send(());
        let _ = resume.recv();
    });
    let hold = HeldCoordination {
        release: Some(release),
        thread: Some(thread),
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), ready)
        .await
        .expect("the public-command coordination guard must be acquired")
        .unwrap();
    hold
}

fn private_points_snapshot(f: &Fixture) -> (f64, i64) {
    let db = rusqlite::Connection::open(f.paths.points_database()).unwrap();
    (
        db.query_row("SELECT points FROM viewers WHERE unique_id='alice'", [], |row| row.get(0))
            .unwrap(),
        db.query_row("SELECT count(*) FROM control_commands WHERE principal_id='__approved-points-runtime-v1'", [], |row| row.get(0))
            .unwrap(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn private_fresh_and_retained_points_share_public_command_admission_before_the_writer_gate() {
    let f = Fixture::new();
    seed(&f, "alice", 0.0);
    approved_points_rule(&f, "core.points", json!({"uniqueId":"alice","delta":2.0}));
    assert_eq!(
        run(
            &f,
            native_input(42, "initial-connection", "baseline", "alice")
        )
        .await["status"],
        "ok"
    );
    // The first actual adjustment drains the ordinary seed and establishes a
    // durable baseline; these assertions do not race optimistic seed writes.
    assert_eq!(private_points_snapshot(&f), (2.0, 1));
    let mut projection = f.core().events.subscribe_projection();

    for (retained, before, after) in [(false, (2.0, 1), (4.0, 2)), (true, (4.0, 2), (4.0, 2))] {
        let hold = hold_shared_coordination(Arc::clone(f.core())).await;
        let (waiting, blocked) = tokio::sync::oneshot::channel();
        assert!(f
            .core()
            .approved_points_coordination_wait
            .lock()
            .unwrap()
            .replace(waiting)
            .is_none());
        f.core().automation.clear_runs();
        let core = Arc::clone(f.core());
        let task = tokio::spawn(async move {
            core.run_automation_event(native_input(43, "reconnected", "next", "alice"))
                .await;
            let mut runs = core.automation_runs();
            assert_eq!(runs.len(), 1, "{runs:?}");
            runs.remove(0)
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), blocked)
            .await
            .expect("approved points must reach the held shared coordination gate")
            .unwrap();
        // This signal comes from the real failed try_lock, not a task-start
        // marker or a timed absence assertion. Shared admission precedes the
        // automation writer and cannot race another business ledger's cap.
        assert!(f.core().automation_mutations.try_lock().is_ok());
        assert_eq!(private_points_snapshot(&f), before);
        assert!(drain_points(&mut projection).is_empty());
        drop(hold);
        let completed = tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .expect("released coordination must let the points action finish")
            .unwrap();
        if retained {
            assert_retained(&completed);
            assert!(drain_points(&mut projection).is_empty());
        } else {
            assert_eq!(completed["status"], "ok", "{completed}");
            assert_eq!(drain_points(&mut projection), [("alice".into(), 2.0, 4.0)]);
        }
        assert_eq!(private_points_snapshot(&f), after);
        assert_eq!(balance(&f, "alice"), after.0);
    }
    f.core().shutdown().await;
}
