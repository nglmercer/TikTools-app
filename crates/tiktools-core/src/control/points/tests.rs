use super::*;
use crate::{
    collaboration::*,
    control::{CommandCaller, WorkflowMutation},
    paths::AppPaths,
};
use serde_json::json;

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    paths: AppPaths,
}
impl Fixture {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("tt-point-command-{}-{nonce}", std::process::id()));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent-builtin"),
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
        let core = Some(Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        )));
        Self { core, paths }
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    fn request(&self, id: &str) -> AdditiveCommandRequest {
        AdditiveCommandRequest {
            command_id: id.into(),
            host_instance_id: self.core().collaboration().unwrap().instance_id().into(),
        }
    }
    fn adjust(
        &self,
        id: &str,
        viewer: &str,
        delta: f64,
    ) -> Result<Versioned<Value>, OperationError> {
        self.core().points_command(
            CommandCaller::LocalOwner,
            self.request(id),
            PointsAdjustment {
                expected_currency: None,
                unique_id: viewer.into(),
                delta,
            },
        )
    }
    fn pair(&self, grant: Grant) -> (Authorization, Credentials) {
        let service = self.core().collaboration().unwrap();
        let mut configuration = service.configuration().unwrap();
        configuration.enabled = true;
        configuration.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(configuration).unwrap();
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let secret = "a".repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Points moderator".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&secret),
                },
                "fixture",
            )
            .unwrap();
        service.decide_request(&pending.request_id, true).unwrap();
        let credentials = service
            .collect(
                Collect {
                    request_id: pending.request_id,
                    exchange_secret: secret,
                },
                "fixture",
            )
            .unwrap();
        (
            service
                .authenticate_live(&credentials.access_token, false, false)
                .unwrap(),
            credentials,
        )
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.core().db.points_path()).unwrap()
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
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(ResourceKind::Viewer, ["alice".into()].into())].into(),
        ..Grant::default()
    }
}

#[test]
fn checked_currency_rejects_before_acceptance_but_replays_the_original_label_after_a_rename() {
    let f = Fixture::new();
    let op = |currency: &str| PointsAdjustment {
        expected_currency: Some(currency.into()),
        unique_id: "alice".into(),
        delta: 2.0,
    };
    let bad = f
        .core()
        .points_command(CommandCaller::LocalOwner, f.request("checked"), op("Gems"))
        .unwrap_err();
    assert_eq!(bad.code, "currency_changed");
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM control_commands", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let request = f.request("checked");
    let original = f
        .core()
        .points_command(CommandCaller::LocalOwner, request.clone(), op("Points"))
        .unwrap();
    assert_eq!(original.value["currencyName"], "Points");
    f.core()
        .points_update_config(serde_json::from_value(json!({"currencyName":"Gems"})).unwrap());
    let replay = f
        .core()
        .points_command(CommandCaller::LocalOwner, request.clone(), op("Points"))
        .unwrap();
    assert_eq!(replay.value, original.value);
    assert_eq!(
        f.core()
            .points_command(CommandCaller::LocalOwner, request, op("Gems"))
            .unwrap_err()
            .code,
        "command_id_reused"
    );
    assert_eq!(
        f.core()
            .points_command(
                CommandCaller::LocalOwner,
                f.request("after-rename"),
                op("Points")
            )
            .unwrap_err()
            .code,
        "currency_changed"
    );
    let current = f
        .core()
        .points_command(
            CommandCaller::LocalOwner,
            f.request("after-rename"),
            op("Gems"),
        )
        .unwrap();
    assert_eq!(current.value["totalPoints"], 4.0);
    assert_eq!(current.value["currencyName"], "Gems");
}

