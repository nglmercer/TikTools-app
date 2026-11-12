//! Stamped event admission, bounded backpressure and explicit overload loss.
use super::*;

#[cfg(feature = "native-tiktok")]
pub(crate) fn classify_data_event(event: &NativeLiveEvent) -> DataClass {
    use tiktools_tiktok::events::CanonicalLiveEvent;
    match &event.base {
        // Every gift packet can contribute a monetary increment. Protect
        // progress as well as finals with the existing bounded backpressure.
        CanonicalLiveEvent::Gift(_) => DataClass::Critical,
        CanonicalLiveEvent::Social(social) => {
            match tiktools_tiktok::events::classify_social_action(social.action) {
                tiktools_tiktok::events::SocialKind::Follow
                | tiktools_tiktok::events::SocialKind::Share => DataClass::Critical,
                // An unconfirmed social action has no durable side effect yet,
                // so it sheds under overload like the other high-volume ticks.
                tiktools_tiktok::events::SocialKind::Other
                | tiktools_tiktok::events::SocialKind::Unknown => DataClass::Sheddable,
            }
        }
        // Chat, likes, joins, room stats, and unknown payloads are
        // high-volume ticks: shed with counters, never block the pump.
        _ => DataClass::Sheddable,
    }
}

/// Admits one broadcast event into the bounded ingress queue without
/// awaiting processing. Shed decisions are counted, never propagated.
#[cfg(test)]
pub(crate) async fn admit_event(
    core: &AppCore,
    sender: &tokio::sync::mpsc::Sender<IngressItem>,
    event: ClientEvent,
) {
    admit_stamped_event(
        core,
        sender,
        tiktools_tiktok::StampedClientEvent {
            generation: core.connection_sequence.load(Ordering::Acquire),
            event,
        },
    )
    .await;
}

/// Admits one stamped event into the bounded ingress queue without awaiting
/// processing. Admission observes `AppCore::live_admission`, which stops *new*
/// events only; already-admitted items stay owned by the queue and drain
/// normally. An event released here is counted in `admission_cancelled` so it
/// can never vanish between receive and admission.
pub(crate) async fn admit_stamped_event(
    core: &AppCore,
    sender: &tokio::sync::mpsc::Sender<IngressItem>,
    stamped: tiktools_tiktok::StampedClientEvent,
) {
    let tiktools_tiktok::StampedClientEvent { generation, event } = stamped;
    let stats = &core.live_ingress;
    stats.received.fetch_add(1, Ordering::Relaxed);
    if matches!(&event, ClientEvent::Event(_)) {
        stats.decoded.fetch_add(1, Ordering::Relaxed);
    }
    if core.live_admission.is_cancelled() {
        stats.admission_cancelled.fetch_add(1, Ordering::Relaxed);
        if matches!(&event, ClientEvent::Event(_)) {
            core.events.record_live_ingress_gap(generation);
        }
        return;
    }
    if let ClientEvent::Connected(info) = &event {
        // The label may be a caller-provided fallback. Only discovery proof
        // tying the selected room to this exact creator authorizes a scope.
        match info.verified_creator.as_deref() {
            Some(creator) if creator == info.unique_id => {
                core.events.bind_live_generation(generation, creator);
            }
            _ => core.events.reject_unverified_live_generation(generation),
        }
    }
    match event {
        // Lifecycle transitions are rare and ordering-sensitive: wait for
        // capacity rather than drop or reorder them — unless admission has
        // stopped, in which case the wait is abandoned explicitly.
        lifecycle @ (ClientEvent::Connected(_)
        | ClientEvent::Reconnecting { .. }
        | ClientEvent::Disconnected { .. }
        | ClientEvent::Error { .. }) => {
            tokio::select! {
                biased;
                () = core.live_admission.cancelled() => {
                    stats.admission_cancelled.fetch_add(1, Ordering::Relaxed);
                }
                reserved = sender.reserve() => match reserved {
                    Ok(permit) => publish_reserved(
                        stats,
                        permit,
                        IngressItem {
                            generation,
                            event: lifecycle,
                        },
                    ),
                    Err(_) => tracing::debug!("live ingress worker is gone; pump stops"),
                }
            }
        }
        ClientEvent::Event(data) => {
            let item = IngressItem {
                generation,
                event: ClientEvent::Event(data),
            };
            // Fast path: capacity available.
            match sender.try_reserve() {
                Ok(permit) => publish_reserved(stats, permit, item),
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    core.events.record_live_ingress_gap(generation);
                    tracing::debug!("live ingress worker is gone; pump stops");
                }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    admit_full(core, sender, item).await;
                }
            }
        }
    }
}

/// Account before publishing: the receiver may run on another Tokio worker
/// immediately after `send`. A reserved slot has no payload and cancellation
/// before this synchronous section simply releases the permit.
fn publish_reserved(
    stats: &LiveIngressStats,
    permit: tokio::sync::mpsc::Permit<'_, IngressItem>,
    item: IngressItem,
) {
    stats.accepted.fetch_add(1, Ordering::Relaxed);
    stats.queued.fetch_add(1, Ordering::Relaxed);
    permit.send(item);
}

#[cfg(feature = "native-tiktok")]
async fn admit_full(
    core: &AppCore,
    sender: &tokio::sync::mpsc::Sender<IngressItem>,
    item: IngressItem,
) {
    let stats = &core.live_ingress;
    let critical = match &item.event {
        ClientEvent::Event(data) => classify_data_event(data) == DataClass::Critical,
        _ => false,
    };
    if !critical {
        core.events.record_live_ingress_gap(item.generation);
        if let ClientEvent::Event(data) = &item.event {
            if let tiktools_tiktok::events::CanonicalLiveEvent::Like(like) = &data.base {
                stats.dropped_like_batches.fetch_add(1, Ordering::Relaxed);
                stats
                    .dropped_like_units
                    .fetch_add(like.count, Ordering::Relaxed);
                // Like goals can no longer claim an exact observed count:
                // distinguish this gap from a rule whose filter did not match.
                core.record_event_gap();
                let context = recover_rwlock_read(&core.connection_context, "connection context");
                core.live_totals.mark_like_ingress_loss(context.as_ref());
            }
        }
        core.aggregations.ingress_loss(1);
        stats.dropped_data.fetch_add(1, Ordering::Relaxed);
        stats.note_overload("shedding live data event on saturated ingress");
        return;
    }
    // Critical data: bounded backpressure, then an explicit, gap-reported
    // drop. Delivery beyond this point cannot be guaranteed, and the
    // counters say exactly that. Admission stop abandons the wait too, so a
    // shutdown can never block behind a saturated queue.
    let reserved = tokio::select! {
        biased;
        () = core.live_admission.cancelled() => {
            stats.admission_cancelled.fetch_add(1, Ordering::Relaxed);
            core.events.record_live_ingress_gap(item.generation);
            return;
        }
        reserved = tokio::time::timeout(CRITICAL_SEND_TIMEOUT, sender.reserve()) => reserved,
    };
    match reserved {
        Ok(Ok(permit)) => publish_reserved(stats, permit, item),
        Ok(Err(_)) => {
            core.events.record_live_ingress_gap(item.generation);
            tracing::debug!("live ingress worker is gone; pump stops");
        }
        Err(_) => {
            core.events.record_live_ingress_gap(item.generation);
            stats.dropped_critical.fetch_add(1, Ordering::Relaxed);
            core.aggregations.ingress_loss(1);
            core.record_event_gap();
            stats.note_overload("dropping critical live event after backpressure timeout");
            tracing::warn!(
                "critical live event shed after 1 s of ingress backpressure; gap recorded"
            );
        }
    }
}
