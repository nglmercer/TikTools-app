//! Installable remote-control plugin: sole owner of remote control.
//!
//! Owns its WSS relay transport (see `relay.rs`) and executes every received
//! command through the host's capability-gated `host.call`. It never receives
//! a bearer token, ControlApi, cookies or raw settings.

mod relay;

use std::sync::Mutex;

use serde_json::{json, Value};
use tiktools_plugin_sdk::{
    host_remote_execute, run_process_plugin, ActionCall, ActionResult, DomainEventEnvelope,
    Plugin, PluginContext, PluginError, PluginResult,
};
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub(crate) struct GatewayConfig {
    pub(crate) host_id: String,
    pub(crate) device_id: String,
}

#[derive(Default)]
struct RemoteControl {
    runtime: Mutex<Option<tokio::runtime::Runtime>>,
    stop: Mutex<Option<oneshot::Sender<()>>>,
}

impl RemoteControl {
    fn start_relay(&self, host_id: String, device_id: String) -> PluginResult<()> {
        let mut runtime_slot = self
            .runtime
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))?;
        if runtime_slot.is_some() {
            return Err(PluginError::other("relay already running"));
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| PluginError::other(error.to_string()))?;
        let (stop_tx, stop_rx) = oneshot::channel();
        let config = GatewayConfig {
            host_id,
            device_id,
        };
        runtime.spawn(async move {
            if let Err(error) = relay::run(config, stop_rx).await {
                eprintln!("remote-control relay stopped: {error}");
            }
        });
        *runtime_slot = Some(runtime);
        *self
            .stop
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))? = Some(stop_tx);
        Ok(())
    }

    fn stop_relay(&self) -> PluginResult<()> {
        let stop = self
            .stop
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))?
            .take();
        if let Some(stop) = stop {
            let _ = stop.send(());
        }
        if let Some(runtime) = self
            .runtime
            .lock()
            .map_err(|error| PluginError::other(error.to_string()))?
            .take()
        {
            runtime.shutdown_timeout(std::time::Duration::from_secs(5));
        }
        Ok(())
    }
}

impl Drop for RemoteControl {
    fn drop(&mut self) {
        let _ = self.stop_relay();
    }
}

impl Plugin for RemoteControl {
    fn initialize(&mut self, _context: &PluginContext) -> PluginResult<()> {
        // Auto-start requires a paired host/device. Operators start the relay
        // explicitly after pairing via `start`; `events.subscribe` guarantees
        // this plugin process is running when a login code arrives.
        Ok(())
    }

    fn action(&mut self, _context: &PluginContext, call: ActionCall) -> PluginResult<ActionResult> {
        let config = call.config();
        match call.action_type() {
            Some("tiktools.remote-control.test") => {
                let status = host_remote_execute("cloud.status", json!({}))?;
                Ok(ActionResult::summary(format!("host channel OK: {status}")))
            }
            Some("tiktools.remote-control.start") => {
                let host_id = config
                    .get("hostId")
                    .and_then(Value::as_str)
                    .ok_or_else(|| PluginError::other("hostId is required"))?
                    .to_owned();
                let device_id = config
                    .get("deviceId")
                    .and_then(Value::as_str)
                    .ok_or_else(|| PluginError::other("deviceId is required"))?
                    .to_owned();
                self.start_relay(host_id, device_id)?;
                Ok(ActionResult::summary("relay started"))
            }
            Some("tiktools.remote-control.stop") => {
                self.stop_relay()?;
                Ok(ActionResult::summary("relay stopped"))
            }
            Some("tiktools.remote-control.status") => {
                let running = self
                    .runtime
                    .lock()
                    .map_err(|error| PluginError::other(error.to_string()))?
                    .is_some();
                Ok(ActionResult::summary(format!(
                    "relay: {}",
                    if running { "running" } else { "stopped" }
                )))
            }
            other => Err(PluginError::other(format!("unknown action {other:?}"))),
        }
    }

    fn event(&mut self, _context: &PluginContext, event: DomainEventEnvelope) -> PluginResult<()> {
        if event.topic == "cloud.login.code" {
            // The code is shown by the plugin UI and submitted through
            // `host_remote_execute("cloud.login.code.submit", ...)`. It is
            // never logged or persisted here.
        }
        Ok(())
    }
}

fn main() -> PluginResult<()> {
    run_process_plugin::<RemoteControl>()
}