#[test]
fn additive_result_survives_later_awards_writer_shutdown_and_host_restart_without_rewinding_cache()
{
    let mut f = Fixture::new();
    f.core().points_adjust("alice", 10.0).unwrap();
    let request = f.request("once");
    let result = f.adjust("once", "alice", 2.0).unwrap();
    assert_eq!(result.value["totalPoints"], 12.0);
    assert_eq!(result.resource_versions[0].revision, "2");
    assert_eq!(
        result
            .command
            .as_ref()
            .unwrap()
            .resource
            .as_ref()
            .unwrap()
            .kind,
        ResourceKind::Viewer
    );
    f.core()
        .points
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    assert_eq!(f.adjust("once", "alice", 2.0).unwrap().value, result.value);
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 13.0);
    assert_eq!(
        f.adjust("once", "alice", 3.0).unwrap_err().code,
        "command_id_reused"
    );
    f.core().points.shutdown();
    assert_eq!(f.adjust("once", "alice", 2.0).unwrap().value, result.value);
    assert_eq!(
        f.adjust("closed", "alice", 1.0).unwrap_err().code,
        "overloaded"
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM points_transactions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        3
    );
    drop(f.core.take());
    f.core = Some(Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        f.paths.clone(),
    )));
    let replay = f
        .core()
        .points_command(
            CommandCaller::LocalOwner,
            request.clone(),
            PointsAdjustment {
                expected_currency: None,
                unique_id: "alice".into(),
                delta: 2.0,
            },
        )
        .unwrap();
    assert_eq!(replay.value, result.value);
    assert_ne!(
        replay.host_instance_id.as_deref(),
        Some(request.host_instance_id.as_str())
    );
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 13.0);
    assert_eq!(
        f.core()
            .points_command(
                CommandCaller::LocalOwner,
                AdditiveCommandRequest {
                    command_id: "stale-new".into(),
                    ..request
                },
                PointsAdjustment {
                    expected_currency: None,
                    unique_id: "alice".into(),
                    delta: 1.0
                }
            )
            .unwrap_err()
            .code,
        "host_restarted"
    );
}

#[test]
fn points_and_workflow_commands_share_ids_capacity_and_expired_tombstones() {
    let f = Fixture::new();
    let service = f.core().collaboration().unwrap();
    let mut config = service.configuration().unwrap();
    config.limits.command_ids_per_principal = 2;
    service.configure(config).unwrap();
    let graph = json!({"id":"w","name":"Workflow","enabled":false,"nodes":[],"edges":[]});
    let workflow = |id: &str| CommandRequest {
        command_id: id.into(),
        host_instance_id: service.instance_id().into(),
        expected_revision: "0".into(),
    };
    f.core()
        .workflow_command(
            CommandCaller::LocalOwner,
            workflow("shared"),
            WorkflowMutation::Save(graph.clone()),
        )
        .unwrap();
    assert_eq!(
        f.adjust("shared", "alice", 2.0).unwrap_err().code,
        "command_id_reused"
    );
    f.adjust("points", "alice", 2.0).unwrap();
    assert_eq!(
        f.core()
            .workflow_command(
                CommandCaller::LocalOwner,
                workflow("points"),
                WorkflowMutation::Save(json!({"id":"other","name":"Other"}))
            )
            .unwrap_err()
            .code,
        "command_id_reused"
    );
    assert_eq!(
        f.adjust("third", "alice", 2.0).unwrap_err().code,
        "overloaded"
    );
    assert_eq!(
        f.adjust("points", "alice", 2.0).unwrap().value["totalPoints"],
        2.0
    );
    f.db()
        .execute(
            "UPDATE control_commands SET committed_at=0 WHERE command_id='points'",
            [],
        )
        .unwrap();
    f.core().db.prune_command_outcomes().unwrap();
    assert_eq!(
        f.adjust("points", "alice", 2.0).unwrap_err().code,
        "command_expired"
    );
    assert_eq!(
        f.adjust("points", "alice", 3.0).unwrap_err().code,
        "command_id_reused"
    );
    assert_eq!(
        f.adjust("third", "alice", 1.0).unwrap_err().code,
        "overloaded"
    );
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 2.0);
}

