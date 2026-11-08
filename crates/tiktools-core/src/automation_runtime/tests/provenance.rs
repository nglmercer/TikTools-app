//! Real core writers/runtime admission over private business databases. Editable
//! authority-looking JSON never supplies the trusted execution provenance.
#![cfg(feature = "persistence")]
use super::*;
use crate::{
    collaboration::*,
    control::{AutomationKind, AutomationMutation, CommandCaller},
    paths::AppPaths,
    services::{AwardOptions, PointAction},
};

fn fixture() -> (Arc<AppCore>, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "tt-runtime-provenance-{}-{}",
        std::process::id(),
        fastrand::u64(..)
    ));
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
    paths.ensure_directories().unwrap();
    for path in [
        paths.points_database(),
        paths.automation_database(),
        paths.analytics_database(),
    ] {
        std::fs::File::create(path).unwrap();
    }
    let core = reopen(paths);
    core.points
        .award_points(
            "fenced-viewer",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(10.0),
                ..Default::default()
            },
        )
        .unwrap();
    (core, root)
}
fn reopen(paths: AppPaths) -> Arc<AppCore> {
    Arc::new(AppCore::with_paths(
        Arc::new(NoopEmitter),
        Arc::new(crate::NoopMediaHost),
        paths,
    ))
}
fn auth(core: &AppCore) -> Authorization {
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
                    Capability::AutomationToggle,
                ]
                .into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let exchange = "a".repeat(64);
    let request = service
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Runtime proposal fixture".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "fixture",
        )
        .unwrap();
    service.decide_request(&request.request_id, true).unwrap();
    let credentials = service
        .collect(
            Collect {
                request_id: request.request_id,
                exchange_secret: exchange,
            },
            "fixture",
        )
        .unwrap();
    service
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap()
}
fn command(
    core: &AppCore,
    caller: CommandCaller,
    revision: &str,
    op: AutomationMutation,
) -> Versioned<Value> {
    core.automation_command(
        caller,
        CommandRequest {
            command_id: format!("provenance-{}", fastrand::u64(..)),
            host_instance_id: core.collaboration().unwrap().instance_id().into(),
            expected_revision: revision.into(),
        },
        op,
    )
    .unwrap()
}
fn delegated(core: &AppCore, auth: &Authorization, kind: AutomationKind, mut record: Value) {
    record["enabled"] = false.into();
    // These fields deliberately survive in the editable body. None is trusted.
    record["authority"] = "owner".into();
    record["provenance"] = json!({"authority":{"kind":"owner"}});
    command(
        core,
        CommandCaller::Remote(auth.clone()),
        "0",
        AutomationMutation::Create { kind, record },
    );
}
fn revision(core: &AppCore, kind: AutomationKind, id: &str) -> String {
    core.automation_get_versioned(kind, id)
        .unwrap()
        .resource_versions[0]
        .revision
        .clone()
}
fn authority(core: &AppCore, kind: AutomationKind, id: &str) -> Value {
    core.automation_authority(kind, id).unwrap().value["provenance"]["authority"].clone()
}
fn penalty(id: &str) -> Value {
    json!({"id":id,"name":"Penalty","enabled":true,"typeId":"core.points.subtract","config":{"uniqueId":"fenced-viewer","amount":2}})
}
fn event(id: &str, ids: &[&str]) -> Value {
    json!({"id":id,"name":"Event","enabled":true,"trigger":"tiktok.chat","actionIds":ids})
}
fn input() -> Value {
    json!({"type":"tiktok.chat","user":{"uniqueId":"fenced-viewer"},"data":{}})
}
fn points(core: &AppCore) -> Value {
    core.points.viewer("fenced-viewer").unwrap()["points"].clone()
}
async fn close(core: Arc<AppCore>, root: std::path::PathBuf) {
    core.shutdown().await;
    drop(core);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn delegated_event_stays_quarantined_through_owner_writes_until_explicit_adoption_and_enable()
{
    let (core, root) = fixture();
    let auth = auth(&core);
    core.automation_create(AutomationKind::Action, penalty("penalty"))
        .unwrap();
    delegated(
        &core,
        &auth,
        AutomationKind::Event,
        event("delegated", &["penalty"]),
    );
    let original = authority(&core, AutomationKind::Event, "delegated");
    assert_eq!(original["kind"], "delegated");
    assert_eq!(
        original["principalId"],
        auth.snapshot().unwrap().principal_id
    );
    // Owner saves, updates, toggles and bulk transitions are definition writes,
    // never an implicit transfer of the remote principal's authority.
    let record = event("delegated", &["penalty"]);
    core.automation_save_versioned(AutomationKind::Event, &record, None)
        .unwrap();
    core.automation_update(AutomationKind::Event, "delegated", record.clone())
        .unwrap();
    core.automation_set_enabled(AutomationKind::Event, "delegated", true)
        .unwrap();
    core.automation_set_enablement_versioned(&[(AutomationKind::Event, "delegated".into(), true)])
        .unwrap();
    assert_eq!(
        authority(&core, AutomationKind::Event, "delegated"),
        original
    );
    assert!(core.automation.matching_records(&input()).is_empty());
    assert!(core.automation.manual_record(&record).is_none());
    core.run_automation_event(AutomationEvent::owner(input()))
        .await;
    core.fire_synthetic_event("tiktok.chat", Some(&input()), Some(&record))
        .await;
    let direct = core
        .execute_action(&penalty("penalty"), &input(), Some(&record), false)
        .await;
    assert_eq!(direct["error"], "authority_required");
    assert_eq!(points(&core), 10.0);
    assert!(core.automation.recent_runs().is_empty());
    let current = revision(&core, AutomationKind::Event, "delegated");
    command(
        &core,
        CommandCaller::LocalOwner,
        &current,
        AutomationMutation::Adopt {
            kind: AutomationKind::Event,
            id: "delegated".into(),
            reviewed_versions: None,
        },
    );
    assert_eq!(
        authority(&core, AutomationKind::Event, "delegated")["kind"],
        "owner"
    );
    assert_eq!(
        core.automation_get(AutomationKind::Event, "delegated")
            .unwrap()["enabled"],
        false
    );
    core.run_automation_event(AutomationEvent::owner(input()))
        .await;
    assert_eq!(
        points(&core),
        10.0,
        "adoption must not activate the definition"
    );
    core.automation_set_enabled(AutomationKind::Event, "delegated", true)
        .unwrap();
    core.run_automation_event(AutomationEvent::owner(input()))
        .await;
    assert_eq!(
        points(&core),
        8.0,
        "the explicitly adopted and separately enabled owner chain must execute"
    );
    close(core, root).await;
}

#[tokio::test]
async fn delegated_action_blocks_direct_manual_and_every_owner_event_reference_including_disabled_and_random_steps(
) {
    let (core, root) = fixture();
    let auth = auth(&core);
    delegated(&core, &auth, AutomationKind::Action, penalty("remote-step"));
    core.automation_create(AutomationKind::Action, penalty("owner-step"))
        .unwrap();
    for random in [false, true] {
        let mut record = event(
            if random { "random" } else { "disabled-ref" },
            &["owner-step", "remote-step"],
        );
        if random {
            record["runMode"] = "random".into();
        }
        core.automation_create(AutomationKind::Event, record.clone())
            .unwrap();
        // Even a disabled remote action is a referenced dependency. Skipping it
        // must not cause the owner's remaining effect to run as a weaker chain.
        assert!(core.automation.matching_records(&input()).is_empty());
        assert!(core.automation.actions_for_event(&record).is_empty());
        assert!(core.automation.manual_record(&record).is_none());
        core.run_automation_event(AutomationEvent::owner(input()))
            .await;
        core.fire_synthetic_event("tiktok.chat", Some(&input()), Some(&record))
            .await;
    }
    let step = penalty("remote-step");
    for test in [false, true] {
        assert_eq!(
            core.execute_action(&step, &input(), None, test).await["error"],
            "authority_required"
        );
    }
    core.automation_set_enabled(AutomationKind::Action, "remote-step", true)
        .unwrap();
    core.run_automation_event(AutomationEvent::owner(input()))
        .await;
    assert!(
        core.automation.matching_records(&input()).is_empty(),
        "random selection must not hide an unselected delegated dependency"
    );
    assert_eq!(points(&core), 10.0);
    assert!(core.automation.recent_runs().is_empty());
    close(core, root).await;
}

#[tokio::test]
async fn delegated_tombstones_block_same_id_inline_work_and_owner_recreation_without_adoption() {
    for kind in [AutomationKind::Event, AutomationKind::Action] {
        let (core, root) = fixture();
        let auth = auth(&core);
        core.automation_create(AutomationKind::Action, penalty("owner-step"))
            .unwrap();
        let body = if kind == AutomationKind::Event {
            event("retained", &["owner-step"])
        } else {
            penalty("retained")
        };
        delegated(&core, &auth, kind, body.clone());
        let original = authority(&core, kind, "retained");
        core.automation_delete(kind, "retained").unwrap();
        assert_eq!(authority(&core, kind, "retained"), original);
        if kind == AutomationKind::Event {
            assert!(core.automation.manual_record(&body).is_none());
            core.fire_synthetic_event("tiktok.chat", Some(&input()), Some(&body))
                .await;
        } else {
            assert_eq!(
                core.execute_action(&body, &input(), None, false).await["error"],
                "authority_required"
            );
        }
        core.automation_create(kind, body.clone()).unwrap();
        assert_eq!(authority(&core, kind, "retained"), original);
        if kind == AutomationKind::Event {
            assert!(core.automation.matching_records(&input()).is_empty());
        } else {
            core.automation_create(AutomationKind::Event, event("owner", &["retained"]))
                .unwrap();
            assert!(core.automation.matching_records(&input()).is_empty());
        }
        core.run_automation_event(AutomationEvent::owner(input()))
            .await;
        assert_eq!(points(&core), 10.0);
        close(core, root).await;
    }
}

#[tokio::test]
async fn failed_trusted_snapshot_quarantines_saved_and_inline_work_until_an_explicit_good_refresh()
{
    let (core, root) = fixture();
    core.automation_create(AutomationKind::Action, penalty("owner-step"))
        .unwrap();
    core.automation_create(AutomationKind::Event, event("owner", &["owner-step"]))
        .unwrap();
    let db = rusqlite::Connection::open(core.db.paths().automation_database()).unwrap();
    db.execute_batch("ALTER TABLE behavior_provenance RENAME TO unavailable_provenance;")
        .unwrap();
    assert!(core.behavior_snapshot_versioned().is_err());
    assert!(core.automation.matching_records(&input()).is_empty());
    let inline = event("new-inline", &["owner-step"]);
    assert!(core.automation.manual_record(&inline).is_none());
    assert_eq!(
        core.execute_action(&penalty("new-inline-action"), &input(), None, false)
            .await["error"],
        "authority_required"
    );
    core.fire_synthetic_event("tiktok.chat", Some(&input()), Some(&inline))
        .await;
    assert_eq!(points(&core), 10.0);
    assert!(core.automation.recent_runs().is_empty());
    db.execute_batch("ALTER TABLE unavailable_provenance RENAME TO behavior_provenance;")
        .unwrap();
    core.behavior_snapshot_versioned().unwrap();
    core.run_automation_event(AutomationEvent::owner(input()))
        .await;
    assert_eq!(
        points(&core),
        8.0,
        "recovering a valid trusted snapshot restores legitimate owner work"
    );
    close(core, root).await;
}

#[tokio::test]
async fn restart_never_reseeds_missing_existing_authority_as_owner() {
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        let (core, root) = fixture();
        core.automation_create(AutomationKind::Action, penalty("same"))
            .unwrap();
        core.automation_create(AutomationKind::Event, event("same", &["same"]))
            .unwrap();
        let paths = core.db.paths().clone();
        let db = rusqlite::Connection::open(paths.automation_database()).unwrap();
        db.execute(
            "DELETE FROM behavior_provenance WHERE scope=?1 AND resource_id='same'",
            [if kind == AutomationKind::Action {
                "action"
            } else {
                "event"
            }],
        )
        .unwrap();
        drop(db);
        core.shutdown().await;
        drop(core);
        let core = reopen(paths);
        assert!(core.behavior_snapshot_versioned().is_err());
        assert!(core.automation_authority(kind, "same").unwrap().value["provenance"].is_null());
        assert!(core.automation.matching_records(&input()).is_empty());
        assert!(core
            .automation
            .manual_record(&event("same", &["same"]))
            .is_none());
        core.run_automation_event(AutomationEvent::owner(input()))
            .await;
        assert_eq!(points(&core), 10.0);
        close(core, root).await;
    }
}

