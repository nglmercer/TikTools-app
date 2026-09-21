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
        Ok(direct_client())
    } else {
        TikToolsClient::connect().await
    }
}

/// In-process client over [`headless_api`]. Uses the real data
/// directories unless a [`SandboxHome`] guard redirected `TIKTOOLS_HOME`.
pub fn direct_client() -> TikToolsClient {
    TikToolsClient::direct(Arc::new(headless_api()))
}

/// Throwaway `TIKTOOLS_HOME` for `api verify --sandbox`: the runtime
/// under verification gets fresh databases and no plugins, and the
/// directory is removed when the guard drops. One command per process,
/// so a process-wide variable is safe.
pub struct SandboxHome {
    path: std::path::PathBuf,
}

impl SandboxHome {
    pub fn create() -> std::io::Result<Self> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let path =
            std::env::temp_dir().join(format!("tiktools-verify-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&path)?;
        std::env::set_var("TIKTOOLS_HOME", &path);
        Ok(Self { path })
    }
}

impl Drop for SandboxHome {
    fn drop(&mut self) {
        std::env::remove_var("TIKTOOLS_HOME");
        let _ = std::fs::remove_dir_all(&self.path);
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
