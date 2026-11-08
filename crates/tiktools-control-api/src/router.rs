use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

use serde_json::Value;
use tiktools_core::AppCore;

use crate::{error::ApiError, registry::MethodMeta};

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type HandlerFn = Arc<
    dyn Fn(
            Arc<AppCore>,
            crate::RequestContext,
            Value,
            Option<crate::RequestMetadata>,
        ) -> BoxFuture<Result<HandlerResult, ApiError>>
        + Send
        + Sync,
>;

pub struct HandlerResult {
    pub value: Value,
    pub resource_versions: Option<Vec<tiktools_core::collaboration::ResourceVersion>>,
    pub command: Option<tiktools_core::collaboration::CommandReceipt>,
    pub host_instance_id: Option<String>,
}

impl From<Value> for HandlerResult {
    fn from(value: Value) -> Self {
        Self {
            value,
            resource_versions: None,
            command: None,
            host_instance_id: None,
        }
    }
}

pub struct MethodEntry {
    pub meta: MethodMeta,
    pub handler: HandlerFn,
    pub remote_policy: crate::policy::RemotePolicy,
}

/// Registration-based router: no giant `match`. Each domain module owns its
/// `register` function.
#[derive(Default)]
pub struct ControlRouter {
    methods: BTreeMap<String, MethodEntry>,
}

