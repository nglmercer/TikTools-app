//! Bounded, advisory attribution for losses in the native broadcast stream.
//!
//! The event producer never waits for this bookkeeping. If concurrent sends,
//! contention, or bounded history make the skipped range ambiguous, callers
//! receive no attribution and must not guess from the current connection.
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::broadcast;

use crate::{ClientEvent, StampedClientEvent};

const MAX_RUNS: usize = 4096;
const MAX_BINDINGS: usize = 128;
const MAX_LOST_GENERATIONS: usize = 128;

#[derive(Debug)]
struct Run {
    first: u64,
    last: u64,
    generation: u64,
}

#[derive(Default)]
struct History {
    last: u64,
    runs: VecDeque<Run>,
    // None means the same generation was observed with conflicting identities.
    bindings: BTreeMap<u64, Option<String>>,
    expired_through: Option<u64>,
    exhausted: bool,
}

impl History {
    fn record(&mut self, generation: u64, event: &ClientEvent) {
        let Some(sequence) = self.last.checked_add(1) else {
            self.exhausted = true;
            return;
        };
        self.last = sequence;
        if let Some(last) = self
            .runs
            .back_mut()
            .filter(|run| run.generation == generation)
        {
            last.last = sequence;
        } else {
            self.runs.push_back(Run {
                first: sequence,
                last: sequence,
                generation,
            });
            if self.runs.len() > MAX_RUNS {
                self.runs.pop_front();
            }
        }
        if let ClientEvent::Connected(info) = event {
            if self
                .expired_through
                .is_some_and(|watermark| generation <= watermark)
            {
                return;
            }
            let valid = info.verified_creator.as_deref() == Some(info.unique_id.as_str())
                && !info.unique_id.is_empty()
                && info.unique_id.len() <= 256
                && info.unique_id.trim() == info.unique_id
                && !info.unique_id.chars().any(char::is_control);
            self.bindings
                .entry(generation)
                .and_modify(|known| {
                    if !valid || known.as_deref() != Some(info.unique_id.as_str()) {
                        *known = None;
                    }
                })
                .or_insert_with(|| valid.then(|| info.unique_id.clone()));
            if self.bindings.len() > MAX_BINDINGS {
                if let Some((evicted, _)) = self.bindings.pop_first() {
                    self.expired_through =
                        Some(self.expired_through.map_or(evicted, |old| old.max(evicted)));
                }
            }
        }
    }

    fn generations(&self, first: u64, last: u64) -> Option<Vec<LostGeneration>> {
        if self.exhausted || first > last {
            return None;
        }
        let mut next = first;
        let mut generations = BTreeSet::new();
        for run in &self.runs {
            if run.last < next {
                continue;
            }
            if run.first > next {
                return None;
            }
            generations.insert(run.generation);
            if generations.len() > MAX_LOST_GENERATIONS {
                return None;
            }
            if run.last >= last {
                return generations
                    .into_iter()
                    .map(|generation| {
                        if self
                            .expired_through
                            .is_some_and(|watermark| generation <= watermark)
                        {
                            return None;
                        }
                        let creator = match self.bindings.get(&generation) {
                            Some(Some(creator)) => Some(creator.clone()),
                            Some(None) => return None,
                            None => None,
                        };
                        Some(LostGeneration {
                            generation,
                            creator,
                        })
                    })
                    .collect();
            }
            next = run.last.checked_add(1)?;
        }
        None
    }
}

/// Host-internal identity derived from the native Connected event, when known.
#[derive(Debug, Clone)]
pub struct LostGeneration {
    pub generation: u64,
    pub creator: Option<String>,
}

/// A lag can be unattributable if tracking was contended or history expired.
#[derive(Debug)]
pub enum TrackedReceiveError {
    Lagged {
        count: u64,
        generations: Option<Vec<LostGeneration>>,
    },
    Closed,
}

#[cfg(test)]
type PauseBarriers = Option<(Arc<std::sync::Barrier>, Arc<std::sync::Barrier>)>;

#[derive(Clone, Default)]
pub(crate) struct LossTracker {
    history: Arc<Mutex<History>>,
    uncertainty: Arc<AtomicU64>,
    untracked_inflight: Arc<AtomicUsize>,
    exhausted: Arc<AtomicBool>,
    #[cfg(test)]
    pause_untracked: Arc<Mutex<PauseBarriers>>,
}

