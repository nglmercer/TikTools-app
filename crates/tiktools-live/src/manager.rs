use crate::*;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct AdapterRequest {
    pub target: String,
    pub room_id: Option<String>,
    pub credential: Credential,
}

/// Only application-registered adapters receive an EventSink. RPC callers can
/// request connections but cannot inject native events or assert verification.
pub trait Adapter: Send + Sync {
    fn descriptor(&self) -> PlatformDescriptor;
    fn normalize_target(&self, target: &str) -> Result<String, LiveError> {
        Ok(target.to_owned())
    }
    fn run<'a>(
        &'a self,
        request: AdapterRequest,
        sink: EventSink,
        stop: CancellationToken,
    ) -> BoxFuture<'a, Result<(), LiveError>>;
}

#[derive(Debug, Clone)]
pub enum ManagedEvent {
    State(ConnectionSnapshot),
    Event(LiveEnvelope),
    Gap { connection_id: String, lost: u64 },
}

struct Entry {
    snapshot: Arc<RwLock<ConnectionSnapshot>>,
    stop: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}

pub struct ConnectionManager {
    adapters: RwLock<BTreeMap<PlatformId, Arc<dyn Adapter>>>,
    entries: Mutex<BTreeMap<String, Entry>>,
    sequence: AtomicU64,
    sender: mpsc::Sender<ManagedEvent>,
    receiver: Mutex<Option<mpsc::Receiver<ManagedEvent>>>,
    operation: tokio::sync::Mutex<()>,
    stopped: std::sync::atomic::AtomicBool,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new(1024)
    }
}
impl ConnectionManager {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity.max(1));
        Self {
            adapters: RwLock::new(BTreeMap::new()),
            entries: Mutex::new(BTreeMap::new()),
            sequence: AtomicU64::new(0),
            sender,
            receiver: Mutex::new(Some(receiver)),
            operation: tokio::sync::Mutex::new(()),
            stopped: false.into(),
        }
    }
    pub fn register(&self, adapter: Arc<dyn Adapter>) -> Result<(), LiveError> {
        let mut adapters = self.adapters.write().unwrap_or_else(|e| e.into_inner());
        let platform = adapter.descriptor().platform;
        if adapters.contains_key(&platform) {
            return Err(LiveError::new(
                "already_registered",
                "Platform already registered",
            ));
        }
        adapters.insert(platform, adapter);
        Ok(())
    }
    pub fn descriptors(&self) -> Vec<PlatformDescriptor> {
        self.adapters
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(|a| a.descriptor())
            .collect()
    }
    pub fn take_events(&self) -> Option<mpsc::Receiver<ManagedEvent>> {
        self.receiver
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
    pub fn snapshots(&self) -> Vec<ConnectionSnapshot> {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(|e| e.snapshot.read().unwrap_or_else(|e| e.into_inner()).clone())
            .collect()
    }
    pub async fn connect(
        &self,
        platform: PlatformId,
        mut request: AdapterRequest,
    ) -> Result<ConnectionSnapshot, LiveError> {
        let _operation = self.operation.lock().await;
        if self.stopped.load(Ordering::Acquire) {
            return Err(LiveError::new(
                "unavailable",
                "Connection manager has shut down",
            ));
        }
        if request.target.is_empty()
            || request.target.len() > 256
            || request.target.chars().any(char::is_control)
            || request.room_id.as_ref().is_some_and(|id| id.len() > 256)
        {
            return Err(LiveError::new("invalid_params", "Invalid channel target"));
        }
        let adapter = self
            .adapters
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&platform)
            .cloned()
            .ok_or_else(|| LiveError::new("unsupported", "Platform adapter is not installed"))?;
        request.target = adapter.normalize_target(&request.target)?;
        {
            let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
            // Bound retained state. Inactive entries can be explicitly disconnected.
            if entries.len() >= 32 {
                return Err(LiveError::new(
                    "capacity",
                    "Disconnect an existing connection before adding another",
                ));
            }
            if entries.values_mut().any(|e| {
                let s = e.snapshot.read().unwrap_or_else(|e| e.into_inner());
                s.platform == platform
                    && (s.target == request.target || platform == PlatformId::Tiktok)
            }) {
                return Err(LiveError::new(
                    "already_connected",
                    "Disconnect the existing channel before reconnecting",
                ));
            }
        }
        let id = format!(
            "{}-{}",
            platform.as_str(),
            self.sequence.fetch_add(1, Ordering::Relaxed) + 1
        );
        let snapshot = ConnectionSnapshot {
            connection_id: id.clone(),
            platform,
            target: request.target.clone(),
            state: ConnectionState::Connecting,
            channel: None,
            capabilities: adapter.descriptor().capabilities,
            reconnect_attempt: 0,
            retry_after_ms: None,
            error: None,
        };
        let state = Arc::new(RwLock::new(snapshot.clone()));
        let stop = CancellationToken::new();
        let sink = EventSink {
            snapshot: state.clone(),
            sender: self.sender.clone(),
            stop: stop.clone(),
        };
        let task_stop = stop.clone();
        let task = tokio::spawn(async move {
            if !sink.publish_state().await {
                return;
            }
            let result = tokio::select! { biased; () = task_stop.cancelled() => Ok(()), result = adapter.run(request, sink.clone(), task_stop.clone()) => result };
            sink.finish(result).await;
        });
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
                id,
                Entry {
                    snapshot: state,
                    stop,
                    task,
                },
            );
        Ok(snapshot)
    }
    pub async fn disconnect(&self, id: &str) -> Result<(), LiveError> {
        let _operation = self.operation.lock().await;
        self.disconnect_inner(id).await
    }
    async fn disconnect_inner(&self, id: &str) -> Result<(), LiveError> {
        let entry = self
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id)
            .ok_or_else(|| LiveError::new("not_found", "Connection does not exist"))?;
        entry.stop.cancel();
        let mut task = entry.task;
        if tokio::time::timeout(std::time::Duration::from_secs(5), &mut task)
            .await
            .is_err()
        {
            task.abort();
            let _ = task.await;
        }
        Ok(())
    }
    pub async fn shutdown(&self) {
        let _operation = self.operation.lock().await;
        self.stopped.store(true, Ordering::Release);
        let ids: Vec<_> = self
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect();
        for id in ids {
            let _ = self.disconnect_inner(&id).await;
        }
    }
}
impl Drop for ConnectionManager {
    fn drop(&mut self) {
        for entry in self
            .entries
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .values()
        {
            entry.stop.cancel();
            entry.task.abort();
        }
    }
}

