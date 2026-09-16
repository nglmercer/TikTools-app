use serde_json::{Map, Value};

use super::DatabaseError;

pub(crate) fn required_value_string(
    object: &Map<String, Value>,
    key: &str,
) -> Result<String, DatabaseError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| DatabaseError::Invalid(format!("record field `{key}` is missing")))
}
pub(crate) fn insert_optional_string(
    object: &mut Map<String, Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        object.insert(key.to_owned(), Value::String(value));
    }
}
pub(crate) fn bool_int(value: bool) -> i64 {
    i64::from(value)
}
pub(crate) fn now() -> i64 {
    chrono_like_now()
}
fn chrono_like_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}