#[tokio::test]
async fn hotkey_projection_uses_trusted_owner_events_and_every_action_reference() {
    let (core, root) = fixture();
    let auth = auth(&core);
    delegated(&core, &auth, AutomationKind::Action, penalty("remote-step"));
    let hotkey = |id: &str, key: &str, ids: &[&str]| json!({"id":id,"name":"Hotkey","enabled":true,"trigger":"hotkey.pressed","filters":[{"path":"event.data.key","operator":"eq","value":key},{"path":"event.data.modifiers","operator":"eq","value":"ctrl"}],"actionIds":ids});
    delegated(
        &core,
        &auth,
        AutomationKind::Event,
        hotkey("remote-hotkey", "d", &[]),
    );
    core.automation_set_enabled(AutomationKind::Event, "remote-hotkey", true)
        .unwrap();
    core.automation_create(
        AutomationKind::Event,
        hotkey("owner-dependency", "p", &["remote-step"]),
    )
    .unwrap();
    core.automation_create(AutomationKind::Event, hotkey("owner-hotkey", "o", &[]))
        .unwrap();
    let snapshot = core.load_behavior_snapshot_for_hotkey_sync();
    let config = crate::hotkey_bindings::desired_hotkey_bind_config(&snapshot);
    assert_eq!(
        config,
        json!({"shortcuts":[{"key":"o","modifiers":"ctrl"}],"sequencesNeeded":false})
    );
    close(core, root).await;
}

