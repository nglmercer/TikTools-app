#[cfg(feature = "native-tiktok")]
use super::ingress::{
    admit_stamped_event, run_ingress_worker, IngressLifecycle, INGRESS_QUEUE_CAPACITY,
};
use crate::*;
#[cfg(feature = "native-tiktok")]
use std::time::Duration;

/// Bounded deadlines for the LIVE pump/ingress shutdown ladder. Every stage
/// is bounded so a stalled downstream service (points persistence, a wedged
/// handler) can delay shutdown only by an explicit, configurable amount —
/// never forever. Override with the `TIKTOOLS_LIVE_*_DEADLINE_MS`
/// environment variables for slow environments or tests.
#[cfg(feature = "native-tiktok")]
#[derive(Debug, Clone, Copy)]
pub(crate) struct LiveShutdownDeadlines {
    /// Pump stop: admission is closed first, so the pump only waits for the
    /// event it is currently admitting.
    pub(crate) pump: Duration,
    /// Graceful drain: already-admitted events are processed to completion.
    pub(crate) drain: Duration,
    /// Emergency stop: the in-flight item may finish after the drain
    /// deadline; this bounds that wait before the worker is aborted.
    pub(crate) stop: Duration,
}

#[cfg(feature = "native-tiktok")]
impl Default for LiveShutdownDeadlines {
    fn default() -> Self {
        Self {
            pump: Duration::from_secs(5),
            drain: Duration::from_secs(10),
            stop: Duration::from_secs(5),
        }
    }
}

#[cfg(feature = "native-tiktok")]
impl LiveShutdownDeadlines {
    pub(crate) fn from_env() -> Self {
        let defaults = Self::default();
        Self {
            pump: deadline_from_env("TIKTOOLS_LIVE_PUMP_STOP_DEADLINE_MS", defaults.pump),
            drain: deadline_from_env("TIKTOOLS_LIVE_DRAIN_DEADLINE_MS", defaults.drain),
            stop: deadline_from_env("TIKTOOLS_LIVE_INGRESS_STOP_DEADLINE_MS", defaults.stop),
        }
    }
}

#[cfg(feature = "native-tiktok")]
fn deadline_from_env(name: &str, fallback: Duration) -> Duration {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(fallback)
}

impl AppCore {
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn start_live_event_pump(self: &Arc<Self>) {
        // Never resurrect the pipeline once shutdown began or admission
        // stopped: a pump started after `stop_live_ingress` would never be
        // joined and would hold this `Arc` forever.
        if self.shutdown_started.load(Ordering::Acquire) || self.live_admission.is_cancelled() {
            return;
        }
        if self.live_pump_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut receiver = self.live.subscribe_stamped();
        let pump_core = Arc::clone(self);
        let lag_core = Arc::clone(self);
        tracing::info!("native TikTok event pump started");
        let admission = self.live_admission.clone();
        let emergency = self.live_ingress_stop.clone();
        // The pump only receives and admits: it never awaits processing, so
        // slow points persistence or slow plugins delay the ordered worker,
        // never socket reception. Both tasks are owned and joined here.
        let (sender, ingress_receiver) = tokio::sync::mpsc::channel(INGRESS_QUEUE_CAPACITY);
        let pump_admission = admission.clone();
        let pump = tokio::spawn(async move {
            loop {
                let event = tokio::select! {
                    biased;
                    () = pump_admission.cancelled() => break,
                    event = receiver.recv() => event,
                };
                match event {
                    Ok(event) => {
                        tracing::debug!(
                            kind = client_event_kind(&event.event),
                            "native TikTok event received"
                        );
                        // Admission owns its own cancellation accounting, so
                        // an event received but never admitted is still
                        // counted instead of vanishing with this task.
                        admit_stamped_event(&pump_core, &sender, event).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        lag_core.live_ingress.lagged.fetch_add(1, Ordering::Relaxed);
                        lag_core.record_event_gap();
                        tracing::warn!(count, "native TikTok event receiver lagged; analytics and event stream incomplete")
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::warn!("native TikTok event stream closed");
                        break;
                    }
                }
            }
            // Dropping the sender closes admission to the worker, which is
            // what lets it drain and exit.
        });
        let worker_core = Arc::clone(self);
        let worker = tokio::spawn(run_ingress_worker(worker_core, ingress_receiver, emergency));
        *recover_mutex(&self.live_pump_task, "LIVE pump") = Some(pump);
        *recover_mutex(&self.live_ingress_task, "LIVE ingress worker") = Some(worker);
    }
    #[cfg(not(feature = "native-tiktok"))]
    pub(crate) fn start_live_event_pump(self: &Arc<Self>) {}

