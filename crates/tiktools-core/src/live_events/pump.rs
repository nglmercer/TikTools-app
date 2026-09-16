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
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn connect_native(self: &Arc<Self>, request: ConnectRequest) {
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        self.emit(HostMessage::Connection {
            status: ipc::messages::ConnectionStatus::Connecting,
            unique_id: clean_unique_id(&request.unique_id),
            title: None,
            room_id: request.room_id.clone(),
            avatar_url: None,
        });

        match self.live.connect(request).await {
            Ok(_) => {}
            Err(error) => {
                self.emit(HostMessage::Error {
                    phase: ipc::messages::ErrorPhase::Connect,
                    message: error.to_string(),
                });
                self.live.disconnect().await;
                self.emit(HostMessage::connection_disconnected());
            }
        }
    }
    #[cfg(not(feature = "native-tiktok"))]
    pub(crate) async fn connect_native(self: &Arc<Self>, _request: ConnectRequest) {
        self.emit(HostMessage::Error {
            phase: ipc::messages::ErrorPhase::Connect,
            message: "the native TikTok client is disabled in this build".to_owned(),
        });
        self.emit(HostMessage::connection_disconnected());
    }
    #[cfg(feature = "native-tiktok")]
    pub(crate) async fn pick_live(self: &Arc<Self>, session_cookie: &str) {
        match self.live.live_channels(session_cookie).await {
            Ok(mut rooms) => {
                if rooms.is_empty() {
                    self.emit(HostMessage::Error {
                        phase: ipc::messages::ErrorPhase::Connect,
                        message: "TikTok returned no live rooms.".to_owned(),
                    });
                    return;
                }
                // The native discovery client orders rooms by viewers. Picking
                // the first item is deterministic and avoids a random source
                // in the core; callers can request another room explicitly.
                let room = rooms.remove(0);
                self.connect_native(ConnectRequest {
                    unique_id: room.unique_id,
                    session_cookie: session_cookie.to_owned(),
                    room_id: Some(room.room_id),
                })
                .await;
            }
            Err(error) => {
                self.emit(HostMessage::Error {
                    phase: ipc::messages::ErrorPhase::Connect,
                    message: error.to_string(),
                });
                self.emit(HostMessage::connection_disconnected());
            }
        }
    }
    #[cfg(not(feature = "native-tiktok"))]
    pub(crate) async fn pick_live(&self, _session_cookie: &str) {
        self.emit(HostMessage::Error {
            phase: ipc::messages::ErrorPhase::Connect,
            message: "the native TikTok client is disabled in this build".to_owned(),
        });
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn current_creator_unique_id(&self) -> Option<String> {
        self.connection_context
            .read()
            .expect("connection context lock poisoned")
            .as_ref()
            .map(|context| context.unique_id.clone())
    }
}
