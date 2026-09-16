//! Process-runtime helpers for paths supplied by the TikTools launcher.
//!
//! These are intentionally outside `PluginContext`: WASM and native ABI
//! plugins do not necessarily have meaningful process paths.

use std::{env, ffi::OsString, io, path::PathBuf};

use serde_json::Value;
use tiktools_plugin_api::{
    read_frame, write_frame, FrameError, PluginRequest, PluginResponse, METHOD_CALL,
    TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
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

pub fn run_process_plugin_with<P>(plugin: &mut P, context: &PluginContext) -> PluginResult<()>
where
    P: Plugin,
{
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = io::BufReader::new(stdin.lock());
    let mut writer = io::BufWriter::new(stdout.lock());

    let loop_result = loop {
        let request = match read_frame::<_, PluginRequest>(&mut reader) {
            Ok(request) => request,
            Err(FrameError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                break Ok(())
            }
            Err(error) => break Err(PluginError::other(error.to_string())),
        };
        let response = handle_process_request(plugin, context, request);
        write_frame(&mut writer, &response)
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
