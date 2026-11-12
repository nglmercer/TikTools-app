mod authority;
mod bootstrap;
mod catalog;
mod configuration;
mod generations;
mod metadata;
mod network;
mod packages;
mod status;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::{header, Client, StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_plugin_package::{
    DetachedSignature, KeyRole, KeysDocument, Sha256Digest, TrustStore, TrustedKey,
};

use crate::{
    join_url, model::shard_path, CatalogEntry, CatalogPage, PackageDocument, RegistriesConfig,
    RegistryConfigDocument, RegistryDefaults, RegistryError, RegistrySource, RootDocument,
    REGISTRY_PROTOCOL_VERSION,
};

const MAX_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CATALOG_PAGES: u32 = 64;
const SEARCH_PAGE_SIZE: usize = 24;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    #[serde(default)]
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    #[serde(default)]
    pub page: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchEntry {
    pub registry: String,
    #[serde(flatten)]
    pub entry: CatalogEntry,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub entries: Vec<SearchEntry>,
    pub total: u32,
    pub page: u32,
    pub pages: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegistryStatus {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub priority: i32,
    pub scopes: Vec<String>,
    pub builtin: bool,
    pub verified: bool,
    pub fingerprints: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refreshed_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct RegistryState {
    config: Option<RegistryConfigDocument>,
    source_url: Option<String>,
    refreshed_at: Option<u64>,
    error: Option<String>,
}

pub struct RegistryClient {
    http: Client,
    cache_dir: PathBuf,
    config_path: PathBuf,
    builtin: Vec<RegistrySource>,
    fixed_sources: bool,
    operation: Arc<tokio::sync::Mutex<()>>,
    operation_identity: Arc<()>,
    config: RwLock<RegistriesConfig>,
    trust: Arc<RwLock<TrustStore>>,
    state: Mutex<BTreeMap<String, RegistryState>>,
}

/// Non-forgeable fixed-client lease for one verified generation snapshot.
#[derive(Debug)]
pub struct RegistryTrustGuard {
    owner: Arc<()>,
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod defaults_poison_tests;
