//! Plugin-specific action orchestration kept outside the general automation engine.

mod actions;
mod activation;
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
pub(crate) use descriptors::{manifest_action_declares_field, PluginActionDescriptor};
