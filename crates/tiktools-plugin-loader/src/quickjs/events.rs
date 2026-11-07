//! Push events from a guest: the `tiktools:events` module.
//!
//! Every emit is validated here (shape, declared type, size) and
//! re-validated by core (publish grant) before delivery. Validation works on
//! plain JSON so it is independent of the JavaScript engine.

use std::collections::HashSet;

use tokio::sync::broadcast;

use crate::EmittedEvent;

/// Guest module for push events: `import { emit, emitMany } from
/// "tiktools:events"`. Installed only for plugins whose manifest requests
/// publishing.
pub const EVENTS_CAPABILITY: &str = "tiktools:events";

/// Max serialized bytes of one pushed event (type plus data).
/// Hotkey-class events are ~100 bytes; 64 KiB leaves wide headroom while
/// keeping the bounded push bus small under burst.
pub(super) const MAX_EMIT_EVENT_BYTES: usize = 64 * 1024;

/// Max events per `emitMany` batch: bursts stay bounded per call.
pub(super) const MAX_EMIT_BATCH: usize = 64;

/// Per-plugin push state: what the `tiktools:events` functions validate
/// against and where accepted events go.
pub(crate) struct EventsPush {
    pub(super) plugin_id: String,
    pub(super) declared: HashSet<String>,
    pub(super) tx: broadcast::Sender<EmittedEvent>,
}

impl EventsPush {
    /// Validates one pushed event and enqueues it. Failures throw to the
    /// guest, so a misbehaving plugin learns immediately instead of losing
    /// events.
    pub(super) fn push_one(&self, json: &serde_json::Value) -> Result<(), String> {
        let (event_type, data) = self.check(json, "emit")?;
        self.send(event_type, data, "emit")
    }

    /// Validates a batch and enqueues it, returning the accepted count. The
    /// batch is validated fully before anything is sent: a bad batch sends
    /// nothing.
    pub(super) fn push_many(&self, json: &serde_json::Value) -> Result<usize, String> {
        let events = json
            .as_array()
            .ok_or_else(|| "emitMany: expected an array of events".to_owned())?;
        if events.len() > MAX_EMIT_BATCH {
            return Err(format!("emitMany: batch exceeds {MAX_EMIT_BATCH} events"));
        }
        let mut staged = Vec::with_capacity(events.len());
        for event in events {
            staged.push(self.check(event, "emitMany")?);
        }
        for (event_type, data) in staged {
            self.send(event_type, data, "emitMany")?;
        }
        Ok(events.len())
    }

    fn send(
        &self,
        event_type: String,
        data: serde_json::Value,
        caller: &str,
    ) -> Result<(), String> {
        self.tx
            .send(EmittedEvent {
                plugin_id: self.plugin_id.clone(),
                event_type,
                data,
            })
            .map(|_| ())
            .map_err(|_| format!("{caller}: event bus unavailable"))
    }

    /// Shared shape/type/size validation for pushed events: `{ type, data? }`
    /// with a declared type and a bounded payload.
    fn check(
        &self,
        json: &serde_json::Value,
        caller: &str,
    ) -> Result<(String, serde_json::Value), String> {
        let object = json
            .as_object()
            .ok_or_else(|| format!("{caller}: event must be an object"))?;
        let event_type = object
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|kind| !kind.is_empty())
            .ok_or_else(|| format!("{caller}: event needs a non-empty string type"))?;
        if !self.declared.contains(event_type) {
            return Err(format!("{caller}: undeclared event type `{event_type}`"));
        }
        let bytes = serde_json::to_vec(json)
            .map_err(|error| format!("{caller}: event is not serializable: {error}"))?;
        if bytes.len() > MAX_EMIT_EVENT_BYTES {
            return Err(format!(
                "{caller}: event exceeds {MAX_EMIT_EVENT_BYTES} bytes"
            ));
        }
        let data = object
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        Ok((event_type.to_owned(), data))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn push() -> (EventsPush, broadcast::Receiver<EmittedEvent>) {
        let (tx, rx) = broadcast::channel(8);
        (
            EventsPush {
                plugin_id: "p".to_owned(),
                declared: ["push.tick".to_owned()].into_iter().collect(),
                tx,
            },
            rx,
        )
    }

    #[test]
    fn accepts_declared_events_and_rejects_the_rest() {
        let (push, mut rx) = push();
        push.push_one(&json!({"type":"push.tick","data":{"n":1}}))
            .unwrap();
        let got = rx.try_recv().unwrap();
        assert_eq!(got.plugin_id, "p");
        assert_eq!(got.event_type, "push.tick");
        assert_eq!(got.data, json!({"n": 1}));

        for bad in [
            json!({"type":"nope"}),
            json!({"data":{}}),
            json!({"type":""}),
            json!({"type":42}),
            json!(42),
            json!("push.tick"),
            json!(null),
        ] {
            assert!(push.push_one(&bad).is_err(), "{bad} must be rejected");
        }
        assert!(rx.try_recv().is_err(), "rejected emits send nothing");

        let big = "x".repeat(MAX_EMIT_EVENT_BYTES);
        let error = push
            .push_one(&json!({"type":"push.tick","data":{"s":big}}))
            .unwrap_err();
        assert!(error.contains("exceeds"), "{error}");
    }

    #[test]
    fn batches_validate_before_sending_anything() {
        let (push, mut rx) = push();
        let count = push
            .push_many(&json!([{"type":"push.tick"},{"type":"push.tick","data":2}]))
            .unwrap();
        assert_eq!(count, 2);
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_ok());

        assert!(push
            .push_many(&json!([{"type":"push.tick"},{"type":"nope"}]))
            .is_err());
        assert!(rx.try_recv().is_err());

        assert!(push.push_many(&json!({"type":"push.tick"})).is_err());
        let long = serde_json::Value::Array(vec![json!({"type":"push.tick"}); MAX_EMIT_BATCH + 1]);
        let error = push.push_many(&long).unwrap_err();
        assert!(error.contains("exceeds"), "{error}");
    }

    #[test]
    fn push_without_subscribers_fails_loudly() {
        let (tx, rx) = broadcast::channel(1);
        drop(rx);
        let push = EventsPush {
            plugin_id: "p".to_owned(),
            declared: ["push.tick".to_owned()].into_iter().collect(),
            tx,
        };
        let error = push.push_one(&json!({"type":"push.tick"})).unwrap_err();
        assert!(error.contains("unavailable"), "{error}");
    }
}
