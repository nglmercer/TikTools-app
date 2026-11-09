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

#[test]
fn shared_moderator_edit_requires_explicit_transfer_and_source_grants() {
    let f = Fixture::new();
    let author = f.pair(grant());
    f.draft(
        &author,
        AutomationKind::Action,
        log("shared-moderator", "private source"),
    );
    let mut access = grant();
    access.capabilities.insert(Capability::AutomationToggle);
    access.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Log].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let editor = f.pair(access.clone());
    let identity = editor.snapshot().unwrap();
    let before = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "shared-moderator")
        .unwrap();
    let update = || AutomationMutation::Update {
        kind: AutomationKind::Action,
        id: "shared-moderator".into(),
        record: log("shared-moderator", "edited"),
    };
    assert!(f
        .core()
        .automation_shared_source(AutomationKind::Action, "shared-moderator", &editor, 65536)
        .is_err());
    assert!(f
        .core()
        .automation_command(
            CommandCaller::Remote(editor.clone()),
            f.request(&before.resource_versions[0].revision),
            update()
        )
        .is_err());
    let policy = access.automation_delegation.as_mut().unwrap();
    policy.allow_other_author_edits = true;
    policy.allow_other_author_source = true;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id.clone(),
            expected_revision: identity.grant_revision,
            grant: access.clone(),
        })
        .unwrap();
    assert_eq!(
        f.core()
            .automation_shared_source(AutomationKind::Action, "shared-moderator", &editor, 65536)
            .unwrap()
            .value["record"]["config"]["message"],
        "private source"
    );
    f.core()
        .automation_command(
            CommandCaller::Remote(editor.clone()),
            f.request(&before.resource_versions[0].revision),
            update(),
        )
        .unwrap();
    let provenance = f
        .core()
        .automation_authority(AutomationKind::Action, "shared-moderator")
        .unwrap();
    assert_eq!(
        provenance.value["provenance"]["authority"]["principalId"],
        identity.principal_id
    );
    access.automation_delegation = None;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: "2".into(),
            grant: access,
        })
        .unwrap();
    assert!(f
        .core()
        .automation_shared_source(AutomationKind::Action, "shared-moderator", &editor, 65536)
        .is_err());
}

