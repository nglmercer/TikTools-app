//! Typed Rust SDK over the TikTools control API.
//!
//! Every client speaks the same surface:
//!
//! ```text
//! TikToolsClient -> ControlApi -> AppCore        (direct, in-process)
//! TikToolsClient -> IPC -> ControlApi -> AppCore (connected, local socket)
//! ```
//!
//! Typed methods mirror the registry one-to-one: each RPC name has one
//! method taking typed params and returning a typed result. The parity
//! test pins [`TikToolsClient::covered_methods`] to the registry in both
//! directions, so a new RPC method without a typed wrapper fails CI.

pub mod analytics;
pub mod app;
pub mod automation;
pub mod creators;
pub mod gifts;
pub mod live;
pub mod media;
pub mod plugins;
pub mod points;
pub mod processors;
pub mod rpc;
pub mod settings;
pub mod system;
pub mod workflows;

pub use tiktools_control_api::{ClientError, ControlApi, ControlClient, ControlEvent, MethodMeta};

use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tiktools_control_api::{RpcId, RpcRequest};

/// Request ids for direct (in-process) calls. Direct calls resolve
/// inline instead of demultiplexing by id, so a process-wide counter is
/// enough to keep ids distinct for tracing.
static NEXT_DIRECT_ID: AtomicI64 = AtomicI64::new(1);

/// Typed client over the whole control surface.
///
/// `Direct` wraps an in-process [`ControlApi`] (same-process hosts,
/// tests); `Connected` wraps an IPC [`ControlClient`] (CLI against a
/// running host). Both backends expose identical typed methods and one
/// [`ControlEvent`] stream shape.
#[derive(Clone)]
pub enum TikToolsClient {
    Direct(Arc<ControlApi>),
    Connected(ControlClient),
}

impl TikToolsClient {
    /// Wraps an in-process control API. The caller owns the host
    /// lifecycle; `ControlApi` stays the only caller of `AppCore`.
    pub fn direct(api: Arc<ControlApi>) -> Self {
        Self::Direct(api)
    }

    /// Connects to the running host's local IPC endpoint.
    pub async fn connect() -> Result<Self, ClientError> {
        ControlClient::connect().await.map(Self::Connected)
    }

    /// Subscribes to the host's event broadcast with one stream type on
    /// both backends: authoritative domain events plus explicit
    /// reliable-gap signals.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ControlEvent> {
        match self {
            Self::Direct(api) => api.subscribe_broadcast(),
            Self::Connected(client) => client.subscribe(),
        }
    }

    /// Generic typed call used by every method below and by generic
    /// agent `call` passthroughs. The method name must be registered;
    /// unknown names fail with `method_not_found` on both backends.
    pub async fn call<P, R>(&self, method: &str, params: P) -> Result<R, ClientError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let result = self.call_value(method, params).await?;
        serde_json::from_value(result)
            .map_err(|error| ClientError::new("protocol", format!("bad result shape: {error}")))
    }

    /// Untyped call returning the raw result value.
    pub async fn call_value<P>(&self, method: &str, params: P) -> Result<Value, ClientError>
    where
        P: Serialize,
    {
        match self {
            Self::Direct(api) => call_direct(api, method, params).await,
            Self::Connected(client) => client.call_value(method, params).await,
        }
    }

    /// Every RPC name the typed surface covers. The parity test asserts
    /// this set equals the registry exactly, in both directions.
    pub fn covered_methods() -> Vec<&'static str> {
        [
            analytics::METHODS,
            app::METHODS,
            automation::METHODS,
            creators::METHODS,
            gifts::METHODS,
            live::METHODS,
            media::METHODS,
            plugins::METHODS,
            points::METHODS,
            processors::METHODS,
            rpc::METHODS,
            settings::METHODS,
            system::METHODS,
            workflows::METHODS,
        ]
        .concat()
    }
}

/// Executes one typed call against an in-process [`ControlApi`],
/// mapping the RPC response onto the same [`ClientError`] shape the
/// IPC backend produces so callers never branch on the backend.
async fn call_direct<P>(api: &ControlApi, method: &str, params: P) -> Result<Value, ClientError>
where
    P: Serialize,
{
    let params = serde_json::to_value(params)
        .map_err(|error| ClientError::new("protocol", error.to_string()))?;
    let id = NEXT_DIRECT_ID.fetch_add(1, Ordering::Relaxed);
    let response = api
        .execute(RpcRequest::new(RpcId::Number(id), method, params))
        .await;
    if let Some(error) = response.error_body() {
        return Err(ClientError::new(error.code.clone(), error.message.clone()));
    }
    Ok(response.result().cloned().unwrap_or(Value::Null))
}