impl ControlRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one typed method. Params deserialize into `P`
    /// (`invalid_params` on mismatch) and `R` serializes the result. JSON
    /// Schemas for both are captured for `rpc.discover` / `rpc.schema`.
    pub fn register_typed<P, R, F, Fut>(
        &mut self,
        name: &str,
        description: &str,
        side_effect: bool,
        handler: F,
    ) where
        P: serde::de::DeserializeOwned + schemars::JsonSchema,
        R: serde::Serialize + schemars::JsonSchema,
        F: Fn(Arc<AppCore>, P) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, ApiError>> + Send + 'static,
    {
        self.register_typed_full(name, description, side_effect, false, false, handler);
    }

    /// Registers one typed method with agent-facing risk flags
    /// (`destructive`, `requires_desktop` in `rpc.discover`).
    pub fn register_typed_full<P, R, F, Fut>(
        &mut self,
        name: &str,
        description: &str,
        side_effect: bool,
        destructive: bool,
        requires_desktop: bool,
        handler: F,
    ) where
        P: serde::de::DeserializeOwned + schemars::JsonSchema,
        R: serde::Serialize + schemars::JsonSchema,
        F: Fn(Arc<AppCore>, P) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, ApiError>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let wrapped: HandlerFn = Arc::new(move |core, _context, params, _metadata| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                let params: P = serde_json::from_value(params)
                    .map_err(|error| ApiError::invalid_params(error.to_string()))?;
                let result = handler(core, params).await?;
                serde_json::to_value(result)
                    .map(HandlerResult::from)
                    .map_err(|error| ApiError::internal(error.to_string()))
            }) as BoxFuture<_>
        });
        let params_schema = serde_json::to_value(schemars::schema_for!(P)).unwrap_or(Value::Null);
        let result_schema = serde_json::to_value(schemars::schema_for!(R)).unwrap_or(Value::Null);
        self.methods.insert(
            name.to_owned(),
            MethodEntry {
                remote_policy: crate::policy::RemotePolicy::Deny,
                meta: MethodMeta {
                    name: name.to_owned(),
                    description: description.to_owned(),
                    side_effect,
                    destructive,
                    requires_desktop,
                    concurrency: None,
                    durable_commands: false,
                    params_schema,
                    result_schema,
                },
                handler: wrapped,
            },
        );
    }

    /// Overrides the agent-facing risk flags of a registered method.
    /// Unknown names are ignored so flag lists stay forward-compatible.
    pub fn set_flags(&mut self, name: &str, destructive: bool, requires_desktop: bool) {
        if let Some(entry) = self.methods.get_mut(name) {
            entry.meta.destructive = destructive;
            entry.meta.requires_desktop = requires_desktop;
        }
    }

    pub fn get(&self, method: &str) -> Option<&MethodEntry> {
        self.methods.get(method)
    }

    /// Transport identity is provided by trusted host code, never params.
    pub fn register_contextual<P, R, F, Fut>(&mut self, name: &str, description: &str, handler: F)
    where
        P: serde::de::DeserializeOwned + schemars::JsonSchema,
        R: serde::Serialize + schemars::JsonSchema,
        F: Fn(Arc<AppCore>, crate::RequestContext, P) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, ApiError>> + Send + 'static,
    {
        self.register_typed::<P, R, _, _>(name, description, false, |_, _| async {
            Err(ApiError::new(
                "unsupported_remote",
                "remote context is required",
            ))
        });
        let handler = Arc::new(handler);
        self.methods
            .get_mut(name)
            .expect("registered above")
            .handler = Arc::new(move |core, context, params, _metadata| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                let params = serde_json::from_value::<P>(params)
                    .map_err(|error| ApiError::invalid_params(error.to_string()))?;
                let result = handler(core, context, params).await?;
                serde_json::to_value(result)
                    .map(HandlerResult::from)
                    .map_err(|_| ApiError::internal("response serialization failed"))
            })
        });
    }

    /// Domain payload and versions originate from the same database operation.
    /// Remote policy remains denied until durable commands and effect policy exist.
    pub fn register_versioned<P, R, F, Fut>(
        &mut self,
        name: &str,
        description: &str,
        side_effect: bool,
        handler: F,
    ) where
        P: serde::de::DeserializeOwned + schemars::JsonSchema,
        R: serde::Serialize + schemars::JsonSchema,
        F: Fn(Arc<AppCore>, crate::RequestContext, P, Option<crate::RequestMetadata>) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = Result<tiktools_core::collaboration::Versioned<R>, ApiError>>
            + Send
            + 'static,
    {
        self.register_typed::<P, R, _, _>(name, description, side_effect, |_, _| async {
            Err(ApiError::internal("versioned handler is unavailable"))
        });
        let entry = self.methods.get_mut(name).expect("registered above");
        if side_effect {
            entry.meta.concurrency = Some(crate::registry::ConcurrencyStrategy::CompareAndSet);
        }
        let handler = Arc::new(handler);
        entry.handler = Arc::new(move |core, context, params, metadata| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                let params = serde_json::from_value::<P>(params)
                    .map_err(|error| ApiError::invalid_params(error.to_string()))?;
                let result = handler(core, context, params, metadata).await?;
                Ok(HandlerResult {
                    value: serde_json::to_value(result.value)
                        .map_err(|_| ApiError::internal("response serialization failed"))?,
                    resource_versions: Some(result.resource_versions),
                    command: result.command,
                    host_instance_id: result.host_instance_id,
                })
            })
        });
    }

    /// Declare support only after the domain commits commands with its writes.
    pub fn set_durable_commands(&mut self, name: &str, enabled: bool) {
        if let Some(entry) = self.methods.get_mut(name) {
            entry.meta.durable_commands = enabled;
        }
    }

    pub fn set_concurrency(&mut self, name: &str, strategy: crate::registry::ConcurrencyStrategy) {
        if let Some(entry) = self.methods.get_mut(name) {
            entry.meta.concurrency = Some(strategy);
        }
    }

    /// Legacy local operations may return optimistic cache state. An empty version
    /// vector in those handlers must not masquerade as a durable projection.
    /// Remote reads and real nonempty versions retain the normal envelope.
    pub fn omit_unversioned_owner_metadata(&mut self, name: &str) {
        if let Some(entry) = self.methods.get_mut(name) {
            let original = entry.handler.clone();
            entry.handler = Arc::new(move |core, context, params, metadata| {
                let original = original.clone();
                let owner = matches!(context, crate::RequestContext::LocalOwner);
                Box::pin(async move {
                    let mut result = original(core, context, params, metadata).await?;
                    if owner
                        && result.command.is_none()
                        && result.resource_versions.as_ref().is_some_and(Vec::is_empty)
                    {
                        result.resource_versions = None;
                    }
                    Ok(result)
                })
            });
        }
    }

    /// Explicit host policy; new methods remain denied unless classified.
    pub fn set_remote_policy(&mut self, name: &str, policy: crate::policy::RemotePolicy) {
        if let Some(entry) = self.methods.get_mut(name) {
            entry.remote_policy = policy;
        }
    }

    pub fn metadata(&self) -> Vec<MethodMeta> {
        self.methods
            .values()
            .map(|entry| entry.meta.clone())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.methods.len()
    }

    pub fn is_empty(&self) -> bool {
        self.methods.is_empty()
    }
}
