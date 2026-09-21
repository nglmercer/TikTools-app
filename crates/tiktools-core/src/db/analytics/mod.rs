//! Aggregate-only LIVE analytics in a dedicated SQLite file.
//!
//! Counters and daily buckets only: chat text and other raw payloads are never
//! persisted here. Weeks and months are SQL aggregations of daily rows, and
//! event packets are tracked separately from the quantities they carry
//! (`like_events` vs `likes`, `gift_events` vs `gifts`).

mod aggregate;
mod query;
mod schema;
#[cfg(all(test, feature = "persistence"))]
mod tests;
mod write;

pub use aggregate::{utc_day, utc_hour};

use serde::{Deserialize, Serialize};

pub const ANALYTICS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS live_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  creator_unique_id TEXT NOT NULL,
  room_id TEXT,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  peak_viewers INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_live_sessions_creator ON live_sessions (creator_unique_id, started_at);

CREATE TABLE IF NOT EXISTS analytics_daily (
  creator_unique_id TEXT NOT NULL,
  day INTEGER NOT NULL,

  chats INTEGER NOT NULL DEFAULT 0,

  gift_events INTEGER NOT NULL DEFAULT 0,
  gifts INTEGER NOT NULL DEFAULT 0,
  diamonds INTEGER NOT NULL DEFAULT 0,

  like_events INTEGER NOT NULL DEFAULT 0,
  likes INTEGER NOT NULL DEFAULT 0,

  joins INTEGER NOT NULL DEFAULT 0,
  follows INTEGER NOT NULL DEFAULT 0,
  shares INTEGER NOT NULL DEFAULT 0,

  peak_viewers INTEGER NOT NULL DEFAULT 0,

  PRIMARY KEY (creator_unique_id, day)
);

CREATE TABLE IF NOT EXISTS viewer_daily (
  creator_unique_id TEXT NOT NULL,
  day INTEGER NOT NULL,
  unique_id TEXT NOT NULL,

  chats INTEGER NOT NULL DEFAULT 0,
  gifts INTEGER NOT NULL DEFAULT 0,
  diamonds INTEGER NOT NULL DEFAULT 0,
  likes INTEGER NOT NULL DEFAULT 0,
  shares INTEGER NOT NULL DEFAULT 0,

  first_seen INTEGER,
  last_seen INTEGER,

  PRIMARY KEY (
    creator_unique_id,
    day,
    unique_id
  )
);
CREATE INDEX IF NOT EXISTS idx_viewer_daily_range ON viewer_daily (creator_unique_id, day);

-- Finer-grained counters backing timezone-aware summaries. A UTC day row
-- cannot be split into system-local days, so event quantities are also
-- accumulated per UTC hour and re-bucketed at query time with the caller's
-- zone offset. Created lazily by `ensure_analytics_schema`, hence empty for
-- streams recorded before this table existed (summaries fall back then).
CREATE TABLE IF NOT EXISTS analytics_hourly (
  creator_unique_id TEXT NOT NULL,
  hour INTEGER NOT NULL,

  chats INTEGER NOT NULL DEFAULT 0,

  gift_events INTEGER NOT NULL DEFAULT 0,
  gifts INTEGER NOT NULL DEFAULT 0,
  diamonds INTEGER NOT NULL DEFAULT 0,

  like_events INTEGER NOT NULL DEFAULT 0,
  likes INTEGER NOT NULL DEFAULT 0,

  joins INTEGER NOT NULL DEFAULT 0,
  follows INTEGER NOT NULL DEFAULT 0,
  shares INTEGER NOT NULL DEFAULT 0,

  peak_viewers INTEGER NOT NULL DEFAULT 0,

  PRIMARY KEY (creator_unique_id, hour)
);
CREATE INDEX IF NOT EXISTS idx_analytics_hourly_creator ON analytics_hourly (creator_unique_id, hour);
"#;

/// One normalized analytics increment. All quantities, no raw text.
#[derive(Debug, Clone, Default)]
pub struct AnalyticsEventRecord {
    pub unique_id: String,
    pub chats: i64,
    pub gift_events: i64,
    pub gifts: i64,
    pub diamonds: i64,
    pub like_events: i64,
    pub likes: i64,
    pub joins: i64,
    pub follows: i64,
    pub shares: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsTotals {
    pub chats: i64,
    pub gift_events: i64,
    pub gifts: i64,
    pub diamonds: i64,
    pub like_events: i64,
    pub likes: i64,
    pub joins: i64,
    pub follows: i64,
    pub shares: i64,
    pub peak_viewers: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsDayRow {
    pub day: i64,
    #[serde(flatten)]
    pub totals: AnalyticsTotals,
}

/// One hour of counters inside a single (caller-frame) day. `hour` is the
/// 0..23 wall-clock hour; peaks take MAX while every other counter sums,
/// mirroring the daily semantics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsHourRow {
    pub hour: i64,
    #[serde(flatten)]
    pub totals: AnalyticsTotals,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsTopViewer {
    pub unique_id: String,
    pub chats: i64,
    pub gifts: i64,
    pub diamonds: i64,
    pub likes: i64,
    pub shares: i64,
    pub interactions: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSummaryData {
    pub creator_unique_id: String,
    pub start_day: i64,
    pub end_day: i64,
    pub totals: AnalyticsTotals,
    pub days: Vec<AnalyticsDayRow>,
    pub top_viewers: Vec<AnalyticsTopViewer>,
    pub sessions: i64,
    /// True per-hour counters for single-day spans, in the requested frame
    /// (system-local hours when `tz_offset_secs != 0`). Empty for multi-day
    /// spans and for days recorded before `analytics_hourly` existed.
    #[serde(default)]
    pub hours: Vec<AnalyticsHourRow>,
}

#[cfg(feature = "native-tiktok")]
impl AnalyticsEventRecord {
    /// Map a canonical LIVE event to counters. Returns `None` for room stats
    /// (handled as peaks) and unknown packets.
    pub fn from_canonical_event(
        event: &tiktools_tiktok::events::TikToolsEvent,
    ) -> Option<AnalyticsEventRecord> {
        use tiktools_tiktok::events::CanonicalLiveEvent;

        let unique_id = crate::helpers::clean_unique_id(
            event
                .user()
                .map(|user| user.unique_id.as_str())
                .unwrap_or(""),
        )
        .unwrap_or_default();
        let mut record = AnalyticsEventRecord {
            unique_id,
            ..AnalyticsEventRecord::default()
        };
        match &event.base {
            CanonicalLiveEvent::Chat(_) => {
                record.chats = 1;
            }
            CanonicalLiveEvent::Gift(gift) => {
                let count = gift.repeat_count.max(gift.combo_count).max(1);
                let diamonds = event
                    .gift_diamond_count()
                    .unwrap_or(gift.diamond_count)
                    .max(1);
                record.gift_events = 1;
                record.gifts = count as i64;
                record.diamonds = (diamonds as i64).saturating_mul(count as i64);
            }
            CanonicalLiveEvent::Like(like) => {
                record.like_events = 1;
                record.likes = (like.count.max(1)) as i64;
            }
            CanonicalLiveEvent::Member(_) => {
                record.joins = 1;
            }
            CanonicalLiveEvent::Social(social) => {
                if social.action == 1 {
                    record.follows = 1;
                } else if social.action == 3 {
                    record.shares = 1;
                } else {
                    return None;
                }
            }
            CanonicalLiveEvent::RoomUser(_) | CanonicalLiveEvent::Unknown { .. } => return None,
        }
        Some(record)
    }
}
