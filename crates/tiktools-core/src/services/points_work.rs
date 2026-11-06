//! Owned bounded LIVE points execution, separate from durable SQLite writing.
//! The worker owns only PointsService, never AppCore. An executing synchronous
//! award is awaited to completion; cancellation is only reported before admission.
use super::{AwardOptions, PointAction, PointAward, PointsService};
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

const CAPACITY: usize = 128;

/// Join bound for the work worker during shutdown. A worker wedged inside a
/// synchronous award (waiting on a hung disk, or blocked behind one) must
/// not block process shutdown forever: the wait is abandoned with an
/// explicit report — queued awards behind the wedge are reported as
/// possibly unresolved rather than disappearing — and the ladder moves on
/// to the durable writer join below. Matches the writer's own bound so the
/// two halves of one shutdown report the same policy.
const WORKER_JOIN_DEADLINE: Duration = Duration::from_secs(30);

/// Poll interval while a bounded join waits for the worker to exit.
const WORKER_JOIN_POLL: Duration = Duration::from_millis(5);
struct AwardJob {
    unique_id: String,
    action: PointAction,
    options: AwardOptions,
    answer: oneshot::Sender<Option<PointAward>>,
}
pub(crate) struct PointsWorkService {
    sender: Mutex<Option<mpsc::Sender<AwardJob>>>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
    stopping: CancellationToken,
    /// Workers abandoned past the join deadline (outcome unknown). Zero in
    /// the healthy case; reported by [`Self::shutdown`] when it expires.
    abandoned_joins: Arc<AtomicU64>,
    /// Test-only: simulates a wedged synchronous SQLite award so shutdown
    /// ladders and stall policies can be exercised deterministically. The
    /// gate always releases when shutdown begins, so a stall can delay
    /// shutdown only by the configured bound — never hang it. A wedge
    /// ([`Self::wedge_for_test`]) ignores the release and proves the join
    /// bound instead.
    #[cfg(test)]
    stall_ms: Arc<AtomicU64>,
    #[cfg(test)]
    stall_hard: Arc<AtomicBool>,
}
impl PointsWorkService {
    pub(crate) fn new(points: Arc<PointsService>) -> Self {
        let (sender, mut receiver) = mpsc::channel::<AwardJob>(CAPACITY);
        let stopping = CancellationToken::new();
        let stopping_for_worker = stopping.clone();
        #[cfg(test)]
        let stall_ms = Arc::new(AtomicU64::new(0));
        #[cfg(test)]
        let stall_for_worker = Arc::clone(&stall_ms);
        #[cfg(test)]
        let stall_hard = Arc::new(AtomicBool::new(false));
        #[cfg(test)]
        let hard_for_worker = Arc::clone(&stall_hard);
        let worker = std::thread::Builder::new()
            .name("live-points".into())
            .spawn(move || {
                while let Some(job) = receiver.blocking_recv() {
                    #[cfg(test)]
                    {
                        let stall = stall_for_worker.load(Ordering::Relaxed);
                        if stall > 0 {
                            let hard = hard_for_worker.load(Ordering::Relaxed);
                            // A wedge keeps holding the worker through
                            // shutdown cancellation, so tests can prove the
                            // join bound in `shutdown_with_deadline`.
                            let deadline = if stall == u64::MAX {
                                Instant::now() + Duration::from_secs(120)
                            } else {
                                Instant::now() + Duration::from_millis(stall)
                            };
                            while Instant::now() < deadline
                                && (hard || !stopping_for_worker.is_cancelled())
                            {
                                std::thread::sleep(Duration::from_millis(5));
                            }
                        }
                    }
                    #[cfg(not(test))]
                    let _ = &stopping_for_worker;
                    // Dropped queued callers never cause delayed effects.
                    if job.answer.is_closed() {
                        continue;
                    }
                    let result = points.award_points(&job.unique_id, job.action, job.options);
                    let _ = job.answer.send(result);
                }
            });
        match worker {
            Ok(worker) => Self {
                sender: Mutex::new(Some(sender)),
                worker: Mutex::new(Some(worker)),
                stopping,
                abandoned_joins: Arc::new(AtomicU64::new(0)),
                #[cfg(test)]
                stall_ms,
                #[cfg(test)]
                stall_hard,
            },
            Err(error) => {
                tracing::error!(%error, "LIVE points execution worker unavailable");
                Self {
                    sender: Mutex::new(None),
                    worker: Mutex::new(None),
                    stopping,
                    abandoned_joins: Arc::new(AtomicU64::new(0)),
                    #[cfg(test)]
                    stall_ms,
                    #[cfg(test)]
                    stall_hard,
                }
            }
        }
    }

    /// Test-only: hold the next award inside the worker for up to `duration`
    /// (released earlier if shutdown starts).
    #[cfg(test)]
    pub(crate) fn stall_for_test(&self, duration: Duration) {
        self.stall_ms
            .store(duration.as_millis() as u64, Ordering::SeqCst);
    }

