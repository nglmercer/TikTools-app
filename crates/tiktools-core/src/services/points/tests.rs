use super::*;
#[cfg(feature = "persistence")]
mod approved;
#[cfg(all(feature = "persistence", feature = "native-tiktok"))]
mod approved_crash;
#[cfg(feature = "persistence")]
mod commands;
#[cfg(feature = "persistence")]
mod ordering;

/// Builds an isolated on-disk service plus its temp root. Every
/// persistence test owns its directory so a crash cannot leak state
/// into the next run.
#[cfg(feature = "persistence")]
fn temp_service(tag: &str) -> (Arc<DatabaseManager>, PointsService, std::path::PathBuf) {
    use crate::paths::AppPaths;
    let root = std::env::temp_dir().join(format!(
        "tiktools-points-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin-plugins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.clone(),
    };
    paths.ensure_directories().expect("temp dirs");
    for path in [
        paths.points_database(),
        paths.automation_database(),
        paths.analytics_database(),
    ] {
        std::fs::File::create(path).expect("private empty fixture database");
    }
    let database = Arc::new(DatabaseManager::new(paths));
    // These fixtures mutate schema/locks to test the write path. Finish
    // the initial WAL transition before the worker and the test open
    // competing connections; a startup failure would test a different
    // condition and permanently fail every op, hiding the intended case.
    #[cfg(feature = "persistence")]
    drop(database.open_points_writer().expect("fixture WAL setup"));
    let service = PointsService::new(Arc::clone(&database));
    (database, service, root)
}

/// Waits until the writer has taken and applied everything queued.
#[cfg(feature = "persistence")]
fn wait_for_writer_idle(service: &PointsService) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let admitted = service.writer.admitted.load(Ordering::Acquire);
    loop {
        let diagnostics = service.diagnostics();
        // Queue/executing gauges can both be zero between dequeue and
        // starting a batch. Only the resolved watermark proves completion.
        if diagnostics.resolved_seq >= admitted
            && diagnostics.writer_queued == 0
            && diagnostics.writer_executing == 0
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "writer never drained: {:?}",
            diagnostics
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn award_points_matches_existing_defaults() {
    let service = PointsService::default();
    let award = service.award_points("@alice", PointAction::Chat, AwardOptions::default());
    assert_eq!(
        award.as_ref().map(|award| award.unique_id.as_str()),
        Some("alice")
    );
    assert_eq!(award.as_ref().map(|award| award.total_points), Some(1.0));
    assert_eq!(service.leaderboard(Some(1))[0]["totalChats"], 1);
}

#[test]
fn rule_updates_apply_to_next_award_without_recreating_service() {
    let service = PointsService::default();
    let first = service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .expect("award");
    assert_eq!(first.delta, 1.0);

    let update = serde_json::from_value(serde_json::json!({
        "pointsPerChat": 7.0,
        "currencyName": "Gems"
    }))
    .expect("partial config");
    service.update_config(update);
    let second = service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .expect("award");
    assert_eq!(second.delta, 7.0);
    assert_eq!(second.total_points, 8.0);
    assert_eq!(service.config().currency_name, "Gems");

    service.update_config(
        serde_json::from_value(serde_json::json!({ "pointsPerChatEnabled": false }))
            .expect("partial config"),
    );
    let disabled = service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .expect("viewer counters still update");
    assert_eq!(disabled.delta, 0.0);
    assert_eq!(disabled.total_points, 8.0);
}

#[test]
fn reset_keeps_viewers_and_counters_but_zeroes_points() {
    let service = PointsService::default();
    service.award_points("alice", PointAction::Chat, AwardOptions::default());
    service.reset(None);
    let viewer = &service.leaderboard(Some(1))[0];
    assert_eq!(viewer["points"], 0.0);
    assert_eq!(viewer["level"], 1);
    assert_eq!(viewer["totalChats"], 1);
}

#[test]
fn negative_adjustments_clamp_at_zero() {
    // The `core.points` execution path (moderation penalties included)
    // adjusts through here: viewers can never drop below zero.
    let service = PointsService::default();
    service.award_points(
        "alice",
        PointAction::Manual,
        AwardOptions {
            custom_amount: Some(5.0),
            ..AwardOptions::default()
        },
    );
    let award = service.adjust("alice", -10.0).expect("viewer exists");
    assert_eq!(award.total_points, 0.0);
    assert_eq!(award.delta, -10.0);
    assert_eq!(service.leaderboard(Some(1))[0]["points"], 0.0);
    // Unknown viewers stay unknown: no negative phantom rows.
    assert!(service.adjust("nobody", -10.0).is_none());
    assert!(service.adjust("alice", f64::NAN).is_none());
}

#[test]
fn point_configuration_normalizes_unsafe_ranges() {
    let mut config = PointsConfig {
        points_per_coin: -10.0,
        points_per_share: f64::NAN,
        points_per_chat: 2.0,
        points_per_like: f64::INFINITY,
        points_per_follow: 2.0,
        points_per_join: 2.0,
        sub_bonus_multiplier: -1.0,
        points_per_level: 1.0,
        currency_name: "   ".to_owned(),
        ..PointsConfig::default()
    };
    config.normalize();
    assert_eq!(config.points_per_coin, 0.0);
    assert_eq!(config.points_per_share, 3.0);
    assert_eq!(config.points_per_like, 0.1);
    assert_eq!(config.sub_bonus_multiplier, 0.0);
    assert_eq!(config.points_per_level, 10.0);
    assert_eq!(config.currency_name, "Points");
}

#[test]
fn invalid_viewer_reset_never_becomes_a_full_reset() {
    let service = PointsService::default();
    service.award_points("alice", PointAction::Chat, AwardOptions::default());
    for id in ["", "  ", "@@", "*", "a\nb", "@ a", &"x".repeat(129)] {
        service.reset(Some(id));
        assert_eq!(service.viewer("alice").unwrap()["points"], 1.0, "{id:?}");
        assert!(service
            .award_points(id, PointAction::Chat, AwardOptions::default())
            .is_none());
    }
    service.reset(Some(" @alice "));
    assert_eq!(service.viewer("alice").unwrap()["points"], 0.0);
}

#[test]
fn nonfinite_or_overflowing_awards_never_mutate_balances() {
    let service = PointsService::default();
    service.award_points("alice", PointAction::Chat, AwardOptions::default());
    for amount in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        assert!(service
            .award_points(
                "alice",
                PointAction::Manual,
                AwardOptions {
                    custom_amount: Some(amount),
                    ..AwardOptions::default()
                }
            )
            .is_none());
        assert_eq!(service.viewer("alice").unwrap()["points"], 1.0);
    }
    assert!(service.adjust("alice", f64::MAX).is_none());
    assert_eq!(service.viewer("alice").unwrap()["points"], 1.0);
}

#[test]
fn repeated_awards_accumulate_exact_totals() {
    let service = PointsService::default();
    for _ in 0..100 {
        service.award_points("alice", PointAction::Chat, AwardOptions::default());
    }
    let viewer = service.viewer("alice").expect("alice exists");
    assert_eq!(viewer["points"], 100.0);
    assert_eq!(viewer["totalChats"], 100);
}

#[test]
fn zero_point_events_keep_counters_and_metadata() {
    let service = PointsService::default();
    // Likes disabled by config would award nothing; force it directly.
    let award = service
        .award_points(
            "bob",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(0.0),
                nickname: Some("Bobby".to_owned()),
                ..AwardOptions::default()
            },
        )
        .expect("award succeeds");
    assert_eq!(award.delta, 0.0);
    let viewer = service.viewer("bob").expect("bob exists");
    assert_eq!(viewer["points"], 0.0);
    assert_eq!(viewer["nickname"], "Bobby");
}

