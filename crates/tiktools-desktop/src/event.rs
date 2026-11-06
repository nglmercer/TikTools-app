use crate::window::outbox::ForwardMailbox;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tiktools_core::{ipc::messages::HostMessage, HostEmitter};
use winit::event_loop::EventLoopProxy;

#[derive(Debug)]
#[allow(dead_code)]
pub enum DesktopCommand {
    #[cfg(all(test, feature = "live-benchmark"))]
    BenchmarkSample {
        data: serde_json::Value,
    },
    AdmittedCommand {
        command: Box<DesktopCommand>,
        admission: Arc<tokio::sync::OwnedSemaphorePermit>,
    },
    FrontendReady {
        generation: u64,
    },
    ShowWindow,
    HideWindow,
    OpenDevtools,
    Quit,
    RequestAppUpdateInstall {
        generation: u64,
        spanish: bool,
    },
    BeginAppUpdateShutdown,
    ShutdownComplete,
    IpcFailed(String),
    FlushWebviewBatch,
    HostBatchAcknowledged,
    OpenPluginUi {
        plugin_id: String,
        page_id: String,
    },
    OpenPluginDevtools {
        window_id: winit::window::WindowId,
    },
    ClosePluginUi {
        plugin_id: String,
        page_id: String,
        window_id: Option<winit::window::WindowId>,
        owner: Option<u64>,
    },
    PluginUiRespond {
        window_id: winit::window::WindowId,
        plugin_id: String,
        page_id: String,
        response: Arc<str>,
        effect: Option<PluginUiSubscription>,
        generation: u64,
        admission: Arc<NativeAdmission>,
    },
    PluginUiSubscribe {
        window_id: winit::window::WindowId,
        plugin_id: String,
        page_id: String,
        effect: PluginUiSubscription,
        generation: u64,
        admission: Arc<NativeAdmission>,
    },
}

static NATIVE_ADMISSIONS: AtomicU64 = AtomicU64::new(0);

/// Execution and queued result share one guard. It remains observable even
/// after its native window is removed from the manager map.
#[derive(Debug)]
pub(crate) struct NativeAdmission {
    _permit: tokio::sync::OwnedSemaphorePermit,
}
impl NativeAdmission {
    pub(crate) fn new(permit: tokio::sync::OwnedSemaphorePermit) -> Self {
        NATIVE_ADMISSIONS.fetch_add(1, Ordering::Relaxed);
        Self { _permit: permit }
    }
}
impl Drop for NativeAdmission {
    fn drop(&mut self) {
        NATIVE_ADMISSIONS.fetch_sub(1, Ordering::Relaxed);
    }
}
pub(crate) fn native_admissions() -> u64 {
    NATIVE_ADMISSIONS.load(Ordering::Relaxed)
}

/// Window-manager side of a broker subscription effect (mirrors
/// `plugin_webview::broker::SubscriptionEffect` without coupling the event
/// enum to the broker module's internals).
#[derive(Debug, Clone)]
pub enum PluginUiSubscription {
    Subscribe(Vec<String>),
    Unsubscribe(Vec<String>),
}

impl From<crate::plugin_webview::broker::SubscriptionEffect> for PluginUiSubscription {
    fn from(effect: crate::plugin_webview::broker::SubscriptionEffect) -> Self {
        match effect {
            crate::plugin_webview::broker::SubscriptionEffect::Subscribe(topics) => {
                Self::Subscribe(topics)
            }
            crate::plugin_webview::broker::SubscriptionEffect::Unsubscribe(topics) => {
                Self::Unsubscribe(topics)
            }
        }
    }
}

#[derive(Debug)]
pub enum DesktopEvent {
    Command(DesktopCommand),
}

