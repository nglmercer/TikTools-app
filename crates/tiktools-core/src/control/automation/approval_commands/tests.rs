//! Real Owner approval commands and runtime sinks over isolated business and
//! collaboration databases. A preview or editable body never grants execution.
use super::*;
use crate::{
    collaboration::*,
    control::{ApprovalEnablement, CommandCaller},
    live_events::AutomationEvent,
    paths::AppPaths,
    services::{AwardOptions, PointAction},
};
use std::sync::Arc;

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    paths: AppPaths,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-approved-core-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root,
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        let opened = Self::open(&paths);
        // Approved adjustments retain the ordinary adjustment contract:
        // viewers already exist. Zero-point awards seed private real rows.
        for id in ["alice", "bob"] {
            opened
                .points
                .award_points(
                    id,
                    PointAction::Manual,
                    AwardOptions {
                        custom_amount: Some(0.0),
                        ..AwardOptions::default()
                    },
                )
                .unwrap();
        }
        let core = Some(opened);
        Self { core, paths }
    }
    fn open(paths: &AppPaths) -> Arc<AppCore> {
        Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        ))
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    fn reopen(&mut self) {
        drop(self.core.take());
        self.core = Some(Self::open(&self.paths));
    }
    fn request(&self, expected: &str) -> CommandRequest {
        CommandRequest {
            command_id: format!("owner-approval-{}", fastrand::u64(..)),
            host_instance_id: self.core().collaboration().unwrap().instance_id().into(),
            expected_revision: expected.into(),
        }
    }
    fn pair(&self, grant: Grant) -> Authorization {
        let service = self.core().collaboration().unwrap();
        let mut config = service.configuration().unwrap();
        if !config.enabled {
            config.enabled = true;
            config.advertised_origin = Some("http://127.0.0.1:8788".into());
            service.configure(config).unwrap();
        }
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "e".repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Approval runtime author".into(),
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
        service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap()
    }
    fn draft(&self, auth: &Authorization, kind: AutomationKind, mut record: Value) {
        record["enabled"] = false.into();
        self.core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                self.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    fn approval(
        &self,
        kind: AutomationKind,
        id: &str,
        enable_plan: Vec<ApprovalEnablement>,
    ) -> (CommandRequest, ApprovalActivateParams) {
        let preview = self.core().automation_execution_manifest(kind, id).unwrap();
        (
            self.request(&preview.resource_versions[0].revision),
            ApprovalActivateParams {
                kind,
                id: id.into(),
                reviewed_versions: preview.resource_versions,
                manifest_digest: preview.value["manifestDigest"].as_str().unwrap().into(),
                enable_plan,
                acknowledge_effects: true,
            },
        )
    }
    fn approve(
        &self,
        kind: AutomationKind,
        id: &str,
        enable_plan: Vec<ApprovalEnablement>,
    ) -> Versioned<Value> {
        let (request, params) = self.approval(kind, id, enable_plan);
        self.core()
            .automation_approval_activate(request, params)
            .unwrap()
    }
    fn body(&self, kind: AutomationKind, id: &str) -> Value {
        self.core().automation_get(kind, id).unwrap()
    }
    fn points(&self, id: &str) -> f64 {
        self.core()
            .points_viewer(id)
            .map(|v| v["points"].as_f64().unwrap())
            .unwrap_or(0.0)
    }
    fn assert_delegated(&self, kind: AutomationKind, id: &str) {
        assert_eq!(
            self.core().automation_authority(kind, id).unwrap().value["provenance"]["authority"]
                ["kind"],
            "delegated"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.paths.root);
    }
}
fn grant() -> Grant {
    Grant {
        capabilities: [
            Capability::AutomationRead,
            Capability::AutomationEdit,
            Capability::LiveRead,
            Capability::PointsRead,
            Capability::PointsAdjust,
        ]
        .into(),
        ..Grant::default()
    }
}
fn enable(kind: AutomationKind, id: &str) -> ApprovalEnablement {
    ApprovalEnablement {
        kind,
        id: id.into(),
        enabled: true,
    }
}
fn log(id: &str, message: &str) -> Value {
    json!({"id":id,"name":"Log","enabled":false,"typeId":"core.log","config":{"message":message}})
}
fn points(id: &str, viewer: &str, delta: f64) -> Value {
    json!({"id":id,"name":"Points","enabled":false,"typeId":"core.points","config":{"uniqueId":viewer,"delta":delta}})
}
fn event(id: &str, actions: &[&str]) -> Value {
    json!({"id":id,"name":"Live chat","enabled":false,"trigger":"tiktok.chat","actionIds":actions})
}
fn input(viewer: &str) -> Value {
    json!({"id":format!("input-{}",fastrand::u64(..)),"type":"tiktok.chat","timestamp":123,"user":{"uniqueId":viewer,"nickname":"Public viewer","token":"private-user-token"},"data":{"comment":"hello","secret":"private-data-token"},"authority":{"kind":"owner"},"globals":{"secret":"private-global-token"}})
}
async fn execute(f: &Fixture, id: &str, event: &Value) -> Value {
    f.core()
        .execute_action(&f.body(AutomationKind::Action, id), event, None, false)
        .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activated_saved_log_and_points_execute_directly_and_preserve_delegated_provenance() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    f.draft(
        &auth,
        AutomationKind::Action,
        log("log", "{{event.user.uniqueId}}: {{event.data.comment}}"),
    );
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "alice", 2.0),
    );
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required"
    );
    for id in ["log", "points"] {
        f.approve(
            AutomationKind::Action,
            id,
            vec![enable(AutomationKind::Action, id)],
        );
        f.assert_delegated(AutomationKind::Action, id);
    }
    let logged = execute(&f, "log", &input("alice")).await;
    assert_eq!(logged["status"], "ok");
    assert_eq!(logged["summary"], "alice: hello");
    assert!(!logged.to_string().contains("private-"));
    assert_eq!(execute(&f, "points", &input("alice")).await["status"], "ok");
    assert_eq!(f.points("alice"), 2.0);
    f.assert_delegated(AutomationKind::Action, "points");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activated_event_runs_real_live_actions_without_independent_action_approval() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    f.draft(
        &auth,
        AutomationKind::Action,
        log("log", "live {{event.user.uniqueId}}"),
    );
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "{{event.user.uniqueId}}", 3.0),
    );
    f.draft(
        &auth,
        AutomationKind::Event,
        event("event", &["log", "points"]),
    );
    let receipt = f.approve(
        AutomationKind::Event,
        "event",
        vec![
            enable(AutomationKind::Event, "event"),
            enable(AutomationKind::Action, "log"),
            enable(AutomationKind::Action, "points"),
        ],
    );
    assert_eq!(receipt.resource_versions.len(), 3);
    for (kind, id) in [
        (AutomationKind::Event, "event"),
        (AutomationKind::Action, "log"),
        (AutomationKind::Action, "points"),
    ] {
        f.assert_delegated(kind, id);
    }
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required",
        "event approval does not authorize standalone dependency"
    );
    assert!(f
        .core()
        .automation
        .manual_record(&f.body(AutomationKind::Event, "event"))
        .is_some());
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        3.0,
        "approved event runs: {:?}",
        f.core().automation_runs()
    );
    let runs = f.core().automation_runs();
    assert!(runs
        .iter()
        .any(|r| r["actionId"] == "log" && r["status"] == "ok"));
    assert!(runs
        .iter()
        .any(|r| r["actionId"] == "points" && r["status"] == "ok"));
}

