//! Typed `system.*` methods.

use super::TikToolsClient;
use serde_json::Value;
use tiktools_control_api::modules::system::{DoctorReport, InputAccessResult, ShutdownResult};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::modules::OkResult;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    SYSTEM_INFO,
    SYSTEM_HEALTH,
    SYSTEM_SNAPSHOT,
    SYSTEM_DOCTOR,
    SYSTEM_SHUTDOWN,
    SYSTEM_PING,
    SYSTEM_REQUEST_INPUT_ACCESS,
];

const SYSTEM_INFO: &str = "system.info";
const SYSTEM_HEALTH: &str = "system.health";
const SYSTEM_SNAPSHOT: &str = "system.snapshot";
const SYSTEM_DOCTOR: &str = "system.doctor";
const SYSTEM_SHUTDOWN: &str = "system.shutdown";
const SYSTEM_PING: &str = "system.ping";
const SYSTEM_REQUEST_INPUT_ACCESS: &str = "system.requestInputAccess";

impl TikToolsClient {
    /// Host info: version, features, paths, pid.
    /// RPC method `system.info`.
    pub async fn system_info(&self) -> Result<Value, ClientError> {
        self.call(SYSTEM_INFO, Empty::default()).await
    }
    /// Liveness summary: status, live, plugin and processor counts.
    /// RPC method `system.health`.
    pub async fn system_health(&self) -> Result<Value, ClientError> {
        self.call(SYSTEM_HEALTH, Empty::default()).await
    }
    /// Safe observable state (never includes secrets).
    /// RPC method `system.snapshot`.
    pub async fn system_snapshot(&self) -> Result<Value, ClientError> {
        self.call(SYSTEM_SNAPSHOT, Empty::default()).await
    }
    /// Structured diagnostics: storage, database, plugins, live, processors.
    /// RPC method `system.doctor`.
    pub async fn system_doctor(&self) -> Result<DoctorReport, ClientError> {
        self.call(SYSTEM_DOCTOR, Empty::default()).await
    }
    /// Stops live, plugin polling, and plugin runtimes.
    /// RPC method `system.shutdown`.
    pub async fn system_shutdown(&self) -> Result<ShutdownResult, ClientError> {
        self.call(SYSTEM_SHUTDOWN, Empty::default()).await
    }
    /// Liveness probe (always { ok: true }).
    /// RPC method `system.ping`.
    pub async fn system_ping(&self) -> Result<OkResult, ClientError> {
        self.call(SYSTEM_PING, Empty::default()).await
    }
    /// Probe raw-input access; install the seat rule via one polkit
    /// prompt when blocked. Powers the UI "grant access" button.
    /// RPC method `system.requestInputAccess`.
    pub async fn system_request_input_access(&self) -> Result<InputAccessResult, ClientError> {
        self.call(SYSTEM_REQUEST_INPUT_ACCESS, Empty::default())
            .await
    }
}
