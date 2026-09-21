use rusqlite::params;

use super::super::sqlite::DatabaseError;
use super::super::DatabaseManager;
use super::aggregate::add_hour_totals;
use super::{
    AnalyticsDayRow, AnalyticsHourRow, AnalyticsSummaryData, AnalyticsTopViewer, AnalyticsTotals,
};

impl DatabaseManager {
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
