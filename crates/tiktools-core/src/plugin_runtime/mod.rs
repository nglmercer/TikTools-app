//! Plugin-specific action orchestration kept outside the general automation engine.

mod actions;
mod activation;
// Trusted binding groundwork remains closed to delegated execution until the
// invocation trust contract is integrated into manifest/private policy.
#[allow(dead_code)]
mod binding;
mod descriptors;
mod events;
mod lifecycle;
mod polling;
mod pushed;
#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "persistence"))]
pub(crate) use activation::activation_from_snapshot;
pub(crate) use activation::PluginActivation;
#[allow(unused_imports)]
pub(crate) use binding::TrustedPluginBinding;
pub(crate) use descriptors::{manifest_action_declares_field, PluginActionDescriptor};
