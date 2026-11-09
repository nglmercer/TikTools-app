use super::*;
use crate::{control::EffectState, paths::AppPaths};
use rusqlite::Connection;
use std::{fs, path::PathBuf};

struct Fixture {
    db: Arc<DatabaseManager>,
    journal: EffectJournal,
    root: PathBuf,
}

fn paths(root: PathBuf) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root,
    }
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-effects-{}-{}",
            std::process::id(),
            random_identity().unwrap()
        ));
        let paths = paths(root.clone());
        paths.ensure_directories().unwrap();
        // Never import a developer's legacy ./data during isolated tests.
        for file in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            Connection::open(file).unwrap();
        }
        let db = Arc::new(DatabaseManager::new(paths));
        let journal = EffectJournal::open(db.clone()).unwrap();
        Self { db, journal, root }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn accept(commit: &mut dyn FnMut() -> Result<(), String>) -> Result<(), String> {
    commit()
}
fn key(execution: &EffectExecution) -> EffectKey {
    execution
        .key(
            EffectKind::Http,
            &hash_secret("binding"),
            &hash_secret("request"),
        )
        .unwrap()
}

#[test]
fn points_identity_is_stable_across_semantics_but_separates_operation_root_and_saved_slot() {
    let execution = EffectExecution::fresh()
        .unwrap()
        .named_child("action", "points")
        .unwrap();
    let binding = hash_secret("saved definition");
    let request = hash_secret("viewer and requested delta");
    let original = execution
        .points_key(ApprovedPointsOperation::Adjust, &binding, &request)
        .unwrap();
    for (binding, request) in [
        (hash_secret("changed definition"), request.clone()),
        (binding.clone(), hash_secret("changed rendered viewer")),
    ] {
        let changed = execution
            .points_key(ApprovedPointsOperation::Adjust, &binding, &request)
            .unwrap();
        assert_eq!(original.id(), changed.id());
        assert_ne!(original.fingerprint(), changed.fingerprint());
    }
    for (execution, operation) in [
        (execution.clone(), ApprovedPointsOperation::Subtract),
        (execution.child(1).unwrap(), ApprovedPointsOperation::Adjust),
        (
            EffectExecution::fresh().unwrap(),
            ApprovedPointsOperation::Adjust,
        ),
    ] {
        assert_ne!(
            original.id(),
            execution
                .points_key(operation, &binding, &request)
                .unwrap()
                .id()
        );
    }
    assert!(execution
        .points_key(ApprovedPointsOperation::Adjust, "bad", &request)
        .is_err());
    assert!(execution
        .points_key(ApprovedPointsOperation::Adjust, &binding, "bad")
        .is_err());
}
fn ticket(journal: &EffectJournal, key: EffectKey) -> EffectTicket {
    match journal.reserve(key, accept).unwrap() {
        EffectAdmission::Fresh(ticket) => ticket,
        EffectAdmission::Retained(_) => panic!("expected fresh effect"),
    }
}
fn retained(journal: &EffectJournal, key: EffectKey) -> EffectReceipt {
    match journal
        .reserve(key, |_| panic!("history must not readmit"))
        .unwrap()
    {
        EffectAdmission::Retained(receipt) => receipt,
        EffectAdmission::Fresh(_) => panic!("retained identity was admitted twice"),
    }
}