#[test]
fn failed_command_reservation_rolls_back_balance_history_versions_and_cache() {
    let f = Fixture::new();
    f.adjust("seed", "alice", 5.0).unwrap();
    let db = f.db();
    db.execute_batch("CREATE TRIGGER reject_point_command BEFORE INSERT ON control_command_outbox WHEN NEW.command_id='reject' BEGIN SELECT RAISE(ABORT,'PRIVATE-FAILURE'); END").unwrap();
    assert!(f.adjust("reject", "alice", 2.0).is_err());
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 5.0);
    assert_eq!(
        db.query_row(
            "SELECT points FROM viewers WHERE unique_id='alice'",
            [],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        5.0
    );
    for (table, expected) in [("points_transactions", 1), ("control_commands", 1)] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            expected
        );
    }
    assert_eq!(
        db.query_row(
            "SELECT revision FROM control_resource_versions WHERE resource_id='alice'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    db.execute_batch("DROP TRIGGER reject_point_command")
        .unwrap();
    assert_eq!(
        f.adjust("reject", "alice", 2.0).unwrap().value["totalPoints"],
        7.0
    );
}

#[test]
fn subscriber_bonus_limits_scopes_and_replay_use_current_authorization() {
    let f = Fixture::new();
    f.core()
        .points_update_config(serde_json::from_value(json!({"subBonusMultiplier":100})).unwrap());
    f.core()
        .points
        .award_points(
            "alice",
            PointAction::Chat,
            AwardOptions {
                is_subscriber: Some(true),
                ..AwardOptions::default()
            },
        )
        .unwrap();
    let mut allowed = grant();
    allowed.max_points_delta = Some(3.0);
    let (auth, credentials) = f.pair(allowed.clone());
    let call = |delta| {
        f.core().points_command(
            CommandCaller::Remote(auth.clone()),
            f.request("bonus"),
            PointsAdjustment {
                expected_currency: None,
                unique_id: "alice".into(),
                delta,
            },
        )
    };
    assert_eq!(call(2.0).unwrap_err().code, "forbidden");
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 2.0);
    allowed.max_points_delta = Some(4.0);
    let service = f.core().collaboration().unwrap();
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            grant: allowed.clone(),
            expected_revision: "1".into(),
        })
        .unwrap();
    let result = call(2.0).unwrap();
    assert_eq!(result.value["delta"], 4.0);
    assert_eq!(result.value["totalPoints"], 6.0);
    f.core()
        .points_update_config(serde_json::from_value(json!({"subBonusMultiplier":0})).unwrap());
    assert_eq!(call(2.0).unwrap().value, result.value);
    allowed.max_points_delta = Some(3.0);
    service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id,
            grant: allowed,
            expected_revision: "2".into(),
        })
        .unwrap();
    assert_eq!(call(2.0).unwrap_err().code, "forbidden");
    let outcome = f
        .core()
        .collaboration_command_get(
            CommandCaller::Remote(auth.clone()),
            CommandGet {
                command_id: "bonus".into(),
                principal_id: None,
            },
        )
        .unwrap();
    assert_eq!(outcome.result, result.value);
    assert_eq!(
        f.core()
            .points_command(
                CommandCaller::Remote(auth),
                f.request("hidden"),
                PointsAdjustment {
                    expected_currency: None,
                    unique_id: "bob".into(),
                    delta: 1.0
                }
            )
            .unwrap_err()
            .code,
        "forbidden"
    );
}

#[test]
fn penalties_keep_the_requested_delta_zero_floor_and_original_result_after_later_awards() {
    let f = Fixture::new();
    f.adjust("seed", "alice", 5.0).unwrap();
    let penalty = f.adjust("penalty", "alice", -10.0).unwrap();
    assert_eq!(penalty.value["delta"], -10.0);
    assert_eq!(penalty.value["totalPoints"], 0.0);
    f.core()
        .points
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    assert_eq!(
        f.adjust("penalty", "alice", -10.0).unwrap().value,
        penalty.value
    );
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 1.0);
    let mut limited = grant();
    limited.max_points_delta = Some(2.0);
    let (auth, _) = f.pair(limited);
    // A zero clamp does not let the requested penalty bypass the grant limit.
    assert_eq!(
        f.core()
            .points_command(
                CommandCaller::Remote(auth),
                f.request("excessive-penalty"),
                PointsAdjustment {
                    expected_currency: None,
                    unique_id: "alice".into(),
                    delta: -3.0
                }
            )
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 1.0);
}

