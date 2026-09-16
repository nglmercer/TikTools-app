use std::{
    sync::{atomic::AtomicBool, mpsc, Mutex},
    thread,
    time::{Duration, Instant},
};

use tiktools_plugin_api::PluginRuntimeKind;

use crate::{PluginInstance, PluginLoaderError};

/// One queued request with the absolute deadline the worker enforces.
/// The deadline covers queueing plus execution; the worker discards expired
/// requests before executing them so a timed-out caller never causes stale
/// plugin work to accumulate.
pub(crate) struct QueuedCall {
    pub(crate) request: Vec<u8>,
    pub(crate) timeout: Duration,
    pub(crate) deadline: Instant,
    pub(crate) respond: mpsc::Sender<Result<Vec<u8>, PluginLoaderError>>,
}

pub(crate) enum WorkerMsg {
    Call(QueuedCall),
    Shutdown,
}

/// A running plugin: the worker thread is the sole owner of the instance,
/// so plugin `&mut self` code stays single-threaded without a mutex. The
/// token identifies this start generation so failure cleanup never removes
/// a newer instance. `kind` drives timeout recovery (process workers kill
/// their child on the same deadline, native workers keep running), and
/// `cold` grants the first call of a process generation cold-start grace.
pub(crate) struct RunningInstance {
    pub(crate) token: u64,
    pub(crate) kind: PluginRuntimeKind,
    pub(crate) cold: AtomicBool,
    pub(crate) tx: mpsc::Sender<WorkerMsg>,
    pub(crate) worker: Mutex<Option<thread::JoinHandle<Result<(), PluginLoaderError>>>>,
}

pub(crate) fn run_instance_worker(
    id: String,
    mut instance: Box<dyn PluginInstance>,
    rx: mpsc::Receiver<WorkerMsg>,
) -> Result<(), PluginLoaderError> {
    while let Ok(msg) = rx.recv() {
        let WorkerMsg::Call(call) = msg else {
            break;
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
    for queued in rx.try_iter() {
        if let WorkerMsg::Call(call) = queued {
            let _ = call.respond.send(Err(PluginLoaderError::Runtime(format!(
                "plugin `{id}` stopped"
            ))));
        }
    }
    instance.shutdown()
}
