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

/// Keep the event ID and scope separate: concatenated strings collide when
/// IDs contain ':' or a viewer happens to be named 'global' or 'anonymous'.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CooldownKey {
    pub(super) event_id: String,
    scope: CooldownScope,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CooldownScope {
    Global,
    Anonymous,
    Viewer(String),
}

impl AutomationService {
    pub fn claim_event(&self, record: &Value, event: &Value, now: u64) -> bool {
        // Matching, filter re-evaluation and viewer cooldown keys share the
        // same closed event view for an approved definition.
        let Some(event) = self.scoped_event(record, event) else {
            return false;
        };
        self.claim_projected_event(record, &event, now, || true)
    }

    /// Private final scoped claim. The caller supplies the already closed
    /// event projection and holds its current subject/definition chain through
    /// this entire call. Do not read the automation index here: that same
    /// index read fence is already held by the caller's final admission.
    pub(crate) fn claim_projected_event(
        &self,
        record: &Value,
        event: &Value,
        now: u64,
        admit: impl FnOnce() -> bool,
    ) -> bool {
        if !event_record_matches(record, event) {
            return false;
        }
        let cooldown_ms = record
            .get("cooldownMs")
            .and_then(number_u64)
            .unwrap_or_default()
            .min(MAX_COOLDOWN_MS);
        if cooldown_ms == 0 {
            return admit();
        }
        let id = record.get("id").and_then(Value::as_str).unwrap_or("event");
        let scope = if record.get("cooldownScope").and_then(Value::as_str) == Some("global") {
            CooldownScope::Global
        } else {
            event
                .get("user")
                .and_then(|user| user.get("uniqueId"))
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(|id| CooldownScope::Viewer(id.to_owned()))
                .unwrap_or(CooldownScope::Anonymous)
        };
        let key = CooldownKey {
            event_id: id.to_owned(),
            scope,
        };
        let mut cooldowns = recover_mutex(&self.cooldowns, "automation cooldowns");
        // A scoped caller may have waited for this mutex while its retained
        // subject/index locks were held. Recheck deadlines and shutdown now,
        // before even the expiry sweep changes the cooldown map.
        if !admit() {
            return false;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn final_cooldown_admission_holds_mutation_lock_and_rejection_preserves_the_map() {
        let service = AutomationService::default();
        {
            let mut cooldowns = service.cooldowns.lock().unwrap();
            for id in 0..COOLDOWN_SWEEP_THRESHOLD {
                cooldowns.insert(
                    CooldownKey {
                        event_id: format!("expired-{id}"),
                        scope: CooldownScope::Global,
                    },
                    0,
                );
            }
        }
        let record = json!({"id":"child","filters":[],"cooldownMs":1000,"cooldownScope":"global"});
        let event = json!({"user":{"uniqueId":"alice"}});
        assert!(
            !service.claim_projected_event(&record, &event, MAX_COOLDOWN_MS + 1, || {
                assert!(matches!(
                    service.cooldowns.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                false
            })
        );
        // Denial precedes expiry sweep as well as insertion.
        assert_eq!(
            service.cooldowns.lock().unwrap().len(),
            COOLDOWN_SWEEP_THRESHOLD
        );
        assert!(
            service.claim_projected_event(&record, &event, MAX_COOLDOWN_MS + 1, || {
                assert!(matches!(
                    service.cooldowns.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                true
            })
        );
        let cooldowns = service.cooldowns.lock().unwrap();
        assert_eq!(cooldowns.len(), 1);
        assert_eq!(
            cooldowns.get(&CooldownKey {
                event_id: "child".into(),
                scope: CooldownScope::Global
            }),
            Some(&(MAX_COOLDOWN_MS + 1))
        );
    }

    #[test]
    fn zero_cooldown_still_checks_final_admission() {
        let service = AutomationService::default();
        let record = json!({"id":"child","cooldownMs":0});
        assert!(!service.claim_projected_event(&record, &json!({}), 1, || false));
        assert!(service.claim_projected_event(&record, &json!({}), 1, || true));
        assert!(service.cooldowns.lock().unwrap().is_empty());
    }

    #[test]
    fn event_and_viewer_ids_do_not_collide_or_delete_other_events() {
        let service = AutomationService::default();
        let nested_id = json!({"id":"gift:rose","cooldownMs":1000,"cooldownScope":"user"});
        let prefix_id = json!({"id":"gift","cooldownMs":1000,"cooldownScope":"user"});
        let alice = json!({"user":{"uniqueId":"alice"}});
        let other = json!({"user":{"uniqueId":"rose:alice"}});
        assert!(service.claim_projected_event(&nested_id, &alice, 100, || true));
        assert!(service.claim_projected_event(&prefix_id, &other, 100, || true));
        assert!(!service.claim_projected_event(&nested_id, &alice, 200, || true));
        service.remove_event("gift");
        assert!(!service.claim_projected_event(&nested_id, &alice, 200, || true));
        assert!(service.claim_projected_event(&prefix_id, &other, 200, || true));
    }

    #[test]
    fn global_and_missing_viewer_do_not_collide_with_literal_usernames() {
        let service = AutomationService::default();
        let global = json!({"id":"event","cooldownMs":1000,"cooldownScope":"global"});
        let per_viewer = json!({"id":"event","cooldownMs":1000,"cooldownScope":"user"});
        let global_viewer = json!({"user":{"uniqueId":"global"}});
        let anonymous_viewer = json!({"user":{"uniqueId":"anonymous"}});
        assert!(service.claim_projected_event(&global, &global_viewer, 100, || true));
        assert!(service.claim_projected_event(&per_viewer, &global_viewer, 100, || true));
        assert!(service.claim_projected_event(&per_viewer, &json!({}), 100, || true));
        assert!(service.claim_projected_event(&per_viewer, &anonymous_viewer, 100, || true));
        assert!(!service.claim_projected_event(
            &per_viewer,
            &json!({"user":{"uniqueId":""}}),
            200,
            || true
        ));
        assert!(!service.claim_projected_event(&per_viewer, &anonymous_viewer, 200, || true));
    }
}