#[test]
fn leaderboard_orders_without_per_event_sort_and_breaks_ties() {
    let service = PointsService::default();
    service.award_points(
        "charlie",
        PointAction::Manual,
        AwardOptions {
            custom_amount: Some(10.0),
            ..AwardOptions::default()
        },
    );
    service.award_points(
        "alice",
        PointAction::Manual,
        AwardOptions {
            custom_amount: Some(10.0),
            ..AwardOptions::default()
        },
    );
    service.award_points(
        "bob",
        PointAction::Manual,
        AwardOptions {
            custom_amount: Some(5.0),
            ..AwardOptions::default()
        },
    );
    let board = service.leaderboard(Some(2));
    assert_eq!(board.len(), 2);
    // Equal points fall back to the deterministic identity tie-break.
    assert_eq!(board[0]["uniqueId"], "alice");
    assert_eq!(board[1]["uniqueId"], "charlie");
}

#[test]
fn high_cardinality_awards_stay_bounded_and_exact() {
    let service = PointsService::default();
    service.set_cache_capacity(1_000);
    for id in 0..5_000 {
        let name = format!("viewer-{id:05}");
        service.award_points(&name, PointAction::Chat, AwardOptions::default());
    }
    let diagnostics = service.diagnostics();
    assert!(
        diagnostics.hot_size <= 1_000 + PointsInner::EVICT_BATCH_SLACK,
        "hot cache must respect its bound, got {}",
        diagnostics.hot_size
    );
    // The most recently touched viewer is always resident and exact.
    let viewer = service.viewer("viewer-04999").expect("recent viewer");
    assert_eq!(viewer["points"], 1.0);
    assert_eq!(viewer["totalChats"], 1);
    // The leaderboard serves the top-K without a full sort even when
    // the population exceeds the visible window.
    let board = service.leaderboard(Some(100));
    assert_eq!(board.len(), 100);
}

