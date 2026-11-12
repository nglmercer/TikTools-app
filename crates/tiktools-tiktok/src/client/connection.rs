//! Generation selection and the existing single owned connection task.
use super::{
    config::default_preset, initialize::initialize_connection, pump::run_connection,
    session::resolve_session,
};
use crate::{
    retry, ClientEvent, ConnectRequest, ConnectionInfo, ConnectionState, ErrorPhase,
    NativeTikTokClient, TikTokError,
};

impl NativeTikTokClient {
    pub async fn connect_native(
        &self,
        request: ConnectRequest,
    ) -> Result<ConnectionInfo, TikTokError> {
        self.connect_generation(request).await.map(|(_, info)| info)
    }

    pub async fn connect_generation(
        &self,
        request: ConnectRequest,
    ) -> Result<(u64, ConnectionInfo), TikTokError> {
        self.connect_selected(|_| async move { Ok(request) }).await
    }

    pub async fn pick_generation(
        &self,
        session_cookie: String,
    ) -> Result<(u64, ConnectionInfo), TikTokError> {
        self.connect_selected(move |retry| async move {
            let cookies = retry.run(|| resolve_session(&session_cookie)).await?;
            let client = ttl_live_discovery::DiscoveryClient::new(&default_preset())
                .map_err(TikTokError::from_discovery)?
                .with_session(cookies.to_cookie_string());
            let rooms = retry
                .run(|| async {
                    client
                        .live_channels("live")
                        .await
                        .map_err(TikTokError::from_discovery)
                })
                .await?;
            let room = rooms
                .into_iter()
                .next()
                .ok_or_else(|| TikTokError::Discovery("TikTok returned no live rooms".into()))?;
            Ok(ConnectRequest {
                unique_id: room.unique_id,
                session_cookie: cookies.to_cookie_string(),
                room_id: Some(room.room_id),
            })
        })
        .await
    }

    pub(in super::super) async fn connect_selected<F>(
        &self,
        selection: impl FnOnce(retry::RetryContext) -> F + Send + 'static,
    ) -> Result<(u64, ConnectionInfo), TikTokError>
    where
        F: std::future::Future<Output = Result<ConnectRequest, TikTokError>> + Send + 'static,
    {
        let config = self.config.clone();
        let events = self.events.clone();
        let cooldown = self.cooldown.clone();
        self.owner
            .connect(move |generation, stop, state, ready| async move {
                let retry = retry::RetryContext {
                    cooldown,
                    policy: config.reconnect.clone(),
                    stop: stop.clone(),
                    events: events.clone(),
                    state: state.clone(),
                    generation,
                };
                let initialized = match selection(retry.clone()).await {
                    Ok(request) => initialize_connection(config, request, retry).await,
                    Err(error) => Err(error),
                };
                match initialized {
                    Ok((connection, info)) => {
                        if stop.is_cancelled() {
                            let _ =
                                ready.send(Err(TikTokError::Task("connection superseded".into())));
                            return;
                        }
                        state.transition(generation, ConnectionState::Connected);
                        events.send(generation, ClientEvent::Connected(info.clone()));
                        let gifts = info.gifts.clone();
                        let _ = ready.send(Ok(info));
                        run_connection(
                            connection,
                            stop.clone(),
                            events,
                            state.clone(),
                            generation,
                            gifts,
                        )
                        .await;
                        state.transition(
                            generation,
                            if stop.is_cancelled() {
                                ConnectionState::Idle
                            } else {
                                ConnectionState::Failed
                            },
                        );
                    }
                    Err(error) => {
                        state.transition(
                            generation,
                            if stop.is_cancelled() {
                                ConnectionState::Idle
                            } else {
                                ConnectionState::Failed
                            },
                        );
                        if !stop.is_cancelled() {
                            events.send(
                                generation,
                                ClientEvent::Error {
                                    phase: ErrorPhase::Connect,
                                    message: error.to_string(),
                                },
                            );
                        }
                        let _ = ready.send(Err(error));
                    }
                }
            })
            .await
    }
}
