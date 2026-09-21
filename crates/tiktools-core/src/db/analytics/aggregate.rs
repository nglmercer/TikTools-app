use super::AnalyticsTotals;

/// UTC day bucket for a unix timestamp.
pub fn utc_day(unix_secs: i64) -> i64 {
    unix_secs.div_euclid(86_400)
}

/// UTC hour bucket for a unix timestamp.
pub fn utc_hour(unix_secs: i64) -> i64 {
    unix_secs.div_euclid(3_600)
}

/// Fold one hourly sample into a running total: counters sum, peaks max.
pub(super) fn add_hour_totals(into: &mut AnalyticsTotals, sample: &AnalyticsTotals) {
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