async fn assert_corrupt_runtime_quarantined(core: &Arc<AppCore>, record: &Value, action: &Value) {
    assert!(core.automation.matching_records(&input()).is_empty());
    assert!(core.automation.manual_record(record).is_none());
    assert!(core.automation.actions_for_event(record).is_empty());
    assert_eq!(
        core.fire_synthetic_event("tiktok.chat", Some(&input()), Some(record))
            .await["error"],
        "authority_required"
    );
    assert_eq!(core.test_event(record).await["error"], "authority_required");
    for test in [false, true] {
        assert_eq!(
            core.execute_action(action, &input(), None, test).await["error"],
            "authority_required"
        );
    }
    // Corruption cannot be bypassed by a fresh owner inline identity either.
    assert!(core
        .automation
        .manual_record(&event("fresh-inline", &["owner-step"]))
        .is_none());
    assert_eq!(
        core.execute_action(&penalty("fresh-inline-step"), &input(), None, false)
            .await["error"],
        "authority_required"
    );
    core.run_automation_event(AutomationEvent::owner(input()))
        .await;
    assert_eq!(points(core), 10.0);
    assert!(core.automation.recent_runs().is_empty());
}

#[tokio::test]
async fn corrupt_live_versions_never_reclassify_delegated_definitions_as_new_owner_inline_work() {
    for corruption in ["missing-version", "deleted-version"] {
        for kind in [AutomationKind::Action, AutomationKind::Event] {
            let (core, root) = fixture();
            let auth = auth(&core);
            core.automation_create(AutomationKind::Action, penalty("owner-step"))
                .unwrap();
            let (body, record, action) = if kind == AutomationKind::Event {
                let body = event("retained", &["owner-step"]);
                (body.clone(), body, penalty("owner-step"))
            } else {
                let body = penalty("retained");
                (
                    body.clone(),
                    event("owner-event", &["retained", "owner-step"]),
                    body,
                )
            };
            delegated(&core, &auth, kind, body);
            // Enabled owner writes preserve delegated authority. This remains
            // a quarantined remote identity before metadata is damaged.
            core.automation_set_enabled(kind, "retained", true).unwrap();
            if kind == AutomationKind::Action {
                core.automation_create(AutomationKind::Event, record.clone())
                    .unwrap();
            }
            assert!(core.automation.manual_record(&record).is_none());
            let paths = core.db.paths().clone();
            let db = rusqlite::Connection::open(paths.automation_database()).unwrap();
            let scope = if kind == AutomationKind::Action {
                "action"
            } else {
                "event"
            };
            let sql = if corruption == "missing-version" {
                "DELETE FROM control_resource_versions WHERE kind='automation' AND scope=?1 AND resource_id='retained'"
            } else {
                "UPDATE control_resource_versions SET deleted=1 WHERE kind='automation' AND scope=?1 AND resource_id='retained'"
            };
            assert_eq!(db.execute(sql, [scope]).unwrap(), 1);
            drop(db);
            assert!(
                core.behavior_snapshot_versioned().is_err(),
                "{corruption} for {scope} must fail trusted refresh"
            );
            assert_corrupt_runtime_quarantined(&core, &record, &action).await;
            core.shutdown().await;
            drop(core);

            // Initialization must not reseed a historical missing version or
            // turn the omitted saved body into a supposedly new owner draft.
            let core = reopen(paths);
            assert_eq!(authority(&core, kind, "retained")["kind"], "delegated");
            assert!(
                core.behavior_snapshot_versioned().is_err(),
                "{corruption} for {scope} must remain invalid after restart"
            );
            assert_corrupt_runtime_quarantined(&core, &record, &action).await;
            close(core, root).await;
        }
    }
}