impl LossTracker {
    /// Record and broadcast under the same short nonblocking admission. A
    /// contending sender broadcasts normally but invalidates attribution for
    /// existing subscribers, because the ordering can no longer be proven.
    pub(crate) fn send(
        &self,
        sender: &broadcast::Sender<StampedClientEvent>,
        generation: u64,
        event: ClientEvent,
    ) {
        match self.history.try_lock() {
            Ok(mut history) => {
                history.record(generation, &event);
                let _ = sender.send(StampedClientEvent { generation, event });
            }
            Err(_) => {
                // This counter spans the actual send. A new subscription must
                // not trust its initial sequence if an earlier untracked send
                // could still enter the channel after that subscription.
                self.untracked_inflight.fetch_add(1, Ordering::SeqCst);
                struct Inflight<'a>(&'a AtomicUsize);
                impl Drop for Inflight<'_> {
                    fn drop(&mut self) {
                        self.0.fetch_sub(1, Ordering::SeqCst);
                    }
                }
                let _inflight = Inflight(&self.untracked_inflight);
                if self
                    .uncertainty
                    .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_add(1))
                    .is_err()
                {
                    self.exhausted.store(true, Ordering::SeqCst);
                }
                #[cfg(test)]
                if let Some((entered, release)) = self.pause_untracked.lock().unwrap().clone() {
                    entered.wait();
                    release.wait();
                }
                let _ = sender.send(StampedClientEvent { generation, event });
            }
        }
    }

    pub(crate) fn subscribe(
        &self,
        sender: &broadcast::Sender<StampedClientEvent>,
    ) -> TrackedReceiver {
        let history = self
            .history
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Snapshot before subscribing: a producer that fails try_lock during
        // either step changes this epoch before it emits its untracked event.
        let uncertainty = self.uncertainty.load(Ordering::SeqCst);
        let inflight_before = self.untracked_inflight.load(Ordering::SeqCst);
        let next = history.last.checked_add(1);
        let receiver = sender.subscribe();
        let inflight_after = self.untracked_inflight.load(Ordering::SeqCst);
        let reliable_start = inflight_before == 0
            && inflight_after == 0
            && uncertainty == self.uncertainty.load(Ordering::SeqCst);
        TrackedReceiver {
            receiver,
            tracker: self.clone(),
            uncertainty,
            reliable_start,
            next,
        }
    }
}

/// Receiver of the existing broadcast, with optional loss attribution.
pub struct TrackedReceiver {
    receiver: broadcast::Receiver<StampedClientEvent>,
    tracker: LossTracker,
    uncertainty: u64,
    reliable_start: bool,
    next: Option<u64>,
}