#[tokio::test]
async fn shared_action_keeps_both_moderators_authority_for_existing_event() {
    let f = Fixture::new();
    let mut permission = grant();
    permission.capabilities.insert(Capability::AutomationToggle);
    permission.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Log].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let author = f.pair(permission.clone());
    let author_id = author.snapshot().unwrap();
    let mut action = log("team-action", "original");
    action["enabled"] = true.into();
    let mut rule = event("team-event", &["team-action"]);
    rule["enabled"] = true.into();
    for (kind, record) in [
        (AutomationKind::Action, action),
        (AutomationKind::Event, rule),
    ] {
        f.core()
            .automation_command(
                CommandCaller::Remote(author.clone()),
                f.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    permission
        .automation_delegation
        .as_mut()
        .unwrap()
        .allow_other_author_edits = true;
    let editor = f.pair(permission.clone());
    let version = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "team-action")
        .unwrap()
        .resource_versions[0]
        .revision
        .clone();
    let mut changed = log("team-action", "replacement");
    changed["enabled"] = true.into();
    f.core()
        .automation_command(
            CommandCaller::Remote(editor),
            f.request(&version),
            AutomationMutation::Update {
                kind: AutomationKind::Action,
                id: "team-action".into(),
                record: changed,
            },
        )
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    let count = f
        .core()
        .automation_runs()
        .iter()
        .filter(|run| run["actionId"] == "team-action" && run["status"] == "ok")
        .count();
    assert_eq!(count, 1);
    permission.automation_delegation = None;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: author_id.principal_id,
            expected_revision: author_id.grant_revision,
            grant: permission,
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.core()
            .automation_runs()
            .iter()
            .filter(|run| run["actionId"] == "team-action" && run["status"] == "ok")
            .count(),
        count
    );
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reusable_multi_author_event_chain_requires_both_current_topic_grants() {
    let f = Fixture::new();
    let mut permitted = grant();
    permitted
        .capabilities
        .extend([Capability::AutomationToggle, Capability::EventsEmit]);
    permitted.max_points_delta = Some(5.0);
    permitted.effects.emit_topics.insert("overlay.alert".into());
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [
            DelegatedAutomationEffect::Emit,
            DelegatedAutomationEffect::Points,
        ]
        .into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(permitted.clone());
    let identity = auth.snapshot().unwrap();
    let other = f.pair(permitted.clone());
    let other_identity = other.snapshot().unwrap();
    let definitions = [
        (
            AutomationKind::Action,
            json!({"id":"send","name":"Send","enabled":true,"typeId":"core.emit","config":{"type":"overlay.alert","data":{"comment":"{{event.data.comment}}"}}}),
        ),
        (
            AutomationKind::Action,
            json!({"id":"reward","name":"Reward","enabled":true,"typeId":"core.points","config":{"uniqueId":"alice","delta":2}}),
        ),
        (
            AutomationKind::Event,
            json!({"id":"root","name":"Root","enabled":true,"trigger":"tiktok.chat","actionIds":["send"]}),
        ),
        (
            AutomationKind::Event,
            json!({"id":"child","name":"Child","enabled":true,"trigger":"overlay.alert","actionIds":["reward"]}),
        ),
    ];
    for (index, (kind, record)) in definitions.into_iter().enumerate() {
        let author = if index == 1 || index == 3 {
            &other
        } else {
            &auth
        };
        f.core()
            .automation_command(
                CommandCaller::Remote(author.clone()),
                f.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    f.core()
        .run_automation_event(AutomationEvent::owner_without_delegation(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        0.0,
        "unverified input cannot start the chain"
    );
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        2.0,
        "runs: {:?}",
        f.core().automation_runs()
    );
    let mut direct = input("alice");
    direct["type"] = "overlay.alert".into();
    f.core()
        .run_automation_event(AutomationEvent::owner(direct))
        .await;
    assert_eq!(
        f.points("alice"),
        2.0,
        "owner input cannot impersonate scoped emission"
    );
    let mut narrowed = permitted.clone();
    narrowed.effects.emit_topics.clear();
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: other_identity.principal_id.clone(),
            expected_revision: other_identity.grant_revision,
            grant: narrowed,
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        2.0,
        "child author must retain the topic grant"
    );
    let revision = f
        .core()
        .collaboration()
        .unwrap()
        .principal(&other_identity.principal_id)
        .unwrap()
        .grant_revision;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: other_identity.principal_id,
            expected_revision: revision,
            grant: permitted.clone(),
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        4.0,
        "restoring the child's grant permits the chain again"
    );
    permitted.effects.emit_topics.clear();
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: permitted,
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        4.0,
        "parent author topic revocation must stop later events"
    );
    f.core().shutdown().await;
}

