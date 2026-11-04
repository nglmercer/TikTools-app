//! Official Cloud registry support.
//!
//! A Cloud origin (a RemakeBackend deployment) publishes immutable snapshot
//! generations instead of a single live registry tree. The active generation
//! is discovered through `GET {origin}/api/v1/registry/current`, which is the
//! atomic pointer the v1 protocol lacks and is never cached. Signed registry
//! files then resolve from the immutable generation endpoint
//! `{origin}/api/v1/registry/generations/{n}/files`, which is safe to cache
//! forever: clients that pin the generation first can never observe a mixed
//! snapshot, and superseded generations stay readable.
//!
//! Trust never comes from the server: the root keys pinned on the
//! [`RegistrySource`](crate::RegistrySource) are provisioned independently
//! (compile-time environment pins in Desktop releases). `root.json` served by
//! the Cloud is verified like any other signed document and cannot introduce
//! new trust.

use tiktools_plugin_package::PublicKey;

use crate::{url_policy::checked_url, RegistryError, RegistrySource};

/// Name of the official Cloud registry source.
pub const OFFICIAL_REGISTRY_NAME: &str = "official";
/// Name of the disposable staging Cloud registry source. Staging keys are
/// never production trust anchors.
pub const STAGING_REGISTRY_NAME: &str = "staging";

const CURRENT_PATH: &str = "api/v1/registry/current";

/// Active generation reported by a Cloud origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloudGeneration {
    pub generation: u64,
}

/// Build the immutable file endpoint for one generation.
pub fn generation_files_url(origin: &str, generation: u64) -> Result<String, RegistryError> {
    let origin = normalize_origin(origin)?;
    Ok(format!(
        "{origin}/api/v1/registry/generations/{generation}/files"
    ))
}

/// Build an official-style Cloud registry source. The initial `url` is the
/// bare origin; it is replaced with the resolved generation endpoint on the
/// first refresh, so no network access happens here.
pub fn cloud_source(
    name: &str,
    origin: &str,
    root_keys: Vec<PublicKey>,
) -> Result<RegistrySource, RegistryError> {
    let origin = normalize_origin(origin)?;
    let mut source = RegistrySource::new(name, &origin)?;
    source.root_keys = root_keys;
    source.builtin = true;
    source.cloud_origin = Some(origin);
    Ok(source)
}

/// True when `url` is the origin itself or a generation endpoint under it, in
/// which case a restart must keep a previously resolved generation URL
/// instead of resetting it to the bare origin.
pub fn is_cloud_url(url: &str, origin: &str) -> bool {
    let origin = origin.trim_end_matches('/');
    url == origin || url.starts_with(&format!("{origin}/api/v1/registry/generations/"))
}

fn normalize_origin(origin: &str) -> Result<String, RegistryError> {
    let trimmed = origin.trim_end_matches('/').to_owned();
    checked_url(&trimmed)?;
    Ok(trimmed)
}

pub(crate) fn current_url(origin: &str) -> Result<String, RegistryError> {
    let origin = normalize_origin(origin)?;
    crate::join_url(&origin, CURRENT_PATH)
}

pub(crate) fn parse_generation(body: &[u8]) -> Result<CloudGeneration, RegistryError> {
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|error| RegistryError::document("registry/current", error))?;
    let generation = value
        .get("generation")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            RegistryError::document("registry/current", "missing `generation` number")
        })?;
    Ok(CloudGeneration { generation })
}