#[test]
fn scoped_committed_reads_filter_before_ranking_and_loading_hidden_profiles() {
    let f = Fixture::new();
    f.adjust("a", "alice", 2.0).unwrap();
    f.adjust("b", "bob", 100.0).unwrap();
    f.db()
        .execute(
            "UPDATE viewers SET nickname=?1 WHERE unique_id='bob'",
            ["PRIVATE".repeat(10_000)],
        )
        .unwrap();
    let scoped = grant();
    let board = f
        .core()
        .points_leaderboard_scoped(&scoped, Some(1), 4096)
        .unwrap();
    assert_eq!(board.value.len(), 1);
    assert_eq!(board.value[0]["uniqueId"], "alice");
    assert_eq!(board.resource_versions[0].revision, "1");
    assert_eq!(
        f.core()
            .points_viewer_scoped(&scoped, "bob", 4096)
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert_eq!(
        f.core()
            .points_viewer_scoped(&scoped, "@alice", 4096)
            .unwrap_err()
            .code,
        "forbidden"
    );
    let mut all = scoped.clone();
    all.resources.clear();
    assert_eq!(
        f.core()
            .points_leaderboard_scoped(&all, Some(1), 4096)
            .unwrap_err()
            .code,
        "too_large"
    );
    assert_eq!(f.adjust("big", "bob", 1.0).unwrap_err().code, "too_large");
    all.resources
        .insert(ResourceKind::Live, ["creator".into()].into());
    assert_eq!(
        f.core()
            .points_leaderboard_scoped(&all, None, 4096)
            .unwrap_err()
            .code,
        "forbidden"
    );
}

#[test]
fn concurrent_duplicates_emit_one_business_change_and_one_history_entry() {
    let f = Fixture::new();
    let request = f.request("duplicate");
    let mut events = f.core().events.subscribe_domain();
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let core = f.core().clone();
            let request = request.clone();
            std::thread::spawn(move || {
                core.points_command(
                    CommandCaller::LocalOwner,
                    request,
                    PointsAdjustment {
                        expected_currency: None,
                        unique_id: "alice".into(),
                        delta: 3.0,
                    },
                )
                .unwrap()
            })
        })
        .collect();
    for thread in threads {
        assert_eq!(thread.join().unwrap().value["totalPoints"], 3.0);
    }
    assert!(matches!(
        events.try_recv().unwrap(),
        DomainEvent::PointsChanged { .. }
    ));
    assert!(matches!(
        events.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM points_transactions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM control_command_outbox", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn queued_command_loses_write_authority_before_admission_without_reserving_an_id() {
    let f = Fixture::new();
    let (auth, credentials) = f.pair(grant());
    let gate = f.core().coordination_gate.lock().unwrap();
    let core = f.core().clone();
    let request = f.request("queued");
    let (started, observed) = std::sync::mpsc::sync_channel(0);
    let thread = std::thread::spawn(move || {
        started.send(()).unwrap();
        core.points_command(
            CommandCaller::Remote(auth),
            request,
            PointsAdjustment {
                expected_currency: None,
                unique_id: "alice".into(),
                delta: 2.0,
            },
        )
    });
    observed
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let mut narrowed = grant();
    narrowed.capabilities.remove(&Capability::PointsAdjust);
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: credentials.principal_id,
            grant: narrowed,
            expected_revision: "1".into(),
        })
        .unwrap();
    drop(gate);
    assert_eq!(thread.join().unwrap().unwrap_err().code, "forbidden");
    assert!(f
        .core()
        .db
        .command_resource("local-owner", "queued")
        .unwrap()
        .is_none());
    assert!(f.core().points_viewer("alice").is_err());
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM control_commands", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn awards_racing_with_an_additive_command_never_overwrite_or_duplicate_it() {
    let f = Fixture::new();
    f.adjust("seed", "alice", 0.0).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let core = f.core().clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..200 {
                    core.points
                        .award_points("alice", PointAction::Chat, AwardOptions::default())
                        .unwrap();
                }
            })
        })
        .collect();
    barrier.wait();
    let result = f.adjust("racing", "alice", 10.0);
    for thread in threads {
        thread.join().unwrap();
    }
    match result {
        Ok(_) => {}
        Err(error) => {
            assert_eq!(error.code, "overloaded");
            // Only an explicit known pre-admission rejection is retried here;
            // the client transport never blindly resends an uncertain command.
            assert!(f
                .core()
                .db
                .command_resource("local-owner", "racing")
                .unwrap()
                .is_none());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while f.core().db.load_viewer_by_id("alice").unwrap().unwrap()["points"] != 400.0 {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            f.adjust("racing", "alice", 10.0).unwrap();
        }
    }
    f.core().points.shutdown();
    assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 410.0);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT points FROM viewers WHERE unique_id='alice'",
                [],
                |r| r.get::<_, f64>(0)
            )
            .unwrap(),
        410.0
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT revision FROM control_resource_versions WHERE resource_id='alice'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        402
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM control_commands", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
#[ignore = "subprocess-only points commit crash fixture"]
fn point_command_crash_child() {
    let root =
        std::path::PathBuf::from(std::env::var_os("TIKTOOLS_POINTS_COMMAND_CRASH_ROOT").unwrap());
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent-builtin"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.clone(),
    };
    let core = AppCore::with_paths(Arc::new(Emitter), Arc::new(NoopMediaHost), paths);
    let instance = core.collaboration().unwrap().instance_id().to_owned();
    let file = std::fs::File::create(root.join("intent.json")).unwrap();
    serde_json::to_writer(
        &file,
        &json!({"commandId":"process-once","hostInstanceId":instance}),
    )
    .unwrap();
    file.sync_all().unwrap();
    core.points_command(
        CommandCaller::LocalOwner,
        AdditiveCommandRequest {
            command_id: "process-once".into(),
            host_instance_id: instance,
        },
        PointsAdjustment {
            expected_currency: None,
            unique_id: "alice".into(),
            delta: 2.0,
        },
    )
    .unwrap();
    panic!("points command failpoint did not terminate");
}