#[cfg(feature = "native-tiktok")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_ingress_drives_reusable_internal_event_chain_until_topic_revoked() {
    use std::sync::atomic::Ordering;
    use tiktools_tiktok::events::{CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser};
    let f = Fixture::new();
    let mut permitted = grant();
    permitted
        .capabilities
        .extend([Capability::AutomationToggle, Capability::EventsEmit]);
    permitted.max_points_delta = Some(5.0);
    permitted.effects.emit_topics.insert("overlay.alert".into());
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [
            DelegatedAutomationEffect::Emit,
            DelegatedAutomationEffect::Points,
        ]
        .into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(permitted.clone());
    let identity = auth.snapshot().unwrap();
    for (kind, record) in [
        (
            AutomationKind::Action,
            json!({"id":"send","name":"Send","enabled":true,"typeId":"core.emit","config":{"type":"overlay.alert","data":{"comment":"{{event.data.comment}}"}}}),
        ),
        (
            AutomationKind::Action,
            json!({"id":"reward","name":"Reward","enabled":true,"typeId":"core.points","config":{"uniqueId":"alice","delta":2}}),
        ),
        (
            AutomationKind::Event,
            json!({"id":"root","name":"Root","enabled":true,"trigger":"tiktok.chat","actionIds":["send"]}),
        ),
        (
            AutomationKind::Event,
            json!({"id":"child","name":"Child","enabled":true,"trigger":"overlay.alert","actionIds":["reward"]}),
        ),
    ] {
        f.core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                f.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    let core = f.core().clone();
    *crate::recover_rwlock_write(&core.connection_context, "test connection") =
        Some(crate::LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "connection-17".into(),
            creator_verified: true,
        });
    let dispatch = |id| {
        let before = core
            .automation_state
            .events_completed
            .load(Ordering::Relaxed);
        let native = crate::NativeLiveEvent {
            base: CanonicalLiveEvent::Chat(ChatEvent {
                user: EventUser {
                    unique_id: "alice".into(),
                    nickname: "Alice".into(),
                    ..Default::default()
                },
                comment: "hello".into(),
                ..Default::default()
            }),
            metadata: EventMetadata {
                method: "WebcastChatMessage".into(),
                msg_id: id,
                is_history: false,
            },
            gift: None,
        };
        assert!(core
            .prepare_live_event_with_generation(native, Some(17))
            .is_some());
        before
    };
    async fn wait_done(core: &Arc<AppCore>, before: u64) {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while core
                .automation_state
                .events_completed
                .load(Ordering::Relaxed)
                <= before
            {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("native event dispatch did not finish");
    }
    wait_done(&core, dispatch(1)).await;
    assert_eq!(
        f.points("alice"),
        0.0,
        "an unverified generation cannot start the chain"
    );
    core.events.bind_live_generation(17, "creator");
    // The emitted child and its root each complete once; wait for both before
    // measuring the next native event so an earlier completion cannot satisfy it.
    wait_done(&core, dispatch(2) + 1).await;
    assert_eq!(f.points("alice"), 2.0, "runs: {:?}", core.automation_runs());
    permitted.effects.emit_topics.clear();
    core.collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: permitted,
        })
        .unwrap();
    wait_done(&core, dispatch(3)).await;
    assert_eq!(
        f.points("alice"),
        2.0,
        "topic revocation must stop later native events"
    );
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reusable_grant_runs_saved_live_event_and_revocation_stops_it() {
    let f = Fixture::new();
    let mut permitted = grant();
    permitted.capabilities.insert(Capability::AutomationToggle);
    permitted.max_points_delta = Some(5.0);
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Points].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(permitted.clone());
    let identity = auth.snapshot().unwrap();
    f.draft(
        &auth,
        AutomationKind::Action,
        points("reusable-points", "alice", 2.0),
    );
    f.draft(
        &auth,
        AutomationKind::Event,
        event("reusable-event", &["reusable-points"]),
    );
    for (kind, id) in [
        (AutomationKind::Action, "reusable-points"),
        (AutomationKind::Event, "reusable-event"),
    ] {
        let revision = f
            .core()
            .automation_get_versioned(kind, id)
            .unwrap()
            .resource_versions[0]
            .revision
            .clone();
        let request = f.request(&revision);
        let operation = || AutomationMutation::SetEnabled {
            kind,
            id: id.into(),
            enabled: true,
        };
        let first = f
            .core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                request.clone(),
                operation(),
            )
            .unwrap();
        let replay = f
            .core()
            .automation_command(CommandCaller::Remote(auth.clone()), request, operation())
            .unwrap();
        assert_eq!(first.resource_versions, replay.resource_versions);
        f.assert_delegated(kind, id);
    }
    let preflight = f
        .core()
        .automation_delegation_preflight(AutomationKind::Event, "reusable-event")
        .unwrap();
    assert_eq!(preflight.value["eligible"], true, "{preflight:?}");
    assert_eq!(
        f.core()
            .automation
            .matching_reusable_candidates(&input("alice"), false)
            .len(),
        1
    );
    f.core()
        .run_automation_event(AutomationEvent::owner_without_delegation(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        0.0,
        "unverified ingress cannot borrow delegation"
    );
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        2.0,
        "runs: {:?}",
        f.core().automation_runs()
    );
    permitted.automation_delegation = None;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: permitted,
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 2.0);
}

