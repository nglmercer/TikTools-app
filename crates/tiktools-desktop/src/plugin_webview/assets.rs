//! Asset serving for isolated plugin WebViews.
//!
//! Plugin pages load from `tiktools-plugin://app/{plugin-id}/{asset}`,
//! both inline (sandboxed frames in the main window's plugin tabs) and in
//! separate native pop-out windows. Two servers share the file-serving
//! core below:
//!
//! - [`PluginAssetServer`] is constructed per pop-out window and bound to
//!   one plugin id plus its resolved asset root (the entry file's own
//!   directory).
//! - [`SharedPluginAssetServer`] serves the main window's inline frames
//!   and resolves the owner plugin per request through the core.
//!
//! Either way a plugin page can read neither other plugins' files nor its
//! own manifest, settings, or backend.

use std::{borrow::Cow, fs, sync::Arc};

use tiktools_core::AppCore;
use wry::http::{
    header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE},
    Request, Response, StatusCode,
};

use crate::webview::{content_type, requested_path};

pub const PLUGIN_ASSET_SCHEME: &str = "tiktools-plugin";
pub const PLUGIN_ASSET_HOST: &str = "app";
/// Windows rewrite of `tiktools-plugin://app/...` through WebView2 (see
/// `custom_protocol_workaround` in wry): `{scheme}://{rest}` becomes
/// `http://{scheme}.{rest}`.
pub const WINDOWS_PLUGIN_HOST: &str = "tiktools-plugin.app";

/// Strict plugin-page policy: no network, no subframes, no plugins. Page
/// scripts and styles load from the served assets only; audio previews may
/// stream from loopback (the plugin's local server); everything else the
/// page needs travels through the restricted broker IPC. Embedding is
/// limited to the TikTools host itself (packaged origin plus loopback dev
/// servers), so a foreign page cannot frame a plugin UI and impersonate
/// its host to harvest typed secrets.
pub const PLUGIN_CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'none'; ",
    "base-uri 'none'; ",
    "object-src 'none'; ",
    "frame-ancestors tiktools://app http://tiktools.app http://tiktools.localhost http://127.0.0.1:* http://localhost:*; ",
    "frame-src 'none'; ",
    "form-action 'none'; ",
    // NOTE: resource directives use explicit `tiktools-plugin:` scheme
    // sources instead of `'self'` on purpose. Inline plugin frames are
    // sandboxed without `allow-same-origin`, so their documents carry an
    // opaque origin, and strict CSP engines (WebKitGTK in the desktop
    // shell) never match `'self'` against an opaque origin. Scheme
    // sources match regardless of origin handling, so they behave
    // identically in every engine, framed or top-level.
    //
    // NOTE: every directive also lists the Windows WebView2 rewrite
    // (`http://tiktools-plugin.app`, see `WINDOWS_PLUGIN_HOST`). A
    // `tiktools-plugin:` scheme source never matches the rewritten
    // `http:` URLs, so without the explicit host source all subresources
    // would be blocked on Windows. Keep the two in sync.
    "script-src tiktools-plugin: http://tiktools-plugin.app; ",
    "style-src tiktools-plugin: http://tiktools-plugin.app 'unsafe-inline'; ",
    "img-src tiktools-plugin: http://tiktools-plugin.app data:; ",
    "font-src tiktools-plugin: http://tiktools-plugin.app data:; ",
    // Note: no `http://[::1]:*` — Chromium rejects an IPv6 loopback with a
    // wildcard port as an invalid CSP source and ignores it.
    "media-src tiktools-plugin: http://tiktools-plugin.app blob: http://localhost:* http://127.0.0.1:*; ",
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
    if !is_plugin_origin(&url) {
        return false;
    }
    url.path_segments()
        .and_then(|mut segments| segments.next())
        .is_some_and(|segment| segment == plugin_id)
}

/// Navigation policy for the main window's inline plugin frames: any
/// well-formed plugin asset URL. The shape check is deliberately
/// syntactic (installation is enforced per request by the asset server):
/// a top-level navigation to a non-installed plugin's URL renders an
/// error page, and a framed document without a host-bound broker is inert.
pub fn allows_plugin_navigation(raw_url: &str) -> bool {
    let Ok(url) = url::Url::parse(raw_url) else {
        return false;
    };
    if !is_plugin_origin(&url) {
        return false;
    }
    url.path_segments()
        .and_then(|mut segments| segments.next())
        .is_some_and(super::is_valid_ui_id)
}

