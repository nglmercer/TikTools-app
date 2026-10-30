use crate::*;

impl AppCore {
    /// Domain events are authoritative: the normalized event fans out to
    /// WebView/IPC/CLI immediately, before (and independent of) the
    /// automation pipeline, which may throttle or drop on saturation.
    pub(crate) fn publish_live_domain_event(&self, event: &serde_json::Value) {
        if let Some(event_type) = event.get("type").and_then(Value::as_str) {
            if event_type.starts_with("tiktok.") {
                self.events
                    .publish_domain(crate::events::DomainEvent::LiveEvent {
                        event_type: event_type.to_owned(),
                        event: event.clone(),
                    });
            }
            // Validated plugin events are first-class domain events,
            // published here — before automation enrichment/execution — so
            // subscribers observe them even when automation has no match,
            // fails, or is saturated. Generic over all plugin-declared
            // event types; ownership comes from the host stamp.
            if let Some(plugin_id) = plugin_owner(event) {
                self.record_plugin_event(&plugin_id, event);
                self.events
                    .publish_domain(crate::events::DomainEvent::PluginEvent {
                        plugin_id,
                        event_type: event_type.to_owned(),
                        event: event.clone(),
                    });
            }
        }
    }
    pub(crate) async fn publish_automation_event(self: &Arc<Self>, event: serde_json::Value) {
        self.publish_live_domain_event(&event);
        let enriched = self.enrich_automation_event(event).await;
        self.remember_automation_event(&enriched);
        Box::pin(self.run_automation_event(enriched)).await;
    }
    // Only native LIVE ingestion and persisted competitions queue events.
    #[cfg_attr(
        not(any(feature = "native-tiktok", feature = "persistence")),
        allow(dead_code)
    )]
    pub(crate) fn queue_automation_event(self: &Arc<Self>, event: serde_json::Value) {
        // Control/UI delivery must never depend on automation capacity.
        self.publish_live_domain_event(&event);
        let event_type = event
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let Ok(permit) = Arc::clone(&self.automation_state.slots).try_acquire_owned() else {
            self.automation_state
                .rejected
                .fetch_add(1, Ordering::Relaxed);
            tracing::warn!(
                event_type,
                "automation concurrency limit reached; dropping automation work only (domain event already delivered)"
            );
            return;
        };
        self.automation_state
            .admitted
            .fetch_add(1, Ordering::Relaxed);
        let core = Arc::clone(self);
        let shutdown = self.plugin_state.observer_shutdown.clone();
        tokio::spawn(async move {
            // Cancellation is observed between stages: a shutdown never
            // waits for queued automation, and queued work never runs
            // after the observers it reports to are gone.
            if shutdown.is_cancelled() {
                return;
            }
            // Enrichment runs inside the automation slot so a slow processor
            // delays only its own event; failures fail open to the raw event.
            // It already short-circuits when no processors match, so idle
            // rooms pay no enrichment cost.
            let enriched = core.enrich_automation_event(event).await;
            if shutdown.is_cancelled() {
                return;
            }
            core.remember_automation_event(&enriched);
            Box::pin(core.run_automation_event(enriched)).await;
            drop(permit);
        });
    }
    pub(crate) async fn publish_disconnected_event(self: &Arc<Self>) {
        let context = recover_rwlock_write(&self.connection_context, "connection context").take();
        let Some(context) = context else { return };
        #[cfg(feature = "persistence")]
        self.write_live_session(&context.unique_id, None, false)
            .await;
        self.publish_automation_event(
            self.make_automation_event_with_context(
                "tiktok.disconnected",
                serde_json::to_value(crate::contracts::ConnectionAutomationData {
                    unique_id: context.unique_id.clone(),
                    room_id: context.room_id.clone(),
                })
                .expect("disconnection automation data must serialize"),
                None,
                &context,
            ),
        )
        .await;
    }
}
