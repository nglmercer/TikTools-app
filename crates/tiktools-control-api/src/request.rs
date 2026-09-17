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
