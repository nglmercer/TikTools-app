use super::super::DatabaseManager;
use super::*;
use crate::paths::AppPaths;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn test_manager() -> DatabaseManager {
    let sequence = TEST_DB_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "tiktools-analytics-test-{}-{}",
        std::process::id(),
        sequence
    ));
    let _ = std::fs::remove_dir_all(&root);
    let data = root.join("data");
    std::fs::create_dir_all(&data).expect("test data dir");
    DatabaseManager::new(AppPaths {
        root: root.clone(),
        data,
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("builtin-plugins"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
    })
}

fn record_chats(unique_id: &str, chats: i64) -> AnalyticsEventRecord {
    AnalyticsEventRecord {
        unique_id: unique_id.to_owned(),
        chats,
        ..AnalyticsEventRecord::default()
    }
}

#[test]
fn utc_day_buckets_whole_days() {
    assert_eq!(utc_day(0), 0);
    assert_eq!(utc_day(86_399), 0);
    assert_eq!(utc_day(86_400), 1);
    assert_eq!(utc_day(1_751_616_000), 20_273);
}

#[test]
fn record_and_summarize_across_days() {
    let db = test_manager();
    db.ensure_analytics_schema().expect("schema");
    let monday = 1_751_616_000; // 2026-07-06 08:00 UTC
    let tuesday = monday + 86_400;

    db.record_analytics_event("creator", &record_chats("alice", 3), monday)
        .expect("record");
    db.record_analytics_event(
        "creator",
        &AnalyticsEventRecord {
            unique_id: "bob".to_owned(),
            gift_events: 2,
            gifts: 5,
            diamonds: 100,
            ..AnalyticsEventRecord::default()
        },
        monday,
    )
    .expect("record");
    db.record_analytics_event("creator", &record_chats("alice", 1), tuesday)
        .expect("record");
    db.record_analytics_event("other", &record_chats("mallory", 9), monday)
        .expect("record");
    db.record_analytics_viewers("creator", 120, monday)
        .expect("viewers");
    db.record_analytics_viewers("creator", 40, monday)
        .expect("viewers");

    let summary = db
        .analytics_summary("creator", utc_day(monday), utc_day(tuesday), 10, 0)
        .expect("summary");
    assert_eq!(summary.totals.chats, 4);
    assert_eq!(summary.totals.gift_events, 2);
    assert_eq!(summary.totals.gifts, 5);
    assert_eq!(summary.totals.diamonds, 100);
    assert_eq!(summary.totals.peak_viewers, 120);
    assert_eq!(summary.days.len(), 2);
    assert_eq!(summary.days[0].totals.chats, 3);
    assert_eq!(summary.days[1].totals.chats, 1);

    assert_eq!(summary.top_viewers.len(), 2);
    assert_eq!(summary.top_viewers[0].unique_id, "bob");
    assert_eq!(summary.top_viewers[0].interactions, 5);
    assert_eq!(summary.top_viewers[0].diamonds, 100);
    assert_eq!(summary.top_viewers[1].unique_id, "alice");
    assert_eq!(summary.top_viewers[1].interactions, 4);
}

#[test]
fn sessions_open_close_and_count() {
    let db = test_manager();
    db.ensure_analytics_schema().expect("schema");
    let start = 1_751_616_000;

    db.open_live_session("creator", Some("room-1"), start)
        .expect("open");
    // A second open closes the stale row instead of stacking opens.
    db.open_live_session("creator", Some("room-1"), start + 60)
        .expect("reopen");
    db.record_analytics_viewers("creator", 77, start + 61)
        .expect("viewers");
    db.close_live_sessions("creator", start + 3_600)
        .expect("close");

    let summary = db
        .analytics_summary("creator", utc_day(start), utc_day(start), 10, 0)
        .expect("summary");
    assert_eq!(summary.sessions, 2);
    assert_eq!(summary.totals.peak_viewers, 77);
}

#[test]
fn empty_creator_returns_zero_summary() {
    let db = test_manager();
    db.ensure_analytics_schema().expect("schema");
    let summary = db
        .analytics_summary("nobody", 20_275, 20_281, 10, 0)
        .expect("summary");
    assert_eq!(summary.totals, AnalyticsTotals::default());
    assert!(summary.days.is_empty());
    assert!(summary.top_viewers.is_empty());
    assert_eq!(summary.sessions, 0);
}

#[test]
fn utc_hour_buckets_whole_hours() {
    assert_eq!(utc_hour(0), 0);
    assert_eq!(utc_hour(3_599), 0);
    assert_eq!(utc_hour(3_600), 1);
    assert_eq!(utc_hour(1_751_616_000), 486_560);
}

