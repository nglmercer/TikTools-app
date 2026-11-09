use std::{
    sync::{atomic::AtomicBool, Mutex},
    thread,
    time::{Duration, Instant},
};

use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};
use tokio::sync::{mpsc, oneshot, OwnedSemaphorePermit, Semaphore};

/// Includes the executing call. Immediate rejection avoids unlimited waiters.
pub(crate) const PLUGIN_CALL_CAPACITY: usize = 128;

use crate::{PluginInstance, PluginLoaderError};

/// One queued request with the absolute deadline the worker enforces.
/// The deadline covers queueing plus execution; the worker discards expired
/// requests before executing them so a timed-out caller never causes stale
/// plugin work to accumulate.
pub(crate) struct QueuedCall {
    pub(crate) admission: Option<OwnedSemaphorePermit>,
    pub(crate) request: Vec<u8>,
    pub(crate) timeout: Duration,
    pub(crate) deadline: Instant,
    pub(crate) respond: oneshot::Sender<Result<Vec<u8>, PluginLoaderError>>,
}

pub(crate) enum WorkerMsg {
    Call(QueuedCall),
    /// Host-originated work arrived while the owner was idle: pump the VM
    /// event loop. Only runtimes with threaded host ingress (napi-vm native
    /// callbacks) register wakes, and only on their own queue; the generic
    /// worker below ignores it defensively instead of treating it as
    /// shutdown.
    HostEvent,
    Shutdown,
}

/// A running plugin: the worker thread is the sole owner of the instance,
/// so plugin `&mut self` code stays single-threaded without a mutex. The
/// token identifies this start generation so failure cleanup never removes
/// a newer instance. `kind` drives timeout recovery (process workers kill
/// their child on the same deadline, native workers keep running), and
/// `cold` grants the first call of a process generation cold-start grace.
///
/// Calls carry one of 128 admission permits, host wakes are coalesced, and
/// shutdown has no admission requirement. The underlying channel is unbounded
/// only to reserve independent shutdown admission; payload counts are bounded.
/// Async callers send without
/// blocking and await a `oneshot` response, so no `spawn_blocking` thread
/// is spent per call. Worker threads are plain OS threads that receive
/// through `blocking_recv`, which needs no runtime context.
pub(crate) struct RunningInstance {
    pub(crate) token: u64,
    /// Manifest used to construct this exact worker, independent of scans.
    pub(crate) manifest: std::sync::Arc<PluginManifest>,
    pub(crate) events: crate::PluginEventSender,
    pub(crate) artifact_fingerprint: Option<String>,
    pub(crate) kind: PluginRuntimeKind,
    pub(crate) cold: AtomicBool,
    pub(crate) tx: mpsc::UnboundedSender<WorkerMsg>,
    pub(crate) admission: std::sync::Arc<Semaphore>,
    pub(crate) worker: Mutex<Option<thread::JoinHandle<Result<(), PluginLoaderError>>>>,
}

impl RunningInstance {
    /// True once the worker thread has exited, or its handle was already
    /// taken by a stop. Used by `start` so a generation that died without
    /// being observed is restarted instead of being reported as running
    /// forever.
    pub(crate) fn worker_finished(&self) -> bool {
        tiktools_plugin_api::sync::recover_mutex(&self.worker, "plugin worker")
            .as_ref()
            .is_none_or(thread::JoinHandle::is_finished)
    }
}

pub(crate) fn run_instance_worker(
    id: String,
    mut instance: Box<dyn PluginInstance>,
    mut rx: mpsc::UnboundedReceiver<WorkerMsg>,
) -> Result<(), PluginLoaderError> {
    while let Some(msg) = rx.blocking_recv() {
        let call = match msg {
            WorkerMsg::Call(call) => call,
            WorkerMsg::Shutdown => break,
            WorkerMsg::HostEvent => continue,
        };
        if call.respond.is_closed()
            || call
                .admission
                .as_ref()
                .is_some_and(|permit| permit.semaphore().is_closed())
        {
            continue;
        }
        if Instant::now() >= call.deadline {
            let _ = call.respond.send(Err(PluginLoaderError::Timeout(format!(
                "plugin `{id}` call expired while queued"
            ))));
            continue;
        }
        let remaining = call.deadline.saturating_duration_since(Instant::now());
        let result =
            instance.handle_message_with_timeout(&call.request, remaining.min(call.timeout));
        let _ = call.respond.send(result);
    }
    // Fail waiters queued behind the shutdown instead of leaving them on
    // their deadlines.
    while let Ok(msg) = rx.try_recv() {
        if let WorkerMsg::Call(call) = msg {
            let _ = call.respond.send(Err(PluginLoaderError::Runtime(format!(
                "plugin `{id}` stopped"
            ))));
        }
    }
    instance.shutdown()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    struct CountingPlugin(Arc<AtomicUsize>);
    impl PluginInstance for CountingPlugin {
        fn id(&self) -> &str {
            "test"
        }
        fn handle_message(&mut self, _: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(Vec::new())
        }
        fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
            Ok(())
        }
    }

    #[test]
    fn saturated_admission_and_shutdown_release_all_calls_without_execution() {
        let admission = Arc::new(Semaphore::new(PLUGIN_CALL_CAPACITY));
        let (tx, rx) = mpsc::unbounded_channel();
        let mut answers = Vec::new();
        for _ in 0..PLUGIN_CALL_CAPACITY {
            let permit = Arc::clone(&admission).try_acquire_owned().unwrap();
            let (respond, answer) = oneshot::channel();
            tx.send(WorkerMsg::Call(QueuedCall {
                admission: Some(permit),
                request: vec![0; 1024],
                timeout: Duration::from_secs(30),
                deadline: Instant::now() + Duration::from_secs(30),
                respond,
            }))
            .unwrap();
            answers.push(answer);
        }
        assert!(Arc::clone(&admission).try_acquire_owned().is_err());
        admission.close();
        tx.send(WorkerMsg::Shutdown).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        run_instance_worker(
            "test".into(),
            Box::new(CountingPlugin(Arc::clone(&calls))),
            rx,
        )
        .unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(admission.available_permits(), PLUGIN_CALL_CAPACITY);
        assert!(answers.iter_mut().all(|answer| answer.try_recv().is_err()));
    }

    #[test]
    fn cancelled_and_expired_calls_never_execute() {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut retained_answers = Vec::new();
        for cancelled in [true, false] {
            let (respond, answer) = oneshot::channel();
            tx.send(WorkerMsg::Call(QueuedCall {
                admission: None,
                request: Vec::new(),
                timeout: Duration::from_secs(1),
                deadline: if cancelled {
                    Instant::now() + Duration::from_secs(1)
                } else {
                    Instant::now()
                },
                respond,
            }))
            .unwrap();
            if cancelled {
                drop(answer);
            } else {
                retained_answers.push(answer);
            }
        }
        tx.send(WorkerMsg::Shutdown).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        run_instance_worker(
            "test".into(),
            Box::new(CountingPlugin(Arc::clone(&calls))),
            rx,
        )
        .unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }
}
