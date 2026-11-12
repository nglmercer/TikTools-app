//! Transport-neutral event/gap envelopes and domain subscription adapters.
use crate::{ControlApi, ControlEvent};
use serde_json::Value;

impl ControlApi {
    /// Subscribes to the domain event bus (reliable + lossy lanes,
    /// reliable drained first). Each event maps to one JSON-RPC
    /// notification via [`event_notification`].
    pub fn subscribe(&self) -> tiktools_core::events::DomainSubscription {
        self.core.events.subscribe_domain()
    }

    /// Subscribes to the domain event bus through the shared
    /// [`ControlEvent`] broadcast shape, so direct (in-process) and
    /// connected (IPC) subscribers consume one stream type.
    pub fn subscribe_broadcast(&self) -> tokio::sync::broadcast::Receiver<ControlEvent> {
        crate::client::bridge_subscription(self.subscribe())
    }
}

/// Maps one domain event onto the JSON-RPC notification envelope:
/// `{ jsonrpc, method: "event", params: { topic, data } }`.
pub fn event_notification(event: &tiktools_core::events::DomainEvent) -> Value {
    // The notification params carry the event's own `{topic, data}` serde
    // form (unit variants serialize without a `data` key); only a
    // conversion failure substitutes the explicit error marker below.
    match serde_json::to_value(event) {
        Ok(params) => serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": params,
        }),
        Err(error) => {
            tracing::error!(topic = event.topic(), %error, "domain event failed envelope conversion");
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "event",
                "params": {
                    "topic": event.topic(),
                    "data": { "error": "event_serialization_failed" },
                },
            })
        }
    }
}

/// Maps a reliable-lane lag onto the JSON-RPC gap notification envelope:
/// `{ jsonrpc, method: "event.gap", params: { lost, resync: true } }`.
/// `lost` counts the skipped authoritative events. The stream is no
/// longer complete: the client must refresh authoritative state instead
/// of reconstructing the missing events.
pub fn gap_notification(lost: u64) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "event.gap",
        "params": {
            "lost": lost,
            "resync": true,
        },
    })
}