#[test]
fn stale_manifest_and_unsupported_global_object_or_private_fields_cannot_be_approved() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    f.draft(&auth, AutomationKind::Action, log("stale", "one"));
    let (request, params) = f.approval(
        AutomationKind::Action,
        "stale",
        vec![enable(AutomationKind::Action, "stale")],
    );
    f.core()
        .automation_update(AutomationKind::Action, "stale", log("stale", "two"))
        .unwrap();
    assert!(f
        .core()
        .automation_approval_activate(request, params)
        .is_err());
    let cases = [
        log("global", "{{globals.secret}}"),
        log("object", "{{event.user}}"),
        log("private", "{{event.data.secret}}"),
        json!({"id":"http","name":"HTTP","enabled":false,"typeId":"core.fetch","config":{"url":"https://example.test/"}}),
        json!({"id":"script","name":"Script","enabled":false,"typeId":"core.code","config":{"source":"({log:['no']})"}}),
    ];
    for record in cases {
        let id = record["id"].as_str().unwrap().to_owned();
        f.draft(&auth, AutomationKind::Action, record);
        let (request, params) = f.approval(
            AutomationKind::Action,
            &id,
            vec![enable(AutomationKind::Action, &id)],
        );
        assert!(
            f.core()
                .automation_approval_activate(request, params)
                .is_err(),
            "unsupported {id} was approved"
        );
        assert_eq!(f.body(AutomationKind::Action, &id)["enabled"], false);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exact_saved_body_is_required_and_public_named_objects_are_not_rendered() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    f.draft(
        &auth,
        AutomationKind::Action,
        log("log", "{{event.data.comment}}"),
    );
    f.approve(
        AutomationKind::Action,
        "log",
        vec![enable(AutomationKind::Action, "log")],
    );
    let body = f.body(AutomationKind::Action, "log");
    let mut changed = body.clone();
    changed["name"] = "different reviewed body".into();
    assert_eq!(
        f.core()
            .execute_action(&changed, &input("alice"), None, false)
            .await["error"],
        "authority_required"
    );
    changed = body.clone();
    changed["config"]["message"] = "{{globals.secret}}".into();
    assert_eq!(
        f.core()
            .execute_action(&changed, &input("alice"), None, true)
            .await["error"],
        "authority_required"
    );
    let mut object = input("alice");
    object["data"]["comment"] = json!({"secret":"MUST-NOT-RENDER"});
    let run = f.core().execute_action(&body, &object, None, false).await;
    assert_eq!(run["status"], "ok");
    assert!(!run.to_string().contains("MUST-NOT-RENDER"));
    assert!(!run.to_string().contains("private-"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_dependency_is_never_implicitly_enabled_by_event_approval() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    f.core()
        .automation_create(AutomationKind::Action, points("owner-points", "alice", 4.0))
        .unwrap();
    f.draft(
        &auth,
        AutomationKind::Event,
        event("event", &["owner-points"]),
    );
    let (request, params) = f.approval(
        AutomationKind::Event,
        "event",
        vec![
            enable(AutomationKind::Event, "event"),
            enable(AutomationKind::Action, "owner-points"),
        ],
    );
    assert!(f
        .core()
        .automation_approval_activate(request, params)
        .is_err());
    f.approve(
        AutomationKind::Event,
        "event",
        vec![enable(AutomationKind::Event, "event")],
    );
    assert_eq!(
        f.body(AutomationKind::Action, "owner-points")["enabled"],
        false
    );
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persisted_approval_survives_restart_but_replay_after_edit_delete_or_revoke_never_reinstalls(
) {
    let mut f = Fixture::new();
    let auth = f.pair(grant());
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "alice", 2.0),
    );
    let (request, params) = f.approval(
        AutomationKind::Action,
        "points",
        vec![enable(AutomationKind::Action, "points")],
    );
    let approved = f
        .core()
        .automation_approval_activate(request.clone(), params.clone())
        .unwrap();
    drop(auth);
    f.reopen();
    assert_eq!(execute(&f, "points", &input("alice")).await["status"], "ok");
    assert_eq!(f.points("alice"), 2.0);
    let mut edited = f.body(AutomationKind::Action, "points");
    edited["name"] = "edited".into();
    f.core()
        .automation_update(AutomationKind::Action, "points", edited)
        .unwrap();
    assert_eq!(
        f.core()
            .automation_approval_activate(request.clone(), params.clone())
            .unwrap()
            .value,
        approved.value
    );
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required"
    );
    f.core()
        .automation_delete(AutomationKind::Action, "points")
        .unwrap();
    assert_eq!(
        f.core()
            .automation_approval_activate(request.clone(), params.clone())
            .unwrap()
            .value,
        approved.value
    );
    f.core()
        .automation_create(AutomationKind::Action, points("points", "alice", 2.0))
        .unwrap();
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required"
    );
    let (second_request, second_params) = f.approval(
        AutomationKind::Action,
        "points",
        vec![enable(AutomationKind::Action, "points")],
    );
    let second = f
        .core()
        .automation_approval_activate(second_request.clone(), second_params.clone())
        .unwrap();
    let revision = second.resource_versions[0].revision.clone();
    f.core()
        .automation_approval_revoke(
            f.request(&revision),
            ApprovalRevokeParams {
                kind: AutomationKind::Action,
                id: "points".into(),
                acknowledge_revocation: true,
            },
        )
        .unwrap();
    f.core()
        .automation_approval_activate(second_request, second_params)
        .unwrap();
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required"
    );
    assert_eq!(f.points("alice"), 2.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grant_and_configuration_changes_away_and_back_do_not_revive_old_approval() {
    let f = Fixture::new();
    let original = grant();
    let auth = f.pair(original.clone());
    let identity = auth.snapshot().unwrap();
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "alice", 2.0),
    );
    f.approve(
        AutomationKind::Action,
        "points",
        vec![enable(AutomationKind::Action, "points")],
    );
    let service = f.core().collaboration().unwrap();
    let mut reduced = original.clone();
    reduced.capabilities.remove(&Capability::PointsAdjust);
    service
        .set_grant(GrantSet {
            principal_id: identity.principal_id.clone(),
            expected_revision: identity.grant_revision,
            grant: reduced,
        })
        .unwrap();
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required"
    );
    service
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: "2".into(),
            grant: original,
        })
        .unwrap();
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required"
    );
    f.approve(
        AutomationKind::Action,
        "points",
        vec![enable(AutomationKind::Action, "points")],
    );
    assert_eq!(execute(&f, "points", &input("alice")).await["status"], "ok");
    let mut config = service.configuration().unwrap();
    config.enabled = false;
    service.configure(config).unwrap();
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required"
    );
    let mut config = service.configuration().unwrap();
    config.enabled = true;
    service.configure(config).unwrap();
    assert_eq!(
        execute(&f, "points", &input("alice")).await["error"],
        "authority_required"
    );
    assert_eq!(f.points("alice"), 2.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_viewer_and_rounded_delta_are_checked_at_the_real_points_sink() {
    let f = Fixture::new();
    let mut scoped = grant();
    scoped
        .resources
        .insert(ResourceKind::Viewer, ["alice".into()].into());
    scoped.max_points_delta = Some(3.0);
    let auth = f.pair(scoped);
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "{{event.user.uniqueId}}", 2.0),
    );
    f.approve(
        AutomationKind::Action,
        "points",
        vec![enable(AutomationKind::Action, "points")],
    );
    assert_eq!(execute(&f, "points", &input("alice")).await["status"], "ok");
    assert_eq!(f.points("alice"), 2.0);
    assert_eq!(
        execute(&f, "points", &input("bob")).await["status"],
        "error"
    );
    assert_eq!(f.points("bob"), 0.0);
    let mut missing = input("alice");
    missing["user"].as_object_mut().unwrap().remove("uniqueId");
    assert_eq!(
        crate::services::automation_approval_policy::render_template(
            "{{event.user.uniqueId}}",
            &missing,
            256,
        )
        .unwrap(),
        "",
        "missing identity must never become a literal points target"
    );
    assert_eq!(execute(&f, "points", &missing).await["status"], "error");
    assert_eq!(f.points("alice"), 2.0);
    assert!(f.core().points_viewer("{{event.user.uniqueId}}").is_err());
    let mut rounded = grant();
    rounded.max_points_delta = Some(0.006);
    let rounded_author = f.pair(rounded);
    f.draft(
        &rounded_author,
        AutomationKind::Action,
        points("rounded", "alice", 0.006),
    );
    f.approve(
        AutomationKind::Action,
        "rounded",
        vec![enable(AutomationKind::Action, "rounded")],
    );
    let before = f.points("alice");
    let denied = execute(&f, "rounded", &input("alice")).await;
    assert_eq!(denied["status"], "error");
    assert_eq!(
        f.points("alice"),
        before,
        "actual cent-rounded delta exceeds the saved subject's limit"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revocation_during_delay_prevents_later_points_in_the_same_live_event() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    let identity = auth.snapshot().unwrap();
    f.draft(&auth, AutomationKind::Action, log("begin", "started"));
    f.draft(&auth,AutomationKind::Action,json!({"id":"delay","name":"Delay","enabled":false,"typeId":"core.delay","config":{"ms":500}}));
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "alice", 5.0),
    );
    f.draft(
        &auth,
        AutomationKind::Event,
        event("event", &["begin", "delay", "points"]),
    );
    f.approve(
        AutomationKind::Event,
        "event",
        vec![
            enable(AutomationKind::Event, "event"),
            enable(AutomationKind::Action, "begin"),
            enable(AutomationKind::Action, "delay"),
            enable(AutomationKind::Action, "points"),
        ],
    );
    let core = f.core().clone();
    let worker = tokio::spawn(async move {
        core.run_automation_event(AutomationEvent::owner(input("alice")))
            .await;
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if f.core()
                .automation_runs()
                .iter()
                .any(|r| r["actionId"] == "begin" && r["status"] == "ok")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("event never started approved log action");
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    f.core()
        .collaboration()
        .unwrap()
        .revoke_session(&identity.session_id, "local-owner")
        .unwrap();
    worker.await.unwrap();
    assert_eq!(f.points("alice"), 0.0);
    assert!(!f
        .core()
        .automation_runs()
        .iter()
        .any(|r| r["actionId"] == "points" && r["status"] == "ok"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_action_rechecks_revoked_grant_when_automation_admission_opens() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    let identity = auth.snapshot().unwrap();
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "alice", 2.0),
    );
    f.approve(
        AutomationKind::Action,
        "points",
        vec![enable(AutomationKind::Action, "points")],
    );
    let body = f.body(AutomationKind::Action, "points");
    let gate = f.core().automation_gate().unwrap();
    let core = f.core().clone();
    let worker = tokio::spawn(async move {
        core.execute_action(&body, &input("alice"), None, false)
            .await
    });
    tokio::task::yield_now().await;
    let mut revoked = grant();
    revoked.capabilities.remove(&Capability::PointsAdjust);
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: revoked,
        })
        .unwrap();
    drop(gate);
    assert_eq!(worker.await.unwrap()["error"], "authority_required");
    assert_eq!(f.points("alice"), 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_and_reapprove_during_delay_never_revives_the_captured_old_run() {
    let f = Fixture::new();
    let auth = f.pair(grant());
    f.draft(&auth, AutomationKind::Action, log("begin", "started"));
    f.draft(&auth,AutomationKind::Action,json!({"id":"delay","name":"Delay","enabled":false,"typeId":"core.delay","config":{"ms":500}}));
    f.draft(
        &auth,
        AutomationKind::Action,
        points("points", "alice", 5.0),
    );
    f.draft(
        &auth,
        AutomationKind::Event,
        event("event", &["begin", "delay", "points"]),
    );
    let first = f.approve(
        AutomationKind::Event,
        "event",
        vec![
            enable(AutomationKind::Event, "event"),
            enable(AutomationKind::Action, "begin"),
            enable(AutomationKind::Action, "delay"),
            enable(AutomationKind::Action, "points"),
        ],
    );
    let core = f.core().clone();
    let worker = tokio::spawn(async move {
        core.run_automation_event(AutomationEvent::owner(input("alice")))
            .await;
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if f.core()
                .automation_runs()
                .iter()
                .any(|r| r["actionId"] == "begin" && r["status"] == "ok")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("event never entered approved sequence");
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    f.core()
        .automation_approval_revoke(
            f.request(&first.resource_versions[0].revision),
            ApprovalRevokeParams {
                kind: AutomationKind::Event,
                id: "event".into(),
                acknowledge_revocation: true,
            },
        )
        .unwrap();
    let second = f.approve(
        AutomationKind::Event,
        "event",
        vec![enable(AutomationKind::Event, "event")],
    );
    assert_ne!(
        first.value["approvalGeneration"],
        second.value["approvalGeneration"]
    );
    worker.await.unwrap();
    assert_eq!(
        f.points("alice"),
        0.0,
        "old matched record must retain its old approval fence"
    );
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        5.0,
        "a separately captured fresh event uses the new approval"
    );
}

mod external;
mod scoped_flow;
