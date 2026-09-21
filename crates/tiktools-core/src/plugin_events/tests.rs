//! Plugin event observer unit tests.

use super::queue::{WorkerHandle, PLUGIN_EVENT_QUEUE_CAPACITY};
use super::supervisor::reconcile_workers;
use crate::events::DomainEvent;
use crate::AppCore;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tiktools_plugin_api::DomainEventEnvelope;

struct NoopEmitter;

impl crate::HostEmitter for NoopEmitter {
    fn emit(&self, _message: crate::ipc::messages::HostMessage) {}
}

fn worker_handle() -> WorkerHandle {
    let (sender, _) = tokio::sync::mpsc::channel(PLUGIN_EVENT_QUEUE_CAPACITY);
    WorkerHandle {
        sender,
        token: tokio_util::sync::CancellationToken::new(),
        pending_reliable_gaps: Arc::new(AtomicU64::new(0)),
    }
}

#[test]
fn reconcile_prunes_workers_without_a_live_plugin() {
    let core = AppCore::new(Arc::new(NoopEmitter));
    let plugins = core.plugins.list();
    let mut workers = BTreeMap::from([
        ("ghost-a".to_owned(), worker_handle()),
        ("ghost-b".to_owned(), worker_handle()),
    ]);
    reconcile_workers(&core, &plugins, &mut workers);
    assert!(
        workers.is_empty(),
        "workers for unknown plugins must be pruned, keeping {:?}",
        workers.keys().collect::<Vec<_>>()
    );
}

#[test]
fn reconcile_keeps_nothing_when_no_workers_exist() {
    let core = AppCore::new(Arc::new(NoopEmitter));
    let plugins = core.plugins.list();
    let mut workers = BTreeMap::new();
    reconcile_workers(&core, &plugins, &mut workers);
    assert!(workers.is_empty());
}

#[test]
fn stable_domain_payload_does_not_expose_the_internal_enum() {
    let event = DomainEvent::PointsChanged {
        unique_id: "viewer".to_owned(),
        delta: 2.0,
        total_points: 4.0,
        level: 1,
    };
    let envelope = event.to_envelope().expect("variant must convert");
    assert_eq!(envelope.topic, "points.changed");
    assert_eq!(envelope.data["uniqueId"], "viewer");
    assert!(serde_json::to_value(envelope)
        .unwrap()
        .get("topic")
        .is_some());
}
