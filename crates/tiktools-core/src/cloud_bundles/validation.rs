//! Bounded input validation before any Cloud request is sent.
use super::OperationError;
use super::{PackageItemParams, KINDS, MAX_DOCUMENT_BYTES, MAX_LABEL_CHARS};
use crate::cloud_teams::valid_uuid;
use serde_json::Value;

/// The hyphenated lowercase form of an identifier, which is what goes in a path.
pub(super) fn canonical_id(value: &str) -> Result<String, OperationError> {
    uuid::Uuid::parse_str(value)
        .map(|id| id.hyphenated().to_string())
        .map_err(|_| OperationError::invalid("identifier is not valid"))
}

/// Opaque cursors are base64url JSON; anything else is refused before it is sent.
pub(super) fn check_cursor(cursor: Option<&str>) -> Result<(), OperationError> {
    if let Some(cursor) = cursor {
        if cursor.is_empty()
            || cursor.len() > 1024
            || !cursor
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_=".contains(&b))
        {
            return Err(OperationError::invalid("cursor is not valid"));
        }
    }
    Ok(())
}

/// Query pairs for a paged list, validated. Unset values are not sent.
pub(super) fn page_params(
    cursor: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<(&'static str, String)>, OperationError> {
    check_cursor(cursor)?;
    let mut params = Vec::new();
    if let Some(limit) = limit {
        if !(1..=50).contains(&limit) {
            return Err(OperationError::invalid("limit must be between 1 and 50"));
        }
        params.push(("limit", limit.to_string()));
    }
    if let Some(cursor) = cursor {
        params.push(("cursor", cursor.to_owned()));
    }
    Ok(params)
}

/// Label rule of the Cloud: trimmed, 1 to 120 characters, no control
/// characters and no line or paragraph separators.
pub(super) fn clean_label(label: &str) -> Result<String, OperationError> {
    let label = label.trim();
    let count = label.chars().count();
    if count == 0
        || count > MAX_LABEL_CHARS
        || label
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err(OperationError::invalid(
            "the label must be 1 to 120 characters without line breaks",
        ));
    }
    Ok(label.to_owned())
}

pub(super) fn bounded_document(value: &Value) -> Result<(), OperationError> {
    if !value.is_object() {
        return Err(OperationError::invalid("the document must be an object"));
    }
    let size = serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_DOCUMENT_BYTES {
        return Err(OperationError::invalid("the document is too large"));
    }
    Ok(())
}

pub(super) fn validate_item(item: &PackageItemParams) -> Result<(), OperationError> {
    if !KINDS.contains(&item.kind.as_str()) {
        return Err(OperationError::invalid(
            "package item kind is not supported",
        ));
    }
    if item.kind == "plugin" {
        let version = item
            .version
            .as_deref()
            .ok_or_else(|| OperationError::invalid("a plugin item pins a version"))?;
        if version.is_empty() || version.len() > 64 || version.chars().any(char::is_control) {
            return Err(OperationError::invalid("plugin version is not valid"));
        }
        if item.revision.is_some() {
            return Err(OperationError::invalid(
                "a plugin item pins a version, not a revision",
            ));
        }
    } else {
        valid_uuid(&item.id)?;
        let revision = item
            .revision
            .ok_or_else(|| OperationError::invalid("this item pins a revision"))?;
        if revision < 1 {
            return Err(OperationError::invalid("revision must be positive"));
        }
        if item.version.is_some() {
            return Err(OperationError::invalid("only plugins pin a version"));
        }
    }
    Ok(())
}
