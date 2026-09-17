use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

use serde_json::Value;
use tiktools_core::AppCore;

use crate::{error::ApiError, registry::MethodMeta};

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type HandlerFn =
    Arc<dyn Fn(Arc<AppCore>, Value) -> BoxFuture<Result<Value, ApiError>> + Send + Sync>;

pub struct MethodEntry {
    pub meta: MethodMeta,
    pub handler: HandlerFn,
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
        let handler = Arc::new(handler);
        let wrapped: HandlerFn = Arc::new(move |core, params| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                let params: P = serde_json::from_value(params)
                    .map_err(|error| ApiError::invalid_params(error.to_string()))?;
                let result = handler(core, params).await?;
                serde_json::to_value(result).map_err(|error| ApiError::internal(error.to_string()))
            }) as BoxFuture<_>
        });
        let params_schema = serde_json::to_value(schemars::schema_for!(P)).unwrap_or(Value::Null);
        let result_schema = serde_json::to_value(schemars::schema_for!(R)).unwrap_or(Value::Null);
        self.methods.insert(
            name.to_owned(),
            MethodEntry {
                meta: MethodMeta {
                    name: name.to_owned(),
                    description: description.to_owned(),
                    side_effect,
                    params_schema,
                    result_schema,
                },
                handler: wrapped,
            },
        );
    }

    pub fn get(&self, method: &str) -> Option<&MethodEntry> {
        self.methods.get(method)
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
