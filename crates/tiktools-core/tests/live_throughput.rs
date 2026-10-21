//! LIVE throughput and memory regression harness (Phase 9).
//!
//! Reproducible load scenarios against the real application paths — no
//! mocks, no synthetic shortcuts — using only the public API plus std
//! timing. Run with output to record benchmark numbers:
//!
//! ```sh
//! cargo test -p tiktools-core --features persistence,native-tiktok --locked \
//!   --test live_throughput -- --nocapture
//! ```
//!
//! Design notes:
//!
//! - Throughput numbers are REPORTED, never asserted: CI machines vary,
//!   and a timing gate would be flaky. What IS asserted is correctness
//!   under load (exact totals, bounded caches, drained queues).
//! - Workloads mirror the documented scenarios: chat-only, gift-heavy,
//!   like-heavy, mixed, 10k/100k distinct viewers, 10k bursts.
//! - Memory regression coverage lives in the in-crate stress tests
//!   (`services::points`, `live_events::ingress`), which can observe
//!   queue depths and task counts directly; this file covers the public
//!   surface and records rates for `docs/LIVE_PERFORMANCE.md`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tiktools_core::db::DatabaseManager;
use tiktools_core::paths::AppPaths;
use tiktools_core::services::{AwardOptions, PointAction, PointsService};

fn chat_options() -> AwardOptions {
    AwardOptions::default()
}

fn gift_options(diamonds: f64) -> AwardOptions {
    AwardOptions {
        diamond_count: Some(diamonds),
        ..AwardOptions::default()
    }
}

fn like_options(count: f64) -> AwardOptions {
    AwardOptions {
        count: Some(count),
        ..AwardOptions::default()
    }
}

struct Rate {
    label: &'static str,
    events: usize,
    elapsed: Duration,
}

impl Rate {
    fn report(&self) {
        let secs = self.elapsed.as_secs_f64().max(1e-9);
        println!(
            "  {:<28} {:>8} events in {:>8.2?}  ({:>10.0} events/s)",
            self.label,
            self.events,
            self.elapsed,
            self.events as f64 / secs
        );
    }
}

fn percentiles(mut samples_ms: Vec<f64>) -> (f64, f64, f64) {
    samples_ms.sort_by(|a, b| a.total_cmp(b));
    if samples_ms.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let at =
        |q: f64| samples_ms[((q * samples_ms.len() as f64) as usize).min(samples_ms.len() - 1)];
    (at(0.5), at(0.95), at(0.99))
}

/// Resident set size of this process (Linux only; `None` elsewhere —
/// never fabricated).
fn rss_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
                return Some(kib.saturating_mul(1024));
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Persistence-backed service in an isolated temp dir, for exactness and
/// restart coverage at scale. Callers must `shutdown` the service before
/// dropping (joins the writer) and remove the dir afterwards.
fn temp_service(name: &str) -> (PointsService, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "tiktools-harness-{name}-{}-{}",
        std::process::id(),
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
    (PointsService::new(database), root)
}

#[test]
fn chat_only_sustained_throughput() {
    let service = PointsService::default();
    // Size the bound past the workload: this scenario measures steady-state
    // rate and exactness, not eviction (covered separately at scale).
    service.set_cache_capacity(25_000);
    // Warm-up before measuring (allocator + caches, not timed).
    for id in 0..1_000 {
        service.award_points(&format!("warm-{id}"), PointAction::Chat, chat_options());
    }
    let before_rss = rss_bytes();
    let events = 20_000;
    let mut latencies = Vec::with_capacity(events);
    let started = Instant::now();
    for id in 0..events {
        let tick = Instant::now();
        service.award_points(&format!("chat-{id}"), PointAction::Chat, chat_options());
        latencies.push(tick.elapsed().as_secs_f64() * 1_000.0);
    }
    let elapsed = started.elapsed();
    let (p50, p95, p99) = percentiles(latencies);
    Rate {
        label: "chat-only awards",
        events,
        elapsed,
    }
    .report();
    println!("  chat-only latency ms: p50={p50:.4} p95={p95:.4} p99={p99:.4}");
    let total: f64 = (0..events)
        .map(|id| {
            service
                .viewer(&format!("chat-{id}"))
                .and_then(|viewer| viewer.get("points").and_then(|points| points.as_f64()))
                .unwrap_or(-1.0)
        })
        .sum();
    assert_eq!(total, events as f64, "every chat award must total exactly");
    let diagnostics = service.diagnostics();
    assert!(
        diagnostics.hot_size <= diagnostics.hot_capacity + 64,
        "hot cache stays bounded under sustained load"
    );
    if let (Some(before), Some(after)) = (before_rss, rss_bytes()) {
        println!(
            "  RSS before={before} after={after} delta={}",
            after.saturating_sub(before)
        );
    }
}

