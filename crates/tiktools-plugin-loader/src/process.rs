//! Framed process runtime for crash-sensitive standalone plugins.

use std::{
    env, fs,
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    sync::{mpsc, Arc},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use serde_json::Value;
use tiktools_plugin_api::{
    read_frame, write_frame, FrameError, HostCallRequest, PluginFrame, PluginManifest,
    PluginRequest, PluginResponse, PluginRuntimeKind, METHOD_HOST_CALL,
    TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
};

use crate::host_call_channel::{HostCallChannel, HostCallGate};
use crate::{HostCallHandler, PluginInstance, PluginLoaderError, PluginRuntime};

/// Maximum reverse calls a plugin may make during one host→plugin call.
const MAX_HOST_CALLS_PER_REQUEST: usize = 8;

const PROCESS_CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Bound for joining this instance's helper threads (per-call I/O, stderr
/// drainer). On the healthy path the thread has already finished its work,
/// so the join is instantaneous; after a timeout the child kill closes the
/// pipes and unblocks it. A thread that still will not finish is abandoned
/// with an explicit report instead of blocking the plugin worker forever —
/// a grandchild that inherited the stderr pipe is the known case.
const PROCESS_HELPER_JOIN_DEADLINE: Duration = Duration::from_secs(3);

/// Poll interval while a bounded helper join waits for a thread to exit.
const PROCESS_HELPER_JOIN_POLL: Duration = Duration::from_millis(5);

/// Joins one of this instance's helper threads within
/// [`PROCESS_HELPER_JOIN_DEADLINE`]. `thread` is taken first, so a deadline
/// expiry leaves it `None` and the handle is dropped (detached) *after* the
/// abandonment has been logged — never silently.
fn join_helper_thread(id: &str, phase: &str, thread: &mut Option<JoinHandle<()>>) {
    join_helper_thread_within(id, phase, thread, PROCESS_HELPER_JOIN_DEADLINE);
}

fn join_helper_thread_within(
    id: &str,
    phase: &str,
    thread: &mut Option<JoinHandle<()>>,
    deadline: Duration,
) {
    let Some(handle) = thread.take() else {
        return;
    };
    let started = Instant::now();
    while !handle.is_finished() {
        if started.elapsed() >= deadline {
            tracing::error!(
                plugin = %id,
                phase,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "plugin helper thread did not exit; abandoning it (outcome unknown)"
            );
            return;
        }
        std::thread::sleep(PROCESS_HELPER_JOIN_POLL);
    }
    let _ = handle.join();
}

#[derive(Default)]
pub struct ProcessPluginRuntime;

impl PluginRuntime for ProcessPluginRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Process
    }

    fn load(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        self.load_at(manifest, directory, None)
    }

    fn load_isolated(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
        data_directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        self.load_at(manifest, directory, Some(data_directory))
    }
}

