use super::*;

pub(super) async fn asset(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    request: Request,
) -> Response {
    if request.uri().path().starts_with("/__plugins/") {
        return plugin_asset(app, headers, request).await;
    }
    if !app.authorized(&headers) {
        return (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            PAIR_PAGE,
        )
            .into_response();
    }
    let raw = request.uri().path();
    // URL decoding is deliberately disallowed for static paths in the first adapter.
    if raw.contains('%') || raw.contains('\\') || raw.split('/').any(|part| part == "..") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let path = app.root.join(raw.trim_start_matches('/'));
    let path = if raw == "/" {
        app.root.join("index.html")
    } else {
        path
    };
    let path = match tokio::fs::canonicalize(path).await {
        Ok(path) if path.starts_with(&app.root) && path.is_file() => path,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let mime = match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    };
    if !matches!(tokio::fs::metadata(&path).await, Ok(meta) if meta.len() <= 32 * 1024 * 1024) {
        return StatusCode::NOT_FOUND.into_response();
    }
    if path == app.root.join("index.html") {
        return match tokio::fs::read_to_string(path).await {
            Ok(html) => (
                [(header::CONTENT_TYPE, mime)],
                html.replacen(
                    "<head>",
                    &format!("<head><meta name=\"tiktools-transport\" content=\"local-web\"><meta name=\"tiktools-plugin-assets\" content=\"/__plugins/{}/\">", app.asset_token),
                    1,
                ),
            )
                .into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        };
    }
    match tokio::fs::read(path).await {
        Ok(bytes) if bytes.len() <= 32 * 1024 * 1024 => {
            ([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

// The opaque iframe receives an asset-only random prefix, never the HTTP session
// cookie. This lets module requests omit credentials while the broker remains
// its only way to request control operations. Tokens expire with the session.
/// Resolves `file` inside a plugin's asset root, or `None` when it is
/// missing, not a regular file, or escapes the root (including through
/// symlinks). Both sides are canonicalized: comparing a canonical file
/// against a raw root fails for Windows `\\?\` paths and for data
/// directories reached through a symlink.
pub(super) async fn resolve_plugin_asset(root: &Path, file: &str) -> Option<PathBuf> {
    let root = tokio::fs::canonicalize(root).await.ok()?;
    let path = tokio::fs::canonicalize(root.join(file)).await.ok()?;
    (path.starts_with(&root) && path.is_file()).then_some(path)
}

async fn plugin_asset(app: Arc<App>, headers: HeaderMap, request: Request) -> Response {
    let valid_session = app
        .auth
        .lock()
        .ok()
        .and_then(|auth| {
            auth.session
                .as_ref()
                .map(|(_, issued)| issued.elapsed() < SESSION_LIFETIME)
        })
        .unwrap_or(false);
    let prefix = format!("/__plugins/{}/", app.asset_token);
    let Some(rest) = request.uri().path().strip_prefix(&prefix) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !valid_session
        || headers
            .get(header::ORIGIN)
            .is_some_and(|value| value != "null" && value != app.origin.as_str())
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some((id, file)) = rest.split_once('/') else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        || file.is_empty()
        || file.contains('%')
        || file.contains('\\')
        || file
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let client = match ControlClient::connect().await {
        Ok(client) => client,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let assets: tiktools_control_api::modules::plugins::PluginUiAssetsResult = match client
        .call("plugins.ui.assets", json!({"pluginId":id}))
        .await
    {
        Ok(assets) => assets,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let Some(canonical) = resolve_plugin_asset(Path::new(&assets.asset_root), file).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !matches!(tokio::fs::metadata(&canonical).await, Ok(meta) if meta.len() <= 8 * 1024 * 1024) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mime = match canonical
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        _ => "application/octet-stream",
    };
    match tokio::fs::read(canonical).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
