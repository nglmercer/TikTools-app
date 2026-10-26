//! Connection task ownership. Replacement waits for cancellation to complete;
//! an uncooperative initialization retains its handle and blocks replacements.
use crate::{ClientEvent, ConnectionInfo, TikTokError};
use serde::Serialize;
use std::{future::Future, time::Duration};
use tokio::sync::{broadcast, oneshot, watch, Mutex};
use tokio_util::sync::CancellationToken;

const TEARDOWN_DEADLINE: Duration = Duration::from_secs(5);
const NETWORK_DEADLINE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    Reconnecting,
    Disconnecting,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSnapshot {
    pub generation: u64,
    pub state: ConnectionState,
}

#[derive(Debug, Clone)]
pub struct StampedClientEvent {
    pub generation: u64,
    pub event: ClientEvent,
}

#[derive(Clone)]
pub(super) struct EventSender {
    legacy: broadcast::Sender<ClientEvent>,
    stamped: broadcast::Sender<StampedClientEvent>,
}

impl EventSender {
    pub(super) fn new() -> Self {
        Self {
            legacy: broadcast::channel(512).0,
            stamped: broadcast::channel(512).0,
        }
    }
    pub(super) fn send(&self, generation: u64, event: ClientEvent) {
        // Preserve the public stream for existing consumers. With no legacy
        // consumers, avoid retaining a second copy of every event.
        if self.legacy.receiver_count() > 0 {
            let _ = self.legacy.send(event.clone());
        }
        let _ = self.stamped.send(StampedClientEvent { generation, event });
    }
    pub(super) fn subscribe(&self) -> broadcast::Receiver<ClientEvent> {
        self.legacy.subscribe()
    }
    pub(super) fn subscribe_stamped(&self) -> broadcast::Receiver<StampedClientEvent> {
        self.stamped.subscribe()
    }
}

#[derive(Clone)]
pub(super) struct LifecycleState(watch::Sender<ConnectionSnapshot>);
impl LifecycleState {
    pub(super) fn new() -> Self {
        Self(
            watch::channel(ConnectionSnapshot {
                generation: 0,
                state: ConnectionState::Idle,
            })
            .0,
        )
    }
    pub(super) fn snapshot(&self) -> ConnectionSnapshot {
        *self.0.borrow()
    }
    pub(super) fn transition(&self, generation: u64, state: ConnectionState) {
        self.0.send_if_modified(|current| {
            if current.generation != generation {
                return false;
            }
            current.state = state;
            true
        });
    }
    fn advance(&self, state: ConnectionState) -> u64 {
        let mut generation = 0;
        self.0.send_modify(|current| {
            current.generation = current
                .generation
                .checked_add(1)
                .expect("connection generation exhausted");
            current.state = state;
            generation = current.generation;
        });
        generation
    }
}

struct TaskExit {
    generation: u64,
    state: LifecycleState,
    stop: CancellationToken,
}
impl Drop for TaskExit {
    fn drop(&mut self) {
        self.state.transition(
            self.generation,
            if self.stop.is_cancelled() {
                ConnectionState::Idle
            } else {
                ConnectionState::Failed
            },
        );
    }
}

struct ActiveConnection {
    stop: CancellationToken,
    task: tokio::task::JoinHandle<()>,
}

pub(super) struct ConnectionOwner {
    active: Mutex<Option<ActiveConnection>>,
    state: LifecycleState,
}

impl ConnectionOwner {
    pub(super) fn new() -> Self {
        Self {
            active: Mutex::new(None),
            state: LifecycleState::new(),
        }
    }
    pub(super) fn snapshot(&self) -> ConnectionSnapshot {
        self.state.snapshot()
    }

