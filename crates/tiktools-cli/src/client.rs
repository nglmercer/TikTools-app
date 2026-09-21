//! Client setup: resolves one [`TikToolsClient`] per invocation.
//!
//! Commands run against the running host over local IPC by default; when
//! no host is running they fail with `host_unavailable` instead of
//! starting a second runtime. `--standalone` runs against an isolated
//! in-process [`ControlApi`]. Host-serving paths build the same
//! [`ControlApi`] via [`headless_api`].

use std::sync::Arc;

use tiktools_client::{ClientError, TikToolsClient};
use tiktools_control_api::ControlApi;
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

struct NullEmitter;

impl HostEmitter for NullEmitter {
    fn emit(&self, _message: HostMessage) {}
}

pub fn headless_api() -> ControlApi {
    ControlApi::new(Arc::new(AppCore::new(Arc::new(NullEmitter))))
}

pub async fn resolve_client(standalone: bool) -> Result<TikToolsClient, ClientError> {
    if standalone {
        Ok(TikToolsClient::direct(Arc::new(headless_api())))
    } else {
        TikToolsClient::connect().await
    }
}

/// Stable exit mapping: dropped connections and malformed results are
/// transport failures (3); host operation errors keep exit 1.
pub fn exit_code_for(error: &ClientError) -> i32 {
    if error.code == "transport" || error.code == "protocol" {
        3
    } else {
        1
    }
}

pub fn print_error(json_mode: bool, code: &str, message: &str) {
    if json_mode {
        println!(
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": {"code": code, "message": message},
            })
        );
    } else {
        eprintln!("tiktools: [{code}] {message}");
    }
}
