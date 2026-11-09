/// Build-time Cloud instance configuration. Debug builds additionally accept
/// this environment variable at runtime for isolated development and testing.
/// An empty string, `off` or `none` disables Cloud when the value is applicable.
pub const ENV_CLOUD_ORIGIN: &str = "TIKTOOLS_CLOUD_ORIGIN";

/// Official Cloud origin used when the operator did not configure the build.
pub const DEFAULT_CLOUD_ORIGIN: &str = "https://remakebackend-production-a872.up.railway.app";

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
    resolve_origin_from(
        std::env::var(ENV_CLOUD_ORIGIN).ok().as_deref(),
        option_env!("TIKTOOLS_CLOUD_ORIGIN"),
        cfg!(debug_assertions),
    )
}

fn resolve_origin_from(
    runtime: Option<&str>,
    compiled: Option<&str>,
    development: bool,
) -> Option<String> {
    fn configured(value: &str) -> Option<String> {
        let value = value.trim().trim_end_matches('/');
        match value.to_ascii_lowercase().as_str() {
            "" | "off" | "none" => None,
            _ => Some(value.to_owned()),
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
    use super::{resolve_origin_from, DEFAULT_CLOUD_ORIGIN};

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
}
