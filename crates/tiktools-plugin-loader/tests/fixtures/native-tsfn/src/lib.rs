//! Minimal napi-rs fixture for the napi-vm native addon tests.
//!
//! It stands in for a real device package such as `rdev-node`: synchronous
//! native calls plus a threadsafe-function listener with its own stop API,
//! so shutdown and reload stay deterministic. Built by the integration test
//! with `cargo build --offline --release`; the committed `Cargo.lock` pins
//! the exact dependency closure the test rebuilds.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use napi::bindgen_prelude::{FnArgs, Function};
use napi::threadsafe_function::ThreadsafeFunctionCallMode;
use napi_derive::napi;

struct ListenerState {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

static LISTENER: Mutex<Option<ListenerState>> = Mutex::new(None);

#[napi]
pub fn add(left: i32, right: i32) -> i32 {
    left + right
}

/// Starts a detached worker that delivers one threadsafe-function event
/// after about 50ms and then exits. The host VM thread is typically idle
/// by then, which is exactly what the idle-delivery test observes.
#[napi]
pub fn start_listener(callback: Function<'_, FnArgs<(String,)>, ()>) -> napi::Result<bool> {
    let tsfn = callback
        .build_threadsafe_function::<FnArgs<(String,)>>()
        .build()?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let worker = thread::Builder::new()
        .name("tiktools-tsfn-fixture".into())
        .spawn(move || {
            thread::sleep(Duration::from_millis(50));
            if !stop_flag.load(Ordering::Acquire) {
                let _ = tsfn.call(
                    FnArgs {
                        data: ("key:space".to_owned(),),
                    },
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
        })
        .map_err(|error| napi::Error::from_reason(format!("spawn failed: {error}")))?;
    *LISTENER.lock().unwrap() = Some(ListenerState {
        stop,
        worker: Some(worker),
    });
    Ok(true)
}

/// Signals the worker to skip its event and joins it. Guests call this
/// from `onUnload` so stop and reload never strand a live native thread.
#[napi]
pub fn stop_listener() -> bool {
    let mut guard = LISTENER.lock().unwrap();
    let Some(mut state) = guard.take() else {
        return false;
    };
    state.stop.store(true, Ordering::Release);
    if let Some(worker) = state.worker.take() {
        let _ = worker.join();
    }
    true
}
