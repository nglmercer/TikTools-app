use std::sync::Arc;

use tiktools_core::{ipc::messages::HostMessage, HostEmitter};
use winit::event_loop::EventLoopProxy;

#[derive(Debug)]
#[allow(dead_code)]
pub enum DesktopCommand {
    EmitToWebview(String),
    FrontendReady,
    ShowWindow,
    HideWindow,
    OpenDevtools,
    Quit,
    ShutdownComplete,
    IpcFailed(String),
    FlushWebviewBatch,
}

#[derive(Debug)]
pub enum DesktopEvent {
    Command(DesktopCommand),
}

/// Converts framework-independent core messages into UI-thread commands. The
/// core never receives a WebView handle and can therefore be tested headlessly.
pub struct EventLoopEmitter {
    proxy: EventLoopProxy<DesktopEvent>,
}

impl EventLoopEmitter {
    pub fn new(proxy: EventLoopProxy<DesktopEvent>) -> Self {
        Self { proxy }
    }
}

impl HostEmitter for EventLoopEmitter {
    fn emit(&self, message: HostMessage) {
        match message.to_json() {
            Ok(payload) => {
                if let Err(error) =
                    self.proxy
                        .send_event(DesktopEvent::Command(DesktopCommand::EmitToWebview(
                            payload,
                        )))
                {
                    tracing::debug!(%error, "UI event loop is no longer accepting host messages");
                }
            }
            Err(error) => tracing::error!(%error, "could not serialize host message"),
        }
    }
}

pub fn shared_emitter(proxy: EventLoopProxy<DesktopEvent>) -> Arc<EventLoopEmitter> {
    Arc::new(EventLoopEmitter::new(proxy))
}
