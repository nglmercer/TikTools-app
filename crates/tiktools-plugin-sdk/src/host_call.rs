//! Thread-safe plugin → host call transport.
//!
//! The legacy reverse path lives on the plugin's stdio and can only be used
//! while a host→plugin call is in flight. A remote-control plugin owns a
//! listener thread and must call the host from there, so this module connects
//! to the host's dedicated loopback reverse channel (address and one-time
//! token supplied through `TIKTOOLS_PLUGIN_HOST_CALL_*`). All state is global
//! and mutex-protected, so any plugin thread can call `host_remote_execute`.
//!
//! No token, `ControlApi` or host handle is involved: only `{method, params}`
//! JSON frames and the host's sanitized answer cross this connection.

use std::{
    env,
    io::{BufReader, BufWriter},
    net::TcpStream,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use serde_json::Value;
use tiktools_plugin_api::{
    read_frame, write_frame, HostCallRequest, PluginRequest, PluginResponse, METHOD_HOST_CALL,
    TIKTOOLS_PLUGIN_PROTOCOL_VERSION,
};

use crate::{PluginError, PluginResult};

struct ReverseChannel {
    writer: BufWriter<TcpStream>,
    reader: BufReader<TcpStream>,
    next_id: u64,
}

static CHANNEL: OnceLock<Mutex<Option<ReverseChannel>>> = OnceLock::new();

fn channel() -> &'static Mutex<Option<ReverseChannel>> {
    CHANNEL.get_or_init(|| Mutex::new(None))
}

fn connect() -> PluginResult<()> {
    let addr = env::var("TIKTOOLS_PLUGIN_HOST_CALL_ADDR")
        .map_err(|_| PluginError::other("host call channel is not available".to_owned()))?;
    let token = env::var("TIKTOOLS_PLUGIN_HOST_CALL_TOKEN")
        .map_err(|_| PluginError::other("host call token is missing".to_owned()))?;
    let stream = TcpStream::connect(&addr).map_err(|error| {
        PluginError::other(format!("could not reach host call channel: {error}"))
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| PluginError::other(error.to_string()))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| PluginError::other(error.to_string()))?;
    let mut stream = BufWriter::new(stream);
    write_frame(&mut stream, &Value::String(token))
        .map_err(|error| PluginError::other(format!("host call handshake failed: {error}")))?;
    use std::io::Write;
    stream
        .flush()
        .map_err(|error| PluginError::other(error.to_string()))?;
    let inner = stream
        .into_inner()
        .map_err(|error| PluginError::other(error.to_string()))?;
    let reader = BufReader::new(
        inner
            .try_clone()
            .map_err(|error| PluginError::other(error.to_string()))?,
    );
    let writer = BufWriter::new(inner);
    *channel().lock().unwrap_or_else(|error| error.into_inner()) = Some(ReverseChannel {
        writer,
        reader,
        next_id: 0,
    });
    Ok(())
}

/// Ask the host to execute one allowlisted operation. Safe to call from any
/// plugin thread, including a listener. The host validates the plugin's own
/// catalog capability, applies its policy and returns only the RPC result.
pub fn host_remote_execute(method: &str, params: Value) -> PluginResult<Value> {
    if method.is_empty() || method.len() > 128 {
        return Err(PluginError::other("invalid host call method".to_owned()));
    }
    let payload = serde_json::to_value(HostCallRequest {
        method: method.to_owned(),
        params,
    })
    .map_err(|error| PluginError::other(error.to_string()))?;

    let mut slot = channel().lock().unwrap_or_else(|error| error.into_inner());
    if slot.is_none() {
        drop(slot);
        connect()?;
        slot = channel().lock().unwrap_or_else(|error| error.into_inner());
    }
    let channel = slot
        .as_mut()
        .ok_or_else(|| PluginError::other("host call channel closed".to_owned()))?;
    channel.next_id = channel.next_id.wrapping_add(1);
    let id = format!("host-{}", channel.next_id);
    let request = PluginRequest::new(id.clone(), METHOD_HOST_CALL, payload);
    write_frame(&mut channel.writer, &request)
        .map_err(|error| PluginError::other(error.to_string()))?;
    use std::io::Write;
    channel
        .writer
        .flush()
        .map_err(|error| PluginError::other(error.to_string()))?;
    let response: PluginResponse =
        read_frame(&mut channel.reader).map_err(|error| PluginError::other(error.to_string()))?;
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
}