#[test]
fn stable_identity_uses_trusted_execution_slot_kind_and_conflicts_on_changed_semantics() {
    let f = Fixture::new();
    let execution = EffectExecution::fresh().unwrap().child(4).unwrap();
    let original = key(&execution);
    let changed_request = execution
        .key(
            EffectKind::Http,
            &hash_secret("binding"),
            &hash_secret("changed request"),
        )
        .unwrap();
    let changed_binding = execution
        .key(
            EffectKind::Http,
            &hash_secret("changed binding"),
            &hash_secret("request"),
        )
        .unwrap();
    assert_eq!(original.id(), changed_request.id());
    assert_eq!(original.id(), changed_binding.id());
    assert_ne!(original.id(), key(&execution.child(0).unwrap()).id());
    assert_ne!(original.id(), key(&EffectExecution::fresh().unwrap()).id());
    assert_ne!(
        original.id(),
        execution
            .key(
                EffectKind::Audio,
                &hash_secret("binding"),
                &hash_secret("request")
            )
            .unwrap()
            .id()
    );
    let accepted = ticket(&f.journal, original.clone());
    assert_eq!(
        retained(&f.journal, original.clone()).state,
        EffectState::Accepted
    );
    for changed in [changed_request, changed_binding] {
        assert!(f
            .journal
            .reserve(changed, accept)
            .err()
            .unwrap()
            .contains("different semantics"));
    }
    let dispatched = accepted.dispatch(accept).unwrap();
    assert_eq!(
        retained(&f.journal, original.clone()).state,
        EffectState::Dispatched
    );
    let receipt = dispatched.complete(EffectOutcome::Succeeded).unwrap();
    assert_eq!(receipt.state, EffectState::Succeeded);
    assert_eq!(retained(&f.journal, original), receipt);
}

#[test]
fn actual_acceptance_and_dispatch_commits_are_inside_final_authority_callback() {
    let f = Fixture::new();
    let original = key(&EffectExecution::fresh().unwrap());
    let id = original.id().to_owned();
    let guarded = std::sync::Mutex::new(());
    let accepted = match f
        .journal
        .reserve(original.clone(), |commit| {
            let _held = guarded.lock().unwrap();
            assert!(f.db.load_effect_receipt(&id).unwrap().is_none());
            commit()?;
            // Another connection sees the committed row before the guard releases.
            assert_eq!(
                f.db.load_effect_receipt(&id).unwrap().unwrap().state,
                EffectState::Accepted
            );
            Ok(())
        })
        .unwrap()
    {
        EffectAdmission::Fresh(ticket) => ticket,
        EffectAdmission::Retained(_) => unreachable!(),
    };
    let dispatched = accepted
        .dispatch(|commit| {
            let _held = guarded.lock().unwrap();
            commit()?;
            assert_eq!(
                f.db.load_effect_receipt(&id).unwrap().unwrap().state,
                EffectState::Dispatched
            );
            Ok(())
        })
        .unwrap();
    dispatched.complete(EffectOutcome::Failed).unwrap();
    assert_eq!(retained(&f.journal, original).state, EffectState::Failed);
}

#[test]
fn denied_or_missing_final_admission_cannot_dispatch_and_tickets_are_one_shot() {
    let f = Fixture::new();
    let denied = key(&EffectExecution::fresh().unwrap());
    let denied_id = denied.id().to_owned();
    assert!(f
        .journal
        .reserve(denied.clone(), |_| Err("revoked".into()))
        .is_err());
    assert!(f.db.load_effect_receipt(&denied_id).unwrap().is_none());
    assert!(f.journal.reserve(denied.clone(), |_| Ok(())).is_err());
    assert!(f.db.load_effect_receipt(&denied_id).unwrap().is_none());
    let accepted = ticket(&f.journal, denied.clone());
    assert!(accepted
        .dispatch(|_| Err("revoked after preparation".into()))
        .is_err());
    assert_eq!(retained(&f.journal, denied).state, EffectState::Failed);

    let double = key(&EffectExecution::fresh().unwrap());
    assert!(f
        .journal
        .reserve(double.clone(), |commit| {
            commit()?;
            commit()
        })
        .is_err());
    // The first durable acceptance remains consumed, with no returned ticket.
    assert_eq!(retained(&f.journal, double).state, EffectState::Accepted);
}