/// Converts framework-independent core messages into UI-thread commands. The
/// core never receives a WebView handle and can therefore be tested headlessly.
pub struct EventLoopEmitter {
    proxy: EventLoopProxy<DesktopEvent>,
    pub(crate) generation: AtomicU64,
    pub(crate) projection: Arc<crate::window::projection::MainPageProjection>,
    pub(crate) batch_sequence: AtomicU64,
    batch_acknowledged: AtomicBool,
    batch_ack_wake_pending: AtomicBool,
    pub(crate) acknowledged_batches: AtomicU64,
    pub(crate) batch_messages: AtomicU64,
    pub(crate) batch_bytes: AtomicU64,
    pub(crate) ready_pending: AtomicBool,
    pub(crate) update_prompt_pending: AtomicBool,
    ready_generation: AtomicU64,
    pub(crate) inbound_rejected: AtomicU64,
    pub(crate) inbound_timeouts: AtomicU64,
    pub(crate) evaluations: AtomicU64,
    pub(crate) evaluation_bytes: AtomicU64,
    pub(crate) evaluation_errors: AtomicU64,
    pub(crate) ack_latency_us: AtomicU64,
    pub(crate) ack_latency_max_us: AtomicU64,
    pub(crate) ack_latency_total_us: AtomicU64,
    pub(crate) ipc_slots: Arc<tokio::sync::Semaphore>,
    pub(crate) ui_snapshot: Mutex<serde_json::Value>,
    pub(crate) mailbox: Arc<Mutex<ForwardMailbox>>,
}

impl EventLoopEmitter {
    pub fn new(proxy: EventLoopProxy<DesktopEvent>) -> Self {
        Self {
            proxy,
            generation: AtomicU64::new(0),
            projection: Arc::new(crate::window::projection::MainPageProjection::default()),
            batch_sequence: AtomicU64::new(0),
            batch_acknowledged: AtomicBool::new(false),
            batch_ack_wake_pending: AtomicBool::new(false),
            acknowledged_batches: AtomicU64::new(0),
            batch_messages: AtomicU64::new(0),
            batch_bytes: AtomicU64::new(0),
            ready_pending: AtomicBool::new(false),
            update_prompt_pending: AtomicBool::new(false),
            ready_generation: AtomicU64::new(0),
            inbound_rejected: AtomicU64::new(0),
            inbound_timeouts: AtomicU64::new(0),
            evaluations: AtomicU64::new(0),
            evaluation_bytes: AtomicU64::new(0),
            evaluation_errors: AtomicU64::new(0),
            ack_latency_us: AtomicU64::new(0),
            ack_latency_max_us: AtomicU64::new(0),
            ack_latency_total_us: AtomicU64::new(0),
            ipc_slots: Arc::new(tokio::sync::Semaphore::new(128)),
            ui_snapshot: Mutex::new(serde_json::Value::Null),
            mailbox: Arc::new(Mutex::new(ForwardMailbox::default())),
        }
    }
}

impl EventLoopEmitter {
    pub(crate) fn consume_ready(&self) -> u64 {
        self.ready_pending.store(false, Ordering::Release);
        self.ready_generation.load(Ordering::Acquire)
    }

    pub(crate) fn request_ready(&self, generation: u64) {
        if generation != self.generation.load(Ordering::Acquire) {
            return;
        }
        self.ready_generation
            .fetch_max(generation, Ordering::AcqRel);
        if self.ready_pending.swap(true, Ordering::AcqRel) {
            return;
        }
        if self
            .proxy
            .send_coalesced(DesktopEvent::Command(DesktopCommand::FrontendReady {
                generation,
            }))
            .is_err()
        {
            self.ready_pending.store(false, Ordering::Release);
        }
    }

    pub(crate) fn request_batch_ack(&self, generation: u64, sequence: u64) {
        if generation != self.generation.load(Ordering::Acquire)
            || sequence != self.batch_sequence.load(Ordering::Acquire)
        {
            return;
        }
        self.batch_acknowledged.store(true, Ordering::Release);
        if self.batch_ack_wake_pending.swap(true, Ordering::AcqRel) {
            return;
        }
        if self
            .proxy
            .send_coalesced(DesktopEvent::Command(DesktopCommand::HostBatchAcknowledged))
            .is_err()
        {
            self.batch_ack_wake_pending.store(false, Ordering::Release);
        }
    }

    pub(crate) fn consume_batch_ack(&self) -> bool {
        self.batch_ack_wake_pending.store(false, Ordering::Release);
        self.batch_acknowledged.swap(false, Ordering::AcqRel)
    }

    pub(crate) fn clear_batch(&self) {
        self.batch_messages.store(0, Ordering::Relaxed);
        self.batch_bytes.store(0, Ordering::Relaxed);
        self.batch_acknowledged.store(false, Ordering::Release);
    }

    pub(crate) fn advance_generation(&self) {
        let mut mailbox = self.mailbox.lock().unwrap_or_else(|e| e.into_inner());
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.projection.retire(generation);
        self.clear_batch();
        mailbox.retire_page();
    }

