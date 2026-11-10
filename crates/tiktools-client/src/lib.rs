//! Typed Rust SDK over the TikTools control API.
//!
//! Every client speaks the same surface:
//!
//! ```text
//! TikToolsClient -> ControlApi -> AppCore        (direct, in-process)
//! TikToolsClient -> IPC -> ControlApi -> AppCore (connected, local socket)
//! TikToolsClient -> HTTPS/WSS -> ControlApi      (optional remote, no client core)
//! ```
//!
//! Typed methods mirror the registry one-to-one: each RPC name has one
//! method taking typed params and returning a typed result. The parity
//! test pins [`TikToolsClient::covered_methods`] to the registry in both
//! directions, so a new RPC method without a typed wrapper fails CI.

pub mod analytics;
pub mod app;
pub mod automation;
pub mod automation_adoption;
pub mod automation_approval;
pub mod automation_definition;
pub mod automation_effects;
pub mod automation_library;
#[cfg(feature = "cloud-auth")]
pub mod cloud;
pub mod collaboration;
pub mod competitions;
pub mod creators;
pub mod gifts;
pub mod globals;
#[cfg(feature = "cloud-auth")]
pub mod integrations;
pub mod live;
pub mod media;
pub mod packages;
pub mod plugins;
pub mod points;
pub mod processors;
#[cfg(feature = "cloud-auth")]
pub mod publishers;
pub mod rankings;
pub mod registry;
#[cfg(feature = "remote")]
pub mod remote;
pub mod rpc;
pub mod settings;
#[cfg(feature = "cloud-auth")]
pub mod sync;
pub mod system;
#[cfg(feature = "cloud-auth")]
pub mod teams;
pub mod ui;
pub mod updates;
pub mod validate;
pub mod widgets;
pub mod workflows;

pub use tiktools_control_api::{
    CallResult, ClientError, ControlApi, ControlClient, ControlEvent, MethodMeta, RequestMetadata,
    ResponseMetadata,
};
pub use validate::validate_params;

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
/// With `remote`, `Remote` uses an authenticated HTTPS/WSS connection. Remote
/// identity and permissions are always enforced by that host; no local fallback
/// or automatic business-call retry is performed.
#[derive(Clone)]
pub enum TikToolsClient {
    Direct(Arc<ControlApi>),
    Connected(ControlClient),
    #[cfg(feature = "remote")]
    Remote(remote::RemoteClient),
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
            #[cfg(feature = "remote")]
            Self::Remote(client) => client.subscribe(),
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
        self.call_value_with_meta(method, params, None)
            .await
            .map(|response| response.result)
    }

    pub async fn call_with_meta<P, R>(
        &self,
        method: &str,
        params: P,
        meta: Option<RequestMetadata>,
    ) -> Result<CallResult<R>, ClientError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let response = self.call_value_with_meta(method, params, meta).await?;
        Ok(CallResult {
            result: serde_json::from_value(response.result)
                .map_err(|_| ClientError::new("protocol", "bad result shape"))?,
            meta: response.meta,
        })
    }

    pub async fn call_value_with_meta<P>(
        &self,
        method: &str,
        params: P,
        meta: Option<RequestMetadata>,
    ) -> Result<CallResult<Value>, ClientError>
    where
        P: Serialize,
    {
        match self {
            Self::Direct(api) => call_direct(api, method, params, meta).await,
            Self::Connected(client) => client.call_value_with_meta(method, params, meta).await,
            #[cfg(feature = "remote")]
            Self::Remote(client) => client.call_value_with_meta(method, params, meta).await,
        }
    }

    /// Every RPC name the typed surface covers. The parity test asserts
    /// this set equals the registry exactly, in both directions.
    pub fn covered_methods() -> Vec<&'static str> {
        [
            analytics::METHODS,
            app::METHODS,
            automation::METHODS,
            automation_adoption::METHODS,
            automation_approval::METHODS,
            automation_definition::METHODS,
            automation_effects::METHODS,
            automation_library::METHODS,
            #[cfg(feature = "cloud-auth")]
            cloud::METHODS,
            creators::METHODS,
            competitions::METHODS,
            collaboration::METHODS,
            gifts::METHODS,
            globals::METHODS,
            live::METHODS,
            media::METHODS,
            packages::METHODS,
            plugins::METHODS,
            #[cfg(feature = "cloud-auth")]
            publishers::METHODS,
            #[cfg(feature = "cloud-auth")]
            integrations::METHODS,
            #[cfg(feature = "cloud-auth")]
            teams::METHODS,
            points::METHODS,
            rankings::METHODS,
            registry::METHODS,
            processors::METHODS,
            rpc::METHODS,
            settings::METHODS,
            #[cfg(feature = "cloud-auth")]
            sync::METHODS,
            system::METHODS,
            ui::METHODS,
            updates::METHODS,
            widgets::METHODS,
            workflows::METHODS,
        ]
        .concat()
    }
}

/// Executes one typed call against an in-process [`ControlApi`],
/// mapping the RPC response onto the same [`ClientError`] shape the
/// IPC backend produces so callers never branch on the backend.
async fn call_direct<P>(
    api: &ControlApi,
    method: &str,
    params: P,
    meta: Option<RequestMetadata>,
) -> Result<CallResult<Value>, ClientError>
where
    P: Serialize,
{
    let params = serde_json::to_value(params)
        .map_err(|error| ClientError::new("protocol", error.to_string()))?;
    let id = NEXT_DIRECT_ID.fetch_add(1, Ordering::Relaxed);
    let mut request = RpcRequest::new(RpcId::Number(id), method, params);
    request.meta = meta;
    let response = api.execute(request).await;
    if let Some(error) = response.error {
        return Err(ClientError {
            code: error.code,
            message: error.message,
            data: error.data,
        });
    }
    Ok(CallResult {
        result: response.result.unwrap_or(Value::Null),
        meta: response.meta,
    })
}
