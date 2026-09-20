//! Optional `tiktools.event-gateway` process plugin.
//!
//! The host only delivers stable `DomainEventEnvelope` values through the
//! generic plugin event hook. All HTTP, WebSocket, CORS, origin, token, and
//! widget transport behavior belongs to this process.

mod gateway;

use std::process::ExitCode;

use tiktools_plugin_sdk::{run_process_plugin_with, PluginContext, PluginError, PluginResult};

fn main() -> ExitCode {
    let context = PluginContext::from_process_environment();
    let mut plugin = gateway::EventGatewayPlugin::default();
    let result = initialize_and_run(&mut plugin, &context);
    if let Err(error) = result {
        eprintln!("event-gateway: {error}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn initialize_and_run(
    plugin: &mut gateway::EventGatewayPlugin,
    context: &PluginContext,
) -> PluginResult<()> {
    use tiktools_plugin_sdk::Plugin;

    plugin.initialize(context)?;
    run_process_plugin_with(plugin, context)
}

#[allow(dead_code)]
fn _startup_error(message: impl Into<String>) -> PluginError {
    PluginError::other(message)
}
