//! Process-runtime helpers for paths supplied by the TikTools launcher.
//!
//! These are intentionally outside `PluginContext`: WASM and native ABI
//! plugins do not necessarily have meaningful process paths.

use std::{
    cell::RefCell,
    env,
    ffi::OsString,
    io::{self, BufReader, BufWriter, Stdin, Stdout},
    path::PathBuf,
};

use serde_json::Value;
use tiktools_plugin_api::{
    read_frame, write_frame, FrameError, HostCallRequest, PluginFrame, PluginRequest,
    PluginResponse, METHOD_CALL, METHOD_HOST_CALL, TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
};

use crate::{dispatch_plugin_call, Plugin, PluginCall, PluginContext, PluginError, PluginResult};

const DATA_DIR: &str = "TIKTOOLS_PLUGIN_DATA_DIR";
const STORAGE_FILE: &str = "TIKTOOLS_PLUGIN_STORAGE_FILE";

pub fn data_dir() -> PluginResult<PathBuf> {
    path_from_environment(DATA_DIR)
}

pub fn storage_file() -> PluginResult<PathBuf> {
    path_from_environment(STORAGE_FILE)
}

fn path_from_environment(name: &str) -> PluginResult<PathBuf> {
    path_from_os(name, env::var_os(name))
}

pub(crate) fn path_from_os(name: &str, value: Option<OsString>) -> PluginResult<PathBuf> {
    let value = value.ok_or_else(|| {
        PluginError::other(format!(
            "required process environment variable {name} is missing"
        ))
    })?;
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() {
        return Err(PluginError::other(format!(
            "required process environment variable {name} is empty"
        )));
    }
    Ok(path)
}
fn response_ok(id: String, result: Value) -> PluginResponse {
    PluginResponse {
        protocol_version: TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
        id,
        ok: true,
        result: Some(result),
        error: None,
    }
}

fn response_failure(id: String, error: impl Into<String>) -> PluginResponse {
    PluginResponse {
        protocol_version: TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
        id,
        ok: false,
        result: None,
        error: Some(error.into()),
    }
}

/// Runs a plugin using the existing framed stdin/stdout process protocol.
pub fn run_process_plugin<P>() -> PluginResult<()>
where
    P: Plugin + Default,
{
    let context = PluginContext::from_process_environment();
    let mut plugin = P::default();
    plugin.initialize(&context)?;
    run_process_plugin_with(&mut plugin, &context)
}

struct ProcessChannel {
    reader: BufReader<StdinLock<'static>>,
    writer: BufWriter<StdoutLock<'static>>,
}

type StdinLock<'a> = io::StdinLock<'a>;
type StdoutLock<'a> = io::StdoutLock<'a>;

thread_local! {
    /// Shared by the dispatch loop and `host_remote_execute`. All access is on
    /// the same thread, and a reverse call happens while the loop is awaiting
    /// the plugin's answer, so frames can never interleave.
    static CHANNEL: RefCell<Option<ProcessChannel>> = const { RefCell::new(None) };
}

fn with_channel<T>(action: impl FnOnce(&mut ProcessChannel) -> T) -> T {
    CHANNEL.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            let stdin: &'static Stdin = Box::leak(Box::new(io::stdin()));
            let stdout: &'static Stdout = Box::leak(Box::new(io::stdout()));
            *slot = Some(ProcessChannel {
                reader: BufReader::new(stdin.lock()),
                writer: BufWriter::new(stdout.lock()),
            });
        }
        action(slot.as_mut().expect("channel was just initialized"))
    })
}

/// Ask the host to execute one allowlisted operation on this plugin's behalf.
///
/// The host validates the plugin's `host.call` capability against its own
/// manifest catalog and applies its remote policy. No token, `ControlApi` or
/// other host handle is returned to the plugin: only the RPC result.
pub fn host_remote_execute(method: &str, params: Value) -> PluginResult<Value> {
    if method.is_empty() || method.len() > 128 {
        return Err(PluginError::other("invalid host call method".to_owned()));
    }
    let payload = serde_json::to_value(HostCallRequest {
        method: method.to_owned(),
        params,
    })
    .map_err(|error| PluginError::other(error.to_string()))?;
    let id = format!("host-{}", next_host_call_id());
    let request = PluginRequest::new(id.clone(), METHOD_HOST_CALL, payload);
    with_channel(|channel| {
        write_frame(&mut channel.writer, &request)
            .map_err(|error| PluginError::other(error.to_string()))?;
        let response = read_frame::<_, PluginResponse>(&mut channel.reader)
            .map_err(|error| PluginError::other(error.to_string()))?;
        if response.protocol_version != TIKTOOLS_PLUGIN_PROTOCOL_VERSION || response.id != id {
            return Err(PluginError::other(
                "host returned a mismatched call response".to_owned(),
            ));
        }
        if !response.ok {
            return Err(PluginError::other(
                response
                    .error
                    .unwrap_or_else(|| "host rejected the call".to_owned()),
            ));
        }
        Ok(response.result.unwrap_or(Value::Null))
    })
}

fn next_host_call_id() -> u64 {
    thread_local! {
        static NEXT: RefCell<u64> = const { RefCell::new(0) };
    }
    NEXT.with(|next| {
        let mut next = next.borrow_mut();
        let value = *next;
        *next = next.saturating_add(1);
        value
    })
}

pub fn run_process_plugin_with<P>(plugin: &mut P, context: &PluginContext) -> PluginResult<()>
where
    P: Plugin,
{
    let loop_result = loop {
        let request =
            match with_channel(|channel| read_frame::<_, PluginFrame>(&mut channel.reader)) {
                Ok(PluginFrame::Request(request)) => request,
                Ok(PluginFrame::Response(_)) => {
                    break Err(PluginError::other(
                        "host sent a response without a request".to_owned(),
                    ))
                }
                Err(FrameError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    break Ok(())
                }
                Err(error) => break Err(PluginError::other(error.to_string())),
            };
        let response = handle_process_request(plugin, context, request);
        with_channel(|channel| write_frame(&mut channel.writer, &response))
            .map_err(|error| PluginError::other(error.to_string()))?;
    };
    let shutdown = plugin.shutdown(context);
    loop_result.and(shutdown)
}

pub(crate) fn handle_process_request<P: Plugin>(
    plugin: &mut P,
    context: &PluginContext,
    request: PluginRequest,
) -> PluginResponse {
    if request.protocol_version != TIKTOOLS_PLUGIN_PROTOCOL_VERSION {
        return response_failure(
            request.id,
            format!("unsupported protocol version {}", request.protocol_version),
        );
    }
    if request.method != METHOD_CALL {
        return response_failure(
            request.id,
            format!("unsupported method `{}`", request.method),
        );
    }
    let call = match serde_json::from_value::<PluginCall>(request.payload) {
        Ok(call) => call,
        Err(error) => return response_failure(request.id, format!("invalid plugin call: {error}")),
    };
    match dispatch_plugin_call(plugin, context, call) {
        Ok(result) => response_ok(request.id, result),
        Err(error) => response_failure(request.id, error.to_string()),
    }
}
