use std::{
    sync::{atomic::AtomicBool, Mutex},
    thread,
    time::{Duration, Instant},
};

use tiktools_plugin_api::PluginRuntimeKind;
use tokio::sync::{mpsc, oneshot};

use crate::{PluginInstance, PluginLoaderError};

/// One queued request with the absolute deadline the worker enforces.
/// The deadline covers queueing plus execution; the worker discards expired
/// requests before executing them so a timed-out caller never causes stale
/// plugin work to accumulate.
pub(crate) struct QueuedCall {
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
/// The queue is a Tokio unbounded channel: async callers send without
/// blocking and await a `oneshot` response, so no `spawn_blocking` thread
/// is spent per call. Worker threads are plain OS threads that receive
/// through `blocking_recv`, which needs no runtime context.
pub(crate) struct RunningInstance {
    pub(crate) token: u64,
    pub(crate) kind: PluginRuntimeKind,
    pub(crate) cold: AtomicBool,
    pub(crate) tx: mpsc::UnboundedSender<WorkerMsg>,
    pub(crate) worker: Mutex<Option<thread::JoinHandle<Result<(), PluginLoaderError>>>>,
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
        if Instant::now() >= call.deadline {
            let _ = call.respond.send(Err(PluginLoaderError::Timeout(format!(
                "plugin `{id}` call expired while queued"
            ))));
            continue;
        }
        let result = instance.handle_message_with_timeout(&call.request, call.timeout);
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