    /// Stops the LIVE pump and drains the ingress queue, in that order.
    ///
    /// 1. Close admission and join the pump. The pump's exit drops the queue
    ///    sender, which is what tells the worker that no more work arrives.
    /// 2. Join the worker under the drain deadline: every already-admitted
    ///    event is processed, so graceful shutdown never discards accepted
    ///    LIVE work.
    /// 3. Only then cancel the emergency stop, which makes the worker stop
    ///    between items; a second deadline bounds the in-flight item.
    /// 4. As a last resort the worker task is aborted and everything still
    ///    outstanding is accounted explicitly (`cancelled`/`abandoned`) so
    ///    the counters never lie and shutdown never deadlocks.
    ///
    /// Both task handles are consumed, so both tasks release their
    /// `Arc<AppCore>` references when this returns.
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn stop_live_ingress(&self) {
        let deadlines = *recover_mutex(&self.live_shutdown_deadlines, "LIVE shutdown deadlines");
        self.live_admission.cancel();
        self.live_ingress
            .advance_lifecycle(IngressLifecycle::StoppingAdmission);

        let pump = recover_mutex(&self.live_pump_task, "LIVE pump").take();
        if let Some(mut pump) = pump {
            match tokio::time::timeout(deadlines.pump, &mut pump).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::error!(%error, "LIVE pump task failed while stopping");
                }
                Err(_) => {
                    tracing::error!(
                        "LIVE pump did not stop within {:?}; aborting it",
                        deadlines.pump
                    );
                    pump.abort();
                    let _ = pump.await;
                }
            }
        }
        self.live_ingress
            .advance_lifecycle(IngressLifecycle::Draining);

        let worker = recover_mutex(&self.live_ingress_task, "LIVE ingress worker").take();
        if let Some(mut worker) = worker {
            match tokio::time::timeout(deadlines.drain, &mut worker).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::error!(%error, "LIVE ingress worker failed while draining");
                }
                Err(_) => {
                    tracing::warn!(
                        drain_deadline_ms = deadlines.drain.as_millis() as u64,
                        "LIVE ingress did not drain before the deadline; emergency stop"
                    );
                    self.live_ingress_stop.cancel();
                    match tokio::time::timeout(deadlines.stop, &mut worker).await {
                        Ok(_) => {}
                        Err(_) => {
                            tracing::error!(
                                stop_deadline_ms = deadlines.stop.as_millis() as u64,
                                "LIVE ingress worker stalled past the emergency deadline; \
                                 aborting and accounting for the work it still owned"
                            );
                            worker.abort();
                            let _ = worker.await;
                            self.live_ingress.abandon_outstanding();
                        }
                    }
                }
            }
        }
        self.live_ingress
            .advance_lifecycle(IngressLifecycle::Stopped);
    }

    // Live connect/pick moved to the value-returning control operations
    // (`AppCore::live_connect` / `AppCore::live_pick`); every client,
    // including the WebView, funnels through them.
    #[cfg(feature = "persistence")]
    pub(crate) fn current_creator_unique_id(&self) -> Option<String> {
        recover_rwlock_read(&self.connection_context, "connection context")
            .as_ref()
            .map(|context| context.unique_id.clone())
    }
}

#[cfg(all(test, feature = "native-tiktok"))]
mod tests {
    use super::super::ingress::{
        admit_stamped_event, run_ingress_worker, IngressItem, IngressLifecycle,
        INGRESS_QUEUE_CAPACITY,
    };
    use crate::*;
    struct NoopEmitter;
    impl HostEmitter for NoopEmitter {
        fn emit(&self, _: HostMessage) {}
    }

    /// Registers a stand-in for the pump that mirrors its shutdown
    /// behaviour: it returns as soon as admission stops, dropping the queue
    /// sender — which is what tells the worker that no more work arrives.
    fn register_pump_stand_in(core: &Arc<AppCore>, sender: tokio::sync::mpsc::Sender<IngressItem>) {
        let admission = core.live_admission.clone();
        let task = tokio::spawn(async move {
            admission.cancelled().await;
            drop(sender);
        });
        *recover_mutex(&core.live_pump_task, "LIVE pump") = Some(task);
    }

    fn register_worker(core: &Arc<AppCore>, receiver: tokio::sync::mpsc::Receiver<IngressItem>) {
        let task = tokio::spawn(run_ingress_worker(
            Arc::clone(core),
            receiver,
            core.live_ingress_stop.clone(),
        ));
        *recover_mutex(&core.live_ingress_task, "LIVE ingress worker") = Some(task);
    }