    /// Test-only: wedge the worker so shutdown cancellation cannot release
    /// it, so tests can prove the join bound in [`Self::shutdown_with_deadline`].
    #[cfg(test)]
    pub(crate) fn wedge_for_test(&self) {
        self.stall_ms.store(u64::MAX, Ordering::SeqCst);
        self.stall_hard.store(true, Ordering::SeqCst);
    }
    pub(crate) async fn award(
        &self,
        unique_id: String,
        action: PointAction,
        options: AwardOptions,
    ) -> Result<Option<PointAward>, String> {
        let sender = crate::recover_mutex(&self.sender, "LIVE points sender")
            .clone()
            .ok_or("LIVE points execution stopped")?;
        let permit = tokio::select! {
            biased;
            () = self.stopping.cancelled() => return Err("LIVE points execution stopped before admission".into()),
            permit = sender.reserve() => permit.map_err(|_| "LIVE points execution unavailable")?,
        };
        let (answer, response) = oneshot::channel();
        permit.send(AwardJob {
            unique_id,
            action,
            options,
            answer,
        });
        // No timeout drops a running award. The ordered ingress worker
        // cannot move to a later event while an earlier effect is unresolved.
        response
            .await
            .map_err(|_| "LIVE points execution worker failed".into())
    }
    pub(crate) async fn shutdown(&self) -> bool {
        self.shutdown_with_deadline(WORKER_JOIN_DEADLINE).await
    }

    /// Cancels admission (dropping queued callers with an explicit error),
    /// then joins the worker within `deadline`. The join is bounded: a
    /// worker wedged inside an award is abandoned with a loud report that
    /// awards behind it may remain unresolved — the same policy as the
    /// durable writer join that runs after this in the shutdown ladder.
    pub(crate) async fn shutdown_with_deadline(&self, deadline: Duration) -> bool {
        self.stopping.cancel();
        crate::recover_mutex(&self.sender, "LIVE points sender").take();
        let worker = crate::recover_mutex(&self.worker, "LIVE points worker").take();
        let Some(worker) = worker else {
            return true;
        };
        let abandoned = Arc::clone(&self.abandoned_joins);
        match tokio::task::spawn_blocking(move || {
            let started = Instant::now();
            while !worker.is_finished() {
                if started.elapsed() >= deadline {
                    abandoned.fetch_add(1, Ordering::Relaxed);
                    return Err("join deadline expired");
                }
                std::thread::sleep(WORKER_JOIN_POLL);
            }
            worker.join().map_err(|_| "worker panicked")
        })
        .await
        {
            Ok(Ok(())) => true,
            Ok(Err(cause)) => {
                tracing::error!(
                    cause,
                    deadline_ms = deadline.as_millis() as u64,
                    "LIVE points worker failed to stop; queued awards may remain unresolved"
                );
                false
            }
            Err(error) => {
                tracing::error!(%error, "LIVE points worker join task failed");
                false
            }
        }
    }

    /// Workers abandoned past the join deadline, outcome unknown
    /// (diagnostics). Zero in the healthy case.
    pub(crate) fn abandoned_join_count(&self) -> u64 {
        self.abandoned_joins.load(Ordering::Relaxed)
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        let sender = crate::recover_mutex(&self.sender, "LIVE points sender");
        serde_json::json!({ "capacity": CAPACITY, "queued": sender.as_ref().map_or(0, |sender| CAPACITY - sender.capacity()), "stopping": self.stopping.is_cancelled(), "available": sender.as_ref().is_some_and(|sender| !sender.is_closed()), "abandonedJoins": self.abandoned_join_count() })
    }
}
impl Drop for PointsWorkService {
    fn drop(&mut self) {
        self.stopping.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn wedged_worker_is_abandoned_at_the_join_deadline_instead_of_hanging() {
        let points = Arc::new(PointsService::default());
        let worker = Arc::new(PointsWorkService::new(Arc::clone(&points)));
        // Wedge the worker through shutdown cancellation so only the join
        // deadline can free the ladder.
        worker.wedge_for_test();
        let wedged = tokio::spawn({
            let worker = Arc::clone(&worker);
            async move {
                let _ = worker
                    .award("viewer".into(), PointAction::Chat, AwardOptions::default())
                    .await;
            }
        });
        // Give the worker time to pick the job up and enter the wedge.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let started = Instant::now();
        worker
            .shutdown_with_deadline(Duration::from_millis(100))
            .await;
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(5),
            "shutdown must give up at the join deadline (took {elapsed:?})"
        );
        assert_eq!(
            worker.abandoned_join_count(),
            1,
            "the abandoned worker must be counted, not silently detached"
        );
        // The wedged award never resolves; drop the pending caller.
        wedged.abort();
    }

    #[tokio::test]
    async fn repeated_workers_join_release_owner_and_apply_exact_order() {
        let points = Arc::new(PointsService::default());
        for _ in 0..128 {
            let worker = PointsWorkService::new(Arc::clone(&points));
            for _ in 0..8 {
                worker
                    .award("viewer".into(), PointAction::Chat, AwardOptions::default())
                    .await
                    .unwrap()
                    .unwrap();
            }
            worker.shutdown().await;
            assert_eq!(Arc::strong_count(&points), 1);
            assert_eq!(worker.diagnostics()["queued"], 0);
            assert!(worker
                .award("viewer".into(), PointAction::Chat, AwardOptions::default())
                .await
                .is_err());
        }
        assert_eq!(points.viewer("viewer").unwrap()["totalChats"], 1024);
    }
}
