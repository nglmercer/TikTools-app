//! Automation runtime unit tests.

use super::http_actions::http_error_reason;
use serde_json::{json, Value};

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
