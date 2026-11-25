//! Dedicated plugin → host call channel.
//!
//! The legacy reverse channel piggybacks on the host→plugin stdio frames and
//! can only be drained while a host call is in flight. A remote-control plugin
//! owns a listener thread and must be able to call the host at any time, from
//! any thread. This module gives each process instance a private loopback
//! socket with a random handshake token; the host runs one pump thread that
//! demultiplexes `host.call` requests independently of stdio.
//!
//! No bearer token, `ControlApi` or host handle crosses this boundary: only
//! `{method, params}` JSON frames and the host's sanitized result.

use std::{
    io,
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use serde_json::Value;
use tiktools_plugin_api::{
    read_frame, write_frame, FrameError, HostCallRequest, PluginFrame, PluginResponse,
    METHOD_HOST_CALL, TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
};

use crate::HostCallHandler;

/// Maximum concurrent host calls per plugin instance on the reverse channel.
const MAX_INFLIGHT: usize = 8;
/// Per-call budget. The plugin protocol's own frame cap still applies.
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// Healthy pumps observe the stop flag within one nonblocking accept sleep or
/// one 200ms connection read. A partial handshake can still hold the pump in
/// its longer read timeout; teardown reports and detaches that pathological
/// thread instead of blocking the owning plugin worker.
const DROP_JOIN_DEADLINE: Duration = Duration::from_millis(500);
const DROP_JOIN_POLL: Duration = Duration::from_millis(5);

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS random source unavailable");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Handle to a plugin's reverse channel. Dropping it signals the pump to stop.
pub struct HostCallChannel {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    plugin_id: String,
}

impl HostCallChannel {
    /// Bind a loopback listener and spawn the pump. Returns the connection
    /// address and the one-time handshake token to pass to the plugin through
    /// its environment. The handler is installed later via
    /// [`HostCallChannel::set_handler`] (the instance may be created before the
    /// host installs the global sink).
    pub fn bind() -> io::Result<(String, String, Arc<HostCallGate>, TcpListener)> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let token = random_token();
        let gate = Arc::new(HostCallGate::default());
        Ok((addr.to_string(), token, gate, listener))
    }

    /// Spawn the pump thread. Called once the instance has a handler available.
    pub fn spawn(
        listener: TcpListener,
        expected_token: String,
        gate: Arc<HostCallGate>,
        plugin_id: String,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = Arc::clone(&stop);
        let owner_id = plugin_id.clone();
        let thread = std::thread::Builder::new()
            .name(format!("tiktools-plugin-hostcall-{plugin_id}"))
            .spawn(move || {
                // Keep the listener nonblocking: `Drop` only has the stop
                // flag, so a blocking `accept()` would otherwise keep the
                // pump — and therefore the plugin worker teardown — alive
                // forever when no new connection arrives.
                loop {
                    if stop_thread.load(Ordering::Relaxed) {
                        break;
                    }
                    let (mut stream, _) = match listener.accept() {
                        Ok(pair) => pair,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(20));
                            continue;
                        }
                        Err(_) => break,
                    };
                    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
                    let _ = stream.set_write_timeout(Some(CALL_TIMEOUT));
                    // One connection per instance: a second connection is a
                    // protocol violation, so close it immediately.
                    let handshake: Result<Value, FrameError> = read_frame(&mut stream);
                    if !matches!(&handshake, Ok(value) if value.as_str() == Some(expected_token.as_str())) {
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                        continue;
                    }
                    if let Err(error) = serve_reverse(stream, &gate, &plugin_id, &stop_thread) {
                        tracing::debug!(plugin = %plugin_id, %error, "plugin reverse channel closed");
                    }
                    break;
                }
            });
        Self {
            stop,
            thread: thread.ok(),
            plugin_id: owner_id,
        }
    }
}

impl Drop for HostCallChannel {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let Some(thread) = self.thread.take() else {
            return;
        };
        let started = std::time::Instant::now();
        while !thread.is_finished() {
            if started.elapsed() >= DROP_JOIN_DEADLINE {
                tracing::error!(
                    plugin = %self.plugin_id,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "plugin reverse-channel pump did not exit; abandoning it (outcome unknown)"
                );
                return;
            }
            std::thread::sleep(DROP_JOIN_POLL);
        }
        let _ = thread.join();
    }
}

/// Shared gate: installs the handler and bounds concurrent calls.
#[derive(Default)]
pub struct HostCallGate {
    handler: Mutex<Option<Arc<dyn HostCallHandler>>>,
    inflight: Arc<Mutex<usize>>,
}

impl HostCallGate {
    pub fn set_handler(&self, handler: Arc<dyn HostCallHandler>) {
        *self
            .handler
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(handler);
    }