#[test]
fn single_day_exposes_true_hourly_rows() {
    let db = test_manager();
    db.ensure_analytics_schema().expect("schema");
    // 2026-07-06 08:00 UTC and 21:30 UTC: same UTC day, distinct hours.
    let morning = 1_751_616_000;
    let evening = 1_751_616_000 - 8 * 3_600 + 21 * 3_600 + 1_800;
    assert_eq!(utc_day(morning), utc_day(evening));

    db.record_analytics_event("creator", &record_chats("alice", 3), morning)
        .expect("record");
    db.record_analytics_event("creator", &record_chats("bob", 5), evening)
        .expect("record");
    db.record_analytics_viewers("creator", 120, evening)
        .expect("viewers");

    let day = utc_day(morning);
    let summary = db
        .analytics_summary("creator", day, day, 10, 0)
        .expect("summary");
    assert_eq!(summary.totals.chats, 8);
    assert_eq!(summary.totals.peak_viewers, 120);
    assert_eq!(summary.hours.len(), 24);
    let at = |hour: usize| summary.hours[hour].totals.chats;
    assert_eq!(summary.hours[8].hour, 8);
    assert_eq!(at(8), 3);
    assert_eq!(at(21), 5);
    assert_eq!(at(9), 0);
    assert_eq!(summary.hours[21].totals.peak_viewers, 120);

    // Multi-day spans keep `hours` empty (daily trend chart territory).
    let wide = db
        .analytics_summary("creator", day - 1, day + 1, 10, 0)
        .expect("summary");
    assert!(wide.hours.is_empty());
    assert_eq!(wide.totals.chats, 8);
}

#[test]
fn offset_rebuckets_days_and_hours_into_local_frame() {
    let db = test_manager();
    db.ensure_analytics_schema().expect("schema");
    // Zone UTC+2 (offset 7200). 23:00 UTC day D is 01:00 local day D+1.
    let offset = 7_200;
    let day_d = 20_300;
    let late = day_d * 86_400 + 23 * 3_600; // 23:00 UTC, day D
    let early = (day_d + 1) * 86_400 + 3_600; // 01:00 UTC, day D+1

    db.record_analytics_event("creator", &record_chats("alice", 4), late)
        .expect("record");
    db.record_analytics_event("creator", &record_chats("bob", 6), early)
        .expect("record");

    // Local day D+1 owns both events (01:00 and 03:00 wall-clock).
    let summary = db
        .analytics_summary("creator", day_d + 1, day_d + 1, 10, offset)
        .expect("summary");
    assert_eq!(summary.totals.chats, 10);
    assert_eq!(summary.days.len(), 1);
    assert_eq!(summary.days[0].day, day_d + 1);
    assert_eq!(summary.days[0].totals.chats, 10);
    assert_eq!(summary.hours.len(), 24);
    assert_eq!(summary.hours[1].totals.chats, 4);
    assert_eq!(summary.hours[3].totals.chats, 6);

    // Local day D holds nothing: the 23:00 UTC event belongs to D+1, and
    // hourly coverage makes that zero exact rather than a fallback.
    let empty = db
        .analytics_summary("creator", day_d, day_d, 10, offset)
        .expect("summary");
    assert_eq!(empty.totals.chats, 0);
    assert!(empty.days.is_empty());
    assert_eq!(empty.hours.len(), 24);

    // Offset zero keeps the legacy UTC framing for the same data.
    let utc = db
        .analytics_summary("creator", day_d, day_d, 10, 0)
        .expect("summary");
    assert_eq!(utc.totals.chats, 4);
    assert_eq!(utc.hours[23].totals.chats, 4);
}

#[test]
fn offset_without_hourly_rows_falls_back_to_closest_utc_day() {
    let db = test_manager();
    db.ensure_analytics_schema().expect("schema");
    // Simulate a pre-hourly stream: daily row only, no hourly rows.
    let day_d = 20_300;
    {
        let connection = db.open(&db.analytics_path()).expect("open");
        connection
            .execute(
                "INSERT INTO analytics_daily (creator_unique_id, day, chats)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params!["creator", day_d, 7],
            )
            .expect("insert");
    }
    // No hourly rows anywhere: UTC fallback. Local day D at UTC+2 has
    // its noon at 10:00 UTC of the same label, so UTC day D resolves.
    let summary = db
        .analytics_summary("creator", day_d, day_d, 10, 7_200)
        .expect("summary");
    assert_eq!(summary.totals.chats, 7);
    assert!(summary.hours.is_empty());
    // The adjacent local day finds no closest-UTC row and stays empty.
    let neighbor = db
        .analytics_summary("creator", day_d + 1, day_d + 1, 10, 7_200)
        .expect("summary");
    assert_eq!(neighbor.totals.chats, 0);
    assert!(neighbor.hours.is_empty());
}