fn is_plugin_origin(url: &url::Url) -> bool {
    let same_host = (url.scheme() == PLUGIN_ASSET_SCHEME
        && url.host_str() == Some(PLUGIN_ASSET_HOST))
        || (url.scheme() == "http" && url.host_str() == Some(WINDOWS_PLUGIN_HOST));
    same_host && url.username().is_empty() && url.password().is_none()
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
        let raw_path = request.uri().path().to_owned();
        let path = match plugin_asset_path(&self.plugin_id, &raw_path) {
            Ok(path) => path,
            Err((status, message)) => {
                return plugin_error_response(&self.plugin_id, &raw_path, status, message);
            }
        };
        serve_asset_file(&self.plugin_id, self.root.as_path(), &path, &raw_path)
    }
}

/// Serves every installed plugin's UI assets to the main window's inline
/// frames. The owner segment resolves per request through the core, so
/// only discovered webview-mode plugins with confined entries serve
/// anything; each response is still confined to that plugin's own entry
/// directory with the strict plugin CSP.
#[derive(Clone)]
pub struct SharedPluginAssetServer {
    core: Arc<AppCore>,
}

impl SharedPluginAssetServer {
    pub fn new(core: Arc<AppCore>) -> Self {
        Self { core }
    }

    pub fn respond(&self, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
        let raw_path = request.uri().path().to_owned();
        let mut segments = raw_path.split('/').filter(|segment| !segment.is_empty());
        let owner = segments.next().unwrap_or_default();
        if !super::is_valid_ui_id(owner) {
            return plugin_error_response(
                owner,
                &raw_path,
                StatusCode::BAD_REQUEST,
                "invalid plugin id".to_owned(),
            );
        }
        let assets = match self.core.plugin_ui_assets(owner) {
            Ok(assets) => assets,
            Err(error) if error.code() == "not_found" => {
                return plugin_error_response(
                    owner,
                    &raw_path,
                    StatusCode::NOT_FOUND,
                    "plugin UI not found".to_owned(),
                );
            }
            Err(_) => {
                return plugin_error_response(
                    owner,
                    &raw_path,
                    StatusCode::FORBIDDEN,
                    "plugin UI unavailable".to_owned(),
                );
            }
        };
        let rest: Vec<&str> = segments.collect();
        let path = match requested_path(&rest.join("/")) {
            Ok(path) => path,
            Err(message) => {
                return plugin_error_response(owner, &raw_path, StatusCode::BAD_REQUEST, message);
            }
        };
        serve_asset_file(owner, &assets.asset_root, &path, &raw_path)
    }
}

/// One response builder for every plugin asset response, success or
/// failure. Plugin documents load sandboxed without `allow-same-origin`,
/// so they carry an opaque (`null`) origin, and strict engines (observed
/// on WebKitGTK) CORS-check even same-scheme subresource loads from the
/// custom protocol. Without `Access-Control-Allow-Origin: *` the bundle's
/// external module scripts and stylesheets are rejected and the frame
/// stays blank — with a bare 200 status that hides the real cause.
/// Errors carry the same headers so a missing asset surfaces as its real
/// HTTP status instead of a misleading CORS failure.
fn plugin_response_builder(status: StatusCode) -> wry::http::response::Builder {
    Response::builder()
        .status(status)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header("content-security-policy", PLUGIN_CONTENT_SECURITY_POLICY)
}

/// Plugin-scoped error response: same CORS/CSP headers as successful
/// asset responses, plus a one-line diagnostic log. Only the plugin id,
/// request path, status, and reason are logged — never file contents,
/// settings, or credentials.
fn plugin_error_response(
    plugin_id: &str,
    raw_path: &str,
    status: StatusCode,
    message: String,
) -> Response<Cow<'static, [u8]>> {
    // Traversal and cross-plugin probes are worth surfacing; routine
    // missing-asset 404s stay at debug to avoid noisy logs.
    if status == StatusCode::FORBIDDEN {
        tracing::warn!(
            plugin = plugin_id,
            path = raw_path,
            %status,
            reason = %message,
            "rejected plugin asset request"
        );
    } else {
        tracing::debug!(
            plugin = plugin_id,
            path = raw_path,
            %status,
            reason = %message,
            "plugin asset request failed"
        );
    }
    plugin_response_builder(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Cow::Owned(message.into_bytes()))
        .expect("plugin error response builder should accept static headers")
}