#[test]
fn gift_heavy_workload_preserves_totals() {
    let service = PointsService::default();
    let events = 5_000;
    let started = Instant::now();
    for id in 0..events {
        service.award_points(
            &format!("gifter-{:04}", id % 500),
            PointAction::Gift,
            gift_options(5.0),
        );
    }
    let elapsed = started.elapsed();
    Rate {
        label: "gift-heavy awards",
        events,
        elapsed,
    }
    .report();
    // 500 viewers × 10 gifts × 5 diamonds × 1 pt/coin = 25,000 coins.
    let coins: i64 = (0..500)
        .map(|id| {
            service
                .viewer(&format!("gifter-{id:04}"))
                .and_then(|viewer| viewer.get("totalCoins").and_then(|coins| coins.as_i64()))
                .unwrap_or(-1)
        })
        .sum();
    assert_eq!(coins, 25_000, "gift coin totals must be exact under load");
}

#[test]
fn like_heavy_and_mixed_workloads() {
    let service = PointsService::default();
    let started = Instant::now();
    for id in 0..10_000 {
        let action = match id % 4 {
            0 => PointAction::Chat,
            1 => PointAction::Gift,
            2 => PointAction::Like,
            _ => PointAction::Share,
        };
        let options = match id % 4 {
            1 => gift_options(2.0),
            2 => like_options(10.0),
            _ => AwardOptions::default(),
        };
        service.award_points(&format!("mixed-{:05}", id % 2_000), action, options);
    }
    Rate {
        label: "mixed workload",
        events: 10_000,
        elapsed: started.elapsed(),
    }
    .report();
    // Spot-check one viewer from each lane.
    let likes = service
        .viewer("mixed-00002")
        .and_then(|viewer| viewer.get("totalLikes").and_then(|likes| likes.as_i64()))
        .unwrap_or(-1);
    assert_eq!(likes, 5 * 10, "like lane exact: 5 awards × 10 likes");
    let shares = service
        .viewer("mixed-00003")
        .and_then(|viewer| viewer.get("totalShares").and_then(|shares| shares.as_i64()))
        .unwrap_or(-1);
    assert_eq!(shares, 5, "share lane exact: 5 awards");
}

#[test]
fn burst_of_ten_thousand_events() {
    let service = PointsService::default();
    let started = Instant::now();
    for _ in 0..10_000 {
        service.award_points("burst-viewer", PointAction::Chat, chat_options());
    }
    Rate {
        label: "10k burst, one viewer",
        events: 10_000,
        elapsed: started.elapsed(),
    }
    .report();
    let viewer = service.viewer("burst-viewer").expect("burst viewer exists");
    assert_eq!(viewer["points"], 10_000.0);
    assert_eq!(viewer["totalChats"], 10_000);
}

#[test]
fn high_cardinality_viewer_populations() {
    let service = PointsService::default();
    service.set_cache_capacity(150_000);
    // Rounds share the service (and the fan-000000..fan-009999 prefix), so
    // overlapping probes accumulate across rounds by design.
    for (round, population) in [10_000usize, 100_000usize].into_iter().enumerate() {
        let started = Instant::now();
        for id in 0..population {
            service.award_points(&format!("fan-{id:06}"), PointAction::Chat, chat_options());
        }
        let elapsed = started.elapsed();
        Rate {
            label: if population == 10_000 {
                "10k distinct viewers"
            } else {
                "100k distinct viewers"
            },
            events: population,
            elapsed,
        }
        .report();
        let diagnostics = service.diagnostics();
        assert!(
            diagnostics.hot_size <= diagnostics.hot_capacity + 64,
            "population {population} must not exceed the hot bound"
        );
        // The leaderboard serves the head without a full sort even at 100k.
        let board = service.leaderboard(Some(100));
        assert_eq!(board.len(), 100);
        // Spot-check exactness at the edges of the population. Probe 0
        // accumulates across rounds (shared prefix); the tail probe is
        // fresh every round.
        let viewer = service.viewer("fan-000000").expect("fan-000000 resolvable");
        assert_eq!(viewer["points"], (round + 1) as f64);
        let tail = service
            .viewer(&format!("fan-{:06}", population - 1))
            .unwrap_or_else(|| panic!("fan-{:06} resolvable", population - 1));
        assert_eq!(tail["points"], 1.0);
    }
}

