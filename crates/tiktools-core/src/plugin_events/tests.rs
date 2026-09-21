//! Plugin event observer unit tests.

use crate::events::DomainEvent;
use tiktools_plugin_api::DomainEventEnvelope;

#[test]
fn stable_domain_payload_does_not_expose_the_internal_enum() {
    let event = DomainEvent::PointsChanged {
        unique_id: "viewer".to_owned(),
        delta: 2.0,
        total_points: 4.0,
        level: 1,
    };
    let envelope = DomainEventEnvelope::new(
        event.topic(),
        serde_json::to_value(&event).unwrap()["data"].clone(),
    );
    assert_eq!(envelope.topic, "points.changed");
    assert_eq!(envelope.data["uniqueId"], "viewer");
    assert!(serde_json::to_value(envelope)
        .unwrap()
        .get("topic")
        .is_some());
}
