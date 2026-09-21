//! Origin policy and token authentication.

use super::config::GatewayConfig;
use std::collections::HashMap;
use url::form_urlencoded;

pub(crate) fn origin_allowed(origin: Option<&str>, config: &GatewayConfig) -> bool {
    origin.is_none_or(|origin| {
        config
            .allowed_origins
            .iter()
            .any(|allowed| allowed == origin)
    })
}

pub(crate) fn authorization_token(headers: &HashMap<String, String>) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
}

pub(crate) fn authorized_http(
    headers: &HashMap<String, String>,
    query: &str,
    config: &GatewayConfig,
) -> bool {
    authorization_token(headers).is_some_and(|token| token == config.token)
        || form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == "token")
            .is_some_and(|(_, token)| token == config.token)
}
