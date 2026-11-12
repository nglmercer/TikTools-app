//! Host-clock expiry of rolling contributions and daily accounting windows.
use super::*;

impl Engine {
    pub(super) fn expire_ranking(ranking: &mut Ranking, now: u64) -> Result<(), Error> {
        if let Window::Daily { utc_offset_minutes } = ranking.definition.window {
            let day = |time: u64| {
                (time.min(i64::MAX as u64) as i64)
                    .saturating_add(i64::from(utc_offset_minutes) * 60_000)
                    .div_euclid(86_400_000)
            };
            let previous = ranking.last_time;
            ranking.last_time = previous.max(now);
            if previous != 0 && day(ranking.last_time) != day(previous) {
                ranking.state.reset()?;
                ranking.users.clear();
                ranking.rejected = 0;
                ranking.dropped = 0;
                ranking.limited = 0;
                ranking.cooldown_rejected = 0;
            }
            return Ok(());
        }
        ranking.last_time = ranking.last_time.max(now);
        let Window::Rolling { duration_ms, .. } = ranking.definition.window else {
            return Ok(());
        };
        while let Some((at, key, amount)) = ranking.window_events.front() {
            if ranking.last_time.saturating_sub(*at) < duration_ms {
                break;
            }
            ranking.state.expire(key, *amount)?;
            ranking.window_events.pop_front();
        }
        Ok(())
    }

    /// Expires contributions even when no new events arrive.
    pub fn expire_at(&mut self, now: u64) {
        for ranking in self.rankings.values_mut() {
            if Self::expire_ranking(ranking, now).is_err() {
                ranking.rejected = ranking.rejected.saturating_add(1);
            }
        }
    }
}