#[test]
fn cancellation_is_failed_before_handoff_and_unknown_after_handoff() {
    let f = Fixture::new();
    let canceled = key(&EffectExecution::fresh().unwrap());
    ticket(&f.journal, canceled.clone()).cancel().unwrap();
    assert_eq!(retained(&f.journal, canceled).state, EffectState::Failed);
    let dropped = key(&EffectExecution::fresh().unwrap());
    drop(ticket(&f.journal, dropped.clone()));
    assert_eq!(retained(&f.journal, dropped).state, EffectState::Failed);
    let ambiguous = key(&EffectExecution::fresh().unwrap());
    drop(
        ticket(&f.journal, ambiguous.clone())
            .dispatch(accept)
            .unwrap(),
    );
    assert_eq!(retained(&f.journal, ambiguous).state, EffectState::Unknown);
}

#[test]
fn successor_fences_old_host_and_unknown_can_never_become_fresh() {
    let f = Fixture::new();
    let accepted_key = key(&EffectExecution::fresh().unwrap());
    let accepted = ticket(&f.journal, accepted_key.clone());
    let dispatched_key = key(&EffectExecution::fresh().unwrap());
    let dispatched = ticket(&f.journal, dispatched_key.clone())
        .dispatch(accept)
        .unwrap();
    let successor = EffectJournal::open(f.db.clone()).unwrap();
    assert!(accepted.dispatch(accept).is_err());
    assert!(dispatched.complete(EffectOutcome::Succeeded).is_err());
    assert_eq!(
        retained(&successor, accepted_key).state,
        EffectState::Unknown
    );
    assert_eq!(
        retained(&successor, dispatched_key).state,
        EffectState::Unknown
    );
    assert!(f
        .journal
        .reserve(key(&EffectExecution::fresh().unwrap()), accept)
        .is_err());
}

#[test]
fn expired_outcomes_retain_identity_and_history_contains_metadata_only() {
    let f = Fixture::new();
    let secret = "do-not-store-source-url-query-body-settings-response";
    let root = EffectExecution::fresh().unwrap();
    let original = root
        .key(
            EffectKind::Plugin,
            &hash_secret(secret),
            &hash_secret(secret),
        )
        .unwrap();
    let id = original.id().to_owned();
    ticket(&f.journal, original.clone())
        .dispatch(accept)
        .unwrap()
        .complete(EffectOutcome::Succeeded)
        .unwrap();
    let db = Connection::open(f.db.automation_path()).unwrap();
    db.execute(
        "UPDATE automation_external_effects SET accepted_at=1,updated_at=1 WHERE effect_id=?1",
        [&id],
    )
    .unwrap();
    let receipt = retained(&f.journal, original);
    assert!(receipt.outcome_expired);
    assert_eq!(receipt.state, EffectState::Unknown);
    let page = f.db.load_effect_history(None, 1).unwrap();
    assert_eq!(page.items, vec![receipt]);
    let serialized = serde_json::to_string(&page).unwrap();
    assert!(!serialized.contains(secret));
    assert!(!serialized.contains("bindingDigest"));
    assert!(!serialized.contains("requestDigest"));
    assert!(!fs::read(f.db.automation_path())
        .unwrap()
        .windows(secret.len())
        .any(|bytes| bytes == secret.as_bytes()));
    assert!(f.db.load_effect_receipt("caller-supplied-root-id").is_err());
    assert!(f.db.load_effect_history(None, 101).is_err());
}

#[test]
fn history_pagination_does_not_skip_retained_tombstones() {
    let f = Fixture::new();
    for _ in 0..3 {
        ticket(&f.journal, key(&EffectExecution::fresh().unwrap()))
            .cancel()
            .unwrap();
    }
    let first = f.db.load_effect_history(None, 2).unwrap();
    assert_eq!(first.items.len(), 2);
    let second =
        f.db.load_effect_history(first.next_before.as_deref(), 2)
            .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.next_before.is_none());
    assert!(first
        .items
        .iter()
        .all(|a| a.effect_id != second.items[0].effect_id));
}