#[test]
fn process_death_before_or_after_commit_preserves_atomic_balance_and_the_original_command_id() {
    for (stage, exit) in [("before_commit", 96), ("after_commit", 97)] {
        let mut f = Fixture::new();
        drop(f.core.take());
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "control::points::tests::point_command_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("TIKTOOLS_POINTS_COMMAND_CRASH_ROOT", &f.paths.root)
            .env("TIKTOOLS_POINTS_COMMAND_CRASH_STAGE", stage)
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        let intent: Value =
            serde_json::from_slice(&std::fs::read(f.paths.root.join("intent.json")).unwrap())
                .unwrap();
        let request = AdditiveCommandRequest {
            command_id: intent["commandId"].as_str().unwrap().into(),
            host_instance_id: intent["hostInstanceId"].as_str().unwrap().into(),
        };
        f.core = Some(Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            f.paths.clone(),
        )));
        let mut events = f.core().events.subscribe_domain();
        let operation = PointsAdjustment {
            expected_currency: None,
            unique_id: "alice".into(),
            delta: 2.0,
        };
        if stage == "before_commit" {
            assert!(f.core().points_viewer("alice").is_err());
            assert!(f
                .core()
                .db
                .command_resource("local-owner", "process-once")
                .unwrap()
                .is_none());
            assert_eq!(
                f.core()
                    .points_command(
                        CommandCaller::LocalOwner,
                        request.clone(),
                        operation.clone()
                    )
                    .unwrap_err()
                    .code,
                "host_restarted"
            );
            f.adjust("process-once", "alice", 2.0).unwrap();
            events.try_recv().unwrap();
        } else {
            assert_eq!(
                f.core()
                    .points_command(
                        CommandCaller::LocalOwner,
                        request.clone(),
                        operation.clone()
                    )
                    .unwrap()
                    .value["totalPoints"],
                2.0
            );
        }
        assert!(matches!(
            events.try_recv(),
            Err(crate::events::DomainTryRecvError::Empty)
        ));
        f.core()
            .points
            .award_points("alice", PointAction::Chat, AwardOptions::default())
            .unwrap();
        let result = f
            .core()
            .points_command(CommandCaller::LocalOwner, request, operation)
            .unwrap();
        assert_eq!(result.value["totalPoints"], 2.0);
        assert_eq!(f.core().points_viewer("alice").unwrap()["points"], 3.0);
        f.core().points.shutdown();
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM control_commands", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM points_transactions", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
}

#[test]
fn retained_points_replay_reads_the_original_outcome_while_another_writer_holds_a_balance_change() {
    let f = Fixture::new();
    let request = f.request("replay-under-writer");
    let operation = PointsAdjustment {
        expected_currency: None,
        unique_id: "alice".into(),
        delta: 2.0,
    };
    let original = f
        .core()
        .points_command(
            CommandCaller::LocalOwner,
            request.clone(),
            operation.clone(),
        )
        .unwrap();
    assert_eq!(original.value["totalPoints"], 2.0);
    let cached = f.core().points_viewer("alice").unwrap();
    let mut events = f.core().events.subscribe_domain();
    let writer = f.db();
    writer
        .execute_batch("BEGIN IMMEDIATE; UPDATE viewers SET points=99 WHERE unique_id='alice';")
        .unwrap();

    // Keep the writer reservation through the assertions. A replay that asks
    // for writer admission would exhaust its timeout instead of reading the
    // immutable committed receipt alongside this uncommitted balance.
    let replay = f
        .core()
        .points_command(CommandCaller::LocalOwner, request, operation)
        .unwrap();
    assert_eq!(replay, original);
    assert_eq!(f.core().points_viewer("alice").unwrap(), cached);
    assert!(!writer.is_autocommit());
    assert_eq!(
        writer
            .query_row(
                "SELECT points FROM viewers WHERE unique_id='alice'",
                [],
                |r| { r.get::<_, f64>(0) }
            )
            .unwrap(),
        99.0,
        "replay must leave the competing writer's transaction untouched"
    );
    assert!(matches!(
        events.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));

    writer.execute_batch("ROLLBACK;").unwrap();
    let persisted: (f64, i64, i64, i64) = f
        .db()
        .query_row(
            "SELECT points,(SELECT count(*) FROM control_commands),(SELECT count(*) FROM points_transactions),(SELECT revision FROM control_resource_versions WHERE kind='viewer' AND scope='host' AND resource_id='alice') FROM viewers WHERE unique_id='alice'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(persisted, (2.0, 1, 1, 1));
    assert_eq!(f.core().points_viewer("alice").unwrap(), cached);
    assert!(matches!(
        events.try_recv(),
        Err(crate::events::DomainTryRecvError::Empty)
    ));
}
