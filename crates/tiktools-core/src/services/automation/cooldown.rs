//! Per-event cooldown windows (per-viewer or global).

use super::filters::{event_record_matches, number_u64};
use super::*;

const MAX_COOLDOWN_MS: u64 = 24 * 60 * 60 * 1_000;
/// Sweep the map only once it reaches this size: expiry is still enforced
/// per key on every read, so amortizing the full scan changes no decision.
const COOLDOWN_SWEEP_THRESHOLD: usize = 1_024;
/// Hard bound on retained cooldown keys. Past this the map drains and
/// protection degrades to no-cooldown (fail-open) rather than growing
/// memory without limit under adversarial key cardinality.
const COOLDOWN_MAP_BOUND: usize = 8_192;

impl AutomationService {
    pub fn claim_event(&self, record: &Value, event: &Value, now: u64) -> bool {
        // Matching, filter re-evaluation and viewer cooldown keys share the
        // same closed event view for an approved definition.
        let Some(event) = self.scoped_event(record, event) else {
            return false;
        };
        if !event_record_matches(record, &event) {
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
        // Amortized expiry sweep: per-key decisions below already ignore
        // stale entries, so the scan only reclaims memory, O(1) amortized
        // instead of O(map) per claimed event.
        if cooldowns.len() >= COOLDOWN_SWEEP_THRESHOLD {
            cooldowns.retain(|_, previous| now.saturating_sub(*previous) <= MAX_COOLDOWN_MS);
        }
        if cooldowns.len() >= COOLDOWN_MAP_BOUND {
            // Bounded fail-open: drop tracking rather than memory. Every
            // retained key is at most a day old, so the blast radius is one
            // cooldown window of over-execution, never unbounded growth.
            cooldowns.clear();
        }
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
