use std::sync::Arc;

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
}