impl ProcessPluginRuntime {
    fn load_at(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
        isolated_data: Option<&Path>,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        let entry = manifest.entry.as_str();
        let package_root = fs::canonicalize(directory).map_err(|error| {
            PluginLoaderError::Runtime(format!(
                "could not resolve plugin directory {}: {error}",
                directory.display()
            ))
        })?;
        let entry_path = fs::canonicalize(directory.join(entry)).map_err(|error| {
            PluginLoaderError::Runtime(format!("could not resolve plugin entry {entry}: {error}"))
        })?;
        if !entry_path.starts_with(&package_root) {
            return Err(PluginLoaderError::Runtime(format!(
                "plugin entry escapes its package directory: {entry}"
            )));
        }
        if !entry_path.is_file() {
            return Err(PluginLoaderError::Runtime(format!(
                "entry does not exist: {entry}"
            )));
        }

        let (program, args) = process_command(&entry_path)?;
        let data_directory = isolated_data.map(Path::to_path_buf).unwrap_or_else(|| {
            env::var_os("TIKTOOLS_PLUGIN_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| directory.join(".data"))
        });
        let storage_file = if isolated_data.is_some() {
            data_directory.join("storage.json")
        } else {
            env::var_os("TIKTOOLS_PLUGIN_STORAGE_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|| data_directory.join("storage.json"))
        };
        // Dedicated reverse channel for unsolicited plugin → host calls. A
        // loopback socket keeps this cross-platform and avoids unsafe fd
        // inheritance; the one-time token authenticates the child.
        let (host_call_addr, host_call_token, host_call_gate, host_call_listener) =
            HostCallChannel::bind().map_err(|error| {
                PluginLoaderError::Runtime(format!(
                    "could not bind plugin reverse channel: {error}"
                ))
            })?;
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(&package_root)
            // A process plugin is still trusted executable code, but it does
            // not need the host's complete environment. Clear first, then pass
            // only the explicit plugin contract, plus the desktop-session
            // variables below so display/input integrations (X11, Wayland,
            // D-Bus) keep working inside the child. The reverse-channel
            // variables must be set after the clear: setting them before it
            // silently dropped them and every host call then failed.
            .env_clear()
            .env("TIKTOOLS_PLUGIN_HOST_CALL_ADDR", &host_call_addr)
            .env("TIKTOOLS_PLUGIN_HOST_CALL_TOKEN", &host_call_token);
        forward_desktop_environment(&mut command);
        command
            .env("TIKTOOLS_PLUGIN_ID", &manifest.id)
            .env("TIKTOOLS_PLUGIN_VERSION", &manifest.version)
            .env("TIKTOOLS_PLUGIN_DIRECTORY", &package_root)
            .env("TIKTOOLS_PLUGIN_DATA_DIR", data_directory)
            .env("TIKTOOLS_PLUGIN_STORAGE_FILE", storage_file)
            .env(
                "TIKTOOLS_PLUGIN_PERMISSIONS",
                manifest.permissions.join(","),
            )
            .env(
                "TIKTOOLS_PLUGIN_CAPABILITIES",
                manifest.capabilities.join(","),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        {
            // Process plugins talk over piped stdio and never need a
            // console. Without this flag every console-subsystem plugin
            // (for example the hotkey listener) pops a visible window.
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command.spawn().map_err(|error| {
            PluginLoaderError::Runtime(format!("could not start plugin host: {error}"))
        })?;
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(PluginLoaderError::Runtime(
                    "plugin stdin was not available".to_owned(),
                ));
            }
        };
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(PluginLoaderError::Runtime(
                    "plugin stdout was not available".to_owned(),
                ));
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(PluginLoaderError::Runtime(
                    "plugin stderr was not available".to_owned(),
                ));
            }
        };
        let stderr_thread = match drain_stderr(manifest.id.clone(), stderr) {
            Ok(thread) => Some(thread),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(PluginLoaderError::Runtime(format!(
                    "could not spawn plugin stderr worker: {error}"
                )));
            }
        };
        Ok(Box::new(ProcessPluginInstance {
            id: manifest.id.clone(),
            child,
            stdin: Some(BufWriter::new(stdin)),
            stdout: Some(BufReader::new(stdout)),
            stderr_thread,
            io_thread: None,
            next_request_id: 0,
            terminated: false,
            host_call_handler: None,
            host_call_gate: Some(host_call_gate),
            host_call_listener: Some(host_call_listener),
            host_call_token: host_call_token.clone(),
            host_call_channel: None,
        }))
    }
}

/// Desktop-session variables forwarded across the `.env_clear()` boundary.
///
/// Process plugins are launched with a cleared environment for hygiene, but
/// display/input integrations resolve their session from the environment:
/// X11 needs `DISPLAY`/`XAUTHORITY`, Wayland needs `WAYLAND_DISPLAY`, and the
/// desktop portal needs the D-Bus session address plus the XDG session
/// descriptors. Without these, an X11 listener fails with a display error
/// even on a healthy X11 session, and Wayland portal/evdev backends cannot
/// find the session bus.
///
/// The list is intentionally conservative: only session/locale/identity
/// variables are forwarded, never the host's complete environment (no
/// `LD_PRELOAD`, no tokens, no app-specific secrets).
pub const FORWARDED_DESKTOP_ENV_KEYS: &[&str] = &[
    "DISPLAY",
    "XAUTHORITY",
    "WAYLAND_DISPLAY",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "DESKTOP_SESSION",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "PATH",
    "HOME",
    // Windows: Winsock and the crypto/DNS providers read the system catalog
    // through SystemRoot. Without it, a plugin that binds a loopback socket
    // fails with WSAEPROVIDERFAILEDINIT (os error 10106) even though the same
    // binary works when launched with the full environment.
    #[cfg(target_os = "windows")]
    "SystemRoot",
    #[cfg(target_os = "windows")]
    "windir",
    #[cfg(target_os = "windows")]
    "ComSpec",
    #[cfg(target_os = "windows")]
    "NUMBER_OF_PROCESSORS",
    #[cfg(target_os = "windows")]
    "PROCESSOR_ARCHITECTURE",
    #[cfg(target_os = "windows")]
    "PATHEXT",
    #[cfg(target_os = "windows")]
    "TEMP",
    #[cfg(target_os = "windows")]
    "TMP",
    // Native Termux SDK clients resolve their CA bundle through PREFIX or
    // SSL_CERT_FILE. Rust temporary files require the app-private TMPDIR.
    #[cfg(target_os = "android")]
    "PREFIX",
    #[cfg(target_os = "android")]
    "TMPDIR",
    #[cfg(target_os = "android")]
    "SSL_CERT_FILE",
];