    #[tokio::test]
    async fn shutdown_releases_idle_live_pump_owner() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let baseline = Arc::strong_count(&core);
        let health = core.system_health();
        assert_eq!(health["plugins"]["eventWorkers"], 0);
        assert_eq!(health["plugins"]["outstandingCalls"], 0);
        assert_eq!(health["automationActiveTasks"], 0);
        assert!(health["liveGeneration"].is_null());
        core.start_live_event_pump();
        assert!(Arc::strong_count(&core) > baseline);
        tokio::time::timeout(std::time::Duration::from_secs(3), core.shutdown())
            .await
            .unwrap();
        assert_eq!(Arc::strong_count(&core), baseline);
        assert!(recover_mutex(&core.live_pump_task, "LIVE pump").is_none());
        assert!(recover_mutex(&core.live_ingress_task, "LIVE ingress worker").is_none());
    }

    /// The documented graceful contract, end to end: thousands of accepted
    /// events are processed before shutdown returns, both tasks are joined,
    /// and both release their `Arc<AppCore>` references.
    #[tokio::test]
    async fn stop_live_ingress_drains_accepted_events_and_releases_owners() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let baseline = Arc::strong_count(&core);
        let (sender, receiver) = tokio::sync::mpsc::channel::<IngressItem>(INGRESS_QUEUE_CAPACITY);
        for _ in 0..INGRESS_QUEUE_CAPACITY {
            admit_stamped_event(
                &core,
                &sender,
                tiktools_tiktok::StampedClientEvent {
                    generation: 0,
                    event: ClientEvent::Reconnecting {
                        attempt: 1,
                        delay_ms: 0,
                    },
                },
            )
            .await;
        }
        // A newer session fences every queued lifecycle item, so the drain
        // is deterministic while still walking the full backlog.
        core.connection_sequence.store(9, Ordering::Release);
        register_pump_stand_in(&core, sender);
        register_worker(&core, receiver);
        tokio::time::timeout(std::time::Duration::from_secs(15), core.stop_live_ingress())
            .await
            .expect("draining shutdown must be bounded");
        assert_eq!(
            core.live_ingress.stale.load(Ordering::Relaxed),
            INGRESS_QUEUE_CAPACITY as u64
        );
        assert_eq!(core.live_ingress.cancelled.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.abandoned.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.queued.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.in_flight.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.outstanding(), 0);
        assert_eq!(core.live_ingress.lifecycle(), IngressLifecycle::Stopped);
        assert!(recover_mutex(&core.live_pump_task, "LIVE pump").is_none());
        assert!(recover_mutex(&core.live_ingress_task, "LIVE ingress worker").is_none());
        assert_eq!(
            Arc::strong_count(&core),
            baseline,
            "both tasks must release their AppCore owner"
        );
    }

    /// A producer admitting while shutdown runs: the ladder stops admission,
    /// the producer stops promptly, and the accounting still closes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stop_live_ingress_racing_an_active_producer_settles() {
        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let (sender, receiver) = tokio::sync::mpsc::channel::<IngressItem>(32);
        register_worker(&core, receiver);
        let producer_core = Arc::clone(&core);
        let producer = tokio::spawn(async move {
            let mut attempted = 0u64;
            loop {
                attempted += 1;
                admit_stamped_event(
                    &producer_core,
                    &sender,
                    tiktools_tiktok::StampedClientEvent {
                        generation: 0,
                        event: ClientEvent::Reconnecting {
                            attempt: 1,
                            delay_ms: 0,
                        },
                    },
                )
                .await;
                if producer_core.live_admission.is_cancelled() {
                    break;
                }
            }
            drop(sender);
            attempted
        });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        tokio::time::timeout(std::time::Duration::from_secs(15), core.stop_live_ingress())
            .await
            .expect("shutdown racing a producer must be bounded");
        let attempted = tokio::time::timeout(std::time::Duration::from_secs(10), producer)
            .await
            .expect("producer stops once admission closes")
            .expect("producer task");
        let accepted = core.live_ingress.accepted.load(Ordering::Relaxed);
        assert_eq!(
            accepted
                + core
                    .live_ingress
                    .admission_cancelled
                    .load(Ordering::Relaxed),
            attempted,
            "every attempted admission is accepted or explicitly released"
        );
        assert_eq!(core.live_ingress.outstanding(), 0);
        assert_eq!(core.live_ingress.lifecycle(), IngressLifecycle::Stopped);
    }

    /// A worker stuck inside synchronous points persistence must not be able
    /// to deadlock shutdown: drain deadline, emergency stop, abort, and
    /// explicit accounting of both the queued and the in-flight event.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn stalled_points_persistence_cannot_deadlock_shutdown() {
        use tiktools_tiktok::events::{ChatEvent, EventMetadata, EventUser};
        fn chat() -> tiktools_tiktok::events::TikToolsEvent {
            tiktools_tiktok::events::TikToolsEvent {
                base: tiktools_tiktok::events::CanonicalLiveEvent::Chat(ChatEvent {
                    user: EventUser {
                        id: 7,
                        unique_id: "spectator".to_owned(),
                        nickname: "Spectator".to_owned(),
                        sec_uid: "sec-spectator".to_owned(),
                        avatar_url: None,
                    },
                    comment: "hello".to_owned(),
                    ..Default::default()
                }),
                metadata: EventMetadata {
                    method: "WebcastTest".to_owned(),
                    msg_id: 4242,
                    is_history: false,
                },
                gift: None,
            }
        }

        let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
        let baseline = Arc::strong_count(&core);
        *recover_mutex(&core.live_shutdown_deadlines, "LIVE shutdown deadlines") =
            super::LiveShutdownDeadlines {
                pump: std::time::Duration::from_millis(300),
                drain: std::time::Duration::from_millis(300),
                stop: std::time::Duration::from_millis(300),
            };
        // The first award wedges inside the synchronous points worker; the
        // gate releases itself when shutdown cancels the service.
        core.live_points_work
            .stall_for_test(std::time::Duration::from_secs(3));
        let (sender, receiver) = tokio::sync::mpsc::channel::<IngressItem>(INGRESS_QUEUE_CAPACITY);
        for msg_id in 4242..4246 {
            let mut event = chat();
            event.metadata.msg_id = msg_id;
            admit_stamped_event(
                &core,
                &sender,
                tiktools_tiktok::StampedClientEvent {
                    generation: 0,
                    event: ClientEvent::Event(event),
                },
            )
            .await;
        }
        register_pump_stand_in(&core, sender);
        register_worker(&core, receiver);
        let started = std::time::Instant::now();
        tokio::time::timeout(std::time::Duration::from_secs(30), core.shutdown())
            .await
            .expect("shutdown must be bounded even with a stalled points award");
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(20),
            "shutdown took {elapsed:?}"
        );
        assert_eq!(core.live_ingress.accepted.load(Ordering::Relaxed), 4);
        // Originals now finish before persistence. Shutdown accounts for
        // the independently queued effects, including the uncertain award.
        assert_eq!(core.live_ingress.processed.load(Ordering::Relaxed), 4);
        assert_eq!(core.live_ingress.ui_published.load(Ordering::Relaxed), 4);
        assert_eq!(core.live_ingress.cancelled.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.abandoned.load(Ordering::Relaxed), 0);
        assert_eq!(core.live_ingress.effects_dropped.load(Ordering::Relaxed), 4);
        assert_eq!(core.live_ingress.effects_queued.load(Ordering::Relaxed), 0);
        assert_eq!(
            core.live_ingress.effects_in_flight.load(Ordering::Relaxed),
            0
        );
        assert_eq!(core.live_ingress.outstanding(), 0);
        assert_eq!(core.live_ingress.lifecycle(), IngressLifecycle::Stopped);
        assert!(recover_mutex(&core.live_pump_task, "LIVE pump").is_none());
        assert!(recover_mutex(&core.live_ingress_task, "LIVE ingress worker").is_none());
        assert_eq!(Arc::strong_count(&core), baseline);
    }

    /// Repeated start/stop cycles must not accumulate tasks or owners: every
    /// cycle returns to the exact baseline ownership and task state.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn repeated_start_stop_cycles_release_every_owner() {
        for cycle in 0..8 {
            let core = Arc::new(AppCore::new(Arc::new(NoopEmitter)));
            let baseline = Arc::strong_count(&core);
            // Starting twice must not double-register the pipeline.
            core.start_live_event_pump();
            core.start_live_event_pump();
            assert!(Arc::strong_count(&core) > baseline, "cycle {cycle}");
            tokio::time::timeout(std::time::Duration::from_secs(10), core.shutdown())
                .await
                .unwrap_or_else(|_| panic!("cycle {cycle} shutdown timed out"));
            assert_eq!(Arc::strong_count(&core), baseline, "cycle {cycle}");
            assert!(
                recover_mutex(&core.live_pump_task, "LIVE pump").is_none(),
                "cycle {cycle}"
            );
            assert!(
                recover_mutex(&core.live_ingress_task, "LIVE ingress worker").is_none(),
                "cycle {cycle}"
            );
            // A pump started after shutdown must never be registered: it
            // would hold an owner that nothing ever joins.
            core.start_live_event_pump();
            assert!(
                recover_mutex(&core.live_pump_task, "LIVE pump").is_none(),
                "cycle {cycle}: no pump after shutdown"
            );
        }
    }
}
