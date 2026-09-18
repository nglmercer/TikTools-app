use std::sync::Arc;

use thiserror::Error;

use crate::{
    ipc::messages::{IpcMessageError, PageMessage},
    AppCore,
};

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("could not parse page message: {0}")]
    Message(#[from] IpcMessageError),
}

/// Compatibility layer for the legacy `{"type": ...}` WebView messages.
///
/// Every client (Vue, CLI, agents) speaks JSON-RPC through `ControlApi` now;
/// this router only exists so older frontends keep working during the
/// migration. Each legacy message is an adapter over the same authoritative
/// `AppCore` control operations (`handle_page_message` implements no
/// business logic of its own). Do not add new `PageMessage` variants.
#[derive(Clone)]
pub struct IpcRouter {
    core: Arc<AppCore>,
}

impl IpcRouter {
    pub fn new(core: Arc<AppCore>) -> Self {
        Self { core }
    }

    pub async fn dispatch(&self, raw: &str) -> Result<(), IpcError> {
        let message = PageMessage::parse(raw)?;
        tracing::debug!(message = %message, "WebView IPC message received");
        self.core.handle_page_message(message).await;
        Ok(())
    }

    /// Dispatches an already-parsed payload, so the WebView router parses
    /// inbound JSON exactly once. The caller already size-checked `raw`.
    pub async fn dispatch_value(&self, value: &serde_json::Value) -> Result<(), IpcError> {
        let message = PageMessage::parse_value(value)?;
        tracing::debug!(message = %message, "WebView IPC message received");
        self.core.handle_page_message(message).await;
        Ok(())
    }
}
