//! Asset serving for isolated plugin WebViews.
//!
//! Each plugin page opens in its own native window whose WebView loads
//! from `tiktools-plugin://app/{plugin-id}/{asset}`. The server below is
//! constructed per window and bound to one plugin id plus the resolved
//! asset root (the entry file's own directory), so a compromised plugin
//! page can read neither other plugins' files nor its own manifest,
//! settings, or backend.

use std::{borrow::Cow, fs, sync::Arc};

use wry::http::{header::CONTENT_TYPE, Request, Response, StatusCode};

use crate::webview::{content_type, error_response, requested_path};

pub const PLUGIN_ASSET_SCHEME: &str = "tiktools-plugin";
pub const PLUGIN_ASSET_HOST: &str = "app";
/// Windows rewrite of `tiktools-plugin://app/...` through WebView2 (see
/// `custom_protocol_workaround` in wry): `{scheme}://{rest}` becomes
/// `http://{scheme}.{rest}`.
pub const WINDOWS_PLUGIN_HOST: &str = "tiktools-plugin.app";

/// Strict plugin-page policy: no network, no frames, no plugins. Page
/// scripts and styles load from the served assets only; audio previews may
/// stream from loopback (the plugin's local server); everything else the
/// page needs travels through the restricted broker IPC.
pub const PLUGIN_CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'none'; ",
    "base-uri 'none'; ",
    "object-src 'none'; ",
    "frame-ancestors 'none'; ",
    "frame-src 'none'; ",
    "form-action 'none'; ",
    "script-src 'self'; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data:; ",
    "font-src 'self' data:; ",
    // Note: no `http://[::1]:*` — Chromium rejects an IPv6 loopback with a
    // wildcard port as an invalid CSP source and ignores it.
    "media-src 'self' blob: http://localhost:* http://127.0.0.1:*; ",
    "connect-src 'none'"
);

/// Entry URL for a plugin page. Fragments never reach the asset server,
/// so the page id travels in the hash for the plugin router.
pub fn plugin_page_url(plugin_id: &str, entry_file: &str, page_id: &str) -> String {
    format!("{PLUGIN_ASSET_SCHEME}://{PLUGIN_ASSET_HOST}/{plugin_id}/{entry_file}#page={page_id}")
}

/// Navigation policy for one plugin window: only its own origin, and only
/// paths under its own plugin id segment.
pub fn allows_navigation(plugin_id: &str, raw_url: &str) -> bool {
    let Ok(url) = url::Url::parse(raw_url) else {
        return false;
    };
    let same_host = (url.scheme() == PLUGIN_ASSET_SCHEME
        && url.host_str() == Some(PLUGIN_ASSET_HOST))
        || (url.scheme() == "http" && url.host_str() == Some(WINDOWS_PLUGIN_HOST));
    if !same_host || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    url.path_segments()
        .and_then(|mut segments| segments.next())
        .is_some_and(|segment| segment == plugin_id)
}

/// Serves one plugin's built UI assets. The `plugin_id` is the window's
/// bound owner: requests for any other first segment are rejected before
/// touching the filesystem.
#[derive(Clone)]
pub struct PluginAssetServer {
    plugin_id: String,
    root: Arc<std::path::PathBuf>,
}

impl PluginAssetServer {
    pub fn new(plugin_id: String, root: Arc<std::path::PathBuf>) -> Self {
        Self { plugin_id, root }
    }

    pub fn respond(&self, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
        let path = match plugin_asset_path(&self.plugin_id, request.uri().path()) {
            Ok(path) => path,
            Err((status, message)) => return error_response(status, message),
        };
        let root = match fs::canonicalize(self.root.as_path()) {
            Ok(root) => root,
            Err(error) => {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
            }
        };
        let candidate = root.join(&path);
        let canonical = match fs::canonicalize(&candidate) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return error_response(StatusCode::NOT_FOUND, "asset not found".to_owned());
            }
            Err(error) => {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
            }
        };
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return error_response(
                StatusCode::FORBIDDEN,
                "asset path escapes plugin UI root".to_owned(),
            );
        }
        let bytes = match fs::read(&canonical) {
            Ok(bytes) => bytes,
            Err(error) => {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
            }
        };
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, content_type(&canonical))
            .header("content-security-policy", PLUGIN_CONTENT_SECURITY_POLICY)
            .body(Cow::Owned(bytes))
            .expect("asset response builder should accept static headers")
    }
}

/// Splits `/{plugin-id}/{asset}`: the owner segment must match the bound
/// plugin id exactly, and the remainder goes through the same traversal
/// validation as the main frontend assets.
fn plugin_asset_path(
    plugin_id: &str,
    raw_path: &str,
) -> Result<std::path::PathBuf, (StatusCode, String)> {
    let mut segments = raw_path.split('/').filter(|segment| !segment.is_empty());
    let owner = segments.next().unwrap_or_default();
    if owner != plugin_id {
        return Err((
            StatusCode::FORBIDDEN,
            "asset belongs to another plugin".to_owned(),
        ));
    }
    let rest: Vec<&str> = segments.collect();
    let rest = rest.join("/");
    requested_path(&rest).map_err(|message| (StatusCode::BAD_REQUEST, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn navigation_is_confined_to_the_owning_plugin() {
        assert!(allows_navigation(
            "sonicboom.server",
            "tiktools-plugin://app/sonicboom.server/index.html#page=tts"
        ));
        assert!(allows_navigation(
            "sonicboom.server",
            "tiktools-plugin://app/sonicboom.server/assets/app.js"
        ));
        assert!(allows_navigation(
            "sonicboom.server",
            "http://tiktools-plugin.app/sonicboom.server/index.html"
        ));
        assert!(!allows_navigation(
            "sonicboom.server",
            "tiktools-plugin://app/other.plugin/index.html"
        ));
        assert!(!allows_navigation(
            "sonicboom.server",
            "tiktools-plugin://app/index.html"
        ));
        assert!(!allows_navigation(
            "sonicboom.server",
            "tiktools://app/index.html"
        ));
        assert!(!allows_navigation(
            "sonicboom.server",
            "https://example.com/sonicboom.server/index.html"
        ));
        assert!(!allows_navigation("sonicboom.server", "not a url"));
    }

    #[test]
    fn cross_plugin_asset_requests_are_rejected_before_io() {
        let server = PluginAssetServer::new(
            "owner".to_owned(),
            Arc::new(env::temp_dir().join("tiktools-plugin-assets-missing")),
        );
        let response = server.respond(
            Request::builder()
                .uri("/other/index.html")
                .body(Vec::new())
                .unwrap(),
        );
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = server.respond(
            Request::builder()
                .uri("/owner/%2e%2e/plugin.json")
                .body(Vec::new())
                .unwrap(),
        );
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn serves_entry_with_the_strict_plugin_csp() {
        let root = env::temp_dir().join(format!(
            "tiktools-plugin-assets-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("index.html"), "<html></html>").unwrap();
        let server = PluginAssetServer::new("owner".to_owned(), Arc::new(root.clone()));
        let response = server.respond(
            Request::builder()
                .uri("/owner/index.html")
                .body(Vec::new())
                .unwrap(),
        );
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-security-policy")
                .and_then(|value| value.to_str().ok()),
            Some(PLUGIN_CONTENT_SECURITY_POLICY)
        );
        assert!(PLUGIN_CONTENT_SECURITY_POLICY.contains("connect-src 'none'"));
        let _ = fs::remove_dir_all(root);
    }
}