    pub(crate) fn send(&self, payload: String) {
        self.send_generation(payload, None);
    }

    pub(crate) fn send_generation(&self, payload: String, generation: Option<u64>) {
        let wake = {
            let mut mailbox = self.mailbox.lock().unwrap_or_else(|e| e.into_inner());
            if generation
                .is_some_and(|expected| expected != self.generation.load(Ordering::Acquire))
            {
                return;
            }
            mailbox.push(payload)
        };
        self.wake(wake);
    }

    pub(crate) fn send_value_generation(&self, value: &serde_json::Value, generation: u64) {
        let wake = {
            let mut mailbox = self.mailbox.lock().unwrap_or_else(|e| e.into_inner());
            if generation != self.generation.load(Ordering::Acquire) {
                return;
            }
            mailbox.push_value(value)
        };
        self.wake(wake);
    }

    pub(crate) fn fail(&self) {
        let wake = self
            .mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .fail();
        self.wake(wake);
    }

    fn wake(&self, wake: bool) {
        if wake
            && self
                .proxy
                .send_coalesced(DesktopEvent::Command(DesktopCommand::FlushWebviewBatch))
                .is_err()
        {
            self.mailbox
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .shutdown();
        }
    }
}

impl HostEmitter for EventLoopEmitter {
    fn wants_automation_context_updates(&self) -> bool {
        false
    }
    fn wants_automation_history(&self) -> bool {
        false
    }
    fn wants_leaderboard_snapshots(&self) -> bool {
        false
    }
    fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "mailbox": self.mailbox.lock().unwrap_or_else(|e| e.into_inner()).diagnostics(),
            "inboundCapacity": 128,
            "inboundInFlight": 128usize.saturating_sub(self.ipc_slots.available_permits()),
            "inboundRejected": self.inbound_rejected.load(Ordering::Relaxed),
            "inboundTimeouts": self.inbound_timeouts.load(Ordering::Relaxed),
            "generation": self.generation.load(Ordering::Acquire),
            "acknowledgedBatches": self.acknowledged_batches.load(Ordering::Relaxed),
            "batchInFlightMessages": self.batch_messages.load(Ordering::Relaxed),
            "batchInFlightBytes": self.batch_bytes.load(Ordering::Relaxed),
            "evaluateCalls": self.evaluations.load(Ordering::Relaxed),
            "evaluateBytes": self.evaluation_bytes.load(Ordering::Relaxed),
            "ackLatencyUs": self.ack_latency_us.load(Ordering::Relaxed),
            "ackLatencyMaxUs": self.ack_latency_max_us.load(Ordering::Relaxed),
            "ackLatencyTotalUs": self.ack_latency_total_us.load(Ordering::Relaxed),
            "evaluateErrors": self.evaluation_errors.load(Ordering::Relaxed),
            "projection": self.projection.diagnostics(),
            "ui": self.ui_snapshot.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        })
    }

    fn emit(&self, message: HostMessage) {
        // High-rate compatibility pushes opt out at their producers.
        // Explicit legacy requests still receive their historical response.
        match crate::window::outbox::encode_bounded(&message) {
            Ok(payload) => self.send(payload),
            Err(error) => {
                tracing::error!(%error, "could not serialize bounded host message");
                self.fail();
            }
        }
    }
}

pub fn shared_emitter(proxy: EventLoopProxy<DesktopEvent>) -> Arc<EventLoopEmitter> {
    Arc::new(EventLoopEmitter::new(proxy))
}

// The desktop owns one Winit event loop per process. These shared flags also
// cover producers (tray / activation listener) that exist before DesktopApp.
static PENDING_SIGNALS: AtomicU64 = AtomicU64::new(0);
static DESIRED_VISIBILITY: AtomicBool = AtomicBool::new(true);

fn signal_bit(command: &DesktopCommand) -> u64 {
    match command {
        DesktopCommand::ShowWindow | DesktopCommand::HideWindow => 1,
        DesktopCommand::OpenDevtools => 2,
        DesktopCommand::Quit => 4,
        DesktopCommand::ShutdownComplete => 8,
        DesktopCommand::IpcFailed(_) => 16,
        DesktopCommand::FlushWebviewBatch => 32,
        _ => 0,
    }
}

pub(crate) trait CoalescedSignals {
    fn send_coalesced(
        &self,
        event: DesktopEvent,
    ) -> Result<(), winit::event_loop::EventLoopClosed<DesktopEvent>>;
}

