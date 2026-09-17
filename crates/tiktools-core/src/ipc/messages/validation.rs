use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IpcMessageError {
    #[error("IPC message is larger than the 2 MB limit")]
    TooLarge,
    #[error("IPC message is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IPC message field `{0}` is invalid")]
    InvalidField(&'static str),
}
pub(crate) fn bounded_string(
    value: &str,
    _field: &'static str,
    max: usize,
) -> Result<(), IpcMessageError> {
    if value.is_empty() || value.len() > max {
        return Err(IpcMessageError::InvalidField(_field));
    }
    Ok(())
}

pub(crate) fn bounded_value(
    value: &str,
    field: &'static str,
    max: usize,
) -> Result<(), IpcMessageError> {
    if value.len() > max {
        return Err(IpcMessageError::InvalidField(field));
    }
    Ok(())
}

pub(crate) fn optional_bounded(
    value: Option<&str>,
    field: &'static str,
    max: usize,
) -> Result<(), IpcMessageError> {
    if let Some(value) = value {
        bounded_string(value, field, max)?;
    }
    Ok(())
}

pub(crate) fn valid_token(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '_' | '-')
        })
}

/// Accepts legacy option sources plus the canonical plugin shape
/// `plugin-action-options:<actionType>:<field>`. Segments keep the token
/// charset so a page can never smuggle a URL or path into the resolver.
pub(crate) fn valid_option_source(value: &str) -> bool {
    if value.len() > 256 {
        return false;
    }
    if valid_token(value) {
        return true;
    }
    let Some(rest) = value.strip_prefix("plugin-action-options:") else {
        return false;
    };
    let Some((action_type, field)) = rest.split_once(':') else {
        return false;
    };
    !field.contains(':') && valid_token(action_type) && valid_option_field(field)
}

fn valid_option_field(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z' | 'A'..='Z'))
        && value.len() <= 128
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

pub(crate) fn valid_setting_key(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z' | 'A'..='Z'))
        && value.len() <= 64
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

pub(crate) fn is_primitive_setting(value: &Value) -> bool {
    match value {
        Value::String(value) => value.len() <= 4_096,
        Value::Number(_) | Value::Bool(_) => true,
        _ => false,
    }
}
