use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use tiktools_core::control::DoctorReport;
pub use tiktools_core::control::InputAccessResult;
use tiktools_core::AppCore;

use crate::{
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};

/// Largest text export accepted through `system.saveFile`. The RPC
/// is reachable from the WebView, so the payload stays bounded.
const MAX_SAVE_FILE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveFileParams {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveFileResult {
    pub ok: bool,
    pub path: String,
}

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, Value, _, _>(
        "system.info",
        "Host info: version, features, paths, pid",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<Value, ApiError>(core.system_info())
        },
    );
    router.register_typed::<Empty, Value, _, _>(
        "system.health",
        "Liveness summary: status, live, plugin and processor counts",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            Ok::<Value, ApiError>(core.system_health())
        },
    );
    router.register_typed::<Empty, Value, _, _>(
        "system.snapshot",
        "Safe observable state (never includes secrets)",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            // Snapshot fans out over SQLite plus in-memory services.
            let snapshot =
                crate::modules::blocking_task("system.snapshot", move || core.system_snapshot())
                    .await?;
            Ok::<Value, ApiError>(snapshot)
        },
    );
    router.register_typed::<Empty, DoctorReport, _, _>(
        "system.doctor",
        "Structured diagnostics: storage, database, plugins, live, processors",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            // Filesystem probes plus database checks stay off Tokio workers.
            let report =
                crate::modules::blocking_task("system.doctor", move || core.system_doctor())
                    .await?;
            Ok::<DoctorReport, ApiError>(report)
        },
    );
    router.register_typed::<Empty, ShutdownResult, _, _>(
        "system.shutdown",
        "Stops live, plugin polling, and plugin runtimes",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.shutdown().await;
            Ok::<ShutdownResult, ApiError>(ShutdownResult {
                ok: true,
                message: "shutdown started".to_owned(),
            })
        },
    );
    router.register_typed::<Empty, InputAccessResult, _, _>(
        "system.requestInputAccess",
        "Probe raw-input access; install the seat rule via one polkit prompt when blocked",
        true,
        |core: Arc<AppCore>, _params: Empty| async move {
            if cfg!(target_os = "android") {
                return Err(ApiError::capability_unavailable(
                    "global input access has no Android backend",
                ));
            }
            // Subprocesses (polkit prompt) stay off Tokio workers.
            let outcome = crate::modules::blocking_task("system.requestInputAccess", move || {
                core.request_input_access()
            })
            .await?;
            Ok::<InputAccessResult, ApiError>(outcome)
        },
    );
    // Back-compat alias used by older automation clients.
    router.register_typed::<Empty, OkResult, _, _>(
        "system.ping",
        "Liveness probe (always { ok: true })",
        false,
        |_core: Arc<AppCore>, _params: Empty| async move {
            Ok::<OkResult, ApiError>(OkResult::ok())
        },
    );
    // WebView anchor downloads never reach the download signal:
    // the navigation policy cancels the `blob:` URL first. Exports
    // therefore travel over IPC and the host writes the file.
    router.register_typed::<SaveFileParams, SaveFileResult, _, _>(
        "system.saveFile",
        "Writes one text file into the user's Downloads folder",
        true,
        |_core: Arc<AppCore>, params: SaveFileParams| async move {
            let name = sanitize_filename(&params.filename)
                .ok_or_else(|| ApiError::invalid_params("filename must be a plain file name"))?;
            if params.content.len() > MAX_SAVE_FILE_BYTES {
                return Err(ApiError::too_large());
            }
            let content = params.content;
            let path = crate::modules::blocking_task("system.saveFile", move || {
                write_user_file(&name, &content)
            })
            .await?
            .map_err(|error| ApiError::internal(format!("could not write file: {error}")))?;
            Ok::<SaveFileResult, ApiError>(SaveFileResult {
                ok: true,
                path: path.display().to_string(),
            })
        },
    );
}

/// Accepts only a plain file name: the final path component with
/// no separators, traversal, or control characters, and a bounded
/// length. The RPC is reachable from the WebView, so the name
/// never decides where the file lands — it always lands in the
/// user's Downloads folder.
fn sanitize_filename(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return None;
    }
    let name = Path::new(trimmed).file_name()?.to_str()?;
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    Some(name.to_owned())
}

/// The user's Downloads folder without external crates:
/// `XDG_DOWNLOAD_DIR` on Linux (xdg-user-dirs), the platform
/// default otherwise.
fn download_directory() -> PathBuf {
    if cfg!(target_os = "windows") {
        env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Downloads")
    } else {
        env::var_os("XDG_DOWNLOAD_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("Downloads")
            })
    }
}

fn write_user_file(name: &str, content: &str) -> io::Result<PathBuf> {
    let directory = download_directory();
    fs::create_dir_all(&directory)?;
    let path = directory.join(name);
    fs::write(&path, content)?;
    Ok(path)
}