#[tokio::test]
async fn missing_tombstone_authority_never_turns_deleted_delegated_ids_into_owner_inline_work() {
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        let (core, root) = fixture();
        let auth = auth(&core);
        core.automation_create(AutomationKind::Action, penalty("owner-step"))
            .unwrap();
        let (body, record, action) = if kind == AutomationKind::Event {
            let body = event("retained", &["owner-step"]);
            (body.clone(), body, penalty("owner-step"))
        } else {
            let body = penalty("retained");
            (
                body.clone(),
                event("inline-event", &["retained", "owner-step"]),
                body,
            )
        };
        delegated(&core, &auth, kind, body);
        core.automation_delete(kind, "retained").unwrap();
        assert_eq!(authority(&core, kind, "retained")["kind"], "delegated");
        let paths = core.db.paths().clone();
        let db = rusqlite::Connection::open(paths.automation_database()).unwrap();
        assert_eq!(
            db.execute(
                "DELETE FROM behavior_provenance WHERE scope=?1 AND resource_id='retained'",
                [if kind == AutomationKind::Action {
                    "action"
                } else {
                    "event"
                }],
            )
            .unwrap(),
            1
        );
        drop(db);
        assert!(core.behavior_snapshot_versioned().is_err());
        assert_corrupt_runtime_quarantined(&core, &record, &action).await;
        core.shutdown().await;
        drop(core);

        let core = reopen(paths);
        assert!(core.automation_authority(kind, "retained").unwrap().value["provenance"].is_null());
        assert!(core.behavior_snapshot_versioned().is_err());
        assert_corrupt_runtime_quarantined(&core, &record, &action).await;
        close(core, root).await;
    }
}