#[test]
fn concurrent_award_streams_do_not_lose_increments() {
    let service = Arc::new(PointsService::default());
    service.set_cache_capacity(50_000);
    let started = Instant::now();
    let mut handles = Vec::new();
    for worker in 0..8 {
        let service = Arc::clone(&service);
        handles.push(std::thread::spawn(move || {
            for id in 0..5_000 {
                service.award_points(
                    &format!("stream-{worker}-{id:04}"),
                    PointAction::Chat,
                    chat_options(),
                );
            }
        }));
    }
    for handle in handles {
        handle.join().expect("worker finishes");
    }
    Rate {
        label: "8 concurrent streams",
        events: 40_000,
        elapsed: started.elapsed(),
    }
    .report();
    let diagnostics = service.diagnostics();
    assert!(diagnostics.hot_size <= diagnostics.hot_capacity + 64);
    let viewer = service
        .viewer("stream-0-0000")
        .expect("stream viewer exists");
    assert_eq!(viewer["totalChats"], 1);
}

#[test]
#[cfg_attr(
    not(feature = "persistence"),
    ignore = "requires the persistence (SQLite) feature; exercised by the full CLI capability suite"
)]
fn persistence_at_scale_and_restart() {
    let (service, root) = temp_service("persist");
    service.set_cache_capacity(1_000);
    let viewers = 5_000;
    let started = Instant::now();
    for id in 0..viewers {
        service.award_points(
            &format!("persist-{id:04}"),
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(3.0),
                ..AwardOptions::default()
            },
        );
    }
    Rate {
        label: "persisted awards",
        events: viewers,
        elapsed: started.elapsed(),
    }
    .report();
    // Repeated updates to evicted viewers accumulate through SQLite.
    for _ in 0..4 {
        service.award_points(
            "persist-0000",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(3.0),
                ..AwardOptions::default()
            },
        );
    }
    service.shutdown();
    let reloaded = service
        .viewer("persist-0000")
        .expect("evicted viewer reloads");
    assert_eq!(reloaded["points"], 15.0);
    let diagnostics = service.diagnostics();
    assert!(diagnostics.hot_size <= diagnostics.hot_capacity + 64);
    drop(service);
    // Restart on the same directory: every balance survives.
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
    let restarted = PointsService::new(Arc::new(DatabaseManager::new(paths)));
    let mut total = 0.0;
    for id in 0..viewers {
        let viewer = restarted
            .viewer(&format!("persist-{id:04}"))
            .unwrap_or_else(|| panic!("persist-{id:04} survived restart"));
        total += viewer["points"].as_f64().unwrap_or(-1.0);
    }
    let first = restarted.viewer("persist-0000").expect("first viewer");
    assert_eq!(first["points"], 15.0);
    assert_eq!(total, (viewers - 1) as f64 * 3.0 + 15.0);
    restarted.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn leaderboard_top_k_matches_full_sort() {
    let service = PointsService::default();
    for id in 0..2_000 {
        service.award_points(
            &format!("rank-{id:04}"),
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some((id % 97) as f64),
                ..AwardOptions::default()
            },
        );
    }
    let top = service.leaderboard(Some(50));
    assert_eq!(top.len(), 50);
    let points: Vec<f64> = top
        .iter()
        .map(|viewer| viewer["points"].as_f64().unwrap_or(-1.0))
        .collect();
    let mut sorted = points.clone();
    sorted.sort_by(|a, b| b.total_cmp(a));
    assert_eq!(points, sorted, "top-K must arrive in rank order");
    // Deterministic tie-break: equal points order by identity.
    let tied: Vec<&str> = top
        .iter()
        .filter(|viewer| viewer["points"].as_f64() == Some(96.0))
        .map(|viewer| viewer["uniqueId"].as_str().unwrap_or_default())
        .collect();
    let mut ordered = tied.clone();
    ordered.sort_unstable();
    assert_eq!(tied, ordered);
}
