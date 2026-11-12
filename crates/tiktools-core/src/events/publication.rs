//! Synchronous event publication with ordered cursor/history recording.
use super::{
    project_creator_event, valid_creator_id, DomainEvent, EventBus, FeedHistoryEntry,
    PublishedEvent,
};
use serde_json::Value;
use std::sync::Arc;

impl EventBus {
    /// Publishes to the reliable or lossy lane by event class. Like
    /// `broadcast::send`, this never blocks and never fails for a slow
    /// subscriber: laggards skip on their own cursor only.
    pub fn publish_domain(&self, event: DomainEvent) {
        self.publish(event, true, None);
    }

    /// Trusted LIVE ingestion binds the creator out of band. JSON fields
    /// cannot place a generic event into a creator-scoped feed.
    pub fn publish_live_event_from_creator(&self, event: Value, creator: Option<&str>) {
        self.publish(DomainEvent::LiveUiEvent { event }, true, creator);
    }

    /// Preserve state/history/UI delivery without invoking arbitrary plugin
    /// observers from an approved restricted effect. These events never enter
    /// the plugin rings, so bursts cannot manufacture executable gap events.
    pub(crate) fn publish_presentation_domain(&self, event: DomainEvent) {
        self.publish(event, false, None);
    }

    pub(super) fn publish(
        &self,
        mut event: DomainEvent,
        observer_allowed: bool,
        creator: Option<&str>,
    ) {
        if matches!(event, DomainEvent::LiveUiEvent { .. }) {
            // Serialize sequence assignment with publication, so concurrent
            // producers cannot reverse feed sequence order. No wire changes.
            let mut sequence = self.ui_sequence.lock().unwrap_or_else(|e| e.into_inner());
            *sequence += 1;
            // Preserve the authoritative cursor through typed IPC clients and
            // browser adapters, which reserialize DomainEvent and retain only
            // the UI event payload. Desktop also sends the outer cursor.
            if let DomainEvent::LiveUiEvent { event: payload } = &mut event {
                if let Some(payload) = payload.as_object_mut() {
                    payload.insert("feedCursor".into(), serde_json::json!(*sequence));
                }
            }
            let event = Arc::new(event);
            {
                let mut history = self
                    .feed_history
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let creator = match event.as_ref() {
                    DomainEvent::LiveUiEvent { event }
                        if project_creator_event(0, event).is_some() =>
                    {
                        creator.filter(|id| valid_creator_id(id))
                    }
                    _ => None,
                };
                let scoped = creator.and_then(|creator| {
                    let stream = history.creator_stream(creator, *sequence, |bytes| {
                        getrandom::fill(bytes).is_ok()
                    })?;
                    stream.next = stream.next.checked_add(1)?;
                    stream.last_global = *sequence;
                    let DomainEvent::LiveUiEvent { event } = event.as_ref() else {
                        return None;
                    };
                    stream
                        .events
                        .push_back(project_creator_event(stream.next, event)?);
                    while stream.events.len() > 64 {
                        stream.events.pop_front();
                    }
                    Some(())
                });
                let _ = scoped;
                history.entries.push_back(FeedHistoryEntry {
                    cursor: *sequence,
                    event: Arc::clone(&event),
                });
                while history.entries.len() > 512 {
                    history.entries.pop_front();
                }
            }
            self.send(
                PublishedEvent {
                    event,
                    ui_sequence: Some(*sequence),
                    observer_allowed,
                },
                true,
            );
        } else {
            let lossy = event.is_lossy();
            let published = PublishedEvent {
                event: Arc::new(event),
                ui_sequence: None,
                observer_allowed,
            };
            self.send(published, lossy);
        }
    }

    pub(super) fn send(&self, published: PublishedEvent, lossy: bool) {
        if published.observer_allowed {
            let _ = if lossy {
                self.plugin_lossy.send(published.clone())
            } else {
                self.plugin_reliable.send(published.clone())
            };
        }
        let _ = if lossy {
            self.lossy.send(published)
        } else {
            self.reliable.send(published)
        };
    }
}
