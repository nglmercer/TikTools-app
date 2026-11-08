use crate::{http, state::State, Error};
use axum::{extract::ConnectInfo, Extension, Router};
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tiktools_collaboration::{CollaborationService, Configuration, GatewayStatus};
use tiktools_control_api::ControlApi;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::Semaphore,
    task::{JoinHandle, JoinSet},
};
use tokio_rustls::TlsAcceptor;
use tokio_util::sync::CancellationToken;

pub struct RemoteGateway {
    address: SocketAddr,
    configuration_revision: String,
    stopping: CancellationToken,
    task: Option<JoinHandle<Result<(), Error>>>,
}

impl RemoteGateway {
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }
    pub fn configuration_revision(&self) -> &str {
        &self.configuration_revision
    }
    pub fn is_finished(&self) -> bool {
        self.task.as_ref().is_none_or(JoinHandle::is_finished)
    }
    pub async fn shutdown(mut self) -> Result<(), Error> {
        self.stopping.cancel();
        if let Some(mut task) = self.task.take() {
            match tokio::time::timeout(Duration::from_secs(6), &mut task).await {
                Ok(result) => result.map_err(|_| Error::Task)?,
                Err(_) => {
                    task.abort();
                    Err(Error::Task)
                }
            }
        } else {
            Ok(())
        }
    }
}
impl Drop for RemoteGateway {
    fn drop(&mut self) {
        self.stopping.cancel();
    }
}

/// Starts only a configured listener against the existing authoritative host.
pub async fn start(api: Arc<ControlApi>) -> Result<RemoteGateway, Error> {
    let host = Arc::clone(&api);
    let (service, configuration, revision, tls, fingerprint) =
        tokio::task::spawn_blocking(move || prepare(&host))
            .await
            .map_err(|_| Error::Task)??;
    let listener = TcpListener::bind(&configuration.bind_address).await?;
    launch(
        api,
        service,
        configuration,
        revision,
        tls,
        fingerprint,
        listener,
    )
    .await
}

/// Socket activation and integration tests may pass a listener whose resolved
/// address matches the persisted configuration. It never changes host policy.
pub async fn start_on(api: Arc<ControlApi>, listener: TcpListener) -> Result<RemoteGateway, Error> {
    let host = Arc::clone(&api);
    let (service, configuration, revision, tls, fingerprint) =
        tokio::task::spawn_blocking(move || prepare(&host))
            .await
            .map_err(|_| Error::Task)??;
    let configured: SocketAddr = configuration
        .bind_address
        .parse()
        .map_err(|_| Error::Configuration)?;
    if listener.local_addr()? != configured {
        return Err(Error::Configuration);
    }
    launch(
        api,
        service,
        configuration,
        revision,
        tls,
        fingerprint,
        listener,
    )
    .await
}

type Prepared = (
    Arc<CollaborationService>,
    Configuration,
    String,
    Option<TlsAcceptor>,
    Option<String>,
);
fn prepare(api: &ControlApi) -> Result<Prepared, Error> {
    let service = api.core().collaboration()?;
    let (configuration, revision) = service.configuration_snapshot()?;
    if !configuration.enabled {
        return Err(Error::Configuration);
    }
    let origin = url::Url::parse(
        configuration
            .advertised_origin
            .as_deref()
            .ok_or(Error::Configuration)?,
    )
    .map_err(|_| Error::Configuration)?;
    let bind: SocketAddr = configuration
        .bind_address
        .parse()
        .map_err(|_| Error::Configuration)?;
    if configuration.tls_proxy {
        if !bind.ip().is_loopback() || origin.scheme() != "https" {
            return Err(Error::Configuration);
        }
        return Ok((service, configuration, revision, None, None));
    }
    if let (Some(certificate), Some(key)) = (
        &configuration.tls_certificate,
        &configuration.tls_private_key,
    ) {
        if origin.scheme() != "https" {
            return Err(Error::Configuration);
        }
        let key_metadata = std::fs::symlink_metadata(key).map_err(|_| Error::Tls)?;
        let cert_metadata = std::fs::symlink_metadata(certificate).map_err(|_| Error::Tls)?;
        if !key_metadata.is_file()
            || !cert_metadata.is_file()
            || key_metadata.len() > 65536
            || cert_metadata.len() > 65536
        {
            return Err(Error::Tls);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if key_metadata.permissions().mode() & 0o077 != 0 {
                return Err(Error::Tls);
            }
        }
        let certificates: Vec<_> = CertificateDer::pem_file_iter(certificate)
            .map_err(|_| Error::Tls)?
            .collect::<Result<_, _>>()
            .map_err(|_| Error::Tls)?;
        let first = certificates.first().ok_or(Error::Tls)?;
        let fingerprint = ring::digest::digest(&ring::digest::SHA256, first.as_ref())
            .as_ref()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let private_key = PrivateKeyDer::from_pem_file(key).map_err(|_| Error::Tls)?;
        let server = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| Error::Tls)?
        .with_no_client_auth()
        .with_single_cert(certificates, private_key)
        .map_err(|_| Error::Tls)?;
        Ok((
            service,
            configuration,
            revision,
            Some(TlsAcceptor::from(Arc::new(server))),
            Some(fingerprint),
        ))
    } else {
        if !bind.ip().is_loopback() || origin.scheme() != "http" {
            return Err(Error::Configuration);
        }
        Ok((service, configuration, revision, None, None))
    }
}

