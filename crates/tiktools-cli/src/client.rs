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
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
    previous_directory: std::path::PathBuf,
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
        let previous_directory = std::env::current_dir()?;
        // Legacy database migration and development plugin discovery must not
        // inspect the caller's checkout while verifying a clean sandbox.
        std::env::set_current_dir(&path)?;
        let mut previous = Vec::new();
        for (key, directory) in [
            ("TIKTOOLS_HOME", "home"),
            ("TIKTOOLS_DATA_DIR", "data"),
            ("TIKTOOLS_PLUGINS_DIR", "plugins"),
            ("TIKTOOLS_PLUGIN_DATA_DIR", "plugin-data"),
            ("TIKTOOLS_LOG_DIR", "logs"),
            ("TIKTOOLS_TEMP_DIR", "temp"),
            ("TIKTOOLS_BUILTIN_PLUGINS_DIR", "builtin-plugins"),
            ("TIKTOOLS_DEV_PLUGINS_DIR", "development-plugins"),
        ] {
            previous.push((key, std::env::var_os(key)));
            std::env::set_var(key, path.join(directory));
        }
        previous.push((
            "TIKTOOLS_SESSION_COOKIE",
            std::env::var_os("TIKTOOLS_SESSION_COOKIE"),
        ));
        std::env::remove_var("TIKTOOLS_SESSION_COOKIE");
        Ok(Self {
            path,
            previous,
            previous_directory,
        })
    }
}

impl Drop for SandboxHome {
    fn drop(&mut self) {
        for (key, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        let _ = std::env::set_current_dir(&self.previous_directory);
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
