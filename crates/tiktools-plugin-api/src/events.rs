//! Stable event contracts shared by the host and event-subscribing plugins.
//!
//! The host deliberately converts its internal domain enum into this small
//! JSON envelope before crossing a plugin boundary.  Plugin authors therefore
//! depend on a versioned topic/data shape instead of TikTools' Rust event ABI.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Serialized domain event delivered to a plugin that declares
/// `events.subscribe`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainEventEnvelope {
    pub topic: String,
    pub data: Value,
}

impl DomainEventEnvelope {
    pub fn new(topic: impl Into<String>, data: Value) -> Self {
        Self {
            topic: topic.into(),
            data,
        }
    }
}

/// Matches one manifest subscription against one serialized domain topic.
/// Supported forms are `*`, an exact topic, and a namespace wildcard such as
/// `live.*`.  A namespace wildcard matches descendants below the dot, not the
/// namespace label by itself.
pub fn event_subscription_matches(subscription: &str, topic: &str) -> bool {
    subscription == "*"
        || subscription == topic
        || subscription.strip_suffix(".*").is_some_and(|prefix| {
            !prefix.is_empty()
                && topic
                    .strip_prefix(prefix)
                    .is_some_and(|remainder| remainder.starts_with('.'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_is_stable_json() {
        let envelope = DomainEventEnvelope::new("points.changed", serde_json::json!({"delta": 1}));
        assert_eq!(
            serde_json::to_value(&envelope).unwrap(),
            serde_json::json!({"topic": "points.changed", "data": {"delta": 1}})
        );
    }

    #[test]
    fn subscriptions_match_exact_and_namespace_wildcards() {
        assert!(event_subscription_matches("*", "live.event"));
        assert!(event_subscription_matches("live.*", "live.event"));
        assert!(event_subscription_matches("live.*", "live.reconnecting"));
        assert!(event_subscription_matches("live.event", "live.event"));
        assert!(!event_subscription_matches("live.*", "points.changed"));
        assert!(!event_subscription_matches("live.*", "live"));
    }
}
