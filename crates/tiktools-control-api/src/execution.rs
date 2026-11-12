//! Request validation, deadlines, handler dispatch and response reauthorization.
use crate::{
    context, modules, owner_params_bytes, policy, registry, request_timeout_for, ApiError,
    ControlApi, RequestContext, ResponseMetadata, RpcRequest, RpcResponse, MAX_PARAMS_BYTES,
};

mod raw;

impl ControlApi {
    /// Executes one validated request.
    pub async fn execute(&self, request: RpcRequest) -> RpcResponse {
        self.execute_with_context(&RequestContext::LocalOwner, request)
            .await
    }

    /// Remote gateways MUST use this path with a transport-authenticated
    /// session. Identity and grants cannot come from request params.
    pub async fn execute_with_context(
        &self,
        context: &RequestContext,
        request: RpcRequest,
    ) -> RpcResponse {
        let mut response = self.execute_authorized(context, request).await;
        if matches!(context, RequestContext::Remote(_)) {
            if let Some(error) = &mut response.error {
                policy::redact_remote_failure(error);
            }
        }
        response
    }

    pub(super) async fn execute_authorized(
        &self,
        context: &RequestContext,
        request: RpcRequest,
    ) -> RpcResponse {
        if let RequestContext::Remote(session) = context {
            let authorization = self.authorize_remote(session, &request.method);
            if let Err(error) = authorization {
                return RpcResponse::error(request.id, error);
            }
        }
        if let Some(version) = request.jsonrpc.as_deref() {
            if version != "2.0" {
                return RpcResponse::error(
                    request.id,
                    ApiError::invalid_params("jsonrpc must be \"2.0\""),
                );
            }
        }
        let Some(entry) = self.router.get(&request.method) else {
            return RpcResponse::error(request.id, ApiError::method_not_found(&request.method));
        };
        if let Some(meta) = &request.meta {
            // Never acknowledge command metadata that a method cannot enforce.
            if (meta.command_id.is_some() || meta.host_instance_id.is_some())
                && !entry.meta.durable_commands
            {
                return RpcResponse::error(
                    request.id,
                    ApiError::new(
                        "unsupported_remote",
                        "this method does not support durable commands",
                    ),
                );
            }
            if (meta.command_id.is_some() || meta.host_instance_id.is_some())
                && (meta.command_id.is_none()
                    || meta.host_instance_id.is_none()
                    || (entry.meta.concurrency
                        == Some(registry::ConcurrencyStrategy::CompareAndSet)
                        && meta.expected_revision.is_none()))
            {
                return RpcResponse::error(
                    request.id,
                    ApiError::invalid_params(
                        "commandId and hostInstanceId are required; compare-and-set also requires expectedRevision",
                    ),
                );
            }
            if meta.expected_revision.is_some()
                && entry.meta.concurrency != Some(registry::ConcurrencyStrategy::CompareAndSet)
            {
                return RpcResponse::error(
                    request.id,
                    ApiError::invalid_params("this method does not accept expectedRevision"),
                );
            }
        }
        let params_limit = if matches!(context, RequestContext::LocalOwner) {
            owner_params_bytes(&request.method)
        } else {
            MAX_PARAMS_BYTES
        };
        if request.params_size() > params_limit {
            return RpcResponse::error(request.id, ApiError::too_large());
        }
        let params = request.params_object();
        if let RequestContext::Remote(session) = context {
            if request.method == "rpc.discover" {
                if let Err(error) = serde_json::from_value::<modules::Empty>(params) {
                    return RpcResponse::error(
                        request.id,
                        ApiError::invalid_params(error.to_string()),
                    );
                }
                let methods: Vec<_> = self
                    .router
                    .metadata()
                    .into_iter()
                    .filter(|meta| {
                        self.router
                            .get(&meta.name)
                            .is_some_and(|_| self.authorize_remote(session, &meta.name).is_ok())
                    })
                    .collect();
                return match session.authorize(None) {
                    Ok(()) => {
                        RpcResponse::success(request.id, serde_json::json!({"methods": methods}))
                    }
                    Err(error) => RpcResponse::error(request.id, error),
                };
            }
            if request.method == "rpc.schema" {
                let params = match serde_json::from_value::<modules::rpc::SchemaParams>(params) {
                    Ok(params) => params,
                    Err(error) => {
                        return RpcResponse::error(
                            request.id,
                            ApiError::invalid_params(error.to_string()),
                        )
                    }
                };
                let Some(entry) = self.router.get(&params.method) else {
                    return RpcResponse::error(request.id, context::forbidden());
                };
                if let Err(error) = self.authorize_remote(session, &params.method) {
                    return RpcResponse::error(request.id, error);
                }
                return match serde_json::to_value(&entry.meta) {
                    Ok(meta) => RpcResponse::success(request.id, meta),
                    Err(_) => RpcResponse::error(
                        request.id,
                        ApiError::internal("schema serialization failed"),
                    ),
                };
            }
        }
        let outcome = tokio::time::timeout(
            request_timeout_for(&request.method),
            (entry.handler)(self.core.clone(), context.clone(), params, request.meta),
        )
        .await;
        // Do not disclose a result after an in-flight revocation/expiry.
        if let RequestContext::Remote(session) = context {
            let authorization = match &outcome {
                Ok(Ok(result)) => {
                    self.authorize_remote_response(session, &request.method, &result.value)
                }
                Ok(Err(error)) => {
                    self.authorize_remote_failure(session, &request.method, error.data.as_ref())
                }
                Err(_) => self.authorize_remote(session, &request.method),
            };
            if let Err(error) = authorization {
                return RpcResponse::error(request.id, error);
            }
        }
        match outcome {
            Ok(Ok(result)) => {
                let mut response = RpcResponse::success(
                    request.id,
                    if matches!(context, RequestContext::Remote(_)) {
                        policy::redact_remote_result(&request.method, result.value)
                    } else {
                        result.value
                    },
                );
                response.meta =
                    result
                        .resource_versions
                        .map(|resource_versions| ResponseMetadata {
                            resource_versions,
                            command: result.command,
                            host_instance_id: result.host_instance_id,
                        });
                response
            }
            Ok(Err(error)) => RpcResponse::error(request.id, error),
            Err(_) => RpcResponse::error(request.id, ApiError::timeout()),
        }
    }
}