#[test]
fn capacity_refuses_new_identities_while_expired_tombstones_still_deduplicate() {
    let f = Fixture::new();
    let original = key(&EffectExecution::fresh().unwrap());
    ticket(&f.journal, original.clone()).cancel().unwrap();
    let db = Connection::open(f.db.automation_path()).unwrap();
    db.execute(
        "UPDATE automation_external_effects SET accepted_at=1,updated_at=1",
        [],
    )
    .unwrap();
    let newcomer = key(&EffectExecution::fresh().unwrap());
    assert!(f
        .db
        .reserve_external_effect_test_limit(&newcomer, &f.journal.0.instance, 1, |_| panic!(
            "capacity must refuse before admission"
        ))
        .err()
        .unwrap()
        .contains("capacity reached"));
    let historical =
        f.db.reserve_external_effect_test_limit(&original, &f.journal.0.instance, 1, |_| {
            panic!("retained identity must not readmit")
        })
        .unwrap()
        .unwrap();
    assert!(historical.outcome_expired);
    assert_eq!(historical.state, EffectState::Unknown);
    assert!(f.db.load_effect_receipt(newcomer.id()).unwrap().is_none());
}

#[test]
fn named_slots_do_not_depend_on_match_order_and_enforce_bounded_depth() {
    let execution = EffectExecution::fresh().unwrap();
    let first = execution.named_child("event", "welcome").unwrap();
    let same = execution.named_child("event", "welcome").unwrap();
    let other_family = execution.named_child("action", "welcome").unwrap();
    let other_record = execution.named_child("event", "goodbye").unwrap();
    assert_eq!(key(&first).id(), key(&same).id());
    assert_ne!(key(&first).id(), key(&other_family).id());
    assert_ne!(key(&first).id(), key(&other_record).id());
    let mut deepest = execution;
    for _ in 0..MAX_SLOT_DEPTH {
        deepest = deepest.child(0).unwrap();
    }
    assert!(deepest.child(1).is_err());
    assert!(deepest.named_child("event", "welcome").is_err());
}

#[cfg(feature = "native-tiktok")]
#[test]
fn native_roots_survive_reconnect_and_restart_but_distinguish_source_room_and_type() {
    use tiktools_tiktok::events::{CanonicalLiveEvent, ChatEvent, EventMetadata, LikeEvent};
    let f = Fixture::new();
    let event = crate::NativeLiveEvent {
        base: CanonicalLiveEvent::Chat(ChatEvent::default()),
        metadata: EventMetadata {
            method: "WebcastTest".into(),
            msg_id: 42,
            is_history: false,
        },
        gift: None,
    };
    let context = crate::LiveContext {
        unique_id: "creator".into(),
        room_id: "room-1".into(),
        connection_id: "connection-a".into(),
        creator_verified: true,
    };
    let original = key(&EffectExecution::native(&event, &context).unwrap());
    ticket(&f.journal, original.clone())
        .dispatch(accept)
        .unwrap()
        .complete(EffectOutcome::Succeeded)
        .unwrap();
    let mut reconnect = context.clone();
    reconnect.connection_id = "connection-b".into();
    let mut replay = event.clone();
    replay.metadata.is_history = true;
    let successor = EffectJournal::open(f.db.clone()).unwrap();
    let replay_key = key(&EffectExecution::native(&replay, &reconnect).unwrap());
    assert_eq!(original.id(), replay_key.id());
    assert_eq!(
        retained(&successor, replay_key).state,
        EffectState::Succeeded
    );
    let mut other_room = context.clone();
    other_room.room_id = "room-2".into();
    assert_ne!(
        original.id(),
        key(&EffectExecution::native(&event, &other_room).unwrap()).id()
    );
    let mut other_source = context.clone();
    other_source.unique_id = "other-creator".into();
    assert_ne!(
        original.id(),
        key(&EffectExecution::native(&event, &other_source).unwrap()).id()
    );
    let mut other_type = event.clone();
    other_type.base = CanonicalLiveEvent::Like(LikeEvent::default());
    assert_ne!(
        original.id(),
        key(&EffectExecution::native(&other_type, &context).unwrap()).id()
    );
    let mut other_method = event.clone();
    other_method.metadata.method = "DifferentMethod".into();
    assert_ne!(
        original.id(),
        key(&EffectExecution::native(&other_method, &context).unwrap()).id()
    );
    let mut missing = event;
    missing.metadata.msg_id = 0;
    assert_ne!(
        key(&EffectExecution::native(&missing, &context).unwrap()).id(),
        key(&EffectExecution::native(&missing, &context).unwrap()).id()
    );
    let mut missing_room = context;
    missing_room.room_id.clear();
    assert_ne!(
        key(&EffectExecution::native(&replay, &missing_room).unwrap()).id(),
        key(&EffectExecution::native(&replay, &missing_room).unwrap()).id()
    );
}

