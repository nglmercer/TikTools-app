//! Aggregate-only LIVE analytics in a dedicated SQLite file.
//!
//! Counters and daily buckets only: chat text and other raw payloads are never
//! persisted here. Weeks and months are SQL aggregations of daily rows, and
//! event packets are tracked separately from the quantities they carry
//! (`like_events` vs `likes`, `gift_events` vs `gifts`).

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::sqlite::DatabaseError;
use super::DatabaseManager;

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

/// UTC day bucket for a unix timestamp.
pub fn utc_day(unix_secs: i64) -> i64 {
    unix_secs.div_euclid(86_400)
}

/// UTC hour bucket for a unix timestamp.
pub fn utc_hour(unix_secs: i64) -> i64 {
    unix_secs.div_euclid(3_600)
}

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

impl DatabaseManager {
    pub(super) fn ensure_analytics_schema(&self) -> Result<(), DatabaseError> {
        let connection = self.open(&self.analytics_path())?;
        connection.execute_batch(ANALYTICS_SCHEMA)?;
        Ok(())
    }

    /// Record one event's counters into the daily, hourly and per-viewer buckets.
    pub(crate) fn record_analytics_event(
        &self,
        creator_unique_id: &str,
        record: &AnalyticsEventRecord,
        now_unix: i64,
    ) -> Result<(), DatabaseError> {
        let day = utc_day(now_unix);
        let hour = utc_hour(now_unix);
        let mut connection = self.open(&self.analytics_path())?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO analytics_daily (
                creator_unique_id, day, chats, gift_events, gifts, diamonds,
                like_events, likes, joins, follows, shares
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT (creator_unique_id, day) DO UPDATE SET
                chats = chats + excluded.chats,
                gift_events = gift_events + excluded.gift_events,
                gifts = gifts + excluded.gifts,
                diamonds = diamonds + excluded.diamonds,
                like_events = like_events + excluded.like_events,
                likes = likes + excluded.likes,
                joins = joins + excluded.joins,
                follows = follows + excluded.follows,
                shares = shares + excluded.shares",
            params![
                creator_unique_id,
                day,
                record.chats,
                record.gift_events,
                record.gifts,
                record.diamonds,
                record.like_events,
                record.likes,
                record.joins,
                record.follows,
                record.shares,
            ],
        )?;
        transaction.execute(
            "INSERT INTO analytics_hourly (
                creator_unique_id, hour, chats, gift_events, gifts, diamonds,
                like_events, likes, joins, follows, shares
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT (creator_unique_id, hour) DO UPDATE SET
                chats = chats + excluded.chats,
                gift_events = gift_events + excluded.gift_events,
                gifts = gifts + excluded.gifts,
                diamonds = diamonds + excluded.diamonds,
                like_events = like_events + excluded.like_events,
                likes = likes + excluded.likes,
                joins = joins + excluded.joins,
                follows = follows + excluded.follows,
                shares = shares + excluded.shares",
            params![
                creator_unique_id,
                hour,
                record.chats,
                record.gift_events,
                record.gifts,
                record.diamonds,
                record.like_events,
                record.likes,
                record.joins,
                record.follows,
                record.shares,
            ],
        )?;
        if !record.unique_id.is_empty() {
            transaction.execute(
                "INSERT INTO viewer_daily (
                    creator_unique_id, day, unique_id, chats, gifts, diamonds,
                    likes, shares, first_seen, last_seen
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
                ON CONFLICT (creator_unique_id, day, unique_id) DO UPDATE SET
                    chats = chats + excluded.chats,
                    gifts = gifts + excluded.gifts,
                    diamonds = diamonds + excluded.diamonds,
                    likes = likes + excluded.likes,
                    shares = shares + excluded.shares,
                    first_seen = MIN(first_seen, excluded.first_seen),
                    last_seen = MAX(last_seen, excluded.last_seen)",
                params![
                    creator_unique_id,
                    day,
                    record.unique_id,
                    record.chats,
                    record.gifts,
                    record.diamonds,
                    record.likes,
                    record.shares,
                    now_unix,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Fold a room-stats sample into the daily, hourly and session viewer peaks.
    pub(crate) fn record_analytics_viewers(
        &self,
        creator_unique_id: &str,
        viewers: i64,
        now_unix: i64,
    ) -> Result<(), DatabaseError> {
        let day = utc_day(now_unix);
        let hour = utc_hour(now_unix);
        let connection = self.open(&self.analytics_path())?;
        connection.execute(
            "INSERT INTO analytics_daily (creator_unique_id, day, peak_viewers)
             VALUES (?1, ?2, ?3)
             ON CONFLICT (creator_unique_id, day) DO UPDATE SET
                peak_viewers = MAX(peak_viewers, excluded.peak_viewers)",
            params![creator_unique_id, day, viewers.max(0)],
        )?;
        connection.execute(
            "INSERT INTO analytics_hourly (creator_unique_id, hour, peak_viewers)
             VALUES (?1, ?2, ?3)
             ON CONFLICT (creator_unique_id, hour) DO UPDATE SET
                peak_viewers = MAX(peak_viewers, excluded.peak_viewers)",
            params![creator_unique_id, hour, viewers.max(0)],
        )?;
        connection.execute(
            "UPDATE live_sessions SET peak_viewers = MAX(peak_viewers, ?1)
             WHERE creator_unique_id = ?2 AND ended_at IS NULL",
            params![viewers.max(0), creator_unique_id],
        )?;
        Ok(())
    }

    /// Open a session, closing any stale open rows first (crash-safe).
    pub(crate) fn open_live_session(
        &self,
        creator_unique_id: &str,
        room_id: Option<&str>,
        now_unix: i64,
    ) -> Result<(), DatabaseError> {
        let connection = self.open(&self.analytics_path())?;
        connection.execute(
            "UPDATE live_sessions SET ended_at = ?1
             WHERE creator_unique_id = ?2 AND ended_at IS NULL",
            params![now_unix, creator_unique_id],
        )?;
        connection.execute(
            "INSERT INTO live_sessions (creator_unique_id, room_id, started_at)
             VALUES (?1, ?2, ?3)",
            params![creator_unique_id, room_id, now_unix],
        )?;
        Ok(())
    }

    pub(crate) fn close_live_sessions(
        &self,
        creator_unique_id: &str,
        now_unix: i64,
    ) -> Result<(), DatabaseError> {
        let connection = self.open(&self.analytics_path())?;
        connection.execute(
            "UPDATE live_sessions SET ended_at = ?1
             WHERE creator_unique_id = ?2 AND ended_at IS NULL",
            params![now_unix, creator_unique_id],
        )?;
        Ok(())
    }

    /// Range summary: totals, per-day series, top viewers, session count.
    ///
    /// `start_day`/`end_day` are day numbers in the caller's frame: UTC days
    /// when `tz_offset_secs` is 0, system-local days otherwise, where a local
    /// day holds the instants `t` with `(t + offset) / 86400 == day`. A UTC
    /// day row cannot be split across local days, so local-frame totals and
    /// day rows are aggregated from `analytics_hourly` whenever it holds rows
    /// for the window; without hourly rows (streams recorded before the
    /// hourly table existed) the summary falls back to the closest UTC days
    /// and leaves `hours` empty. `offset == 0` reproduces the legacy UTC
    /// behavior exactly.
    pub(crate) fn analytics_summary(
        &self,
        creator_unique_id: &str,
        start_day: i64,
        end_day: i64,
        top_limit: i64,
        tz_offset_secs: i64,
    ) -> Result<AnalyticsSummaryData, DatabaseError> {
        let offset = tz_offset_secs.clamp(-86_400, 86_400);
        let connection = self.open(&self.analytics_path())?;

        // UTC day window overlapped by the requested frame span. Viewer rows
        // cannot be split by hour, so top viewers always cover this window
        // (identical to [start_day, end_day] when offset == 0).
        let window_lo = (start_day * 86_400 - offset).div_euclid(86_400);
        let window_hi = ((end_day + 1) * 86_400 - 1 - offset).div_euclid(86_400);

        // UTC hour window covering the frame span.
        let hour_lo = (start_day * 86_400 - offset).div_euclid(3_600);
        // Exclusive upper bound, rounded up.
        let hour_hi_excl = ((end_day + 1) * 86_400 - offset + 3_599).div_euclid(3_600);

        let mut hourly_statement = connection.prepare(
            "SELECT hour, chats, gift_events, gifts, diamonds, like_events, likes,
                    joins, follows, shares, peak_viewers
             FROM analytics_hourly
             WHERE creator_unique_id = ?1 AND hour >= ?2 AND hour < ?3
             ORDER BY hour ASC",
        )?;
        let samples = hourly_statement
            .query_map(params![creator_unique_id, hour_lo, hour_hi_excl], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    AnalyticsTotals {
                        chats: row.get(1)?,
                        gift_events: row.get(2)?,
                        gifts: row.get(3)?,
                        diamonds: row.get(4)?,
                        like_events: row.get(5)?,
                        likes: row.get(6)?,
                        joins: row.get(7)?,
                        follows: row.get(8)?,
                        shares: row.get(9)?,
                        peak_viewers: row.get(10)?,
                    },
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Local-frame aggregation from hourly rows (exact once data flows).
        // The gate is creator-wide, not window-wide: with any hourly rows on
        // record, zeros inside the window are real zeros; without hourly rows
        // the stream predates the hourly table and UTC fallback applies.
        let has_hourly: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM analytics_hourly WHERE creator_unique_id = ?1)",
            params![creator_unique_id],
            |row| row.get(0),
        )?;
        let mut totals: Option<AnalyticsTotals> = None;
        let mut days: Option<Vec<AnalyticsDayRow>> = None;
        let mut hours: Vec<AnalyticsHourRow> = Vec::new();
        if offset != 0 && has_hourly {
            use std::collections::BTreeMap;
            let mut by_day: BTreeMap<i64, AnalyticsTotals> = BTreeMap::new();
            let mut by_hour: BTreeMap<i64, AnalyticsTotals> = BTreeMap::new();
            for (hour, sample) in &samples {
                let local = hour * 3_600 + offset;
                let day = local.div_euclid(86_400);
                // Edge UTC hours overlapping the span boundary (non-hour
                // offsets) fall outside and are skipped rather than split.
                if day < start_day || day > end_day {
                    continue;
                }
                add_hour_totals(by_day.entry(day).or_default(), sample);
                if start_day == end_day {
                    let wall_hour = local.rem_euclid(86_400) / 3_600;
                    add_hour_totals(by_hour.entry(wall_hour).or_default(), sample);
                }
            }
            let mut local_totals = AnalyticsTotals::default();
            let mut local_days = Vec::new();
            for (day, day_totals) in &by_day {
                add_hour_totals(&mut local_totals, day_totals);
                local_days.push(AnalyticsDayRow {
                    day: *day,
                    totals: day_totals.clone(),
                });
            }
            if start_day == end_day {
                for hour in 0..24 {
                    hours.push(AnalyticsHourRow {
                        hour,
                        totals: by_hour.get(&hour).cloned().unwrap_or_default(),
                    });
                }
            }
            totals = Some(local_totals);
            days = Some(local_days);
        }

        // UTC day bounds for the legacy daily-table path: the requested span
        // itself when offset == 0, else the closest UTC days (local noon) so
        // pre-hourly data still resolves to a single sensible day per request.
        let (query_lo, query_hi) = if offset == 0 {
            (start_day, end_day)
        } else {
            (
                (start_day * 86_400 + 43_200 - offset).div_euclid(86_400),
                (end_day * 86_400 + 43_200 - offset).div_euclid(86_400),
            )
        };
        let totals = match totals {
            Some(totals) => totals,
            None => connection.query_row(
                "SELECT
                    COALESCE(SUM(chats), 0), COALESCE(SUM(gift_events), 0),
                    COALESCE(SUM(gifts), 0), COALESCE(SUM(diamonds), 0),
                    COALESCE(SUM(like_events), 0), COALESCE(SUM(likes), 0),
                    COALESCE(SUM(joins), 0), COALESCE(SUM(follows), 0),
                    COALESCE(SUM(shares), 0), COALESCE(MAX(peak_viewers), 0)
                 FROM analytics_daily
                 WHERE creator_unique_id = ?1 AND day BETWEEN ?2 AND ?3",
                params![creator_unique_id, query_lo, query_hi],
                |row| {
                    Ok(AnalyticsTotals {
                        chats: row.get(0)?,
                        gift_events: row.get(1)?,
                        gifts: row.get(2)?,
                        diamonds: row.get(3)?,
                        like_events: row.get(4)?,
                        likes: row.get(5)?,
                        joins: row.get(6)?,
                        follows: row.get(7)?,
                        shares: row.get(8)?,
                        peak_viewers: row.get(9)?,
                    })
                },
            )?,
        };
        let days = match days {
            Some(days) => days,
            None => {
                let mut days_statement = connection.prepare(
                    "SELECT day, chats, gift_events, gifts, diamonds, like_events, likes,
                            joins, follows, shares, peak_viewers
                     FROM analytics_daily
                     WHERE creator_unique_id = ?1 AND day BETWEEN ?2 AND ?3
                     ORDER BY day ASC",
                )?;
                let rows = days_statement
                    .query_map(params![creator_unique_id, query_lo, query_hi], |row| {
                        Ok(AnalyticsDayRow {
                            day: row.get(0)?,
                            totals: AnalyticsTotals {
                                chats: row.get(1)?,
                                gift_events: row.get(2)?,
                                gifts: row.get(3)?,
                                diamonds: row.get(4)?,
                                like_events: row.get(5)?,
                                likes: row.get(6)?,
                                joins: row.get(7)?,
                                follows: row.get(8)?,
                                shares: row.get(9)?,
                                peak_viewers: row.get(10)?,
                            },
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            }
        };
        let limit = top_limit.clamp(1, 100);
        let mut viewers_statement = connection.prepare(
            "SELECT unique_id,
                    SUM(chats), SUM(gifts), SUM(diamonds), SUM(likes), SUM(shares),
                    SUM(chats + gifts + likes + shares), MAX(last_seen)
             FROM viewer_daily
             WHERE creator_unique_id = ?1 AND day BETWEEN ?2 AND ?3
             GROUP BY unique_id
             ORDER BY SUM(chats + gifts + likes + shares) DESC, MAX(last_seen) DESC
             LIMIT ?4",
        )?;
        let top_viewers = viewers_statement
            .query_map(
                params![creator_unique_id, window_lo, window_hi, limit],
                |row| {
                    Ok(AnalyticsTopViewer {
                        unique_id: row.get(0)?,
                        chats: row.get(1)?,
                        gifts: row.get(2)?,
                        diamonds: row.get(3)?,
                        likes: row.get(4)?,
                        shares: row.get(5)?,
                        interactions: row.get(6)?,
                        last_seen: row.get(7)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        // Session instants are exact, so the frame shift applies cleanly here
        // (identical to the legacy expression when offset == 0).
        let sessions: i64 = connection.query_row(
            "SELECT COUNT(*) FROM live_sessions
             WHERE creator_unique_id = ?1
               AND CAST((started_at + ?2) / 86400 AS INTEGER) BETWEEN ?3 AND ?4",
            params![creator_unique_id, offset, start_day, end_day],
            |row| row.get(0),
        )?;

        // UTC single-day spans keep the previous single-bar behavior on the
        // frontend; still expose true hourly rows so the chart can use them.
        if offset == 0 && start_day == end_day && !samples.is_empty() {
            use std::collections::BTreeMap;
            let mut by_hour: BTreeMap<i64, AnalyticsTotals> = BTreeMap::new();
            for (hour, sample) in &samples {
                let wall_hour = hour.rem_euclid(24);
                add_hour_totals(by_hour.entry(wall_hour).or_default(), sample);
            }
            for wall_hour in 0..24 {
                hours.push(AnalyticsHourRow {
                    hour: wall_hour,
                    totals: by_hour.get(&wall_hour).cloned().unwrap_or_default(),
                });
            }
        }

        Ok(AnalyticsSummaryData {
            creator_unique_id: creator_unique_id.to_owned(),
            start_day,
            end_day,
            totals,
            days,
            top_viewers,
            sessions,
            hours,
        })
    }
}

/// Fold one hourly sample into a running total: counters sum, peaks max.
fn add_hour_totals(into: &mut AnalyticsTotals, sample: &AnalyticsTotals) {
    into.chats += sample.chats;
    into.gift_events += sample.gift_events;
    into.gifts += sample.gifts;
    into.diamonds += sample.diamonds;
    into.like_events += sample.like_events;
    into.likes += sample.likes;
    into.joins += sample.joins;
    into.follows += sample.follows;
    into.shares += sample.shares;
    into.peak_viewers = into.peak_viewers.max(sample.peak_viewers);
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

#[cfg(all(test, feature = "persistence"))]
mod tests {
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
}