#[test]
fn remote_activation_requires_reusable_policy_and_author_ownership() {
    let f = Fixture::new();
    let mut no_policy = grant();
    no_policy.capabilities.insert(Capability::AutomationToggle);
    let auth = f.pair(no_policy);
    let mut active = log("no-policy-create", "hello");
    active["enabled"] = true.into();
    assert!(f
        .core()
        .automation_command(
            CommandCaller::Remote(auth.clone()),
            f.request("0"),
            AutomationMutation::Create {
                kind: AutomationKind::Action,
                record: active
            }
        )
        .is_err());
    assert!(f
        .core()
        .automation_get(AutomationKind::Action, "no-policy-create")
        .is_err());
    f.draft(&auth, AutomationKind::Action, log("unpermitted", "hello"));
    let revision = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "unpermitted")
        .unwrap()
        .resource_versions[0]
        .revision
        .clone();
    let result = f.core().automation_command(
        CommandCaller::Remote(auth),
        f.request(&revision),
        AutomationMutation::SetEnabled {
            kind: AutomationKind::Action,
            id: "unpermitted".into(),
            enabled: true,
        },
    );
    assert!(result.is_err());
    assert_eq!(
        f.body(AutomationKind::Action, "unpermitted")["enabled"],
        false
    );
    assert_eq!(
        f.core()
            .automation_get_versioned(AutomationKind::Action, "unpermitted")
            .unwrap()
            .resource_versions[0]
            .revision,
        revision
    );
    let mut permitted = grant();
    permitted.capabilities.insert(Capability::AutomationToggle);
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Log].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let other = f.pair(permitted);
    assert!(f
        .core()
        .automation_command(
            CommandCaller::Remote(other),
            f.request(&revision),
            AutomationMutation::SetEnabled {
                kind: AutomationKind::Action,
                id: "unpermitted".into(),
                enabled: true
            }
        )
        .is_err());
    assert_eq!(
        f.body(AutomationKind::Action, "unpermitted")["enabled"],
        false
    );
}