fn reserve_signal(command: &DesktopCommand) -> bool {
    match command {
        DesktopCommand::ShowWindow => DESIRED_VISIBILITY.store(true, Ordering::Release),
        DesktopCommand::HideWindow => DESIRED_VISIBILITY.store(false, Ordering::Release),
        _ => {}
    }
    let bit = signal_bit(command);
    bit == 0 || PENDING_SIGNALS.fetch_or(bit, Ordering::AcqRel) & bit == 0
}

impl CoalescedSignals for EventLoopProxy<DesktopEvent> {
    fn send_coalesced(
        &self,
        mut event: DesktopEvent,
    ) -> Result<(), winit::event_loop::EventLoopClosed<DesktopEvent>> {
        let DesktopEvent::Command(command) = &mut event;
        if let DesktopCommand::IpcFailed(message) = command {
            let mut end = message.len().min(4096);
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
        }
        let bit = signal_bit(command);
        if !reserve_signal(command) {
            return Ok(());
        }
        let result = self.send_event(event);
        if result.is_err() {
            PENDING_SIGNALS.fetch_and(!bit, Ordering::AcqRel);
        }
        result
    }
}

pub(crate) fn consume_signal(event: DesktopEvent) -> DesktopEvent {
    let DesktopEvent::Command(command) = event;
    let bit = signal_bit(&command);
    PENDING_SIGNALS.fetch_and(!bit, Ordering::AcqRel);
    if bit == 1 {
        DesktopEvent::Command(if DESIRED_VISIBILITY.load(Ordering::Acquire) {
            DesktopCommand::ShowWindow
        } else {
            DesktopCommand::HideWindow
        })
    } else {
        DesktopEvent::Command(command)
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;

    #[test]
    fn hundred_thousand_blocked_activation_signals_coalesce_to_latest_visibility() {
        PENDING_SIGNALS.fetch_and(!1, Ordering::AcqRel);
        assert!(reserve_signal(&DesktopCommand::ShowWindow));
        for _ in 0..100_000 {
            assert!(!reserve_signal(&DesktopCommand::ShowWindow));
        }
        assert!(!reserve_signal(&DesktopCommand::HideWindow));
        assert!(matches!(
            consume_signal(DesktopEvent::Command(DesktopCommand::ShowWindow)),
            DesktopEvent::Command(DesktopCommand::HideWindow)
        ));
        assert!(reserve_signal(&DesktopCommand::ShowWindow));
        consume_signal(DesktopEvent::Command(DesktopCommand::ShowWindow));
    }

    #[tokio::test]
    async fn queued_broker_results_keep_their_permits_until_ui_consumption() {
        let slots = Arc::new(tokio::sync::Semaphore::new(128));
        let mut commands = Vec::new();
        for index in 0..128 {
            let admission = Arc::new(NativeAdmission::new(
                slots.clone().try_acquire_owned().unwrap(),
            ));
            commands.push(DesktopCommand::PluginUiRespond {
                window_id: winit::window::WindowId::from(1),
                plugin_id: "p".to_owned(),
                page_id: "main".to_owned(),
                response: format!("{{\"id\":{index}}}").into(),
                effect: None,
                generation: 1,
                admission: admission.clone(),
            });
            commands.push(DesktopCommand::PluginUiSubscribe {
                window_id: winit::window::WindowId::from(1),
                plugin_id: "p".to_owned(),
                page_id: "main".to_owned(),
                effect: PluginUiSubscription::Subscribe(vec!["plugin.event".to_owned()]),
                generation: 1,
                admission,
            });
        }
        assert_eq!(commands.len(), 256);
        assert!(slots.clone().try_acquire_owned().is_err());
        commands.clear();
        assert_eq!(slots.available_permits(), 128);
    }

    #[tokio::test]
    async fn cancelled_inbound_tasks_release_all_admission() {
        let slots = Arc::new(tokio::sync::Semaphore::new(128));
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..128 {
            let permit = slots.clone().try_acquire_owned().unwrap();
            tasks.spawn(async move {
                let _permit = permit;
                std::future::pending::<()>().await;
            });
        }
        assert!(slots.clone().try_acquire_owned().is_err());
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        assert_eq!(slots.available_permits(), 128);
        slots.close();
        assert!(slots.clone().try_acquire_owned().is_err());
    }
}
