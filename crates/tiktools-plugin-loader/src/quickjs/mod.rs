//! QuickJS JavaScript runtime for TypeScript-first plugins.
//!
//! Plugins are authored in TypeScript, compiled to JavaScript ahead of time,
//! and executed by QuickJS (`rquickjs`). There is no runtime transpiler and
//! no Node.js dependency: guest code runs in a fresh QuickJS context with
//! deny-by-default host capabilities.
//!
//! Threading: a QuickJS runtime is `!Send`, while [`PluginInstance`] must be
//! `Send`. Each plugin therefore runs on one dedicated owner thread that
//! serves the manager's call queue directly. The runtime never crosses
//! threads: only plain bytes move over the queue, and only the owner thread
//! runs guest code or drains the event loop.
//!
//! The owner sleeps with no timeout while idle: plugin commands and
//! host-event wakes (native callbacks, async completions, and finalizers
//! posted from other threads) are the only work sources, so an idle plugin
//! consumes zero recurring wakeups.
//!
//! Protocol: the host sends `PluginCall` JSON and the guest answers
//! `PluginCallResult` JSON through its `call(request, context)` export.
//!
//! Native addons (napi-rs `.node` binaries) are opt-in per plugin through
//! the manifest `nativeAddons` list, and only for explicitly trusted
//! manifests (see `native_addons_allowed`).

mod events;
mod host;
mod modules;
mod native;
#[cfg(node_api_host)]
mod napi;
mod runtime;
mod timers;

pub use runtime::{native_addons_allowed, QuickJsPluginRuntime};
pub(crate) use runtime::untrusted_native_addons;

/// Renders a QuickJS error as `Name: message` (plus the stack when the
/// thrown value is an `Error`).
pub(crate) fn describe_error(ctx: &rquickjs::Ctx<'_>, error: rquickjs::Error) -> String {
    if matches!(error, rquickjs::Error::Exception) {
        let caught = ctx.catch();
        if let Some(object) = caught.as_object() {
            let name = object.get::<_, Option<String>>("name").ok().flatten();
            let message = object.get::<_, Option<String>>("message").ok().flatten();
            if let Some(message) = message {
                return match name {
                    Some(name) => format!("{name}: {message}"),
                    None => message,
                };
            }
        }
        return caught
            .as_string()
            .and_then(|text| text.to_string().ok())
            .or_else(|| {
                ctx.json_stringify(caught.clone())
                    .ok()
                    .flatten()
                    .and_then(|text| text.to_string().ok())
            })
            .unwrap_or_else(|| "guest threw an exception".to_owned());
    }
    error.to_string()
}