#[test]
fn remote_activation_rejects_effects_outside_reusable_grant() {
    let f = Fixture::new();
    let mut limited = grant();
    limited.capabilities.insert(Capability::AutomationToggle);
    limited.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Log].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(limited);
    f.draft(
        &auth,
        AutomationKind::Action,
        points("outside-policy", "alice", 1.0),
    );
    let before = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "outside-policy")
        .unwrap();
    let revision = &before.resource_versions[0].revision;
    assert!(f
        .core()
        .automation_command(
            CommandCaller::Remote(auth),
            f.request(revision),
            AutomationMutation::SetEnabled {
                kind: AutomationKind::Action,
                id: "outside-policy".into(),
                enabled: true
            },
        )
        .is_err());
    let after = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "outside-policy")
        .unwrap();
    assert_eq!(before.value, after.value);
    assert_eq!(before.resource_versions, after.resource_versions);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reusable_grant_creates_and_edits_active_automation_atomically() {
    let f = Fixture::new();
    let mut permitted = grant();
    permitted.capabilities.insert(Capability::AutomationToggle);
    permitted.max_points_delta = Some(5.0);
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Points].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(permitted);
    let mut action = points("active-points", "alice", 1.0);
    action["enabled"] = true.into();
    let mut rule = event("active-event", &["active-points"]);
    rule["enabled"] = true.into();
    for (kind, record) in [
        (AutomationKind::Action, action),
        (AutomationKind::Event, rule),
    ] {
        let id = record["id"].as_str().unwrap().to_owned();
        let result = f
            .core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                f.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
        assert_eq!(
            result.value,
            json!({"kind":kind,"id":id,"enabled":true,"pendingReview":false})
        );
        f.assert_delegated(kind, &id);
    }
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 1.0);
    let before = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "active-points")
        .unwrap();
    let revision = before.resource_versions[0].revision.clone();
    let mut changed = points("active-points", "alice", 2.0);
    changed["enabled"] = true.into();
    let update = || AutomationMutation::Update {
        kind: AutomationKind::Action,
        id: "active-points".into(),
        record: changed.clone(),
    };
    let request = f.request(&revision);
    let result = f
        .core()
        .automation_command(
            CommandCaller::Remote(auth.clone()),
            request.clone(),
            update(),
        )
        .unwrap();
    assert_eq!(result.value["enabled"], true);
    assert_eq!(
        f.core()
            .automation_command(CommandCaller::Remote(auth.clone()), request, update())
            .unwrap()
            .resource_versions,
        result.resource_versions
    );
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 3.0);
    let before_denied = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "active-points")
        .unwrap();
    let mut forbidden = log("active-points", "outside allowed effects");
    forbidden["enabled"] = true.into();
    assert!(f
        .core()
        .automation_command(
            CommandCaller::Remote(auth),
            f.request(&before_denied.resource_versions[0].revision),
            AutomationMutation::Update {
                kind: AutomationKind::Action,
                id: "active-points".into(),
                record: forbidden
            }
        )
        .is_err());
    let after_denied = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "active-points")
        .unwrap();
    assert_eq!(before_denied.value, after_denied.value);
    assert_eq!(
        before_denied.resource_versions,
        after_denied.resource_versions
    );
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 5.0);
}

