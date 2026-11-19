//! Remote-control plugin: owns its listener and dispatches commands through
//! the host's capability-gated `host.call` API. It never receives a bearer
//! token, ControlApi or raw settings.

use serde_json::json;
use tiktools_plugin_sdk::{
    host_remote_execute, run_process_plugin, ActionCall, ActionResult, DomainEventEnvelope,
    Plugin, PluginContext, PluginError, PluginResult,
};

#[derive(Default)]
struct RemoteControl;

impl Plugin for RemoteControl {
    fn initialize(&mut self, _context: &PluginContext) -> PluginResult<()> {
        // A production build starts its WSS/Iroh listener here. The listener
        // calls `host_remote_execute` from its own thread; the dedicated
        // reverse channel makes that safe and independent of host polling.
        Ok(())
    }

    fn action(
        &mut self,
        _context: &PluginContext,
        call: ActionCall,
    ) -> PluginResult<ActionResult> {
        match call.action_type() {
            Some("tiktools.remote-control.test") => {
                let status = host_remote_execute("cloud.status", json!({}))?;
                Ok(ActionResult::summary(format!(
                    "host channel OK: {}",
                    status
                )))
            }
            other => Err(PluginError::other(format!("unknown action {other:?}"))),
        }
    }

    fn event(
        &mut self,
        _context: &PluginContext,
        event: DomainEventEnvelope,
    ) -> PluginResult<()> {
        // `cloud.login.code` frames are delivered here by the host's event
        // bus. The code can be shown in the plugin UI and submitted with
        // `host_remote_execute("cloud.login.code.submit", ...)`.
        if event.topic == "cloud.login.code" {
            // Display-only: never log or persist the code.
            let _ = &event.data;
        }
        Ok(())
    }
}

fn main() -> PluginResult<()> {
    run_process_plugin::<RemoteControl>()
}
