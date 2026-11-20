/// Build-time Cloud instance configuration. Debug builds additionally accept
/// this environment variable at runtime for isolated development and testing.
/// An empty string, `off` or `none` disables Cloud when the value is applicable.
pub const ENV_CLOUD_ORIGIN: &str = "TIKTOOLS_CLOUD_ORIGIN";

/// Official Cloud origin used when the operator did not configure the build.
pub const DEFAULT_CLOUD_ORIGIN: &str = "https://remakebackend-production-a872.up.railway.app";

/// Canonical account/catalog/registry origin. Credentials, paths, queries,
/// fragments and insecure non-loopback HTTP are never Cloud instances.
pub fn normalize_cloud_origin(raw: &str) -> Option<String> {
    let url = url::Url::parse(raw).ok()?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

/// Resolve the operator-provisioned Cloud instance. Release builds ignore
/// runtime overrides, including attempts to disable or replace the instance.
/// Operators configure a different instance (or disable Cloud) at build time.
/// Debug builds use this precedence for isolated development:
///
/// 1. Runtime `TIKTOOLS_CLOUD_ORIGIN` (development/staging override; an empty
///    value, `off` or `none` disables Cloud).
/// 2. The origin compiled in through `TIKTOOLS_CLOUD_ORIGIN` at build time.
/// 3. [`DEFAULT_CLOUD_ORIGIN`].
pub fn resolve_cloud_origin() -> Option<String> {
    if let Some(origin) = e2e_override() {
        return resolve_e2e_origin(&origin);
    }
    resolve_origin_from(
        std::env::var(ENV_CLOUD_ORIGIN).ok().as_deref(),
        option_env!("TIKTOOLS_CLOUD_ORIGIN"),
        cfg!(debug_assertions),
    )
}

/// Reads `TIKTOOLS_E2E_CLOUD_ORIGIN`, which only builds with the
/// `e2e-cloud-origin` feature honor. It points a release-profile test build at
/// a local RemakeBackend.
#[cfg(feature = "e2e-cloud-origin")]
fn e2e_override() -> Option<String> {
    const ENV_E2E_CLOUD_ORIGIN: &str = "TIKTOOLS_E2E_CLOUD_ORIGIN";
    std::env::var(ENV_E2E_CLOUD_ORIGIN)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[cfg(not(feature = "e2e-cloud-origin"))]
fn e2e_override() -> Option<String> {
    None
}

/// Accepts only a loopback IP origin, over HTTP or HTTPS. Anything else,
/// including `localhost`, names or private addresses, fails closed to `None`
/// so an invalid test setting disables Cloud rather than falling back to the
/// production instance.
pub(crate) fn resolve_e2e_origin(value: &str) -> Option<String> {
    let origin = normalize_cloud_origin(value.trim())?;
    let host = url::Url::parse(&origin).ok()?.host_str()?.to_owned();
    matches!(host.as_str(), "127.0.0.1" | "[::1]").then_some(origin)
}

fn resolve_origin_from(
    runtime: Option<&str>,
    compiled: Option<&str>,
    development: bool,
) -> Option<String> {
    fn configured(value: &str) -> Option<String> {
        let value = value.trim();
        match value.to_ascii_lowercase().as_str() {
            "" | "off" | "none" => None,
            _ => normalize_cloud_origin(value),
        }
    }
    if let Some(runtime) = runtime.filter(|_| development) {
        return configured(runtime);
    }
    match compiled {
        Some(compiled) => configured(compiled),
        None => configured(DEFAULT_CLOUD_ORIGIN),
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_cloud_origin, resolve_origin_from, DEFAULT_CLOUD_ORIGIN};

    #[test]
    fn rejects_non_origin_cloud_urls() {
        for bad in [
            "https://user:pass@example.test",
            "https://example.test/path",
            "https://example.test///",
            "https://example.test/?q=1",
            "https://example.test/#fragment",
            "http://example.test",
        ] {
            assert!(normalize_cloud_origin(bad).is_none(), "accepted {bad}");
            assert!(resolve_origin_from(Some(bad), None, true).is_none());
        }
        assert_eq!(
            normalize_cloud_origin("https://EXAMPLE.test:443/"),
            Some("https://example.test".into())
        );
        assert_eq!(
            normalize_cloud_origin("http://127.0.0.1:8080"),
            Some("http://127.0.0.1:8080".into())
        );
    }

    #[test]
    fn runtime_override_wins_and_can_disable() {
        assert_eq!(
            resolve_origin_from(
                Some(" https://staging.example/ "),
                Some("https://built.example"),
                true,
            ),
            Some("https://staging.example".to_owned())
        );
        for off in ["", "off", "NONE"] {
            assert_eq!(
                resolve_origin_from(Some(off), Some("https://built.example"), true),
                None
            );
        }
    }

    #[test]
    fn compiled_origin_then_builtin_default() {
        assert_eq!(
            resolve_origin_from(None, Some("https://built.example"), false),
            Some("https://built.example".to_owned())
        );
        assert_eq!(
            resolve_origin_from(None, None, false),
            Some(DEFAULT_CLOUD_ORIGIN.to_owned())
        );
    }

    #[test]
    fn release_instance_cannot_be_replaced_or_disabled_at_runtime() {
        for runtime in [
            "https://untrusted.example",
            "http://127.0.0.1:8000",
            "",
            "off",
            "none",
        ] {
            assert_eq!(
                resolve_origin_from(Some(runtime), Some("https://built.example/"), false),
                Some("https://built.example".to_owned()),
            );
            assert_eq!(
                resolve_origin_from(Some(runtime), None, false),
                Some(DEFAULT_CLOUD_ORIGIN.to_owned()),
            );
        }
    }

    #[test]
    fn operator_can_disable_a_release_without_runtime_reactivation() {
        for disabled in ["", "off", "NONE"] {
            assert_eq!(
                resolve_origin_from(Some("https://untrusted.example"), Some(disabled), false),
                None,
            );
        }
    }

    #[test]
    fn e2e_origin_accepts_only_loopback_ip_origins() {
        use super::resolve_e2e_origin;
        assert_eq!(
            resolve_e2e_origin("http://127.0.0.1:8787/").as_deref(),
            Some("http://127.0.0.1:8787"),
        );
        assert_eq!(
            resolve_e2e_origin("https://[::1]:8443").as_deref(),
            Some("https://[::1]:8443"),
        );
        for rejected in [
            "https://remakebackend-production-a872.up.railway.app",
            "http://192.168.1.20:8787",
            "http://localhost:8787",
            "https://localhost:8443",
            "http://127.0.0.1:8787/api",
            "http://user@127.0.0.1:8787",
            "",
            "off",
        ] {
            assert_eq!(resolve_e2e_origin(rejected), None, "accepted {rejected}");
        }
    }
}