#[cfg(feature = "native-tiktok")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_live_ingress_applies_reusable_grants_only_for_verified_generation() {
    use std::sync::atomic::Ordering;
    use tiktools_tiktok::events::{CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser};
    let f = Fixture::new();
    let mut permitted = grant();
    permitted.capabilities.insert(Capability::AutomationToggle);
    permitted.max_points_delta = Some(5.0);
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Points].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(permitted.clone());
    let identity = auth.snapshot().unwrap();
    let mut action = points("native-points", "alice", 2.0);
    action["enabled"] = true.into();
    let mut rule = event("native-event", &["native-points"]);
    rule["enabled"] = true.into();
    for (kind, record) in [
        (AutomationKind::Action, action),
        (AutomationKind::Event, rule),
    ] {
        f.core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                f.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    let native = |id| crate::NativeLiveEvent {
        base: CanonicalLiveEvent::Chat(ChatEvent {
            user: EventUser {
                unique_id: "alice".into(),
                nickname: "Alice".into(),
                ..Default::default()
            },
            comment: "hello".into(),
            ..Default::default()
        }),
        metadata: EventMetadata {
            method: "WebcastChatMessage".into(),
            msg_id: id,
            is_history: false,
        },
        gift: None,
    };
    let core = f.core().clone();
    let dispatch = |id, generation| {
        let before = core
            .automation_state
            .events_completed
            .load(Ordering::Relaxed);
        assert!(core
            .prepare_live_event_with_generation(native(id), Some(generation))
            .is_some());
        before
    };
    async fn wait_done(core: &Arc<AppCore>, before: u64) {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while core
                .automation_state
                .events_completed
                .load(Ordering::Relaxed)
                <= before
            {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("native event dispatch did not finish");
    }
    *crate::recover_rwlock_write(&core.connection_context, "test connection") =
        Some(crate::LiveContext {
            unique_id: "creator".into(),
            room_id: "room".into(),
            connection_id: "connection-17".into(),
            creator_verified: true,
        });
    let before = dispatch(1, 17);
    wait_done(&core, before).await;
    assert_eq!(
        f.points("alice"),
        0.0,
        "an unverified generation must not transfer delegation"
    );
    core.events.bind_live_generation(17, "creator");
    let before = dispatch(2, 17);
    wait_done(&core, before).await;
    assert_eq!(
        f.points("alice"),
        2.0,
        "verified native ingress should run the delegated event"
    );
    permitted.automation_delegation = None;
    core.collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: permitted,
        })
        .unwrap();
    let before = dispatch(3, 17);
    wait_done(&core, before).await;
    assert_eq!(
        f.points("alice"),
        2.0,
        "grant removal must stop subsequent native events"
    );
    core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reusable_grant_revocation_during_delay_stops_later_effects() {
    let f = Fixture::new();
    let mut permitted = grant();
    permitted.capabilities.insert(Capability::AutomationToggle);
    permitted.max_points_delta = Some(5.0);
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [
            DelegatedAutomationEffect::Log,
            DelegatedAutomationEffect::Delay,
            DelegatedAutomationEffect::Points,
        ]
        .into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(permitted);
    let identity = auth.snapshot().unwrap();
    let mut begin = log("reusable-begin", "started");
    begin["enabled"] = true.into();
    let delay = json!({"id":"reusable-delay","name":"Delay","enabled":true,"typeId":"core.delay","config":{"ms":500}});
    let mut award = points("reusable-award", "alice", 2.0);
    award["enabled"] = true.into();
    let mut rule = event(
        "reusable-delayed-event",
        &["reusable-begin", "reusable-delay", "reusable-award"],
    );
    rule["enabled"] = true.into();
    for (kind, record) in [
        (AutomationKind::Action, begin),
        (AutomationKind::Action, delay),
        (AutomationKind::Action, award),
        (AutomationKind::Event, rule),
    ] {
        f.core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                f.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    let core = f.core().clone();
    let worker = tokio::spawn(async move {
        core.run_automation_event(AutomationEvent::owner(input("alice")))
            .await;
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !f
            .core()
            .automation_runs()
            .iter()
            .any(|run| run["actionId"] == "reusable-begin" && run["status"] == "ok")
        {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("reusable event did not start");
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
        .any(|run| run["actionId"] == "reusable-award" && run["status"] == "ok"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reusable_event_intersects_grants_of_multiple_authors() {
    let f = Fixture::new();
    let mut permission = grant();
    permission.capabilities.insert(Capability::AutomationToggle);
    permission.max_points_delta = Some(5.0);
    permission.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Points].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let action_author = f.pair(permission.clone());
    let event_author = f.pair(permission.clone());
    let action_identity = action_author.snapshot().unwrap();
    let mut action = points("multi-author-points", "alice", 2.0);
    action["enabled"] = true.into();
    f.core()
        .automation_command(
            CommandCaller::Remote(action_author),
            f.request("0"),
            AutomationMutation::Create {
                kind: AutomationKind::Action,
                record: action,
            },
        )
        .unwrap();
    let mut rule = event("multi-author-event", &["multi-author-points"]);
    rule["enabled"] = true.into();
    f.core()
        .automation_command(
            CommandCaller::Remote(event_author),
            f.request("0"),
            AutomationMutation::Create {
                kind: AutomationKind::Event,
                record: rule,
            },
        )
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 2.0);
    permission.automation_delegation = None;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: action_identity.principal_id,
            expected_revision: action_identity.grant_revision,
            grant: permission,
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.points("alice"),
        2.0,
        "either author's grant removal must stop the combined rule"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_reusable_grant_can_edit_owner_action_and_revalidates_dependent_event() {
    let f = Fixture::new();
    let mut original = points("owner-shared", "alice", 1.0);
    original["enabled"] = true.into();
    let mut rule = event("owner-shared-event", &["owner-shared"]);
    rule["enabled"] = true.into();
    f.core()
        .automation_create(AutomationKind::Action, original)
        .unwrap();
    f.core()
        .automation_create(AutomationKind::Event, rule)
        .unwrap();
    let mut permitted = grant();
    permitted.capabilities.insert(Capability::AutomationToggle);
    permitted.max_points_delta = Some(5.0);
    permitted.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Points].into(),
        allow_dynamic_targets: false,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(permitted.clone());
    let identity = auth.snapshot().unwrap();
    let mut edited = points("owner-shared", "alice", 2.0);
    edited["enabled"] = true.into();
    let operation = || AutomationMutation::Update {
        kind: AutomationKind::Action,
        id: "owner-shared".into(),
        record: edited.clone(),
    };
    let initial = f
        .core()
        .automation_get_versioned(AutomationKind::Action, "owner-shared")
        .unwrap();
    assert!(f
        .core()
        .automation_command(
            CommandCaller::Remote(auth.clone()),
            f.request(&initial.resource_versions[0].revision),
            operation()
        )
        .is_err());
    assert_eq!(
        f.core()
            .automation_get_versioned(AutomationKind::Action, "owner-shared")
            .unwrap()
            .value,
        initial.value
    );
    permitted
        .automation_delegation
        .as_mut()
        .unwrap()
        .allow_owner_edits = true;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id.clone(),
            expected_revision: identity.grant_revision,
            grant: permitted.clone(),
        })
        .unwrap();
    f.core()
        .automation_command(
            CommandCaller::Remote(auth),
            f.request(&initial.resource_versions[0].revision),
            operation(),
        )
        .unwrap();
    f.assert_delegated(AutomationKind::Action, "owner-shared");
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 2.0);
    permitted.automation_delegation = None;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: "2".into(),
            grant: permitted,
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 2.0);
}

