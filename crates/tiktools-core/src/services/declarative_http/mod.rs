//! Declarative HTTP integrations for schema v3 plugin manifests.
//!
//! A manifest `http` block declares one base URL plus shared auth for the
//! plugin's HTTP actions, option sources, and health probe. The host renders
//! `{{ event.* }}`, `{{ settings.* }}`, and `{{ config.* }}` templates,
//! enforces the loopback trust boundary, and sends through the same hardened
//! client as automation HTTP actions. Secrets stay host-side: settings
//! resolve from raw storage (never the redacted WebView payload), and tokens
//! are redacted from every surfaced string.

mod diagnostics;
mod endpoint;
mod execution;
mod template;
#[cfg(test)]
mod tests;

pub(crate) use diagnostics::{
    declarative_auth_label, declarative_request_line, redact_endpoint_secrets, with_auth_hint,
};
pub(crate) use endpoint::{build_declarative_request, DeclarativeBuild, DeclarativeEndpoint};
pub(crate) use template::render_scoped_template;

/// Permission a manifest must declare before a declarative fetch may leave
/// loopback. Loopback servers work without it.
pub const NETWORK_BIND_PERMISSION: &str = "network.bind";

/// Capability a manifest must declare to run declarative HTTP actions.
pub const HTTP_REQUEST_CAPABILITY: &str = "http.request";
