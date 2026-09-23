//! Optional static hosting for the OBS widgets.
//!
//! When widget assets are present, the gateway serves them at
//! `/widgets/<kind>/` (follow, gift, chat, share, subscribe) so OBS Browser
//! Sources load them straight from the loopback gateway. Assets carry no
//! secrets (the widget token travels in the page URL fragment, which
//! browsers never send to the server), so these routes stay unauthenticated
//! like `/health` — but still loopback-only behind the standard origin check.

use super::config::GatewayConfig;
use super::http::write_http_response;
use super::state::GatewayState;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::BufStream;
use tokio::net::TcpStream;

pub(crate) const WIDGETS_PREFIX: &str = "/widgets/";
pub(crate) const WIDGETS_INDEX: &str = "/widgets";

const WIDGET_INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>TikTools Widgets</title></head>
<body>
<h1>TikTools Widgets</h1>
<ul>
<li><a href="/widgets/follow/">Follow Alert</a></li>
<li><a href="/widgets/gift/">Gift Alert</a></li>
<li><a href="/widgets/chat/">Chat Overlay</a></li>
<li><a href="/widgets/share/">Share Alert</a></li>
<li><a href="/widgets/subscribe/">Subscribe Alert</a></li>
</ul>
</body>
</html>
"#;

pub(crate) fn is_widget_route(path: &str) -> bool {
    path == WIDGETS_INDEX || path.starts_with(WIDGETS_PREFIX)
}

pub(crate) fn is_widget_index(path: &str) -> bool {
    path == WIDGETS_INDEX || path == WIDGETS_PREFIX
}

fn is_widget_name(name: &str) -> bool {
    matches!(name, "follow" | "gift" | "chat" | "share" | "subscribe")
}

/// Resolves the widget asset root: an explicit `widgetsDir` setting wins;
/// otherwise probe the executable directory for `widgets/` (hand-placed dev
/// copy) and `dist/widgets/` (packaged layout — the plugin packager stages
/// `dist/` beside the entry). Returns `None` when hosting is unavailable.
pub(crate) fn widgets_root(config: &GatewayConfig) -> Option<PathBuf> {
    if let Some(dir) = config.widgets_dir.as_ref() {
        return dir.is_dir().then(|| dir.clone());
    }
    let exe_dir = std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)?;
    [
        exe_dir.join("widgets"),
        exe_dir.join("dist").join("widgets"),
    ]
    .into_iter()
    .find(|dir| dir.is_dir())
}

/// Decodes and validates the path after `/widgets/`, mapping bare widget
/// names (`follow`, `follow/`) to their `index.html` entry. Rejects
/// traversal (`..`, backslashes, control characters, percent-encoded
/// separators) and unknown top-level names.
pub(crate) fn widget_asset_segments(request_path: &str) -> Option<Vec<String>> {
    let relative = request_path.strip_prefix(WIDGETS_PREFIX)?;
    if relative.is_empty() {
        return None;
    }
    let decoded = percent_decode(relative)?;
    let mut segments = Vec::new();
    for segment in decoded.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return None;
        }
        if segment.contains('\\') || segment.contains('\0') {
            return None;
        }
        if segment.chars().any(|c| c.is_control()) {
            return None;
        }
        segments.push(segment.to_owned());
    }
    let first = segments.first()?;
    if !is_widget_name(first) {
        return None;
    }
    if segments.len() == 1 {
        segments.push("index.html".to_owned());
    }
    Some(segments)
}

/// Minimal percent-decoder for widget request paths. Rejects truncated
/// escapes, invalid hex, and non-UTF-8 results instead of guessing.
fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'%' {
            decoded.push(byte);
            index += 1;
            continue;
        }
        if index + 2 >= bytes.len() {
            return None;
        }
        let value = (hex_value(bytes[index + 1])? << 4) | hex_value(bytes[index + 2])?;
        decoded.push(value);
        index += 3;
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn widget_content_type(file_name: &str) -> &'static str {
    let extension = file_name.rsplit('.').next().unwrap_or_default();
    if extension.eq_ignore_ascii_case("html") {
        "text/html; charset=utf-8"
    } else if extension.eq_ignore_ascii_case("js") || extension.eq_ignore_ascii_case("mjs") {
        "text/javascript; charset=utf-8"
    } else if extension.eq_ignore_ascii_case("css") {
        "text/css; charset=utf-8"
    } else if extension.eq_ignore_ascii_case("json") {
        "application/json; charset=utf-8"
    } else if extension.eq_ignore_ascii_case("svg") {
        "image/svg+xml"
    } else if extension.eq_ignore_ascii_case("png") {
        "image/png"
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        "image/jpeg"
    } else if extension.eq_ignore_ascii_case("webp") {
        "image/webp"
    } else if extension.eq_ignore_ascii_case("woff") {
        "font/woff"
    } else if extension.eq_ignore_ascii_case("woff2") {
        "font/woff2"
    } else if extension.eq_ignore_ascii_case("wasm") {
        "application/wasm"
    } else {
        "application/octet-stream"
    }
}

pub(crate) async fn serve_widget_request(
    stream: &mut BufStream<TcpStream>,
    state: Arc<GatewayState>,
    request_path: &str,
    origin: Option<&str>,
) -> io::Result<()> {
    if is_widget_index(request_path) {
        return write_http_response(
            stream,
            200,
            "OK",
            "text/html; charset=utf-8",
            WIDGET_INDEX_HTML.as_bytes(),
            origin,
            &state.config,
        )
        .await;
    }
    let (Some(segments), Some(root)) = (
        widget_asset_segments(request_path),
        widgets_root(&state.config),
    ) else {
        return write_widget_404(stream, &state.config, origin).await;
    };
    let mut path = root.clone();
    for segment in &segments {
        path.push(segment);
    }
    // Canonicalize to collapse symlinks, then enforce containment: a
    // compromised asset tree must not serve files outside the widget root.
    let (Ok(canonical), Ok(root_canonical)) = (
        tokio::fs::canonicalize(&path).await,
        tokio::fs::canonicalize(&root).await,
    ) else {
        return write_widget_404(stream, &state.config, origin).await;
    };
    if !canonical.starts_with(&root_canonical) || !canonical.is_file() {
        return write_widget_404(stream, &state.config, origin).await;
    }
    let body = match tokio::fs::read(&canonical).await {
        Ok(body) => body,
        Err(_) => return write_widget_404(stream, &state.config, origin).await,
    };
    let file_name = segments.last().map(String::as_str).unwrap_or_default();
    write_http_response(
        stream,
        200,
        "OK",
        widget_content_type(file_name),
        &body,
        origin,
        &state.config,
    )
    .await
}

async fn write_widget_404(
    stream: &mut BufStream<TcpStream>,
    config: &GatewayConfig,
    origin: Option<&str>,
) -> io::Result<()> {
    write_http_response(
        stream,
        404,
        "Not Found",
        "text/plain; charset=utf-8",
        b"widget not found\n",
        origin,
        config,
    )
    .await
}