async fn launch(
    api: Arc<ControlApi>,
    service: Arc<CollaborationService>,
    configuration: Configuration,
    revision: String,
    tls: Option<TlsAcceptor>,
    fingerprint: Option<String>,
    listener: TcpListener,
) -> Result<RemoteGateway, Error> {
    let address = listener.local_addr()?;
    let stopping = CancellationToken::new();
    let certificate_fingerprint = fingerprint.clone();
    let state = State::new(
        api,
        Arc::clone(&service),
        configuration,
        fingerprint,
        stopping.clone(),
    )?;
    let closing_state = Arc::clone(&state);
    let generation = state.generation.clone();
    let router = http::router(state);
    service.set_gateway_status(GatewayStatus {
        generation,
        available: true,
        running: true,
        connections: 0,
        configuration_revision: Some(revision.clone()),
        address: Some(address.to_string()),
        last_error: None,
        certificate_fingerprint,
        session_connections: Default::default(),
    })?;
    let token = stopping.clone();
    let task = tokio::spawn(async move {
        let cancellation = token.clone();
        let mut result = serve(listener, router, tls, token).await;
        cancellation.cancel();
        if tokio::time::timeout(Duration::from_secs(5), closing_state.wait_closed())
            .await
            .is_err()
        {
            result = Err(Error::Task);
        }
        let _ = service.update_gateway_status(|status| {
            if status.generation != closing_state.generation {
                return;
            }
            status.running = false;
            status.connections = 0;
            status.session_connections.clear();
            status.certificate_fingerprint = None;
            status.address = None;
            status.configuration_revision = None;
            if result.is_err() {
                status.last_error = Some("remote listener stopped unexpectedly".into());
            }
        });
        result
    });
    Ok(RemoteGateway {
        address,
        configuration_revision: revision,
        stopping,
        task: Some(task),
    })
}

async fn serve(
    listener: TcpListener,
    router: Router,
    tls: Option<TlsAcceptor>,
    stopping: CancellationToken,
) -> Result<(), Error> {
    let transport_limit = Arc::new(Semaphore::new(64));
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            _=stopping.cancelled()=>break,
            result=listener.accept()=>{
                let (stream,peer)=result?;
                let Ok(permit)=transport_limit.clone().try_acquire_owned()else{continue;};
                let router=router.clone().layer(Extension(ConnectInfo(peer)));
                let tls=tls.clone();let stopping=stopping.clone();
                connections.spawn(async move{
                    let _permit=permit;
                    if let Some(tls)=tls{
                        if let Ok(Ok(stream))=tokio::time::timeout(Duration::from_secs(5),tls.accept(stream)).await{connection(stream,router,stopping).await;}
                    }else{connection(stream,router,stopping).await;}
                });
            },
            _=connections.join_next(),if !connections.is_empty()=>{},
        }
    }
    connections.abort_all();
    while connections.join_next().await.is_some() {}
    Ok(())
}

async fn connection<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    stream: T,
    router: Router,
    stopping: CancellationToken,
) {
    let mut builder = hyper::server::conn::http1::Builder::new();
    builder
        .timer(TokioTimer::new())
        .header_read_timeout(Duration::from_secs(10))
        .max_buf_size(32768);
    let connection = builder
        .serve_connection(TokioIo::new(stream), TowerToHyperService::new(router))
        .with_upgrades();
    tokio::pin!(connection);
    tokio::select! {_=stopping.cancelled()=>{},_= &mut connection=>{}}
}

/// Observe configuration without creating a database for local-only startup.
/// A persisted enabled configuration is restored when the host starts again.
pub async fn supervise(api: Arc<ControlApi>) {
    api.core().set_collaboration_gateway_available(true);
    let mut current: Option<RemoteGateway> = None;
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    loop {
        interval.tick().await;
        if api.core().is_shutdown() {
            break;
        }
        let host = Arc::clone(&api);
        let configuration = tokio::task::spawn_blocking(move || {
            if !host
                .core()
                .db
                .paths()
                .data
                .join("collaboration/collaboration.db")
                .exists()
            {
                return Ok::<_, Error>(None);
            }
            let service = host.core().collaboration()?;
            service.update_gateway_status(|status| status.available = true)?;
            Ok(Some(service.configuration_snapshot()?))
        })
        .await;
        let Ok(Ok(configuration)) = configuration else {
            continue;
        };
        let changed = current.as_ref().is_some_and(|gateway| {
            configuration
                .as_ref()
                .is_none_or(|(_, revision)| revision != gateway.configuration_revision())
                || gateway.is_finished()
        });
        if changed {
            if let Some(gateway) = current.take() {
                let _ = gateway.shutdown().await;
            }
        }
        if configuration.is_some_and(|(config, _)| config.enabled) && current.is_none() {
            match start(Arc::clone(&api)).await {
                Ok(gateway) => current = Some(gateway),
                Err(_) => {
                    if let Ok(service) = api.core().collaboration() {
                        let _ = service.update_gateway_status(|status| {
                            status.available = true;
                            status.running = false;
                            status.last_error = Some(
                                "Unable to start remote access. Check the configured address and TLS files."
                                    .into(),
                            );
                        });
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    if let Some(gateway) = current {
        let _ = gateway.shutdown().await;
    }
    api.core().set_collaboration_gateway_available(false);
}
