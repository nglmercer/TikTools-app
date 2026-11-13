use crate::{auth::HttpError, Error};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Instant,
};
use tiktools_collaboration::{CollaborationService, Configuration};
use tiktools_control_api::ControlApi;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

pub(crate) struct State {
    pub generation: String,
    pub peer: bool,
    pub api: Arc<ControlApi>,
    pub service: Arc<CollaborationService>,
    pub configuration: Configuration,
    pub origins: std::collections::BTreeSet<String>,
    pub authorities: std::collections::BTreeSet<String>,
    pub fingerprint: Option<String>,
    pub stopping: CancellationToken,
    pub browser: Mutex<BTreeMap<String, BrowserSecrets>>,
    pub rates: Mutex<BTreeMap<String, Bucket>>,
    connections: Arc<Semaphore>,
    principals: Mutex<BTreeMap<String, u32>>,
}

pub(crate) struct BrowserSecrets {
    pub csrf: String,
    pub session_id: String,
    pub expires: Instant,
    pub tickets: BTreeMap<String, Instant>,
}

pub(crate) struct Bucket {
    pub tokens: f64,
    pub updated: Instant,
}

impl State {
    pub fn new(
        api: Arc<ControlApi>,
        service: Arc<CollaborationService>,
        configuration: Configuration,
        fingerprint: Option<String>,
        stopping: CancellationToken,
    ) -> Result<Arc<Self>, Error> {
        Self::build(api, service, configuration, fingerprint, stopping, false)
    }

    #[cfg(feature = "peer")]
    pub fn new_peer(
        api: Arc<ControlApi>,
        service: Arc<CollaborationService>,
        configuration: Configuration,
        stopping: CancellationToken,
    ) -> Result<Arc<Self>, Error> {
        Self::build(api, service, configuration, None, stopping, true)
    }

    fn build(
        api: Arc<ControlApi>,
        service: Arc<CollaborationService>,
        configuration: Configuration,
        fingerprint: Option<String>,
        stopping: CancellationToken,
        peer: bool,
    ) -> Result<Arc<Self>, Error> {
        configuration.limits.validate()?;
        let mut origins = configuration.allowed_origins.clone();
        if let Some(origin) = &configuration.advertised_origin {
            origins.insert(origin.clone());
        } else if !peer {
            return Err(Error::Configuration);
        }
        let mut authorities = std::collections::BTreeSet::new();
        for origin in &origins {
            let parsed = url::Url::parse(origin).map_err(|_| Error::Configuration)?;
            if parsed.origin().ascii_serialization() != *origin {
                return Err(Error::Configuration);
            }
            authorities.insert(
                parsed[url::Position::BeforeHost..url::Position::AfterPort].to_ascii_lowercase(),
            );
        }
        let capacity = configuration.limits.connections as usize;
        Ok(Arc::new(Self {
            generation: uuid::Uuid::new_v4().to_string(),
            peer,
            api,
            service,
            configuration,
            origins,
            authorities,
            fingerprint,
            stopping,
            browser: Mutex::new(BTreeMap::new()),
            rates: Mutex::new(BTreeMap::new()),
            connections: Arc::new(Semaphore::new(capacity)),
            principals: Mutex::new(BTreeMap::new()),
        }))
    }

    pub fn update_status(
        &self,
        update: impl FnOnce(&mut tiktools_collaboration::GatewayStatus),
    ) -> Result<(), tiktools_collaboration::Error> {
        if self.peer {
            self.service
                .update_peer_gateway_status(|status| update(&mut status.gateway))
        } else {
            self.service.update_gateway_status(update)
        }
    }

    pub fn admit(
        self: &Arc<Self>,
        principal: &str,
        session: &str,
    ) -> Result<ConnectionPermit, HttpError> {
        if self.stopping.is_cancelled() {
            return Err(HttpError::unauthorized());
        }
        let global = self
            .connections
            .clone()
            .try_acquire_owned()
            .map_err(|_| HttpError::overloaded())?;
        let mut counts = self.principals.lock().map_err(|_| HttpError::internal())?;
        let count = counts.entry(principal.into()).or_default();
        if *count >= self.configuration.limits.connections_per_principal {
            return Err(HttpError::overloaded());
        }
        let mut current = false;
        self.update_status(|status| {
            if status.generation != self.generation
                || !status.running
                || self.stopping.is_cancelled()
            {
                return;
            }
            current = true;
            status.connections += 1;
            *status
                .session_connections
                .entry(session.into())
                .or_default() += 1;
        })
        .map_err(HttpError::from)?;
        if !current {
            return Err(HttpError::unauthorized());
        }
        *count += 1;
        Ok(ConnectionPermit {
            state: Arc::clone(self),
            principal: principal.into(),
            session: session.into(),
            _global: global,
        })
    }