/// Copies the known desktop-session variables from the host environment into
/// a cleared plugin command. Missing variables are skipped.
pub fn forward_desktop_environment(command: &mut Command) {
    forward_desktop_environment_from(|key| std::env::var_os(key), command);
}

fn forward_desktop_environment_from(
    lookup: impl Fn(&str) -> Option<std::ffi::OsString>,
    command: &mut Command,
) {
    for key in FORWARDED_DESKTOP_ENV_KEYS {
        if let Some(value) = lookup(key) {
            if !value.is_empty() {
                command.env(key, value);
            }
        }
    }
}

fn process_command(entry: &Path) -> Result<(PathBuf, Vec<String>), PluginLoaderError> {
    let extension = entry
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if matches!(
        extension.to_ascii_lowercase().as_str(),
        "js" | "mjs" | "cjs" | "ts"
    ) {
        return Err(PluginLoaderError::RuntimeUnavailable(
            "JavaScript plugin entries are not executable processes; migrate them to the Rust plugin ABI or a standalone executable".to_owned(),
        ));
    }
    Ok((entry.to_owned(), Vec::new()))
}

struct ProcessPluginInstance {
    id: String,
    child: Child,
    stdin: Option<BufWriter<ChildStdin>>,
    stdout: Option<BufReader<ChildStdout>>,
    stderr_thread: Option<JoinHandle<()>>,
    /// Owner of the thread that performs one call's stdin/stdout frame I/O.
    /// It is retained (and joined) rather than detached, so the pipes are
    /// never handed to a new call while the previous thread still holds
    /// them, and a stuck reader is abandoned explicitly instead of leaking.
    io_thread: Option<JoinHandle<()>>,
    next_request_id: u64,
    terminated: bool,
    /// Installed by the host; `None` means the plugin has no host-call grant.
    host_call_handler: Option<Arc<dyn HostCallHandler>>,
    /// Gate shared with the dedicated reverse-channel pump.
    host_call_gate: Option<Arc<HostCallGate>>,
    /// Listener waiting for the plugin to connect its reverse channel.
    host_call_listener: Option<std::net::TcpListener>,
    /// One-time handshake token for the reverse channel.
    host_call_token: String,
    /// Owns the pump thread; dropped when the instance is retired.
    host_call_channel: Option<HostCallChannel>,
}