fn crash_execution() -> EffectExecution {
    EffectExecution {
        root: "01".repeat(32),
        slots: vec![7],
    }
}

#[test]
#[ignore = "subprocess-only process-death fixture"]
fn crash_child() {
    let root = PathBuf::from(std::env::var_os("TIKTOOLS_EFFECT_CRASH_ROOT").expect("child root"));
    let stage = std::env::var("TIKTOOLS_EFFECT_CHILD_PHASE").unwrap();
    let db = Arc::new(DatabaseManager::new(paths(root.clone())));
    let journal = EffectJournal::open(db).unwrap();
    if stage == "before_accept" || stage == "after_accept" {
        std::env::set_var(
            "TIKTOOLS_EFFECT_CRASH_STAGE",
            if stage == "before_accept" {
                "before_commit"
            } else {
                "after_commit"
            },
        );
    }
    let accepted = ticket(&journal, key(&crash_execution()));
    if stage == "before_dispatch" || stage == "after_dispatch" {
        std::env::set_var(
            "TIKTOOLS_EFFECT_CRASH_STAGE",
            if stage == "before_dispatch" {
                "before_commit"
            } else {
                "after_commit"
            },
        );
    }
    let dispatched = accepted.dispatch(accept).unwrap();
    fs::write(root.join("sink-invoked"), b"one invocation").unwrap();
    if stage == "after_sink" {
        std::process::exit(88);
    }
    dispatched.complete(EffectOutcome::Succeeded).unwrap();
    if stage == "after_outcome" {
        std::process::exit(89);
    }
    panic!("process-death failpoint did not terminate");
}

#[test]
fn process_death_at_acceptance_handoff_sink_and_outcome_never_replays_uncertainty() {
    for (stage, code) in [
        ("before_accept", 86),
        ("after_accept", 87),
        ("before_dispatch", 86),
        ("after_dispatch", 87),
        ("after_sink", 88),
        ("after_outcome", 89),
    ] {
        let f = Fixture::new();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "services::effects::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("TIKTOOLS_EFFECT_CRASH_ROOT", &f.root)
            .env("TIKTOOLS_EFFECT_CHILD_PHASE", stage)
            .env_remove("TIKTOOLS_EFFECT_CRASH_STAGE")
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(code),
            "{stage}: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let reopened = EffectJournal::open(f.db.clone()).unwrap();
        let original = key(&crash_execution());
        if stage == "before_accept" {
            assert!(f.db.load_effect_receipt(original.id()).unwrap().is_none());
            ticket(&reopened, original).cancel().unwrap();
        } else {
            let receipt = retained(&reopened, original);
            assert_eq!(
                receipt.state,
                if stage == "after_outcome" {
                    EffectState::Succeeded
                } else {
                    EffectState::Unknown
                },
                "{stage}"
            );
        }
        assert_eq!(
            f.root.join("sink-invoked").exists(),
            matches!(stage, "after_sink" | "after_outcome"),
            "{stage}"
        );
    }
}
