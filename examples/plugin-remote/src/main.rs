//! Installable remote-control plugin.
//!
//! Owns its listener and executes host operations through `host.call`. It
//! never receives a bearer token, ControlApi, cookies or raw settings. The
//! transport below is a bounded loopback JSON-lines gateway; a production
//! build upgrades it to WSS using a relay ticket obtained from the host via
//! `remote.controller.ticket` (no token enters the plugin).

use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use serde_json::{json, Value};
use tiktools_plugin_sdk::{
    host_remote_execute, run_process_plugin, ActionCall, ActionResult, DomainEventEnvelope,
    Plugin, PluginContext, PluginError, PluginResult,
};

const DEFAULT_BIND: &str = "127.0.0.1:18452";
const MAX_FRAME: usize = 64 * 1024;

#[derive(Default)]
struct RemoteControl {
    gateway: Arc<Mutex<Option<Gateway>>>,
}

struct Gateway {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    address: String,
}

impl Drop for Gateway {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl RemoteControl {
    fn start_gateway(&self, bind: &str) -> PluginResult<String> {
        let mut slot = self
            .gateway
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))?;
        if slot.is_some() {
            return Ok(slot.as_ref().unwrap().address.clone());
        }
        let listener = TcpListener::bind(bind)
            .map_err(|error| PluginError::other(format!("could not bind {bind}: {error}")))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| PluginError::other(error.to_string()))?;
        let address = listener
            .local_addr()
            .map_err(|error| PluginError::other(error.to_string()))?
            .to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("remote-control-gateway".to_owned())
            .spawn(move || serve(listener, thread_stop))
            .map_err(|error| PluginError::other(error.to_string()))?;
        *slot = Some(Gateway {
            stop,
            thread: Some(thread),
            address: address.clone(),
        });
        Ok(address)
    }

    fn stop_gateway(&self) -> PluginResult<()> {
        let mut slot = self
            .gateway
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))?;
        *slot = None;
        Ok(())
    }
}

/// Serve one JSON object per line: `{"op":"status"|"command", ...}`. A command
/// is executed only through the host's allowlisted `remote.command.dispatch`,
/// so the plugin never executes remote content itself.
fn serve(listener: TcpListener, stop: Arc<AtomicBool>) {
    for stream in listener.incoming() {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let stream: TcpStream = match stream {
            Ok(stream) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            }
            Err(_) => break,
        };
        let _ = std::thread::Builder::new()
            .name("remote-control-client".to_owned())
            .spawn(move || handle_client(stream));
    }
}

fn handle_client(stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(clone) => clone,
        Err(_) => return,
    });
    let mut writer = match stream.try_clone() {
        Ok(clone) => clone,
        Err(_) => return,
    };
    let mut line = String::new();
    loop {
        line.clear();
        let read = match reader.read_line(&mut line) {
            Ok(size) => size,
            Err(_) => return,
        };
        if read == 0 || line.len() > MAX_FRAME {
            return;
        }
        let request: Value = match serde_json::from_str(line.trim()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let response = match request.get("op").and_then(Value::as_str) {
            Some("status") => json!({"ok": true, "transport": "loopback-jsonl"}),
            Some("command") => {
                let command_id = request
                    .get("commandId")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let operation = request
                    .get("operation")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let args = request.get("args").cloned().unwrap_or(json!({}));
                match host_remote_execute(
                    "remote.command.dispatch",
                    json!({"commandId": command_id, "operation": operation, "args": args}),
                ) {
                    Ok(value) => value,
                    Err(error) => json!({"ok": false, "error": error.to_string()}),
                }
            }
            _ => json!({"ok": false, "error": "unknown op"}),
        };
        if writeln!(writer, "{}", response).is_err() {
            return;
        }
        let _ = writer.flush();
    }
}

impl Plugin for RemoteControl {
    fn initialize(&mut self, _context: &PluginContext) -> PluginResult<()> {
        self.start_gateway(DEFAULT_BIND)?;
        Ok(())
    }

    fn action(&mut self, _context: &PluginContext, call: ActionCall) -> PluginResult<ActionResult> {
        match call.action_type() {
            Some("tiktools.remote-control.test") => {
                let status = host_remote_execute("cloud.status", json!({}))?;
                Ok(ActionResult::summary(format!("host channel OK: {status}")))
            }
            Some("tiktools.remote-control.start") => {
                let address = self.start_gateway(DEFAULT_BIND)?;
                Ok(ActionResult::summary(format!("gateway listening on {address}")))
            }
            Some("tiktools.remote-control.stop") => {
                self.stop_gateway()?;
                Ok(ActionResult::summary("gateway stopped"))
            }
            Some("tiktools.remote-control.status") => {
                let address = self
                    .gateway
                    .lock()
                    .map_err(|error| PluginError::other(error.to_string()))?
                    .as_ref()
                    .map(|gateway| gateway.address.clone())
                    .unwrap_or_else(|| "stopped".to_owned());
                Ok(ActionResult::summary(format!("gateway: {address}")))
            }
            other => Err(PluginError::other(format!("unknown action {other:?}"))),
        }
    }

    fn event(&mut self, _context: &PluginContext, event: DomainEventEnvelope) -> PluginResult<()> {
        if event.topic == "cloud.login.code" {
            // Display-only: never log or persist the code. A real UI binds
            // this to its login-code panel and calls
            // `host_remote_execute("cloud.login.code.submit", ...)`.
        }
        Ok(())
    }
}

fn main() -> PluginResult<()> {
    run_process_plugin::<RemoteControl>()
}
