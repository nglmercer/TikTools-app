mod auth;
mod config;
mod http;
mod server;
mod state;
#[cfg(test)]
mod tests;
mod topics;
mod websocket;

use self::config::{load_config, persist_generated_token};
use self::server::run_server;
use self::state::GatewayState;
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tiktools_plugin_sdk::{DomainEventEnvelope, Plugin, PluginContext, PluginError, PluginResult};

#[derive(Default)]
pub struct EventGatewayPlugin {
    state: Option<Arc<GatewayState>>,
    server_thread: Option<JoinHandle<()>>,
}

impl Plugin for EventGatewayPlugin {
    fn initialize(&mut self, _context: &PluginContext) -> PluginResult<()> {
        let (config, settings_path) = load_config()?;
        persist_generated_token(&config, settings_path.as_ref())?;
        let state = GatewayState::new(config);
        let (ready_sender, ready_receiver) = std_mpsc::channel();
        let server_state = Arc::clone(&state);
        let thread = std::thread::Builder::new()
            .name("tiktools-event-gateway".to_owned())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready_sender
                            .send(Err(format!("could not start gateway runtime: {error}")));
                        return;
                    }
                };
                let result = runtime.block_on(run_server(server_state, ready_sender));
                if let Err(error) = result {
                    eprintln!("event-gateway server stopped: {error}");
                }
            })
            .map_err(|error| {
                PluginError::other(format!("could not start gateway thread: {error}"))
            })?;

        match ready_receiver.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                self.state = Some(state);
                self.server_thread = Some(thread);
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(PluginError::other(error))
            }
            Err(error) => {
                state.server_shutdown.notify_one();
                let join_error = thread.join().err().map(|_| "; gateway thread panicked");
                Err(PluginError::other(format!(
                    "gateway did not become ready: {error}{}",
                    join_error.unwrap_or_default()
                )))
            }
        }
    }

    fn event(&mut self, _context: &PluginContext, event: DomainEventEnvelope) -> PluginResult<()> {
        if let Some(state) = self.state.as_ref() {
            state.publish(event);
        }
        Ok(())
    }

    fn shutdown(&mut self, _context: &PluginContext) -> PluginResult<()> {
        if let Some(state) = self.state.take() {
            // Queue a permit for the listener even if shutdown races the
            // listener's first `select`; the listener fans out a wake to all
            // active HTTP/WebSocket connections before it exits.
            state.server_shutdown.notify_one();
        }
        if let Some(thread) = self.server_thread.take() {
            thread
                .join()
                .map_err(|_| PluginError::other("gateway server thread panicked"))?;
        }
        Ok(())
    }
}
