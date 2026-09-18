use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC request id: string, number, or null (notification-style).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RpcId {
    String(String),
    Number(i64),
    #[default]
    Null,
}

impl RpcId {
    /// Best-effort id extraction for malformed requests, so errors still
    /// correlate. Never fails.
    pub fn extract(raw: &Value) -> Self {
        match raw.get("id") {
            None => Self::Null,
            Some(Value::String(value)) => Self::String(value.chars().take(128).collect()),
            Some(Value::Number(value)) => value.as_i64().map(Self::Number).unwrap_or(Self::Null),
            Some(_) => Self::Null,
        }
    }

    /// Best-effort id recovery from a raw payload that is too large (or
    /// too broken) to fully parse: scans a bounded prefix for a top-level
    /// `"id"` key so `too_large`/invalid-JSON errors still correlate with
    /// the pending call instead of hanging it until timeout. Our clients
    /// emit `{"jsonrpc":"2.0","id":<n>,...}`, so the id always sits within
    /// the first few dozen bytes. Never fails; returns `Null` when no id
    /// is recoverable.
    pub fn extract_from_prefix(raw: &str) -> Self {
        const SCAN_BYTES: usize = 256;
        let bytes = raw.as_bytes();
        let end = bytes.len().min(SCAN_BYTES);
        let mut index = 0;
        let mut in_string = false;
        let mut escaped = false;
        // Brace depth so a nested `"id"` (e.g. inside params) is never
        // mistaken for the request id; top-level keys sit at depth 1.
        let mut depth: u32 = 0;
        while index < end {
            let byte = bytes[index];
            if in_string {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    in_string = false;
                }
                index += 1;
                continue;
            }
            match byte {
                b'{' | b'[' => {
                    depth = depth.saturating_add(1);
                    index += 1;
                }
                b'}' | b']' => {
                    depth = depth.saturating_sub(1);
                    index += 1;
                }
                b'"' => {
                    if depth == 1 && bytes[index..].starts_with(b"\"id\"") {
                        let mut cursor = index + 4;
                        while cursor < end && bytes[cursor].is_ascii_whitespace() {
                            cursor += 1;
                        }
                        if bytes.get(cursor) != Some(&b':') {
                            // `"id"` as a bare string value, not a key.
                            in_string = true;
                            index += 1;
                            continue;
                        }
                        cursor += 1;
                        while cursor < end && bytes[cursor].is_ascii_whitespace() {
                            cursor += 1;
                        }
                        return Self::parse_prefix_value(bytes, cursor, end);
                    }
                    in_string = true;
                    index += 1;
                }
                _ => {
                    index += 1;
                }
            }
        }
        Self::Null
    }

    /// Parses the value after a prefix-matched `"id":`. A value that runs
    /// into the scan boundary is incomplete and untrustworthy, so it
    /// yields `Null` rather than a corrupt id.
    fn parse_prefix_value(bytes: &[u8], start: usize, end: usize) -> Self {
        let Some(&first) = bytes.get(start) else {
            return Self::Null;
        };
        if first == b'"' {
            let mut value: Vec<u8> = Vec::new();
            let mut cursor = start + 1;
            let mut escaped = false;
            while cursor < end {
                let byte = bytes[cursor];
                if escaped {
                    escaped = false;
                    if value.len() < 128 {
                        value.push(byte);
                    }
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    let text = String::from_utf8_lossy(&value);
                    return Self::String(text.chars().take(128).collect());
                } else if value.len() < 128 {
                    value.push(byte);
                }
                cursor += 1;
            }
            return Self::Null;
        }
        if first == b'-' || first.is_ascii_digit() {
            let mut cursor = start;
            if bytes[cursor] == b'-' {
                cursor += 1;
            }
            while cursor < end && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            if cursor == end {
                return Self::Null;
            }
            return bytes
                .get(start..cursor)
                .and_then(|slice| std::str::from_utf8(slice).ok())
                .and_then(|text| text.parse::<i64>().ok())
                .map(Self::Number)
                .unwrap_or(Self::Null);
        }
        Self::Null
    }
}

/// JSON-RPC-style request. `params` is always an object at the handler
/// boundary (`null`/missing normalizes to `{}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: RpcId,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl RpcRequest {
    pub fn new(id: RpcId, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: Some("2.0".to_owned()),
            id,
            method: method.into(),
            params,
        }
    }

    /// Approximate serialized size of the params payload for limit checks.
    pub fn params_size(&self) -> usize {
        match &self.params {
            Value::Null => 0,
            Value::String(value) => value.len(),
            other => serde_json::to_vec(other)
                .map(|bytes| bytes.len())
                .unwrap_or(usize::MAX),
        }
    }

    /// Object form handed to handlers (`null` becomes `{}`).
    pub fn params_object(&self) -> Value {
        match &self.params {
            Value::Null => Value::Object(Default::default()),
            other => other.clone(),
        }
    }
}

#[cfg(test)]
mod prefix_id_tests {
    use super::RpcId;

    #[test]
    fn recovers_number_and_string_ids() {
        assert_eq!(
            RpcId::extract_from_prefix(r#"{"jsonrpc":"2.0","id":42,"method":"system.ping"}"#),
            RpcId::Number(42)
        );
        assert_eq!(
            RpcId::extract_from_prefix(r#"{"id" : "abc-1" ,"method":"x"}"#),
            RpcId::String("abc-1".to_owned())
        );
        assert_eq!(
            RpcId::extract_from_prefix(r#"{"id":-7,"method":"x"}"#),
            RpcId::Number(-7)
        );
    }

    #[test]
    fn ignores_nested_and_decoy_ids() {
        // A nested `"id"` inside params must never win over the top-level
        // one, and an `"id"` substring inside a string value must not
        // match at all.
        assert_eq!(
            RpcId::extract_from_prefix(
                r#"{"jsonrpc":"2.0","id":9,"params":{"id":1,"note":"\"id\":2"}}"#
            ),
            RpcId::Number(9)
        );
        assert_eq!(
            RpcId::extract_from_prefix(r#"{"method":"x","params":{"id":3}}"#),
            RpcId::Null
        );
        assert_eq!(
            RpcId::extract_from_prefix(r#"{"note":"id","id":5}"#),
            RpcId::Number(5)
        );
    }

    #[test]
    fn falls_back_to_null_when_unrecoverable() {
        assert_eq!(RpcId::extract_from_prefix("not json at all"), RpcId::Null);
        assert_eq!(RpcId::extract_from_prefix(r#"{"method":"x"}"#), RpcId::Null);
        assert_eq!(RpcId::extract_from_prefix(r#"{"id":null}"#), RpcId::Null);
        assert_eq!(RpcId::extract_from_prefix(r#"{"id":true}"#), RpcId::Null);
        // An id pushed beyond the scan bound is unrecoverable by design.
        let padding = " ".repeat(300);
        assert_eq!(
            RpcId::extract_from_prefix(&format!("{{{padding}\"id\":12}}")),
            RpcId::Null
        );
    }
}
