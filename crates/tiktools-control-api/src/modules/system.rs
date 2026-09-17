use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiktools_core::{control::DoctorReport, AppCore};

use crate::{
    error::ApiError,
    modules::{Empty, OkResult},
    router::ControlRouter,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownResult {
    pub ok: bool,
    pub message: String,
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
            let snapshot = tokio::task::spawn_blocking(move || core.system_snapshot())
                .await
                .map_err(|error| ApiError::internal(format!("snapshot worker failed: {error}")))?;
            Ok::<Value, ApiError>(snapshot)
        },
    );
    router.register_typed::<Empty, DoctorReport, _, _>(
        "system.doctor",
        "Structured diagnostics: storage, database, plugins, live, processors",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            // Filesystem probes plus database checks stay off Tokio workers.
            let report = tokio::task::spawn_blocking(move || core.system_doctor())
                .await
                .map_err(|error| ApiError::internal(format!("doctor worker failed: {error}")))?;
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
    // Back-compat alias used by older automation clients.
    router.register_typed::<Empty, OkResult, _, _>(
        "system.ping",
        "Liveness probe (always { ok: true })",
        false,
        |_core: Arc<AppCore>, _params: Empty| async move {
            Ok::<OkResult, ApiError>(OkResult::ok())
        },
    );
}