    async fn retire(&self, active: &mut Option<ActiveConnection>) -> Result<(), TikTokError> {
        if let Some(previous) = active.as_mut() {
            previous.stop.cancel();
            let generation = self.snapshot().generation;
            self.state
                .transition(generation, ConnectionState::Disconnecting);
            match tokio::time::timeout(TEARDOWN_DEADLINE, &mut previous.task).await {
                Ok(result) => {
                    active.take();
                    if let Err(error) = result {
                        self.state.transition(generation, ConnectionState::Failed);
                        return Err(TikTokError::Task(error.to_string()));
                    }
                }
                Err(_) => {
                    self.state.transition(generation, ConnectionState::Failed);
                    return Err(TikTokError::Task("connection teardown deadline exceeded; ownership retained and replacement refused".into()));
                }
            }
        }
        Ok(())
    }

    pub(super) async fn connect<F, Fut>(
        &self,
        launch: F,
    ) -> Result<(u64, ConnectionInfo), TikTokError>
    where
        F: FnOnce(
            u64,
            CancellationToken,
            LifecycleState,
            oneshot::Sender<Result<ConnectionInfo, TikTokError>>,
        ) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // The control lock protects ownership changes only. Network setup
        // runs in the owned task, after this guard has been dropped.
        let mut active = self.active.lock().await;
        self.retire(&mut active).await?;
        let generation = self.state.advance(ConnectionState::Connecting);
        let stop = CancellationToken::new();
        let (ready, answer) = oneshot::channel();
        let future = launch(generation, stop.clone(), self.state.clone(), ready);
        let exit = TaskExit {
            generation,
            state: self.state.clone(),
            stop: stop.clone(),
        };
        let task = tokio::spawn(async move {
            let _exit = exit;
            future.await;
        });
        *active = Some(ActiveConnection { stop, task });
        drop(active);
        let info = match answer.await {
            Ok(result) => result?,
            Err(error) => {
                self.state.transition(generation, ConnectionState::Failed);
                return Err(TikTokError::Task(error.to_string()));
            }
        };
        if self.snapshot().generation != generation {
            return Err(TikTokError::Task("connection superseded".into()));
        }
        Ok((generation, info))
    }

    pub(super) async fn disconnect(&self) -> Result<u64, TikTokError> {
        let mut active = self.active.lock().await;
        self.retire(&mut active).await?;
        Ok(self.state.advance(ConnectionState::Idle))
    }
}

impl Drop for ConnectionOwner {
    fn drop(&mut self) {
        if let Some(active) = self.active.get_mut().as_ref() {
            active.stop.cancel();
        }
    }
}