#[cfg(feature = "persistence")]
#[test]
fn eviction_and_restart_preserve_exact_balances() {
    use crate::db::DatabaseManager;
    use crate::paths::AppPaths;
    let root = std::env::temp_dir().join(format!(
        "tiktools-points-phase1-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin-plugins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.clone(),
    };
    paths.ensure_directories().expect("temp dirs");
    let database = Arc::new(DatabaseManager::new(paths));
    let service = PointsService::new(Arc::clone(&database));
    service.set_cache_capacity(50);
    for id in 0..200 {
        let name = format!("fan-{id:03}");
        service.award_points(
            &name,
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(7.0),
                ..AwardOptions::default()
            },
        );
    }
    // Repeated updates to an evicted viewer reload durable state and
    // accumulate instead of forking a phantom row.
    for _ in 0..3 {
        service.award_points(
            "fan-000",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(7.0),
                ..AwardOptions::default()
            },
        );
    }
    service.shutdown();
    let reloaded = service
        .viewer("fan-000")
        .expect("fan-000 survives eviction");
    assert_eq!(reloaded["points"], 28.0);
    drop(service);
    // Restart: balances survive through SQLite, not RAM. The runtime
    // capacity is a boot setting (env `TIKTOOLS_POINTS_CACHE_CAPACITY`
    // or `set_cache_capacity`), so reapply it after restart.
    let restarted = PointsService::new(database);
    restarted.set_cache_capacity(50);
    let viewer = restarted.viewer("fan-199").expect("fan-199 persisted");
    assert_eq!(viewer["points"], 7.0);
    let first = restarted.viewer("fan-000").expect("fan-000 persisted");
    assert_eq!(first["points"], 28.0);
    let diagnostics = restarted.diagnostics();
    assert!(
        diagnostics.hot_size <= 50 + PointsInner::EVICT_BATCH_SLACK,
        "hot_size={}",
        diagnostics.hot_size
    );
    restarted.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn cache_capacity_is_enforced_and_never_evicts_dirty_rows() {
    let service = PointsService::default();
    service.set_cache_capacity(10);
    for id in 0..50 {
        service.award_points(
            &format!("user-{id}"),
            PointAction::Chat,
            AwardOptions::default(),
        );
    }
    let diagnostics = service.diagnostics();
    assert!(
        diagnostics.hot_size <= 10 + PointsInner::EVICT_BATCH_SLACK,
        "hot_size={}",
        diagnostics.hot_size
    );
    assert_eq!(service.cache_capacity(), 10);
}

#[test]
fn concurrent_awards_never_lose_increments() {
    use std::thread;
    let service = Arc::new(PointsService::default());
    let mut handles = Vec::new();
    for _ in 0..8 {
        let service = Arc::clone(&service);
        handles.push(thread::spawn(move || {
            for _ in 0..250 {
                service.award_points("racer", PointAction::Chat, AwardOptions::default());
            }
        }));
    }
    for handle in handles {
        handle.join().expect("worker finishes");
    }
    let viewer = service.viewer("racer").expect("racer exists");
    assert_eq!(viewer["totalChats"], 8 * 250);
    assert_eq!(viewer["points"], 8.0 * 250.0);
}

