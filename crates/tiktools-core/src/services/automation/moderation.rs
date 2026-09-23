//! TextIntel moderation penalty helpers.
//!
//! The TextIntel processor stays side-effect free: it only attaches
//! `moderation.blocked` evidence under its provider namespace. Deducting
//! points is automation work, built here as ordinary behavior records over
//! the existing executable action:
//!
//! ```text
//! trigger: tiktok.chat
//! filter:  event.intel.providers.textintel.comment.moderation.blocked is true
//! action:  core.points { uniqueId: "{{ event.user.uniqueId }}", delta: -10 }
//! ```
//!
//! These builders never touch the points service themselves; the automation
//! engine executes the stored records, so viewer totals keep the existing
//! clamping (never below zero). The visual workflow node `action.adjust-points`
//! is intentionally not used: it is editor-only and never executes at runtime.

use serde_json::{json, Value};

/// Filter path addressing the TextIntel moderation verdict on enriched chat.
pub const MODERATION_BLOCKED_PATH: &str =
    "event.intel.providers.textintel.comment.moderation.blocked";
/// Executable action type carrying the penalty. Never a second points system.
pub const MODERATION_POINTS_ACTION_TYPE: &str = "core.points";
/// Viewer template resolved per event by the automation engine.
pub const MODERATION_VIEWER_TEMPLATE: &str = "{{ event.user.uniqueId }}";

/// Validates a user-configured penalty amount: finite and non-zero, so the
/// stored `core.points` action always executes. Negative values deduct;
/// positive values would award (callers decide the sign; the CLI treats the
/// amount as a deduction-sized magnitude only in its help text).
pub fn validate_penalty_points(points: f64) -> Result<f64, String> {
    if !points.is_finite() {
        return Err("Points penalty must be a finite number.".to_owned());
    }
    if points == 0.0 {
        return Err("Points penalty must be non-zero.".to_owned());
    }
    Ok(points)
}

/// Builds the `core.points` action record for a moderation penalty. The id
/// is left for `automation.create` to generate; the returned record carries
/// the validated delta verbatim (no global hard-coded amount).
pub fn moderation_penalty_action_record(points: f64) -> Result<Value, String> {
    let delta = validate_penalty_points(points)?;
    Ok(json!({
        "name": "Moderation penalty",
        "enabled": true,
        "typeId": MODERATION_POINTS_ACTION_TYPE,
        "config": {
            "uniqueId": MODERATION_VIEWER_TEMPLATE,
            "delta": delta,
        },
    }))
}

/// Builds the behavior event record wiring `tiktok.chat` through the
/// moderation filter to one stored points action.
pub fn moderation_penalty_event_record(action_id: &str) -> Result<Value, String> {
    if action_id.trim().is_empty() {
        return Err("Moderation penalty event needs its points action id.".to_owned());
    }
    Ok(json!({
        "name": "TextIntel moderation penalty",
        "enabled": true,
        "trigger": "tiktok.chat",
        "filters": [
            {"path": MODERATION_BLOCKED_PATH, "operator": "is-true"},
        ],
        "actionIds": [action_id],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::AutomationService;

    #[test]
    fn penalty_validation_rejects_non_finite_and_zero() {
        assert!(validate_penalty_points(-10.0).is_ok());
        assert!(validate_penalty_points(5.0).is_ok());
        assert!(validate_penalty_points(0.0).is_err());
        assert!(validate_penalty_points(f64::NAN).is_err());
        assert!(validate_penalty_points(f64::INFINITY).is_err());
    }

    #[test]
    fn action_record_uses_core_points_with_viewer_template() {
        let action = moderation_penalty_action_record(-25.0).unwrap();
        assert_eq!(action["typeId"], "core.points");
        assert_eq!(action["config"]["uniqueId"], "{{ event.user.uniqueId }}");
        assert_eq!(action["config"]["delta"], -25.0);
        // The amount is caller-configured, never hard-coded.
        let action = moderation_penalty_action_record(-5.0).unwrap();
        assert_eq!(action["config"]["delta"], -5.0);
    }

    #[test]
    fn event_record_filters_on_moderation_blocked() {
        let event = moderation_penalty_event_record("action-1").unwrap();
        assert_eq!(event["trigger"], "tiktok.chat");
        assert_eq!(event["enabled"], true);
        assert_eq!(
            event["filters"],
            json!([{
                "path": "event.intel.providers.textintel.comment.moderation.blocked",
                "operator": "is-true",
            }])
        );
        assert_eq!(event["actionIds"], json!(["action-1"]));
        assert!(moderation_penalty_event_record("  ").is_err());
    }

    #[test]
    fn built_event_matches_blocked_enriched_chat_only() {
        let service = AutomationService::default();
        let record = moderation_penalty_event_record("action-1").unwrap();
        let blocked = json!({
            "type": "tiktok.chat",
            "user": {"uniqueId": "spammer"},
            "data": {"comment": "buy now"},
            "intel": {"providers": {"textintel": {"comment": {
                "moderation": {"blocked": true},
            }}}},
        });
        assert!(service.event_record_matches(&record, &blocked));
        let clean = json!({
            "type": "tiktok.chat",
            "user": {"uniqueId": "alice"},
            "data": {"comment": "hello"},
            "intel": {"providers": {"textintel": {"comment": {
                "moderation": {"blocked": false},
            }}}},
        });
        assert!(!service.event_record_matches(&record, &clean));
        // Raw events without enrichment never match.
        let raw = json!({
            "type": "tiktok.chat",
            "user": {"uniqueId": "alice"},
            "data": {"comment": "hello"},
        });
        assert!(!service.event_record_matches(&record, &raw));
    }
}
