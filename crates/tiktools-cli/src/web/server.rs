use super::*;

pub(super) async fn start(
    assets: PathBuf,
    port: u16,
    join_origin: Option<String>,
) -> Result<(), String> {
    let root = tokio::fs::canonicalize(assets)
        .await
        .map_err(|error| error.to_string())?;
    if !root.join("index.html").is_file() {
        return Err("assets must contain index.html".into());
    }
    let _probe = ControlClient::connect()
        .await
        .map_err(|_| "host_unavailable: start tiktools host --ipc first")?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|error| error.to_string())?;
    let bound_port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let authority = format!("127.0.0.1:{bound_port}");
    let origin = format!("http://{authority}");
    let pairing = mint_pairing()?;
    // Key the readout by the bound port: with `--port 0` the requested
    // port is not where `web --print-code --port N` will look.
    let pairing_path = pairing_file(bound_port);
    publish_pairing_code(&pairing_path, &pairing);
    let join_base = join_origin.unwrap_or_else(|| origin.clone());
    eprintln!(
        "TikTools local browser: {origin}\nOne-use pairing code (valid 5 minutes): {pairing}\nOne-click join: {} (same one-use code, valid 5 minutes)",
        join_link(&join_base, &pairing)
    );
    let state = Arc::new(App {
        root,
        asset_token: secret(),
        origin,
        authority,
        join_base,
        pairing_file: pairing_path.clone(),
        auth: Mutex::new(Auth {
            pairing: Some(pairing),
            issued: Instant::now(),
            session: None,
            paired_at: None,
            attempts: 0,
            renewed_at: None,
        }),
        clients: Arc::new(Semaphore::new(8)),
        stopping: tokio::sync::watch::channel(false).0,
    });
    let shutdown_state = state.clone();
    let pairing_path_for_shutdown = pairing_path;
    let router = Router::new()
        .route("/__local/pair", post(pair))
        .route("/__local/pair/renew", post(renew_pairing))
        .route("/__local/session/touch", post(session_touch))
        .route("/__local/pair.js", get(pair_script))
        .route("/__local/ws", get(upgrade))
        .fallback(get(asset))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(state.clone(), boundary))
        .with_state(state);
    let result = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            #[cfg(unix)]
            {
                if let Ok(mut signal) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = signal.recv() => {} }
                    let _ = shutdown_state.stopping.send(true);
                    return;
                }
            }
            let _ = tokio::signal::ctrl_c().await;
            let _ = shutdown_state.stopping.send(true);
        })
        .await
        .map_err(|error| error.to_string());
    // Best-effort readout cleanup. A stale file is harmless: the reader still
    // enforces the TTL and checks that the adapter is listening.
    let _ = std::fs::remove_file(&pairing_path_for_shutdown);
    result
}

async fn boundary(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    if request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(app.authority.as_str())
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let plugin_asset = request
        .uri()
        .path()
        .starts_with(&format!("/__plugins/{}/", app.asset_token));
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    headers.insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
    headers.insert(header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data: https:; frame-ancestors 'self'; base-uri 'none'; form-action 'self'".parse().unwrap());
    if plugin_asset {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "null".parse().unwrap());
        headers.insert(header::CONTENT_SECURITY_POLICY, "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'none'; frame-ancestors 'self'; sandbox allow-scripts".parse().unwrap());
    }
    response
}