    pub fn rate(&self, session: &str) -> Result<(), HttpError> {
        let (rate, burst) = if session.starts_with("source:") {
            (30.0, 60.0)
        } else {
            (
                self.configuration.limits.requests_per_second as f64,
                self.configuration.limits.request_burst as f64,
            )
        };
        let now = Instant::now();
        let mut rates = self.rates.lock().map_err(|_| HttpError::internal())?;
        rates.retain(|_, bucket| now.duration_since(bucket.updated).as_secs() < 60);
        if rates.len() >= 1024 && !rates.contains_key(session) {
            return Err(HttpError::overloaded());
        }
        let bucket = rates.entry(session.into()).or_insert(Bucket {
            tokens: burst,
            updated: now,
        });
        bucket.tokens =
            (bucket.tokens + now.duration_since(bucket.updated).as_secs_f64() * rate).min(burst);
        bucket.updated = now;
        if bucket.tokens < 1.0 {
            return Err(HttpError::rate_limited());
        }
        bucket.tokens -= 1.0;
        Ok(())
    }

    pub async fn wait_closed(&self) {
        let _ = self
            .connections
            .clone()
            .acquire_many_owned(self.configuration.limits.connections)
            .await;
    }
}

pub(crate) struct ConnectionPermit {
    state: Arc<State>,
    principal: String,
    session: String,
    _global: OwnedSemaphorePermit,
}
impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        if let Ok(mut counts) = self.state.principals.lock() {
            if let Some(count) = counts.get_mut(&self.principal) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    counts.remove(&self.principal);
                }
            }
        }
        let _ = self.state.update_status(|status| {
            if status.generation != self.state.generation {
                return;
            }
            status.connections = status.connections.saturating_sub(1);
            if let Some(count) = status.session_connections.get_mut(&self.session) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    status.session_connections.remove(&self.session);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiktools_collaboration::GatewayStatus;
    use tiktools_core::{
        ipc::messages::HostMessage, paths::AppPaths, AppCore, HostEmitter, NoopMediaHost,
    };
    struct Emitter;
    impl HostEmitter for Emitter {
        fn emit(&self, _: HostMessage) {}
    }
    #[tokio::test]
    async fn a_retired_listener_cannot_change_the_replacements_session_counts() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().to_path_buf();
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            AppPaths {
                data: root.join("data"),
                plugins: root.join("plugins"),
                plugin_data: root.join("plugin-data"),
                builtin_plugins: root.join("builtin"),
                development_plugins: None,
                logs: root.join("logs"),
                temp: root.join("temp"),
                root,
            },
        ));
        let api = Arc::new(ControlApi::new(core.clone()));
        let service = core.collaboration().unwrap();
        let configuration = Configuration {
            enabled: true,
            advertised_origin: Some("http://127.0.0.1:8788".into()),
            ..Default::default()
        };
        service.configure(configuration.clone()).unwrap();
        let first = State::new(
            api.clone(),
            service.clone(),
            configuration.clone(),
            None,
            CancellationToken::new(),
        )
        .unwrap();
        service
            .set_gateway_status(GatewayStatus {
                generation: first.generation.clone(),
                running: true,
                ..Default::default()
            })
            .unwrap();
        let old = first
            .admit("principal", "session")
            .unwrap_or_else(|_| panic!("first listener admission failed"));
        let second = State::new(
            api,
            service.clone(),
            configuration,
            None,
            CancellationToken::new(),
        )
        .unwrap();
        service
            .set_gateway_status(GatewayStatus {
                generation: second.generation.clone(),
                running: true,
                ..Default::default()
            })
            .unwrap();
        let current = second
            .admit("principal", "session")
            .unwrap_or_else(|_| panic!("current listener admission failed"));
        drop(old);
        assert!(first.admit("principal", "session").is_err());
        let status = service.gateway_status().unwrap();
        assert_eq!(status.connections, 1);
        assert_eq!(status.session_connections.get("session"), Some(&1));
        drop(current);
        let status = service.gateway_status().unwrap();
        assert_eq!(status.connections, 0);
        assert!(status.session_connections.is_empty());
        core.shutdown().await;
    }
}