impl ProcessPluginInstance {
    fn handle_message_with_deadline(
        &mut self,
        request: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, PluginLoaderError> {
        let payload: Value = serde_json::from_slice(request).map_err(|error| {
            PluginLoaderError::Runtime(format!("invalid process request JSON: {error}"))
        })?;
        // Reap the previous call's I/O thread before starting a new one: it
        // owns the pipes, so the next call cannot begin until the last one
        // has finished with them.
        join_helper_thread(&self.id, "previous call I/O", &mut self.io_thread);
        let request_id = self.next_request_id.to_string();
        self.next_request_id = self.next_request_id.saturating_add(1);
        let message = PluginRequest::new(request_id.clone(), "call", payload);
        let mut stdin = self.stdin.take().ok_or_else(|| {
            PluginLoaderError::Runtime("plugin process stdin is unavailable".to_owned())
        })?;
        let mut stdout = self.stdout.take().ok_or_else(|| {
            PluginLoaderError::Runtime("plugin process stdout is unavailable".to_owned())
        })?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let thread_id = self.id.clone();
        let call_handler = self.host_call_handler.clone();
        let expected_id = request_id.clone();
        let io_thread = match std::thread::Builder::new()
            .name(format!("tiktools-plugin-io-{thread_id}"))
            .spawn(move || {
                let result = write_frame(&mut stdin, &message).and_then(|()| {
                    serve_call_io(
                        &mut stdout,
                        &mut stdin,
                        &expected_id,
                        &thread_id,
                        call_handler.as_deref(),
                    )
                });
                let _ = sender.send((result, stdin, stdout));
            }) {
            Ok(thread) => thread,
            Err(error) => {
                // The pipes moved into a thread that never started: this
                // generation can no longer serve calls.
                self.terminate_child();
                return Err(PluginLoaderError::Runtime(format!(
                    "could not start plugin process I/O thread: {error}"
                )));
            }
        };
        self.io_thread = Some(io_thread);
        let (result, stdin, stdout) = match wait_for_io(receiver, timeout) {
            Ok(result) => {
                join_helper_thread(&self.id, "call I/O", &mut self.io_thread);
                result
            }
            Err(ProcessIoWaitError::Timeout) => {
                self.terminate_child();
                join_helper_thread(&self.id, "timed-out call I/O", &mut self.io_thread);
                return Err(PluginLoaderError::Runtime(format!(
                    "plugin process call timed out after {} seconds",
                    timeout.as_secs()
                )));
            }
            Err(ProcessIoWaitError::Disconnected) => {
                self.terminate_child();
                join_helper_thread(&self.id, "disconnected call I/O", &mut self.io_thread);
                return Err(PluginLoaderError::Runtime(
                    "plugin process I/O worker stopped unexpectedly".to_owned(),
                ));
            }
        };
        self.stdin = Some(stdin);
        self.stdout = Some(stdout);
        decode_process_response(result, &request_id)
    }

    fn terminate_child(&mut self) {
        if self.terminated {
            return;
        }
        self.terminated = true;
        let _ = self.child.kill();
        let _ = self.child.wait();
        join_helper_thread(&self.id, "stderr drainer", &mut self.stderr_thread);
    }
}

impl Drop for ProcessPluginInstance {
    fn drop(&mut self) {
        self.terminate_child();
    }
}

impl PluginInstance for ProcessPluginInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        self.handle_message_with_deadline(request, PROCESS_CALL_TIMEOUT)
    }

    fn set_host_call_handler(&mut self, handler: Arc<dyn HostCallHandler>) {
        self.host_call_handler = Some(Arc::clone(&handler));
        if let (Some(gate), Some(listener)) =
            (self.host_call_gate.clone(), self.host_call_listener.take())
        {
            gate.set_handler(handler);
            self.host_call_channel = Some(HostCallChannel::spawn(
                listener,
                self.host_call_token.clone(),
                gate,
                self.id.clone(),
            ));
        }
    }

    fn handle_message_with_timeout(
        &mut self,
        request: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, PluginLoaderError> {
        self.handle_message_with_deadline(request, timeout)
    }

    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        self.stdin.take();
        self.stdout.take();
        self.terminate_child();
        Ok(())
    }
}

const MAX_STDERR_LINE_BYTES: usize = 4 * 1024;

fn drain_stderr(id: String, mut stderr: ChildStderr) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(format!("tiktools-plugin-stderr-{id}"))
        .spawn(move || {
            let mut line = Vec::with_capacity(MAX_STDERR_LINE_BYTES);
            let mut truncated = false;
            let mut chunk = [0_u8; 4096];
            loop {
                let read = match stderr.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(read) => read,
                    Err(error) => {
                        tracing::debug!(target: "plugin.stderr", plugin = %id, %error, "plugin stderr reader stopped");
                        break;
                    }
                };
                for byte in &chunk[..read] {
                    if *byte == b'\n' {
                        emit_stderr_line(&id, &line, truncated);
                        line.clear();
                        truncated = false;
                    } else if line.len() < MAX_STDERR_LINE_BYTES {
                        if *byte != b'\r' {
                            line.push(*byte);
                        }
                    } else {
                        truncated = true;
                    }
                }
            }
            if !line.is_empty() {
                emit_stderr_line(&id, &line, truncated);
            }
        })
}

