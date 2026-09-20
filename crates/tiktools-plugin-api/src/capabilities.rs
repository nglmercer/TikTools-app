//! Names, typed identifiers, and matching helpers for host capabilities.

use std::{fmt, ops::Deref};

use crate::manifest::PluginManifest;

/// A capability identifier that preserves unknown extension capabilities.
///
/// The wire format remains a string. This wrapper keeps string comparisons at
/// capability boundaries while allowing future plugins to declare their own
/// names without changing this crate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct CapabilityId(String);

impl CapabilityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for CapabilityId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CapabilityId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl Deref for CapabilityId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A runtime-neutral set of declared capabilities. Unknown names are kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilitySet(Vec<CapabilityId>);

impl CapabilitySet {
    pub fn new(values: impl IntoIterator<Item = CapabilityId>) -> Self {
        Self::from_strings(values.into_iter().map(|value| value.0))
    }

    pub fn from_strings(values: impl IntoIterator<Item = String>) -> Self {
        let mut values = values
            .into_iter()
            .map(CapabilityId::new)
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        Self(values)
    }

    pub fn contains(&self, requested: &str) -> bool {
        self.0
            .iter()
            .any(|declared| capability_matches(declared.as_str(), requested))
    }

    pub fn iter(&self) -> impl Iterator<Item = &CapabilityId> {
        self.0.iter()
    }
}

/// Permissions are deliberately a separate type from capabilities. A
/// capability names an API; a permission represents the authorization needed
/// for sensitive use of that API.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionSet(Vec<CapabilityId>);

impl PermissionSet {
    pub fn from_strings(values: impl IntoIterator<Item = String>) -> Self {
        let mut values = values
            .into_iter()
            .map(CapabilityId::new)
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        Self(values)
    }

    pub fn contains(&self, requested: &str) -> bool {
        self.0
            .iter()
            .any(|declared| capability_matches(declared.as_str(), requested))
    }

    pub fn iter(&self) -> impl Iterator<Item = &CapabilityId> {
        self.0.iter()
    }
}

pub const HTTP: &str = "http";
pub const AUDIO: &str = "audio";
pub const AUDIO_PLAY: &str = "audio.play";
pub const AUDIO_OUTPUT_PERMISSION: &str = "audio.output";
pub const MEDIA_PICK: &str = "media.pick";
pub const MEDIA_READ: &str = "media.read";
pub const TTS: &str = "tts";
pub const POINTS_READ: &str = "points.read";
pub const POINTS_WRITE: &str = "points.write";
pub const STORAGE: &str = "storage";
/// Lets a plugin publish its own declared event types (hotkeys, timers).
pub const EVENTS_PUBLISH: &str = "events.publish";
/// Lets a plugin receive serialized host `DomainEvent` envelopes.
pub const EVENTS_SUBSCRIBE: &str = "events.subscribe";
/// Lets a plugin enrich existing host events before automation filters run.
/// The host checks this before sending any `enrich` call; it grants no
/// unrelated capability (audio, points, storage, HTTP).
pub const EVENTS_ENRICH: &str = "events.enrich";
pub const APP_STATE: &str = "app.state";

/// Returns true when a manifest explicitly declares a capability or a
/// documented wildcard covering it. This gates host-owned protocol operations;
/// it does not restrict the normal OS permissions of a process executable.
/// Native plugins remain trusted code, while a future WASM adapter can enforce
/// the same declarations at its explicit host/WASI import boundary.
pub fn declares_capability(manifest: &PluginManifest, requested: &str) -> bool {
    manifest
        .capabilities
        .iter()
        .any(|declared| capability_matches(declared, requested))
}

pub fn declares_permission(manifest: &PluginManifest, requested: &str) -> bool {
    manifest
        .permissions
        .iter()
        .any(|declared| capability_matches(declared, requested))
}

pub fn capability_matches(declared: &str, requested: &str) -> bool {
    declared == "*"
        || declared == requested
        || (declared.ends_with(".*") && requested.starts_with(declared.trim_end_matches('*')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrich_capability_matches_exact_and_wildcard_declarations() {
        assert_eq!(EVENTS_ENRICH, "events.enrich");
        assert!(capability_matches("events.enrich", EVENTS_ENRICH));
        assert!(capability_matches("events.*", EVENTS_ENRICH));
        assert!(capability_matches("*", EVENTS_ENRICH));
        assert!(!capability_matches("events.publish", EVENTS_ENRICH));
        assert!(!capability_matches("audio.play", EVENTS_ENRICH));
    }
}