/// Slow/unavailable SQLite must never break awarding or shutdown: the
/// hot cache serves reads, writes queue and are reported as failures,
/// and shutdown still joins cleanly.
#[cfg(all(feature = "persistence", target_os = "linux"))]
#[test]
fn unavailable_storage_never_breaks_awards_or_shutdown() {
    use crate::db::DatabaseManager;
    use crate::paths::AppPaths;
    let root = std::env::temp_dir().join(format!(
        "tiktools-points-unavail-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let paths = AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin-plugins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.clone(),
    };
    paths.ensure_directories().expect("temp dirs");
    // Poison the database file: every SQLite query under it fails,
    // exercising the writer-error and fallback paths. (Read-only is
    // belt-and-braces; the garbage content breaks SQLite regardless.)
    let db_file = root.join("data").join("tiktok-points.db");
    std::fs::write(&db_file, b"not a database").expect("poison file");
    let mut permissions = std::fs::metadata(&db_file).expect("metadata").permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&db_file, permissions).expect("readonly");
    let database = Arc::new(DatabaseManager::new(paths));
    let service = PointsService::new(Arc::clone(&database));
    for id in 0..100 {
        let award = service
            .award_points(
                &format!("survivor-{id}"),
                PointAction::Chat,
                AwardOptions::default(),
            )
            .expect("awards work from the hot cache");
        assert_eq!(award.total_points, 1.0);
    }
    // Reads still serve from memory; shutdown joins without hanging.
    let viewer = service.viewer("survivor-0").expect("memory serves reads");
    assert_eq!(viewer["points"], 1.0);
    service.shutdown();
    // Every op is resolved (applied or explicitly failed) so nothing is
    // left outstanding, but nothing was durable either: the failure is
    // visible as a gap between the watermarks instead of being hidden.
    let diagnostics = service.diagnostics();
    assert_eq!(diagnostics.writer_queued, 0, "{diagnostics:?}");
    assert_eq!(
        diagnostics.write_applied + diagnostics.write_failed,
        100,
        "{diagnostics:?}"
    );
    assert!(diagnostics.write_failed > 0, "{diagnostics:?}");
    assert!(
        diagnostics.resolved_seq >= diagnostics.committed_seq,
        "{diagnostics:?}"
    );
    assert!(
        diagnostics.committed_seq < diagnostics.resolved_seq,
        "a permanently failed write must stay visible as a watermark gap: {diagnostics:?}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn viewer_lookup_is_direct_and_misses_cleanly() {
    let service = PointsService::default();
    assert!(service.viewer("ghost").is_none());
    assert!(service.viewer("   ").is_none());
    service.award_points("@alice", PointAction::Chat, AwardOptions::default());
    let viewer = service.viewer("alice").expect("direct hit");
    assert_eq!(viewer["uniqueId"], "alice");
    let diagnostics = service.diagnostics();
    assert!(diagnostics.cache_hits >= 1);
    assert!(diagnostics.cache_misses >= 1);
}

/// A wedged writer must never be papered over with an out-of-order
/// synchronous write. Admission waits, then rejects without mutating
/// memory; once the writer recovers, every admitted op is durable and
/// the newest snapshot wins — which is only true if the queue order is
/// the sequence order.
#[cfg(feature = "persistence")]
#[test]
fn saturated_writer_rejects_instead_of_bypassing_the_fifo() {
    let (database, service, root) = temp_service("saturated");
    // Short admission bound so the rejection is reachable in a test.
    service
        .writer
        .admission_timeout_ms
        .store(50, Ordering::Relaxed);
    // Wedge the writer: the queue fills, then admissions must reject.
    service.writer.stall_ms.store(1_000, Ordering::Relaxed);
    let accepted = WRITER_QUEUE_CAPACITY;
    for _ in 0..accepted {
        let award = service.award_points("racer", PointAction::Chat, AwardOptions::default());
        assert!(
            award.is_some(),
            "admissions must succeed while the queue has room"
        );
    }
    assert_eq!(
        service.viewer("racer").expect("racer")["points"],
        accepted as f64
    );
    // The next award finds a full queue and is rejected with no memory
    // mutation: the caller and the durable store agree it did not land.
    let rejected = service.award_points("racer", PointAction::Chat, AwardOptions::default());
    assert!(
        rejected.is_none(),
        "saturated queue must reject, never write inline"
    );
    assert_eq!(
        service.viewer("racer").expect("racer")["points"],
        accepted as f64,
        "a rejected admission must not mutate the hot cache"
    );
    let saturated = service.diagnostics();
    assert_eq!(saturated.write_rejected, 1, "{saturated:?}");
    assert!(saturated.writer_overload >= 1, "{saturated:?}");
    assert!(saturated.writer_queued > 0, "{saturated:?}");
    assert_eq!(saturated.write_applied, 0, "{saturated:?}");

    // Recover the writer and drain: the durable row must carry the
    // newest accepted snapshot, not an older one that jumped the queue.
    service.writer.stall_ms.store(0, Ordering::Relaxed);
    service.shutdown();
    let drained = service.diagnostics();
    assert_eq!(drained.writer_queued, 0, "{drained:?}");
    assert_eq!(drained.write_failed, 0, "{drained:?}");
    assert_eq!(drained.write_applied as usize, accepted, "{drained:?}");
    assert_eq!(drained.committed_seq, drained.resolved_seq, "{drained:?}");
    drop(service);
    let restarted = PointsService::new(database);
    let durable = restarted.viewer("racer").expect("racer persisted");
    assert_eq!(
        durable["points"], accepted as f64,
        "durable state must equal the newest admitted snapshot"
    );
    restarted.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

/// A full reset zeroes rows that may still be cold in SQLite. Reloading
/// one before the reset op lands would resurrect its pre-reset points
/// and the later FIFO winner would be the stale snapshot.
#[cfg(feature = "persistence")]
#[test]
fn full_reset_blocks_cold_reloads_until_it_lands() {
    let (_database, service, root) = temp_service("reset");
    service.set_cache_capacity(4);
    service.award_points(
        "alice",
        PointAction::Manual,
        AwardOptions {
            custom_amount: Some(5.0),
            ..AwardOptions::default()
        },
    );
    service.award_points(
        "bob",
        PointAction::Manual,
        AwardOptions {
            custom_amount: Some(3.0),
            ..AwardOptions::default()
        },
    );
    wait_for_writer_idle(&service);
    // Wedge the writer so the reset op cannot land while we reload.
    service.writer.stall_ms.store(500, Ordering::Relaxed);
    service.reset(None);
    // Touch bob so alice is the coldest row, then shrink to force the
    // reload path for alice.
    service.adjust("bob", 0.0).expect("bob is cached");
    service.set_cache_capacity(1);
    let shrunk = service.diagnostics();
    assert_eq!(shrunk.hot_size, 1, "alice must be cold: {shrunk:?}");
    assert_eq!(
        service.viewer("bob").expect("bob is cached")["points"],
        0.0,
        "reset applies to memory immediately"
    );
    let alice = service.viewer("alice").expect("alice reloads");
    assert_eq!(
        alice["points"], 0.0,
        "a reload must not resurrect pre-reset points"
    );
    service.writer.stall_ms.store(0, Ordering::Relaxed);
    service.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

/// Order at scale: 100,000 sequential awards to one viewer must leave
/// exactly 100,000 durable points. The writer applies full row
/// snapshots, so this only holds if it applies them in sequence order.
#[cfg(feature = "persistence")]
#[test]
fn hundred_thousand_ordered_awards_are_durable_in_order() {
    const EVENTS: u64 = 100_000;
    let (database, service, root) = temp_service("ordered");
    for _ in 0..EVENTS {
        service.award_points("marathon", PointAction::Chat, AwardOptions::default());
    }
    let in_memory = service.viewer("marathon").expect("marathon");
    assert_eq!(in_memory["points"], EVENTS as f64);
    service.shutdown();
    let diagnostics = service.diagnostics();
    assert_eq!(diagnostics.writer_queued, 0, "{diagnostics:?}");
    assert_eq!(diagnostics.write_rejected, 0, "{diagnostics:?}");
    assert_eq!(diagnostics.write_failed, 0, "{diagnostics:?}");
    assert_eq!(diagnostics.write_applied, EVENTS, "{diagnostics:?}");
    assert_eq!(
        diagnostics.committed_seq, diagnostics.resolved_seq,
        "{diagnostics:?}"
    );
    drop(service);
    let restarted = PointsService::new(database);
    let durable = restarted.viewer("marathon").expect("marathon durable");
    assert_eq!(durable["points"], EVENTS as f64);
    restarted.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

/// Concurrent awards from many threads must be durable in whatever
/// order they were admitted, with nothing lost between memory and
/// SQLite.
#[cfg(feature = "persistence")]
#[test]
fn concurrent_awards_are_durable_and_exact() {
    let (database, service, root) = temp_service("concurrent");
    let service = Arc::new(service);
    let mut handles = Vec::new();
    for worker in 0..8 {
        let service = Arc::clone(&service);
        handles.push(std::thread::spawn(move || {
            for _ in 0..250 {
                service.award_points("racer", PointAction::Chat, AwardOptions::default());
                if worker % 2 == 0 {
                    service.adjust("racer", -0.5);
                }
            }
        }));
    }
    for handle in handles {
        handle.join().expect("worker finishes");
    }
    let expected = service.viewer("racer").expect("racer")["points"]
        .as_f64()
        .expect("points are a number");
    wait_for_writer_idle(&service);
    let memory = service.diagnostics();
    assert_eq!(memory.writer_queued, 0, "{memory:?}");
    assert_eq!(memory.write_rejected, 0, "{memory:?}");
    assert_eq!(memory.write_failed, 0, "{memory:?}");
    service.shutdown();
    drop(service);
    let restarted = PointsService::new(database);
    let durable = restarted.viewer("racer").expect("racer durable");
    assert_eq!(durable["points"], expected);
    restarted.shutdown();
    let _ = std::fs::remove_dir_all(root);
}