fn emit_stderr_line(id: &str, line: &[u8], truncated: bool) {
    let message = bounded_stderr_message(line, truncated);
    tracing::warn!(target: "plugin.stderr", plugin = %id, message = %message, "plugin stderr");
}

fn bounded_stderr_message(line: &[u8], truncated: bool) -> String {
    let mut message = String::from_utf8_lossy(line).into_owned();
    if truncated {
        message.push('…');
    }
    message
}

#[derive(Debug)]
enum ProcessIoWaitError {
    Timeout,
    Disconnected,
}

fn wait_for_io<T>(receiver: mpsc::Receiver<T>, timeout: Duration) -> Result<T, ProcessIoWaitError> {
    match receiver.recv_timeout(timeout) {
        Ok(value) => Ok(value),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(ProcessIoWaitError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(ProcessIoWaitError::Disconnected),
    }
}

/// Read one call's frames from a process, answering reverse `host.call`
/// requests until the original response arrives. Kept generic so the demux can
/// be tested with in-memory streams instead of spawning a child process.
fn serve_call_io<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    expected_id: &str,
    plugin_id: &str,
    handler: Option<&dyn HostCallHandler>,
) -> Result<PluginResponse, FrameError> {
    let mut reverse_calls = 0usize;
    loop {
        match read_frame::<_, PluginFrame>(reader)? {
            PluginFrame::Response(response) => {
                if response.id != expected_id {
                    return Err(invalid_frame_error("response id mismatch"));
                }
                return Ok(response);
            }
            PluginFrame::Request(reverse) => {
                reverse_calls += 1;
                if reverse_calls > MAX_HOST_CALLS_PER_REQUEST {
                    return Err(invalid_frame_error("too many plugin host calls"));
                }
                let answer = match (handler, reverse.method.as_str()) {
                    (Some(handler), METHOD_HOST_CALL) => {
                        serde_json::from_value::<HostCallRequest>(reverse.payload)
                            .map_err(|error| error.to_string())
                            .and_then(|request| {
                                handler.call(plugin_id, &request.method, request.params)
                            })
                    }
                    (None, _) => Err("host calls are not granted to this plugin".to_owned()),
                    (_, method) => Err(format!("unsupported reverse method `{method}`")),
                };
                let (ok, value, error) = match answer {
                    Ok(value) => (true, Some(value), None),
                    Err(error) => (false, None, Some(error)),
                };
                write_frame(
                    writer,
                    &PluginResponse {
                        protocol_version: TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
                        id: reverse.id,
                        ok,
                        result: value,
                        error,
                    },
                )?;
            }
        }
    }
}

fn invalid_frame_error(message: &str) -> FrameError {
    // The marker is irrelevant: FrameError::Json is used only to carry a
    // protocol violation into the existing error path.
    let _ = message;
    FrameError::Json(serde_json::from_str::<Value>("invalid").unwrap_err())
}

