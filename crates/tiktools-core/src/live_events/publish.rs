use crate::*;

impl AppCore {
    /// Approved emitted input has its own data/control path. It reaches local
    /// presentation and scoped rule matching, never executable domain/plugin
    /// observers, enrichment processors or Owner test-preview memory.
    #[cfg_attr(not(feature = "persistence"), allow(dead_code))]
    pub(crate) async fn publish_scoped_automation_event(
        self: &Arc<Self>,
        envelope: AutomationEvent,
    ) -> Result<(), String> {
        if !envelope.context().is_approved() {
            return Err("scoped publication requires an approved execution context".into());
        }
        #[cfg(feature = "persistence")]
        {
            let payload = envelope.context().projected_event(envelope.payload())?;
            let topic = payload
                .get("type")
                .and_then(Value::as_str)
                .ok_or("scoped event type is unavailable")?;
            if !crate::collaboration::valid_emit_topic(topic)
                || normalize_emit_type(topic).as_deref() != Ok(topic)
            {
                return Err("scoped event type is invalid".into());
            }
            {
                let _writer = self.automation_gate().map_err(|error| error.to_string())?;
                envelope.context().with_current_chain(self, |identities| {
                    if self.is_shutdown()
                        || identities
                            .iter()
                            .any(|identity| !identity.grant().allows_emit_topic(topic))
                    {
                        return Err("scoped event is outside inherited execution scope".into());
                    }
                    for identity in identities {
                        identity
                            .check_deadline()
                            .map_err(|_| "scoped execution subject has expired")?;
                    }
                    self.events.publish_presentation_domain(
                        crate::events::DomainEvent::LiveEvent {
                            event_type: topic.to_owned(),
                            event: payload.clone(),
                        },
                    );
                    Ok(())
                })?;
            }
            // No processor, Owner observer publication or remember call may
            // be inserted here; the retained private context is authoritative.
            Box::pin(self.run_automation_event(envelope.replace_payload(payload))).await;
            Ok(())
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = envelope;
            Err("scoped publication requires persistence".into())
        }
    }

    /// Domain events are authoritative: the normalized event fans out to
    /// WebView/IPC/CLI immediately, before (and independent of) the
    /// automation pipeline, which may throttle or drop on saturation.
    pub(crate) fn publish_live_domain_event(&self, envelope: &AutomationEvent) -> bool {
        // The current domain bus/plugin observer has no delegated-context
        // envelope. Never let a restricted emit become an Owner observation.
        // This gate precedes diagnostics, domain fan-out and plugin observers.
        if self.execution_context_guard(envelope.context()).is_err()
            || envelope.context().depth() >= 3
        {
            return false;
        }
        let event = envelope.payload();
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
        true
    }
    pub(crate) async fn publish_automation_event(self: &Arc<Self>, event: AutomationEvent) {
        if !self.publish_live_domain_event(&event) {
            return;
        }
        let Ok(enriched) = self.enrich_automation_event(event).await else {
            return;
        };
        self.remember_automation_event(&enriched);
        Box::pin(self.run_automation_event(enriched)).await;
    }
    // Only native LIVE ingestion and persisted competitions queue events.
    #[cfg_attr(
        not(any(feature = "native-tiktok", feature = "persistence")),
        allow(dead_code)
    )]
    pub(crate) fn queue_automation_event(self: &Arc<Self>, event: AutomationEvent) {
        // Control/UI delivery must never depend on automation capacity.
        if !self.publish_live_domain_event(&event) {
            return;
        }
        let event_type = event
            .payload()
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
            // Recheck the context inside the slot, before any processor or
            // settings read. Replacing the payload never changes authority.
            let Ok(enriched) = core.enrich_automation_event(event).await else {
                return;
            };
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
        #[cfg(feature = "native-tiktok")]
        let creator_verified = context.creator_verified
            && context
                .connection_id
                .strip_prefix("connection-")
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|generation| {
                    context.connection_id == format!("connection-{generation}")
                        && self
                            .events
                            .is_verified_live_generation(generation, &context.unique_id)
                });
        #[cfg(not(feature = "native-tiktok"))]
        let creator_verified = context.creator_verified;
        #[cfg(feature = "persistence")]
        if creator_verified {
            self.write_live_session(&context.unique_id, None, false)
                .await;
        }
        #[cfg(feature = "native-tiktok")]
        let creator_verified = creator_verified
            && context
                .connection_id
                .strip_prefix("connection-")
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|generation| {
                    context.connection_id == format!("connection-{generation}")
                        && self
                            .events
                            .is_verified_live_generation(generation, &context.unique_id)
                });
        let data = serde_json::to_value(crate::contracts::ConnectionAutomationData {
            unique_id: context.unique_id.clone(),
            room_id: context.room_id.clone(),
        })
        .expect("disconnection automation data must serialize");
        let event = if creator_verified {
            self.make_automation_event_with_context("tiktok.disconnected", data, None, &context)
        } else {
            self.make_automation_event_without_context("tiktok.disconnected", data, None)
        };
        self.publish_automation_event(if creator_verified {
            AutomationEvent::owner(event)
        } else {
            AutomationEvent::owner_without_delegation(event)
        })
        .await;
    }
}
