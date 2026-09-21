//! Per-event cooldown windows (per-viewer or global).

use super::filters::{event_record_matches, number_u64};
use super::*;

const MAX_COOLDOWN_MS: u64 = 24 * 60 * 60 * 1_000;

impl AutomationService {
    pub fn claim_event(&self, record: &Value, event: &Value, now: u64) -> bool {
        if !event_record_matches(record, event) {
            return false;
        }
        let cooldown_ms = record
            .get("cooldownMs")
            .and_then(number_u64)
            .unwrap_or_default()
            .min(MAX_COOLDOWN_MS);
        if cooldown_ms == 0 {
            return true;
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or("event");
        let scope = if record.get("cooldownScope").and_then(Value::as_str) == Some("global") {
            "global".to_owned()
        } else {
            event
                .get("user")
                .and_then(|user| user.get("uniqueId"))
                .and_then(Value::as_str)
                .unwrap_or("anonymous")
                .to_owned()
        };
        let key = format!("{id}:{scope}");
        let mut cooldowns = recover_mutex(&self.cooldowns, "automation cooldowns");
        cooldowns.retain(|_, previous| now.saturating_sub(*previous) <= MAX_COOLDOWN_MS);
        if cooldowns
            .get(&key)
            .is_some_and(|previous| now.saturating_sub(*previous) < cooldown_ms)
        {
            return false;
        }
        cooldowns.insert(key, now);
        true
    }
}
