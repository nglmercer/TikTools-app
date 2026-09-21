use crate::*;

impl AppCore {
    #[cfg(feature = "native-tiktok")]
    pub(crate) fn start_live_event_pump(self: &Arc<Self>) {
        if self.live_pump_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut receiver = self.live.subscribe();
        let core = Arc::clone(self);
        tracing::info!("native TikTok event pump started");
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        tracing::debug!(
                            kind = client_event_kind(&event),
                            "native TikTok event received"
                        );
                        core.handle_native_event(event).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!(count, "native TikTok event receiver lagged")
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::warn!("native TikTok event stream closed");
                        break;
                    }
                }
            }
        });
    }
    #[cfg(not(feature = "native-tiktok"))]
    pub(crate) fn start_live_event_pump(self: &Arc<Self>) {}
    // Live connect/pick moved to the value-returning control operations
    // (`AppCore::live_connect` / `AppCore::live_pick`); every client,
    // including the WebView, funnels through them.
    #[cfg(feature = "persistence")]
    pub(crate) fn current_creator_unique_id(&self) -> Option<String> {
        recover_rwlock_read(&self.connection_context, "connection context")
            .as_ref()
            .map(|context| context.unique_id.clone())
    }
}