    fn handler(&self) -> Option<Arc<dyn HostCallHandler>> {
        self.handler
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn acquire(&self) -> bool {
        let mut count = self.inflight.lock().unwrap_or_else(|e| e.into_inner());
        if *count >= MAX_INFLIGHT {
            return false;
        }
        *count += 1;
        true
    }

    fn release(&self) {
        let mut count = self.inflight.lock().unwrap_or_else(|e| e.into_inner());
        *count = count.saturating_sub(1);
    }
}

fn serve_reverse(
    mut stream: TcpStream,
    gate: &HostCallGate,
    plugin_id: &str,
    stop: &AtomicBool,
) -> Result<(), FrameError> {
    // Short read timeout so shutdown is responsive; a frame that is still
    // arriving is resumed on the next loop rather than treated as an error.
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let frame = match read_frame::<_, PluginFrame>(&mut stream) {
            Ok(frame) => frame,
            Err(FrameError::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            Err(error) => return Err(error),
        };
        let PluginFrame::Request(request) = frame else {
            // A plugin must not answer its own host calls on this channel.
            return Err(FrameError::Json(
                serde_json::from_str::<Value>("invalid").unwrap_err(),
            ));
        };
        let answer = if request.method != METHOD_HOST_CALL {
            Err(format!("unsupported reverse method `{}`", request.method))
        } else if !gate.acquire() {
            Err("too many in-flight host calls".to_owned())
        } else {
            let result = serde_json::from_value::<HostCallRequest>(request.payload)
                .map_err(|error| error.to_string())
                .and_then(|call| {
                    gate.handler()
                        .ok_or_else(|| "host calls are not granted to this plugin".to_owned())?
                        .call(plugin_id, &call.method, call.params)
                });
            gate.release();
            result
        };
        let (ok, value, error) = match answer {
            Ok(value) => (true, Some(value), None),
            Err(error) => (false, None, Some(error)),
        };
        write_frame(
            &mut stream,
            &PluginResponse {
                protocol_version: TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
                id: request.id,
                ok,
                result: value,
                error,
            },
        )?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiktools_plugin_api::PluginRequest;

    #[test]
    fn token_is_random_and_not_empty() {
        let a = random_token();
        let b = random_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }

    struct Echo;
    impl HostCallHandler for Echo {
        fn call(&self, plugin_id: &str, method: &str, params: Value) -> Result<Value, String> {
            Ok(serde_json::json!({"plugin": plugin_id, "method": method, "params": params}))
        }
    }

    #[test]
    fn reverse_channel_answers_unsolicited_calls() {
        let (addr, token, gate, listener) = HostCallChannel::bind().unwrap();
        gate.set_handler(Arc::new(Echo));
        let token_for_test = token.clone();
        let channel = HostCallChannel::spawn(listener, token, gate, "demo.plugin".to_owned());
        let mut stream = TcpStream::connect(&addr).unwrap();
        write_frame(&mut stream, &Value::String(token_for_test)).unwrap();
        let request = PluginRequest::new(
            "host-1",
            METHOD_HOST_CALL,
            serde_json::to_value(HostCallRequest {
                method: "remote.status".to_owned(),
                params: serde_json::json!({}),
            })
            .unwrap(),
        );
        write_frame(&mut stream, &PluginFrame::Request(request)).unwrap();
        let response: PluginResponse = read_frame(&mut stream).unwrap();
        assert_eq!(response.id, "host-1");
        assert!(response.ok);
        assert_eq!(response.result.unwrap()["plugin"], "demo.plugin");
        drop(channel);
    }

    #[test]
    fn drop_without_a_connection_does_not_wait_for_accept() {
        let (_addr, token, gate, listener) = HostCallChannel::bind().unwrap();
        gate.set_handler(Arc::new(Echo));
        let channel = HostCallChannel::spawn(listener, token, gate, "demo.plugin".to_owned());
        let (done_tx, done) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            drop(channel);
            let _ = done_tx.send(());
        });
        assert!(
            done.recv_timeout(Duration::from_millis(500)).is_ok(),
            "dropping an idle reverse channel must not wait in accept()"
        );
    }

    #[test]
    fn drop_with_an_incomplete_handshake_is_bounded() {
        let (addr, token, gate, listener) = HostCallChannel::bind().unwrap();
        gate.set_handler(Arc::new(Echo));
        let channel = HostCallChannel::spawn(listener, token, gate, "demo.plugin".to_owned());
        let _stream = TcpStream::connect(&addr).unwrap();
        let (done_tx, done) = std::sync::mpsc::channel();
        let started = std::time::Instant::now();
        std::thread::spawn(move || {
            drop(channel);
            let _ = done_tx.send(());
        });
        assert!(
            done.recv_timeout(Duration::from_secs(1)).is_ok(),
            "dropping a partial-handshake channel must be bounded"
        );
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn gate_bounds_concurrency() {
        let gate = HostCallGate::default();
        for _ in 0..MAX_INFLIGHT {
            assert!(gate.acquire());
        }
        assert!(!gate.acquire());
        gate.release();
        assert!(gate.acquire());
    }
}