#[test]
fn owner_action_edit_rejects_unscoped_or_unsupported_dependent_events() {
    for case in 0..3 {
        let unsupported = case == 1;
        let f = Fixture::new();
        let mut action = log("shared-log", "before");
        action["enabled"] = true.into();
        f.core()
            .automation_create(AutomationKind::Action, action)
            .unwrap();
        if unsupported {
            f.core().automation_create(AutomationKind::Action, json!({"id":"unsupported","name":"Unknown","enabled":true,"typeId":"unknown.extension","config":{}})).unwrap();
        }
        let ids = if unsupported {
            vec!["shared-log", "unsupported"]
        } else {
            vec!["shared-log"]
        };
        let mut rule = event("owner-dependent", &ids);
        rule["enabled"] = true.into();
        if case == 2 {
            rule["filters"] = json!([{"path":"event.data.private","operator":"exists"}]);
        }
        f.core()
            .automation_create(AutomationKind::Event, rule)
            .unwrap();
        let mut permission = grant();
        permission.capabilities.insert(Capability::AutomationToggle);
        permission.automation_delegation = Some(AutomationDelegation {
            effects: [DelegatedAutomationEffect::Log].into(),
            allow_dynamic_targets: false,
            allow_owner_edits: true,
            allow_owner_source: false,
            allow_other_author_edits: false,
            allow_other_author_source: false,
        });
        if case == 0 {
            permission.resources.insert(
                ResourceKind::Automation,
                ["action:shared-log".into()].into(),
            );
        }
        let auth = f.pair(permission);
        let before = f
            .core()
            .automation_get_versioned(AutomationKind::Action, "shared-log")
            .unwrap();
        let mut changed = log("shared-log", "after");
        changed["enabled"] = true.into();
        assert!(f
            .core()
            .automation_command(
                CommandCaller::Remote(auth),
                f.request(&before.resource_versions[0].revision),
                AutomationMutation::Update {
                    kind: AutomationKind::Action,
                    id: "shared-log".into(),
                    record: changed
                }
            )
            .is_err());
        let after = f
            .core()
            .automation_get_versioned(AutomationKind::Action, "shared-log")
            .unwrap();
        assert_eq!(before.value, after.value);
        assert_eq!(before.resource_versions, after.resource_versions);
        assert_eq!(
            f.core()
                .automation_authority(AutomationKind::Action, "shared-log")
                .unwrap()
                .value["provenance"]["authority"]["kind"],
            "owner"
        );
    }
}
