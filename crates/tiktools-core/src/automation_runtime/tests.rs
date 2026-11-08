//! Automation runtime unit tests.
mod provenance;
mod versioned;

use super::http_actions::http_error_reason;
use crate::{AppCore, AutomationEvent, HostEmitter};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;

struct NoopEmitter;
impl HostEmitter for NoopEmitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}

fn automation_counters(core: &AppCore) -> (u64, u64, u64, u64) {
    let state = &core.automation_state;
    (
        state.events_completed.load(Ordering::Relaxed),
        state.actions_ok.load(Ordering::Relaxed),
        state.actions_failed.load(Ordering::Relaxed),
        state.exec_latency_sum_ms.load(Ordering::Relaxed),
    )
}

/// Live execution records completions and distinguishes action success
/// from failure, so saturation (rejected admissions) never masquerades
/// as plugin failure.
#[tokio::test]
async fn automation_execution_records_outcomes() {
    use crate::services::{AwardOptions, PointAction};
    let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
    core.points.award_points(
        "metrics-viewer",
        PointAction::Manual,
        AwardOptions {
            custom_amount: Some(10.0),
            ..AwardOptions::default()
        },
    );
    core.automation.replace_snapshot(&json!({
        "actions": [{
            "id": "act", "name": "Subtract", "enabled": true,
            "typeId": "core.points.subtract",
            "config": {"uniqueId": "{{ event.user.uniqueId }}", "amount": 1.0},
        }],
        "events": [{
            "id": "event", "enabled": true, "trigger": "tiktok.chat",
            "filters": [], "actionIds": ["act"],
        }],
    }));
    let event_for =
        |user: &str| json!({"type": "tiktok.chat", "user": {"uniqueId": user}, "data": {}});
    let (completed, ok, failed, _) = automation_counters(&core);
    core.run_automation_event(AutomationEvent::owner(event_for("metrics-viewer")))
        .await;
    let (completed2, ok2, failed2, _) = automation_counters(&core);
    assert_eq!(completed2, completed + 1);
    assert_eq!(ok2, ok + 1);
    assert_eq!(failed2, failed);
    // Unknown viewer: the action fails deterministically, counted apart.
    core.run_automation_event(AutomationEvent::owner(event_for("metrics-ghost")))
        .await;
    let (completed3, ok3, failed3, _) = automation_counters(&core);
    assert_eq!(completed3, completed + 2);
    assert_eq!(ok3, ok + 1);
    assert_eq!(failed3, failed + 1);
}

#[test]
fn error_reasons_prefer_server_messages_then_snippets() {
    // SonicBoom-style JSON error envelope.
    assert_eq!(
        http_error_reason(
            &json!({"error": "bad_request", "message": "voice must be a valid voice name"})
        )
        .as_deref(),
        Some("voice must be a valid voice name")
    );
    // Code-only envelope falls back to the code.
    assert_eq!(
        http_error_reason(&json!({"error": "bad_request"})).as_deref(),
        Some("bad_request")
    );
    // Plain-text bodies collapse whitespace and truncate.
    assert_eq!(
        http_error_reason(&Value::String("  failed\nbadly  ".to_owned())).as_deref(),
        Some("failed badly")
    );
    let long = "x".repeat(500);
    let reason = http_error_reason(&Value::String(long)).unwrap();
    assert_eq!(reason.chars().count(), 301);
    assert!(reason.ends_with('…'));
    // Empty or scalar bodies keep the legacy bare status text.
    assert_eq!(http_error_reason(&Value::String("   ".to_owned())), None);
    assert_eq!(http_error_reason(&Value::Null), None);
    assert_eq!(http_error_reason(&json!(500)), None);
}
