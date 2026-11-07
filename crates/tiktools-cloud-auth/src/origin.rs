/// Environment override for the Cloud origin (development and staging).
/// Set to an empty string, `off` or `none` to disable Cloud entirely.
pub const ENV_CLOUD_ORIGIN: &str = "TIKTOOLS_CLOUD_ORIGIN";

/// Official Cloud origin used when neither the runtime environment nor the
/// build configures one, so a fresh install can sign in without setup.
pub const DEFAULT_CLOUD_ORIGIN: &str = "https://remakebackend-production-a872.up.railway.app";

/// Resolve the Cloud origin. Precedence:
///
/// 1. Runtime `TIKTOOLS_CLOUD_ORIGIN` (development/staging override; an empty
///    value, `off` or `none` disables Cloud).
/// 2. The origin compiled in through `TIKTOOLS_CLOUD_ORIGIN` at build time.
/// 3. [`DEFAULT_CLOUD_ORIGIN`].
pub fn resolve_cloud_origin() -> Option<String> {
    resolve_origin_from(
        std::env::var(ENV_CLOUD_ORIGIN).ok().as_deref(),
        option_env!("TIKTOOLS_CLOUD_ORIGIN"),
    )
}

fn resolve_origin_from(runtime: Option<&str>, compiled: Option<&str>) -> Option<String> {
    fn configured(value: &str) -> Option<String> {
        let value = value.trim().trim_end_matches('/');
        match value.to_ascii_lowercase().as_str() {
            "" | "off" | "none" => None,
            _ => Some(value.to_owned()),
        }
    }
    if let Some(runtime) = runtime {
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
                Some("https://built.example")
            ),
            Some("https://staging.example".to_owned())
        );
        for off in ["", "off", "NONE"] {
            assert_eq!(
                resolve_origin_from(Some(off), Some("https://built.example")),
                None
            );
        }
    }

    #[test]
    fn compiled_origin_then_builtin_default() {
        assert_eq!(
            resolve_origin_from(None, Some("https://built.example")),
            Some("https://built.example".to_owned())
        );
        assert_eq!(
            resolve_origin_from(None, None),
            Some(DEFAULT_CLOUD_ORIGIN.to_owned())
        );
    }
}
