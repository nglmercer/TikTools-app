//! Shared validation and identifier helpers for control operations.

use super::OperationError;
use crate::*;

pub(crate) fn clean_record_id(value: &str, what: &str) -> Result<String, OperationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err(OperationError::invalid(format!(
            "{what} id must be 1..=128 characters"
        )));
    }
    Ok(trimmed.to_owned())
}

pub(crate) fn clean_plugin_id(value: &str) -> Result<String, OperationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 128 || !is_identifier(trimmed) {
        return Err(OperationError::invalid(format!(
            "plugin id `{value}` is not a valid identifier"
        )));
    }
    Ok(trimmed.to_owned())
}

pub(crate) fn fresh_record_id(prefix: &str) -> String {
    format!("{prefix}-{}-{:08x}", now_millis(), fastrand::u32(..))
}

/// Session-cookie bound for live entry points. Empty is allowed: the native
/// client bootstraps an anonymous guest session, matching the UI's
/// "(optional)" cookie field. Only an overlong value is rejected.
pub(crate) fn check_session_cookie_len(session_cookie: &str) -> Result<(), OperationError> {
    if session_cookie.len() > 16_384 {
        return Err(OperationError::invalid(
            "sessionCookie must be at most 16384 characters",
        ));
    }
    Ok(())
}