impl TrackedReceiver {
    pub async fn recv(&mut self) -> Result<StampedClientEvent, TrackedReceiveError> {
        match self.receiver.recv().await {
            Ok(event) => {
                self.next = self.next.and_then(|n| n.checked_add(1));
                Ok(event)
            }
            Err(broadcast::error::RecvError::Closed) => Err(TrackedReceiveError::Closed),
            Err(broadcast::error::RecvError::Lagged(count)) => {
                let first = self.next;
                let end = first.and_then(|start| start.checked_add(count));
                self.next = end;
                let generations = first.zip(end).and_then(|(start, end)| {
                    if !self.reliable_start
                        || self.tracker.exhausted.load(Ordering::SeqCst)
                        || self.tracker.uncertainty.load(Ordering::SeqCst) != self.uncertainty
                    {
                        return None;
                    }
                    let history = self.tracker.history.try_lock().ok()?;
                    let result = history.generations(start, end.checked_sub(1)?);
                    if self.tracker.uncertainty.load(Ordering::SeqCst) != self.uncertainty {
                        return None;
                    }
                    result
                });
                Err(TrackedReceiveError::Lagged { count, generations })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionInfo;

    fn connected(id: &str) -> ClientEvent {
        ClientEvent::Connected(ConnectionInfo {
            unique_id: id.into(),
            verified_creator: Some(id.into()),
            room_id: "room".into(),
            title: String::new(),
            nickname: String::new(),
            avatar_url: None,
            viewers: 0,
            total_users: 0,
            gifts: Vec::new(),
        })
    }

    fn item() -> ClientEvent {
        ClientEvent::Disconnected {
            reason: "fixture".into(),
        }
    }

    #[tokio::test]
    async fn lag_is_attributed_to_original_generation_without_changing_broadcast() {
        let (sender, _) = broadcast::channel(4);
        let tracker = LossTracker::default();
        let mut tracked = tracker.subscribe(&sender);
        let mut original = sender.subscribe();
        tracker.send(&sender, 1, connected("alice"));
        for _ in 0..5 {
            tracker.send(&sender, 1, item());
        }
        tracker.send(&sender, 2, connected("bob"));
        for _ in 0..2 {
            tracker.send(&sender, 2, item());
        }
        match tracked.recv().await.unwrap_err() {
            TrackedReceiveError::Lagged {
                count,
                generations: Some(generations),
            } => {
                assert_eq!(count, 5);
                assert_eq!(generations.len(), 1);
                assert_eq!(generations[0].generation, 1);
                assert_eq!(generations[0].creator.as_deref(), Some("alice"));
            }
            other => panic!("unexpected loss: {other:?}"),
        }
        assert!(matches!(
            original.recv().await,
            Err(broadcast::error::RecvError::Lagged(5))
        ));
        for _ in 0..4 {
            assert_eq!(
                tracked.recv().await.unwrap().generation,
                original.recv().await.unwrap().generation
            );
        }
    }

    #[tokio::test]
    async fn concurrent_bookkeeping_never_blocks_delivery_or_guesses_loss() {
        let (sender, _) = broadcast::channel(2);
        let tracker = LossTracker::default();
        let mut receiver = tracker.subscribe(&sender);
        let hold = tracker.history.lock().unwrap();
        // This would deadlock if sender waited for attribution bookkeeping.
        tracker.send(&sender, 1, connected("alice"));
        drop(hold);
        for _ in 0..3 {
            tracker.send(&sender, 2, item());
        }
        assert!(matches!(
            receiver.recv().await,
            Err(TrackedReceiveError::Lagged {
                count: 2,
                generations: None
            })
        ));
        assert_eq!(receiver.recv().await.unwrap().generation, 2);
        assert_eq!(receiver.recv().await.unwrap().generation, 2);
    }

    #[tokio::test]
    async fn truncated_history_and_conflicting_identities_remain_unattributed() {
        let (sender, _) = broadcast::channel(2);
        let tracker = LossTracker::default();
        let mut receiver = tracker.subscribe(&sender);
        for n in 0..(MAX_RUNS + 4) {
            tracker.send(&sender, (n % 2) as u64, item());
        }
        assert!(matches!(
            receiver.recv().await,
            Err(TrackedReceiveError::Lagged {
                generations: None,
                ..
            })
        ));

        let (sender, _) = broadcast::channel(2);
        let tracker = LossTracker::default();
        let mut receiver = tracker.subscribe(&sender);
        tracker.send(&sender, 7, connected("alice"));
        tracker.send(&sender, 7, connected("bob"));
        tracker.send(&sender, 7, item());
        tracker.send(&sender, 7, item());
        assert!(matches!(
            receiver.recv().await,
            Err(TrackedReceiveError::Lagged {
                generations: None,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn caller_label_without_discovery_proof_never_attributes_lag() {
        let (sender, _) = broadcast::channel(2);
        let tracker = LossTracker::default();
        let mut receiver = tracker.subscribe(&sender);
        let mut event = connected("alice");
        if let ClientEvent::Connected(info) = &mut event {
            info.verified_creator = None;
        }
        tracker.send(&sender, 7, event);
        for _ in 0..3 {
            tracker.send(&sender, 7, item());
        }
        assert!(matches!(
            receiver.recv().await,
            Err(TrackedReceiveError::Lagged {
                generations: None,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn a_new_subscription_does_not_inherit_prior_contention() {
        let (sender, _) = broadcast::channel(2);
        let tracker = LossTracker::default();
        let hold = tracker.history.lock().unwrap();
        tracker.send(&sender, 1, item());
        drop(hold);
        let mut receiver = tracker.subscribe(&sender);
        tracker.send(&sender, 2, connected("bob"));
        for _ in 0..3 {
            tracker.send(&sender, 2, item());
        }
        match receiver.recv().await.unwrap_err() {
            TrackedReceiveError::Lagged {
                generations: Some(generations),
                ..
            } => {
                assert_eq!(generations.len(), 1);
                assert_eq!(generations[0].generation, 2);
                assert_eq!(generations[0].creator.as_deref(), Some("bob"));
            }
            other => panic!("unexpected loss: {other:?}"),
        }
    }

    #[tokio::test]
    async fn subscription_during_inflight_untracked_send_cannot_claim_attribution() {
        let (sender, _) = broadcast::channel(2);
        let tracker = LossTracker::default();
        tracker.untracked_inflight.store(1, Ordering::SeqCst);
        tracker.uncertainty.store(1, Ordering::SeqCst);
        let mut receiver = tracker.subscribe(&sender);
        tracker.untracked_inflight.store(0, Ordering::SeqCst);
        tracker.send(&sender, 2, connected("bob"));
        for _ in 0..3 {
            tracker.send(&sender, 2, item());
        }
        assert!(matches!(
            receiver.recv().await,
            Err(TrackedReceiveError::Lagged {
                generations: None,
                ..
            })
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_untracked_sender_racing_subscription_never_misattributes() {
        use std::sync::Barrier;
        let (sender, _) = broadcast::channel(2);
        let tracker = LossTracker::default();
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        *tracker.pause_untracked.lock().unwrap() = Some((entered.clone(), release.clone()));
        let history = tracker.history.lock().unwrap();
        let thread = {
            let tracker = tracker.clone();
            let sender = sender.clone();
            std::thread::spawn(move || tracker.send(&sender, 1, connected("alice")))
        };
        entered.wait();
        drop(history);
        // The other sender incremented the epoch but has not emitted yet.
        // A subscription created in this window cannot align its sequence.
        let mut receiver = tracker.subscribe(&sender);
        release.wait();
        thread.join().unwrap();
        *tracker.pause_untracked.lock().unwrap() = None;
        tracker.send(&sender, 2, connected("bob"));
        for _ in 0..3 {
            tracker.send(&sender, 2, item());
        }
        assert!(matches!(
            receiver.recv().await,
            Err(TrackedReceiveError::Lagged {
                generations: None,
                ..
            })
        ));
    }
}