pub(super) async fn interruptible<T>(
    stop: &CancellationToken,
    future: impl Future<Output = Result<T, TikTokError>>,
) -> Result<T, TikTokError> {
    tokio::select! {
        biased;
        () = stop.cancelled() => Err(TikTokError::Task("connection superseded".into())),
        result = tokio::time::timeout(NETWORK_DEADLINE, future) => result.unwrap_or_else(|_| Err(TikTokError::RetryableTransport("connection network deadline exceeded".into()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn info() -> ConnectionInfo {
        ConnectionInfo {
            unique_id: "test".into(),
            room_id: "1".into(),
            title: String::new(),
            nickname: "test".into(),
            avatar_url: None,
            viewers: 0,
            total_users: 0,
            gifts: vec![],
        }
    }

    #[tokio::test]
    async fn cancellation_interrupts_initialization_and_joins_before_replacement() {
        let owner = Arc::new(ConnectionOwner::new());
        let (started, started_rx) = oneshot::channel();
        let first_owner = Arc::clone(&owner);
        let first = tokio::spawn(async move {
            first_owner
                .connect(|_, stop, _, ready| async move {
                    let _ = started.send(());
                    let result = interruptible(
                        &stop,
                        std::future::pending::<Result<ConnectionInfo, TikTokError>>(),
                    )
                    .await;
                    let _ = ready.send(result);
                })
                .await
        });
        started_rx.await.unwrap();
        assert_eq!(owner.snapshot().state, ConnectionState::Connecting);
        let second = owner
            .connect(|generation, stop, state, ready| async move {
                state.transition(generation, ConnectionState::Connected);
                let _ = ready.send(Ok(info()));
                stop.cancelled().await;
            })
            .await
            .unwrap();
        assert_eq!(second.1.unique_id, "test");
        assert!(first.await.unwrap().is_err());
        assert_eq!(owner.snapshot().generation, 2);
        owner.disconnect().await.unwrap();
        assert!(owner.active.lock().await.is_none());
        assert_eq!(owner.snapshot().state, ConnectionState::Idle);
    }

    #[tokio::test]
    async fn disconnect_during_initialization_releases_the_task() {
        let owner = Arc::new(ConnectionOwner::new());
        let (started, started_rx) = oneshot::channel();
        let task_owner = Arc::clone(&owner);
        let request = tokio::spawn(async move {
            task_owner
                .connect(|_, stop, _, ready| async move {
                    let _ = started.send(());
                    stop.cancelled().await;
                    let _ = ready.send(Err(TikTokError::Task("cancelled".into())));
                })
                .await
        });
        started_rx.await.unwrap();
        owner.disconnect().await.unwrap();
        assert!(request.await.unwrap().is_err());
        assert!(owner.active.lock().await.is_none());
    }

    #[tokio::test]
    async fn repeated_cycles_have_one_owned_task_and_no_survivors() {
        let owner = ConnectionOwner::new();
        let active = Arc::new(AtomicUsize::new(0));
        for _ in 0..128 {
            let count = Arc::clone(&active);
            owner
                .connect(move |generation, stop, state, ready| async move {
                    assert_eq!(count.fetch_add(1, Ordering::SeqCst), 0);
                    state.transition(generation, ConnectionState::Connected);
                    let _ = ready.send(Ok(info()));
                    stop.cancelled().await;
                    count.fetch_sub(1, Ordering::SeqCst);
                })
                .await
                .unwrap();
            owner.disconnect().await.unwrap();
            assert_eq!(active.load(Ordering::SeqCst), 0);
        }
        assert_eq!(owner.snapshot().generation, 256);
    }

    #[test]
    fn retired_generations_cannot_change_current_state() {
        let state = LifecycleState::new();
        let first = state.advance(ConnectionState::Connecting);
        let second = state.advance(ConnectionState::Connected);
        state.transition(first, ConnectionState::Idle);
        assert_eq!(state.snapshot().generation, second);
        assert_eq!(state.snapshot().state, ConnectionState::Connected);
        let sender = EventSender::new();
        let mut stream = sender.subscribe_stamped();
        sender.send(
            first,
            ClientEvent::Disconnected {
                reason: "old".into(),
            },
        );
        assert_eq!(stream.try_recv().unwrap().generation, first);
    }
    #[tokio::test]
    async fn unexpected_task_failure_cannot_leave_connected_status() {
        let owner = ConnectionOwner::new();
        let result = owner
            .connect(|generation, _, state, _ready| async move {
                state.transition(generation, ConnectionState::Connected);
                panic!("synthetic connection task crash");
            })
            .await;
        assert!(result.is_err());
        assert_eq!(owner.snapshot().state, ConnectionState::Failed);
        assert!(owner.disconnect().await.is_err());
        assert!(owner.active.lock().await.is_none());
        owner.disconnect().await.unwrap();
    }
    #[tokio::test]
    async fn simultaneous_requests_keep_one_authoritative_task() {
        let owner = Arc::new(ConnectionOwner::new());
        let active = Arc::new(AtomicUsize::new(0));
        let mut requests = tokio::task::JoinSet::new();
        for _ in 0..32 {
            let owner = Arc::clone(&owner);
            let count = Arc::clone(&active);
            requests.spawn(async move {
                owner
                    .connect(move |generation, stop, state, ready| async move {
                        assert_eq!(count.fetch_add(1, Ordering::SeqCst), 0);
                        state.transition(generation, ConnectionState::Connected);
                        let _ = ready.send(Ok(info()));
                        stop.cancelled().await;
                        count.fetch_sub(1, Ordering::SeqCst);
                    })
                    .await
            });
        }
        while let Some(result) = requests.join_next().await {
            let _ = result.unwrap();
        }
        assert_eq!(owner.snapshot().generation, 32);
        assert_eq!(active.load(Ordering::SeqCst), 1);
        owner.disconnect().await.unwrap();
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }
}