#[derive(Clone)]
pub struct EventSink {
    snapshot: Arc<RwLock<ConnectionSnapshot>>,
    sender: mpsc::Sender<ManagedEvent>,
    stop: CancellationToken,
}
impl EventSink {
    async fn send(&self, event: ManagedEvent) -> bool {
        tokio::select! { biased; () = self.stop.cancelled() => false, result = self.sender.send(event) => result.is_ok() }
    }
    pub async fn publish_state(&self) -> bool {
        let state = self
            .snapshot
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        self.send(ManagedEvent::State(state)).await
    }
    pub async fn connected(&self, channel: ChannelIdentity) -> Result<(), LiveError> {
        {
            let mut s = self.snapshot.write().unwrap_or_else(|e| e.into_inner());
            if channel.platform != s.platform || channel.id.is_empty() || channel.room_id.is_empty()
            {
                return Err(LiveError::new(
                    "invalid_response",
                    "Adapter returned invalid channel identity",
                ));
            }
            s.channel = Some(channel);
            s.state = ConnectionState::Connected;
            s.error = None;
            s.reconnect_attempt = 0;
            s.retry_after_ms = None;
        }
        self.publish_state().await;
        Ok(())
    }
    pub async fn reconnecting(&self, attempt: u32, delay_ms: u64) {
        {
            let mut s = self.snapshot.write().unwrap_or_else(|e| e.into_inner());
            s.state = ConnectionState::Reconnecting;
            s.reconnect_attempt = attempt;
            s.retry_after_ms = Some(delay_ms);
        }
        self.publish_state().await;
    }
    pub async fn gap(&self, lost: u64) {
        let id = self
            .snapshot
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .connection_id
            .clone();
        self.send(ManagedEvent::Gap {
            connection_id: id,
            lost,
        })
        .await;
    }
    pub async fn event(
        &self,
        source_event_id: Option<String>,
        occurred_at: u64,
        is_history: bool,
        user: Option<ViewerIdentity>,
        payload: LivePayload,
        extensions: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), LiveError> {
        let s = self
            .snapshot
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let channel = s.channel.ok_or_else(|| {
            LiveError::new("unverified_channel", "Adapter has not resolved a channel")
        })?;
        if s.state != ConnectionState::Connected
            || user
                .as_ref()
                .is_some_and(|u| u.platform != s.platform || u.id.is_empty() || u.id.len() > 256)
        {
            return Err(LiveError::new(
                "invalid_response",
                "Invalid event identity or connection state",
            ));
        }
        let envelope = LiveEnvelope {
            schema_version: 1,
            platform: s.platform,
            connection_id: s.connection_id.clone(),
            session_id: identity_key(s.platform, &channel.room_id),
            channel,
            source_event_id,
            occurred_at,
            received_at: now_millis(),
            is_history,
            user,
            payload,
            extensions,
        };
        if self.send(ManagedEvent::Event(envelope)).await {
            Ok(())
        } else {
            Err(LiveError::new(
                "cancelled",
                "Connection event stream closed",
            ))
        }
    }
    async fn finish(&self, result: Result<(), LiveError>) {
        let state = {
            let mut s = self.snapshot.write().unwrap_or_else(|e| e.into_inner());
            s.state = if result.is_err() {
                ConnectionState::Error
            } else {
                ConnectionState::Disconnected
            };
            s.error = result.err();
            s.retry_after_ms = None;
            s.clone()
        };
        // Shutdown must never wait for a full downstream queue. The caller
        // receives an authoritative snapshot through the disconnect operation.
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            self.sender.send(ManagedEvent::State(state)),
        )
        .await;
    }
}
