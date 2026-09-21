use serde::Serialize;
use serde_json::Value;

use crate::{error::ApiError, request::RpcId};

/// JSON-RPC-style response: exactly one of `result` / `error` is present.
#[derive(Debug, Clone, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    pub id: RpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErrorBody>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcResponse {
    pub fn success(id: RpcId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: RpcId, error: ApiError) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcErrorBody {
                code: error.code,
                message: error.message,
                data: error.data,
            }),
        }
    }

    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }
}