fn decode_process_response(
    result: Result<PluginResponse, FrameError>,
    request_id: &str,
) -> Result<Vec<u8>, PluginLoaderError> {
    let response = result.map_err(|error| PluginLoaderError::Runtime(error.to_string()))?;
    if response.protocol_version != TIKTOOLS_PLUGIN_PROTOCOL_VERSION {
        return Err(PluginLoaderError::Runtime(format!(
            "plugin process protocol mismatch: {}",
            response.protocol_version
        )));
    }
    if response.id != request_id {
        return Err(PluginLoaderError::Runtime(
            "plugin process returned a response for a different request".to_owned(),
        ));
    }
    if !response.ok {
        return Err(PluginLoaderError::Runtime(
            response
                .error
                .unwrap_or_else(|| "plugin process rejected request".to_owned()),
        ));
    }
    serde_json::to_vec(&response.result.unwrap_or(Value::Null))
        .map_err(|error| PluginLoaderError::Runtime(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn response(id: &str) -> PluginResponse {
        PluginResponse {
            protocol_version: TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
            id: id.to_owned(),
            ok: true,
            result: Some(serde_json::json!({"accepted": true})),
            error: None,
        }
    }

    #[test]
    fn accepts_successful_response() {
        let bytes = decode_process_response(Ok(response("request-1")), "request-1").unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            serde_json::json!({"accepted": true})
        );
    }

    #[test]
    fn rejects_response_with_wrong_request_id() {
        let error = decode_process_response(Ok(response("other")), "request-1").unwrap_err();
        assert!(error.to_string().contains("different request"));
    }

    struct EchoHandler;

    impl HostCallHandler for EchoHandler {
        fn call(&self, plugin_id: &str, method: &str, params: Value) -> Result<Value, String> {
            assert_eq!(plugin_id, "demo.plugin");
            assert_eq!(method, "remote.execute");
            Ok(serde_json::json!({"plugin": plugin_id, "params": params}))
        }
    }

    fn host_call_frame(id: &str) -> PluginFrame {
        PluginFrame::Request(PluginRequest::new(
            id,
            METHOD_HOST_CALL,
            serde_json::to_value(HostCallRequest {
                method: "remote.execute".to_owned(),
                params: serde_json::json!({"x": 1}),
            })
            .unwrap(),
        ))
    }

    #[test]
    fn reverse_host_call_is_answered_then_original_response_returned() {
        let mut stream = Vec::new();
        write_frame(&mut stream, &host_call_frame("host-1")).unwrap();
        write_frame(&mut stream, &PluginFrame::Response(response("request-1"))).unwrap();
        let mut reader = std::io::Cursor::new(stream);
        let mut writer = Vec::new();

        let final_response = serve_call_io(
            &mut reader,
            &mut writer,
            "request-1",
            "demo.plugin",
            Some(&EchoHandler),
        )
        .unwrap();
        assert_eq!(final_response.id, "request-1");
        let answer: PluginResponse = read_frame(&mut std::io::Cursor::new(writer)).unwrap();
        assert_eq!(answer.id, "host-1");
        assert!(answer.ok);
        assert_eq!(answer.result.unwrap()["params"]["x"], serde_json::json!(1));
    }

    #[test]
    fn reverse_call_without_a_granted_handler_is_denied_but_call_survives() {
        let mut stream = Vec::new();
        write_frame(&mut stream, &host_call_frame("host-2")).unwrap();
        write_frame(&mut stream, &PluginFrame::Response(response("request-2"))).unwrap();
        let mut reader = std::io::Cursor::new(stream);
        let mut writer = Vec::new();

        let final_response =
            serve_call_io(&mut reader, &mut writer, "request-2", "demo.plugin", None).unwrap();
        assert_eq!(final_response.id, "request-2");
        let answer: PluginResponse = read_frame(&mut std::io::Cursor::new(writer)).unwrap();
        assert!(!answer.ok);
        assert!(answer.error.unwrap().contains("not granted"));
    }

    #[test]
    fn reverse_calls_are_bounded_per_host_request() {
        let mut stream = Vec::new();
        for index in 0..MAX_HOST_CALLS_PER_REQUEST + 1 {
            write_frame(&mut stream, &host_call_frame(&format!("host-{index}"))).unwrap();
        }
        write_frame(&mut stream, &PluginFrame::Response(response("request-3"))).unwrap();
        let mut reader = std::io::Cursor::new(stream);
        let mut writer = Vec::new();

        assert!(serve_call_io(
            &mut reader,
            &mut writer,
            "request-3",
            "demo.plugin",
            Some(&EchoHandler)
        )
        .is_err());
    }

    #[test]
    fn rejects_protocol_mismatch() {
        let mut response = response("request-1");
        response.protocol_version += 1;
        let error = decode_process_response(Ok(response), "request-1").unwrap_err();
        assert!(error.to_string().contains("protocol mismatch"));
    }

    #[test]
    fn rejects_invalid_json_from_process() {
        let json_error = serde_json::from_str::<PluginResponse>("not json").unwrap_err();
        let error =
            decode_process_response(Err(FrameError::Json(json_error)), "request-1").unwrap_err();
        assert!(error.to_string().contains("not valid JSON"));
    }

    #[test]
    fn forwards_desktop_session_variables_and_skips_missing() {
        use std::collections::HashMap;
        use std::ffi::OsString;

        let mut source = HashMap::new();
        source.insert("DISPLAY".to_owned(), OsString::from(":0"));
        source.insert("WAYLAND_DISPLAY".to_owned(), OsString::from("wayland-0"));
        source.insert(
            "DBUS_SESSION_BUS_ADDRESS".to_owned(),
            OsString::from("unix:path=/run/user/1000/bus"),
        );
        source.insert("HOME".to_owned(), OsString::from("/home/tester"));
        // Present but empty values carry no session information.
        source.insert("XAUTHORITY".to_owned(), OsString::new());

        let mut command = Command::new("true");
        command.env_clear();
        forward_desktop_environment_from(|key| source.get(key).cloned(), &mut command);

        let forwarded: std::collections::HashMap<String, String> = command
            .get_envs()
            .filter_map(|(key, value)| {
                Some((key.to_str()?.to_owned(), value?.to_str()?.to_owned()))
            })
            .collect();
        assert_eq!(forwarded.get("DISPLAY").map(String::as_str), Some(":0"));
        assert_eq!(
            forwarded.get("WAYLAND_DISPLAY").map(String::as_str),
            Some("wayland-0")
        );
        assert_eq!(
            forwarded
                .get("DBUS_SESSION_BUS_ADDRESS")
                .map(String::as_str),
            Some("unix:path=/run/user/1000/bus")
        );
        assert_eq!(
            forwarded.get("HOME").map(String::as_str),
            Some("/home/tester")
        );
        // Missing and empty variables are never injected.
        assert!(!forwarded.contains_key("XAUTHORITY"));
        assert!(!forwarded.contains_key("XDG_SESSION_TYPE"));
        // The allowlist never grows silently: every forwarded key is known.
        for key in forwarded.keys() {
            assert!(
                FORWARDED_DESKTOP_ENV_KEYS.contains(&key.as_str()),
                "unexpected forwarded variable {key}"
            );
        }
    }

    #[test]
    fn forwarded_allowlist_stays_conservative() {
        // Guard against accidentally forwarding secrets or loader internals.
        for forbidden in [
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
            "TIKTOOLS_PLUGIN_STORAGE_FILE",
            "TOKENS",
            "SECRET",
            "TIKTOOLS_SESSION_COOKIE",
        ] {
            assert!(
                !FORWARDED_DESKTOP_ENV_KEYS.contains(&forbidden),
                "{forbidden} must never cross the plugin environment boundary"
            );
        }
        assert!(FORWARDED_DESKTOP_ENV_KEYS.contains(&"DISPLAY"));
        assert!(FORWARDED_DESKTOP_ENV_KEYS.contains(&"WAYLAND_DISPLAY"));
        assert!(FORWARDED_DESKTOP_ENV_KEYS.contains(&"DBUS_SESSION_BUS_ADDRESS"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn forwards_windows_system_variables() {
        use std::collections::HashMap;

        let source = HashMap::from([
            (
                "SystemRoot".to_owned(),
                std::ffi::OsString::from(r"C:\Windows"),
            ),
            ("TEMP".to_owned(), std::ffi::OsString::from(r"C:\Temp")),
        ]);
        let mut command = Command::new("cmd.exe");
        command.env_clear();
        forward_desktop_environment_from(|key| source.get(key).cloned(), &mut command);
        let forwarded: HashMap<String, String> = command
            .get_envs()
            .filter_map(|(key, value)| {
                Some((key.to_str()?.to_owned(), value?.to_str()?.to_owned()))
            })
            .collect();
        assert_eq!(
            forwarded.get("SystemRoot").map(String::as_str),
            Some(r"C:\Windows")
        );
        assert_eq!(forwarded.get("TEMP").map(String::as_str), Some(r"C:\Temp"));
    }

    #[test]
    fn reports_io_timeout_without_waiting_for_a_child() {
        let (_sender, receiver) = mpsc::sync_channel::<()>(1);
        let error = wait_for_io(receiver, Duration::from_millis(1)).unwrap_err();
        assert!(matches!(error, ProcessIoWaitError::Timeout));
    }

    #[test]
    fn stderr_message_is_bounded_and_marks_truncation() {
        let message = bounded_stderr_message(&vec![b'x'; MAX_STDERR_LINE_BYTES], true);
        assert_eq!(message.chars().count(), MAX_STDERR_LINE_BYTES + 1);
        assert!(message.ends_with('…'));
    }

    #[test]
    fn terminated_child_can_be_reaped_after_failure() {
        let child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "ping -n 30 127.0.0.1 > NUL"])
                .spawn()
                .unwrap()
        } else {
            Command::new("sh").args(["-c", "sleep 30"]).spawn().unwrap()
        };
        let mut instance = ProcessPluginInstance {
            id: "test".to_owned(),
            child,
            stdin: None,
            stdout: None,
            stderr_thread: None,
            io_thread: None,
            next_request_id: 0,
            terminated: false,
            host_call_handler: None,
            host_call_gate: None,
            host_call_listener: None,
            host_call_token: String::new(),
            host_call_channel: None,
        };
        instance.terminate_child();
        assert!(instance.child.try_wait().unwrap().is_some());
    }

    #[test]
    fn bounded_helper_join_abandons_a_wedged_thread_explicitly() {
        let never = std::thread::spawn(|| loop {
            std::thread::sleep(Duration::from_millis(50));
        });
        let mut slot = Some(never);
        let started = Instant::now();
        join_helper_thread_within(
            "test",
            "wedged helper",
            &mut slot,
            Duration::from_millis(100),
        );
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "the join must give up at the deadline (took {elapsed:?})"
        );
        assert!(
            slot.is_none(),
            "the abandoned handle must be taken so nothing joins it later"
        );
    }

    #[test]
    fn bounded_helper_join_finishes_immediately_for_a_dead_thread() {
        let finished = std::thread::spawn(|| ());
        let mut slot = Some(finished);
        let started = Instant::now();
        join_helper_thread_within("test", "dead helper", &mut slot, Duration::from_millis(100));
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(slot.is_none());
    }

    #[test]
    fn timed_out_call_kills_the_child_and_reaps_its_io_thread() {
        let mut child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "ping -n 30 127.0.0.1 > NUL"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
        } else {
            // Keep stdout open without answering. Termux's shell can exec
            // a final redirected command directly, so `cat > /dev/null`
            // closes the response pipe early instead of exercising timeout.
            // Explicit exec also prevents leaving a sleeping descendant.
            Command::new("sh")
                .args(["-c", "exec sleep 30"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
        };
        let stdin = child.stdin.take().expect("piped plugin stdin");
        let stdout = child.stdout.take().expect("piped plugin stdout");
        let mut instance = ProcessPluginInstance {
            id: "test".to_owned(),
            child,
            stdin: Some(BufWriter::new(stdin)),
            stdout: Some(BufReader::new(stdout)),
            stderr_thread: None,
            io_thread: None,
            next_request_id: 0,
            terminated: false,
            host_call_handler: None,
            host_call_gate: None,
            host_call_listener: None,
            host_call_token: String::new(),
            host_call_channel: None,
        };
        let request = serde_json::to_vec(&serde_json::json!({"type": "poll"})).unwrap();
        let result = instance.handle_message_with_deadline(&request, Duration::from_millis(150));
        assert!(
            matches!(result, Err(PluginLoaderError::Runtime(_))),
            "a child that never answers must time out, got {result:?}"
        );
        assert!(
            instance.terminated,
            "a timed-out call must kill the child: result={result:?}, status={:?}",
            instance.child.try_wait()
        );
        assert!(
            instance.io_thread.is_none(),
            "the timed-out call's I/O thread must be joined (or abandoned), not left owned"
        );
        assert!(instance.child.try_wait().unwrap().is_some());
    }

    #[test]
    fn stderr_worker_spawns_and_exits_on_eof() {
        let mut child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "exit 0"])
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        } else {
            Command::new("sh")
                .args(["-c", "exit 0"])
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        };
        let stderr = child.stderr.take().expect("piped plugin stderr");
        // `drain_stderr` is fallible: a thread-spawn failure must surface
        // as `Err` (so `load` can reap the child) instead of panicking.
        let handle = drain_stderr("test".to_owned(), stderr).expect("stderr worker spawns");
        let _ = child.wait();
        handle.join().expect("stderr worker exits on EOF");
    }
}