/// Serves one validated relative path from a confined root: canonicalize,
/// containment check, strict plugin CSP. Shared by the per-window and
/// main-window servers, so both always emit identical security headers.
fn serve_asset_file(
    plugin_id: &str,
    root: &std::path::Path,
    path: &std::path::Path,
    raw_path: &str,
) -> Response<Cow<'static, [u8]>> {
    // Canonicalizing the root resolves symlinks in the plugin directory
    // itself, so the containment check below also catches symlink escapes
    // staged inside the served directory.
    let root = match fs::canonicalize(root) {
        Ok(root) => root,
        Err(error) => {
            return plugin_error_response(
                plugin_id,
                raw_path,
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            );
        }
    };
    let candidate = root.join(path);
    let canonical = match fs::canonicalize(&candidate) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return plugin_error_response(
                plugin_id,
                raw_path,
                StatusCode::NOT_FOUND,
                "asset not found".to_owned(),
            );
        }
        Err(error) => {
            return plugin_error_response(
                plugin_id,
                raw_path,
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            );
        }
    };
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return plugin_error_response(
            plugin_id,
            raw_path,
            StatusCode::FORBIDDEN,
            "asset path escapes plugin UI root".to_owned(),
        );
    }
    let bytes = match fs::read(&canonical) {
        Ok(bytes) => bytes,
        Err(error) => {
            return plugin_error_response(
                plugin_id,
                raw_path,
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
            );
        }
    };
    plugin_response_builder(StatusCode::OK)
        .header(CONTENT_TYPE, content_type(&canonical))
        .body(Cow::Owned(bytes))
        .expect("asset response builder should accept static headers")
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

    #[test]
    fn plugin_csp_uses_scheme_sources_not_self() {
        // Regression test: inline plugin frames are sandboxed, so their
        // documents carry an opaque origin and strict CSP engines never
        // match `'self'` against it (all subresources 403 in the shell).
        // Resource directives must spell the plugin scheme explicitly.
        for directive in [
            "script-src",
            "style-src",
            "img-src",
            "font-src",
            "media-src",
        ] {
            let body = PLUGIN_CONTENT_SECURITY_POLICY
                .split("; ")
                .find_map(|part| part.strip_prefix(directive))
                .unwrap_or_else(|| panic!("{directive} missing from plugin CSP"));
            assert!(
                body.contains("tiktools-plugin:"),
                "{directive} must allow the plugin scheme"
            );
            assert!(
                !body.contains("'self'"),
                "{directive} must not rely on 'self' (opaque origin in frames)"
            );
        }
    }

    #[test]
    fn plugin_csp_covers_the_windows_webview2_rewrite() {
        // On Windows the custom protocol reaches the WebView as
        // `http://tiktools-plugin.app/...`, which a `tiktools-plugin:`
        // scheme source never matches. Every resource directive must list
        // the rewritten host explicitly, and the framed document must
        // accept the rewritten main-app origins as embedders.
        let rewritten = format!("http://{WINDOWS_PLUGIN_HOST}");
        for directive in [
            "script-src",
            "style-src",
            "img-src",
            "font-src",
            "media-src",
        ] {
            let body = PLUGIN_CONTENT_SECURITY_POLICY
                .split("; ")
                .find_map(|part| part.strip_prefix(directive))
                .unwrap_or_else(|| panic!("{directive} missing from plugin CSP"));
            assert!(
                body.contains(rewritten.as_str()),
                "{directive} must allow the Windows rewrite ({rewritten})"
            );
        }
        let ancestors = PLUGIN_CONTENT_SECURITY_POLICY
            .split("; ")
            .find_map(|part| part.strip_prefix("frame-ancestors"))
            .expect("frame-ancestors missing from plugin CSP");
        for host in [
            "tiktools://app",
            "http://tiktools.app",
            "http://tiktools.localhost",
        ] {
            assert!(
                ancestors.contains(host),
                "frame-ancestors must allow the host origin {host}"
            );
        }
        // The lockdown stays: no network, no subframes, no plugins.
        for pinned in [
            "default-src 'none'",
            "base-uri 'none'",
            "object-src 'none'",
            "frame-src 'none'",
            "form-action 'none'",
            "connect-src 'none'",
        ] {
            assert!(
                PLUGIN_CONTENT_SECURITY_POLICY.contains(pinned),
                "plugin CSP must keep `{pinned}`"
            );
        }
    }

    fn cors_header<'a>(response: &'a Response<Cow<'static, [u8]>>) -> Option<&'a str> {
        response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok())
    }

    fn csp_header<'a>(response: &'a Response<Cow<'static, [u8]>>) -> Option<&'a str> {
        response
            .headers()
            .get("content-security-policy")
            .and_then(|value| value.to_str().ok())
    }

    fn asset_root(tag: &str) -> std::path::PathBuf {
        let root = env::temp_dir().join(format!(
            "tiktools-plugin-cors-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("index.html"), "<html></html>").unwrap();
        fs::write(root.join("assets").join("app.js"), "export {};").unwrap();
        fs::write(root.join("assets").join("app.css"), "body {}").unwrap();
        root
    }

    fn get_one(server: &PluginAssetServer, uri: &str) -> Response<Cow<'static, [u8]>> {
        server.respond(Request::builder().uri(uri).body(Vec::new()).unwrap())
    }

    #[test]
    fn plugin_assets_are_readable_from_the_opaque_origin() {
        // Regression test for the blank-iframe bug: the sandboxed plugin
        // document carries an opaque (`null`) origin, and strict engines
        // (WebKitGTK) CORS-check even custom-protocol subresource loads.
        // Every served asset must answer `Access-Control-Allow-Origin: *`
        // with its correct content type and the strict plugin CSP.
        let root = asset_root("ok");
        let server = PluginAssetServer::new("owner".to_owned(), Arc::new(root.clone()));
        for (uri, content_type) in [
            ("/owner/index.html", "text/html; charset=utf-8"),
            ("/owner/assets/app.js", "text/javascript; charset=utf-8"),
            ("/owner/assets/app.css", "text/css; charset=utf-8"),
        ] {
            let response = get_one(&server, uri);
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            assert_eq!(cors_header(&response), Some("*"), "{uri}");
            assert_eq!(
                csp_header(&response),
                Some(PLUGIN_CONTENT_SECURITY_POLICY),
                "{uri}"
            );
            assert_eq!(
                response
                    .headers()
                    .get("content-type")
                    .and_then(|value| value.to_str().ok()),
                Some(content_type),
                "{uri}"
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_errors_keep_cors_so_statuses_stay_visible() {
        // A missing or rejected asset must surface as its real HTTP
        // status, not hide behind a CORS failure in the frame console.
        let root = asset_root("errors");
        let server = PluginAssetServer::new("owner".to_owned(), Arc::new(root.clone()));
        // Missing file -> 404 with CORS.
        let response = get_one(&server, "/owner/assets/missing.js");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(cors_header(&response), Some("*"));
        assert_eq!(csp_header(&response), Some(PLUGIN_CONTENT_SECURITY_POLICY));
        // Cross-plugin read -> 403 with CORS, before any IO.
        let response = get_one(&server, "/other/index.html");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(cors_header(&response), Some("*"));
        // Traversal (plain, encoded, backslash) -> 400 with CORS.
        // (Raw backslashes never survive URI parsing, so the backslash
        // cases travel percent-encoded, as a real client would send them.)
        for uri in [
            "/owner/../plugin.json",
            "/owner/%2e%2e/plugin.json",
            "/owner/%2E%2E/plugin.json",
            "/owner/..%2fplugin.json",
            "/owner/..%5cplugin.json",
            "/owner/%2e%2e%2fplugin.json",
        ] {
            let response = get_one(&server, uri);
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{uri}");
            assert_eq!(cors_header(&response), Some("*"), "{uri}");
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn symlink_escapes_inside_the_asset_root_are_contained() {
        let root = asset_root("symlink");
        let outside = env::temp_dir().join(format!(
            "tiktools-plugin-cors-outside-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&outside, "secret").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("assets").join("sneaky.js")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&outside, root.join("assets").join("sneaky.js"))
            .unwrap();
        let server = PluginAssetServer::new("owner".to_owned(), Arc::new(root.clone()));
        let response = get_one(&server, "/owner/assets/sneaky.js");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(cors_header(&response), Some("*"));
        assert_ne!(response.body().as_ref(), b"secret");
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(outside);
    }

    #[test]
    fn shared_server_errors_keep_cors_headers() {
        let root = env::temp_dir().join(format!(
            "tiktools-shared-cors-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let server = SharedPluginAssetServer::new(webview_core(&root));
        // Success still carries CORS + CSP + content type.
        let response = get(&server, "/webui/index.html");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(cors_header(&response), Some("*"));
        assert_eq!(csp_header(&response), Some(PLUGIN_CONTENT_SECURITY_POLICY));
        // Unknown plugin, malformed id, and traversal all fail with CORS
        // so the frame console shows the real status.
        for (uri, status) in [
            ("/missing/index.html", StatusCode::NOT_FOUND),
            ("/../index.html", StatusCode::BAD_REQUEST),
            ("/webui/%2e%2e/plugin-secret.txt", StatusCode::BAD_REQUEST),
            ("/webui/..%5cplugin-secret.txt", StatusCode::BAD_REQUEST),
        ] {
            let response = get(&server, uri);
            assert_eq!(response.status(), status, "{uri}");
            assert_eq!(cors_header(&response), Some("*"), "{uri}");
            assert_eq!(
                csp_header(&response),
                Some(PLUGIN_CONTENT_SECURITY_POLICY),
                "{uri}"
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn main_window_navigation_accepts_any_well_formed_plugin_url() {
        assert!(allows_plugin_navigation(
            "tiktools-plugin://app/sonicboom.server/index.html#page=tts"
        ));
        assert!(allows_plugin_navigation(
            "http://tiktools-plugin.app/other.plugin/assets/app.js"
        ));
        // Id-shaped but uninstalled owners pass the syntactic nav check
        // and 404 at serve time (covered by the shared server test).
        assert!(allows_plugin_navigation("tiktools-plugin://app/index.html"));
        assert!(!allows_plugin_navigation("tiktools-plugin://app/"));
        // Percent-encoded traversal stays inside the owner segment, which
        // is never a valid id.
        assert!(!allows_plugin_navigation(
            "tiktools-plugin://app/..%2Fwebui/index.html"
        ));
        assert!(!allows_plugin_navigation("tiktools://app/index.html"));
        assert!(!allows_plugin_navigation("not a url"));
    }

    struct NullEmitter;
    impl tiktools_core::HostEmitter for NullEmitter {
        fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
    }

    fn webview_core(root: &std::path::Path) -> Arc<AppCore> {
        use tiktools_plugin_loader::{PluginManager, PluginRoot, PluginSource};
        let dir = root.join("webui");
        fs::create_dir_all(dir.join("ui/dist")).unwrap();
        fs::write(
            dir.join("plugin.json"),
            r#"{"schemaVersion": 3, "id": "webui", "name": "webui", "version": "1.0.0", "runtime": "declarative", "capabilities": [], "permissions": [], "actionTypes": [], "ui": {"apiVersion": 1, "mode": "webview", "entry": "ui/dist/index.html", "pages": [{"id": "main", "title": {"default": "Main"}}]}}"#,
        )
        .unwrap();
        fs::write(dir.join("ui/dist/index.html"), "<html></html>").unwrap();
        // A decoy outside the entry directory that must never serve.
        fs::write(dir.join("plugin-secret.txt"), "secret").unwrap();
        let manager = PluginManager::new(vec![PluginRoot {
            path: root.to_path_buf(),
            source: PluginSource::Development,
        }]);
        manager.scan().expect("test scan");
        let mut core = AppCore::new(Arc::new(NullEmitter));
        core.plugins = Arc::new(manager);
        Arc::new(core)
    }

    fn get(server: &SharedPluginAssetServer, uri: &str) -> wry::http::Response<Cow<'static, [u8]>> {
        server.respond(Request::builder().uri(uri).body(Vec::new()).unwrap())
    }

    #[test]
    fn shared_server_confines_each_plugin_to_its_entry_dir() {
        let root = env::temp_dir().join(format!(
            "tiktools-shared-assets-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let server = SharedPluginAssetServer::new(webview_core(&root));
        // Installed webview plugin serves with the strict CSP.
        let response = get(&server, "/webui/index.html");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-security-policy")
                .and_then(|value| value.to_str().ok()),
            Some(PLUGIN_CONTENT_SECURITY_POLICY)
        );
        // Unknown plugins 404; malformed ids 400 — before any IO.
        assert_eq!(
            get(&server, "/missing/index.html").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            get(&server, "/../index.html").status(),
            StatusCode::BAD_REQUEST
        );
        // The entry's siblings are unreachable: traversal is rejected and
        // the manifest-adjacent decoy is outside the served root.
        assert_eq!(
            get(&server, "/webui/%2e%2e/plugin-secret.txt").status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            get(&server, "/webui/../../plugin.json").status(),
            StatusCode::BAD_REQUEST
        );
        let _ = fs::remove_dir_all(root);
    }
}
