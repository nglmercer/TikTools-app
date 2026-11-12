//! Raw transport values: policy checks precede parsing private metadata.
use crate::{
    ApiError, ControlApi, RequestContext, RequestMetadata, RpcId, RpcRequest, RpcResponse,
};
use serde_json::Value;

impl ControlApi {
    /// Parses one raw JSON value (a stdio/IPC line) and executes it.
    pub async fn execute_value(&self, raw: &Value) -> RpcResponse {
        self.execute_value_with_context(&RequestContext::LocalOwner, raw)
            .await
    }

    pub async fn execute_value_with_context(
        &self,
        context: &RequestContext,
        raw: &Value,
    ) -> RpcResponse {
        let id = RpcId::extract(raw);
        let Some(object) = raw.as_object() else {
            return RpcResponse::error(id, ApiError::invalid_params("request must be an object"));
        };
        if let Some(version) = object.get("jsonrpc").and_then(Value::as_str) {
            if version != "2.0" {
                return RpcResponse::error(id, ApiError::invalid_params("jsonrpc must be \"2.0\""));
            }
        }
        let method = object
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if method.is_empty() || method.len() > 128 {
            return RpcResponse::error(
                id,
                ApiError::invalid_params("method must be 1..=128 characters"),
            );
        }
        // Preserve policy-before-validation for hidden methods, including a
        // malformed metadata envelope supplied by a remote caller.
        if let RequestContext::Remote(session) = context {
            if let Err(error) = self.authorize_remote(session, method) {
                return RpcResponse::error(id, error);
            }
        }
        let meta = match object.get("meta").filter(|value| !value.is_null()) {
            Some(value) => match serde_json::from_value::<RequestMetadata>(value.clone()) {
                Ok(meta) => Some(meta),
                Err(_) => {
                    return RpcResponse::error(
                        id,
                        ApiError::invalid_params("invalid request metadata"),
                    )
                }
            },
            None => None,
        };
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        self.execute_with_context(
            context,
            RpcRequest {
                jsonrpc: None,
                id,
                method: method.to_owned(),
                params,
                meta,
            },
        )
        .await
    }
}
