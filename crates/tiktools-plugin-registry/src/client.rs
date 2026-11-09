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

impl RegistryClient {
    pub fn new(
        cache_dir: impl Into<PathBuf>,
        config_path: impl Into<PathBuf>,
        builtin: Vec<RegistrySource>,
        trust: Arc<RwLock<TrustStore>>,
    ) -> Result<Self, RegistryError> {
        Self::new_with_policy(cache_dir, config_path, builtin, trust, false)
    }

    /// Product-mode client: only independently provisioned builtins may
    /// participate in trust or networking. Legacy sources stay on disk.
    pub fn new_fixed(
        cache_dir: impl Into<PathBuf>,
        config_path: impl Into<PathBuf>,
        builtin: Vec<RegistrySource>,
        trust: Arc<RwLock<TrustStore>>,
    ) -> Result<Self, RegistryError> {
        Self::new_with_policy(cache_dir, config_path, builtin, trust, true)
    }

    fn new_with_policy(
        cache_dir: impl Into<PathBuf>,
        config_path: impl Into<PathBuf>,
        builtin: Vec<RegistrySource>,
        trust: Arc<RwLock<TrustStore>>,
        fixed_sources: bool,
    ) -> Result<Self, RegistryError> {
        let redirects = if fixed_sources {
            reqwest::redirect::Policy::custom(|attempt| {
                let same_origin = attempt.previous().first().is_some_and(|first| {
                    first.origin() == attempt.url().origin()
                        && attempt.url().username().is_empty()
                        && attempt.url().password().is_none()
                });
                if same_origin && attempt.previous().len() < 5 {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            })
        } else {
            reqwest::redirect::Policy::limited(5)
        };
        let http = tiktools_plugin_sdk::http::client_builder()
            .map_err(|message| RegistryError::http("", message))?
            .timeout(REQUEST_TIMEOUT)
            .redirect(redirects)
            .user_agent(format!(
                "tiktools-plugin-registry/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|error| RegistryError::http("", error))?;
        Self::with_client_policy(http, cache_dir, config_path, builtin, trust, fixed_sources)
    }

    pub fn with_client(
        http: Client,
        cache_dir: impl Into<PathBuf>,
        config_path: impl Into<PathBuf>,
        builtin: Vec<RegistrySource>,
        trust: Arc<RwLock<TrustStore>>,
    ) -> Result<Self, RegistryError> {
        Self::with_client_policy(http, cache_dir, config_path, builtin, trust, false)
    }

    fn with_client_policy(
        http: Client,
        cache_dir: impl Into<PathBuf>,
        config_path: impl Into<PathBuf>,
        builtin: Vec<RegistrySource>,
        trust: Arc<RwLock<TrustStore>>,
        fixed_sources: bool,
    ) -> Result<Self, RegistryError> {
        let config_path = config_path.into();
        let mut config = RegistriesConfig::load(&config_path)?;
        config.merge_builtin(builtin.clone());
        let client = Self {
            http,
            cache_dir: cache_dir.into(),
            config_path,
            builtin,
            fixed_sources,
            operation: Arc::new(tokio::sync::Mutex::new(())),
            operation_identity: Arc::new(()),
            config: RwLock::new(config),
            trust,
            state: Mutex::new(BTreeMap::new()),
        };
        for source in client.config().registries {
            client.pin_roots(&source);
        }
        Ok(client)
    }

    pub fn trust(&self) -> Arc<RwLock<TrustStore>> {
        self.trust.clone()
    }

    /// Whether this client restricts discovery and trust to provisioned sources.
    pub fn enforces_fixed_sources(&self) -> bool {
        self.fixed_sources
    }

    pub fn config(&self) -> RegistriesConfig {
        let mut config = self
            .config
            .read()
            .map(|config| config.clone())
            .unwrap_or_default();
        if self.fixed_sources {
            let persisted = config.registries;
            config.registries = self
                .builtin
                .iter()
                .map(|builtin| {
                    let mut source = builtin.clone();
                    // Retain only a previously resolved generation from this
                    // exact configured origin. Never restore persisted trust,
                    // source controls, or other origins.
                    if let Some(origin) = source.cloud_origin.as_deref() {
                        if let Some(saved) =
                            persisted.iter().find(|entry| entry.name == source.name)
                        {
                            if saved.cloud_origin.as_deref() == Some(origin)
                                && fixed_generation_url(&saved.url, origin)
                            {
                                source.url.clone_from(&saved.url);
                            }
                        }
                    }
                    source
                })
                .collect();
        }
        config
    }

    pub fn defaults(&self) -> RegistryDefaults {
        self.config().defaults
    }

    /// Read product security settings without silently enabling defaults if
    /// the configuration lock was poisoned.
    pub fn try_defaults(&self) -> Result<RegistryDefaults, RegistryError> {
        self.config
            .read()
            .map(|config| config.defaults.clone())
            .map_err(|_| RegistryError::document("registries.json", "configuration lock poisoned"))
    }

    pub fn set_defaults(&self, defaults: RegistryDefaults) -> Result<(), RegistryError> {
        self.update_config(|config| {
            config.defaults = defaults;
            Ok(())
        })
    }

    pub fn add(&self, source: RegistrySource) -> Result<RegistryStatus, RegistryError> {
        if self.fixed_sources {
            return Err(RegistryError::FixedSources);
        }
        let name = source.name.clone();
        self.update_config(|config| {
            if config.get(&source.name).is_some() {
                return Err(RegistryError::DuplicateRegistry(source.name.clone()));
            }
            config.registries.push(source.clone());
            Ok(())
        })?;
        self.pin_roots(&source);
        self.status(&name)
    }

    pub fn remove(&self, name: &str) -> Result<(), RegistryError> {
        if self.fixed_sources {
            return Err(RegistryError::FixedSources);
        }
        if self.builtin.iter().any(|source| source.name == name) {
            return Err(RegistryError::InvalidName(name.to_owned()));
        }
        self.update_config(|config| {
            let before = config.registries.len();
            config.registries.retain(|source| source.name != name);
            if before == config.registries.len() {
                return Err(RegistryError::UnknownRegistry(name.to_owned()));
            }
            Ok(())
        })?;
        if let Ok(mut trust) = self.trust.write() {
            trust.remove_source(name);
        }
        if let Ok(mut state) = self.state.lock() {
            state.remove(name);
        }
        let _ = fs::remove_dir_all(self.cache_dir.join(name));
        Ok(())
    }

    pub fn set_enabled(&self, name: &str, enabled: bool) -> Result<RegistryStatus, RegistryError> {
        if self.fixed_sources {
            return Err(RegistryError::FixedSources);
        }
        self.update_config(|config| {
            let source = config
                .registries
                .iter_mut()
                .find(|source| source.name == name)
                .ok_or_else(|| RegistryError::UnknownRegistry(name.to_owned()))?;
            source.enabled = enabled;
            Ok(())
        })?;
        self.status(name)
    }

    pub async fn fetch_root(&self, url: &str) -> Result<RootDocument, RegistryError> {
        if self.fixed_sources {
            return Err(RegistryError::FixedSources);
        }
        let target = join_url(url, "root.json")?;
        let bytes = self
            .get(&target, None)
            .await?
            .ok_or_else(|| RegistryError::Status {
                url: target.clone(),
                status: 304,
            })?;
        let document: RootDocument = serde_json::from_slice(&bytes.0)
            .map_err(|error| RegistryError::document("root.json", error))?;
        if document.keys.is_empty() {
            return Err(RegistryError::document(
                "root.json",
                "no root keys are listed",
            ));
        }
        for key in &document.keys {
            key.raw()?;
        }
        Ok(document)
    }

    pub fn statuses(&self) -> Vec<RegistryStatus> {
        self.config()
            .registries
            .iter()
            .map(|source| self.describe(source))
            .collect()
    }

    pub fn status(&self, name: &str) -> Result<RegistryStatus, RegistryError> {
        let config = self.config();
        let source = config
            .get(name)
            .ok_or_else(|| RegistryError::UnknownRegistry(name.to_owned()))?;
        Ok(self.describe(source))
    }

    pub async fn refresh(&self, name: &str) -> Result<RegistryStatus, RegistryError> {
        let _operation = self.operation_guard().await;
        self.refresh_unlocked(name).await
    }

    async fn refresh_unlocked(&self, name: &str) -> Result<RegistryStatus, RegistryError> {
        let source = match self.ensure_cloud_generation(name).await {
            Ok(source) => source,
            Err(error) => {
                // Generation discovery failures are availability states the UI
                // surfaces, so they are recorded like any other refresh error.
                if let Ok(mut state) = self.state.lock() {
                    state.entry(name.to_owned()).or_default().error = Some(error.to_string());
                }
                return Err(error);
            }
        };
        let result = self.load_registry(&source).await;
        if let Ok(mut state) = self.state.lock() {
            let entry = state.entry(name.to_owned()).or_default();
            match &result {
                Ok(config) => {
                    entry.config = Some(config.clone());
                    entry.source_url = Some(source.url.clone());
                    entry.refreshed_at = Some(now());
                    entry.error = None;
                }
                Err(error) => {
                    entry.config = None;
                    entry.source_url = None;
                    entry.error = Some(error.to_string());
                }
            }
        }
        result.map(|_| self.describe(&source))
    }

    pub async fn refresh_all(&self) -> Vec<RegistryStatus> {
        let mut statuses = Vec::new();
        for source in self.config().ordered() {
            match self.refresh(&source.name).await {
                Ok(status) => statuses.push(status),
                Err(_) => statuses.push(self.describe(source)),
            }
        }
        statuses
    }

    pub fn is_revoked(&self, digest: &Sha256Digest) -> bool {
        self.try_is_revoked(digest).unwrap_or(true)
    }

    /// Fail closed when the signed registry state cannot be read.
    pub fn try_is_revoked(&self, digest: &Sha256Digest) -> Result<bool, RegistryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RegistryError::document("registry", "registry state is unavailable"))?;
        Ok({
            state.values().any(|entry| {
                entry.config.as_ref().is_some_and(|config| {
                    config
                        .revoked_digests
                        .iter()
                        .filter_map(|value| Sha256Digest::parse(value))
                        .any(|revoked| &revoked == digest)
                })
            })
        })
    }

    pub async fn package(
        &self,
        id: &str,
        registry: Option<&str>,
    ) -> Result<(RegistrySource, PackageDocument), RegistryError> {
        let _operation = self.operation_guard().await;
        self.package_unlocked(id, registry).await
    }

    /// Holds the fixed-source trust snapshot across download and archive
    /// verification. Product callers retain the returned guard until prepare
    /// finishes; generic clients receive no guard.
    pub async fn package_guarded(
        &self,
        id: &str,
        registry: Option<&str>,
    ) -> Result<
        (
            RegistrySource,
            PackageDocument,
            Option<Arc<RegistryTrustGuard>>,
        ),
        RegistryError,
    > {
        let guard = self.operation_guard().await;
        let (source, document) = self.package_unlocked(id, registry).await?;
        Ok((source, document, guard))
    }

    /// Recheck the active Cloud generation under the same local trust lease,
    /// after download/prepare and immediately before the product commits.
    pub async fn check_fixed_generation(
        &self,
        guard: &RegistryTrustGuard,
        registry: &str,
        reviewed_url: &str,
    ) -> Result<u64, RegistryError> {
        if !self.fixed_sources || !Arc::ptr_eq(&self.operation_identity, &guard.owner) {
            return Err(RegistryError::FixedSources);
        }
        let source = self.source(registry)?;
        let origin = source
            .cloud_origin
            .as_deref()
            .ok_or(RegistryError::FixedSources)?;
        let generation = self.cloud_generation(origin).await?;
        let current = crate::cloud::generation_files_url(origin, generation)?;
        if current != reviewed_url {
            return Err(RegistryError::document(
                "registry/current",
                "generation changed; review the package again",
            ));
        }
        Ok(generation)
    }

    async fn package_unlocked(
        &self,
        id: &str,
        registry: Option<&str>,
    ) -> Result<(RegistrySource, PackageDocument), RegistryError> {
        if !tiktools_plugin_api::manifest::is_valid_plugin_id(id) {
            return Err(RegistryError::document(id, "invalid plugin ID"));
        }
        let candidates: Vec<RegistrySource> = match registry {
            Some(name) => vec![self.source(name)?],
            None => self
                .config()
                .candidates_for(id)
                .into_iter()
                .cloned()
                .collect(),
        };
        let mut last_error = None;
        for source in candidates {
            match self.package_from(&source, id).await {
                Ok(result) => return Ok(result),
                Err(RegistryError::Status { status: 404, .. }) => continue,
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error.unwrap_or_else(|| RegistryError::NotFound(id.to_owned())))
    }

    pub async fn catalog_page(
        &self,
        registry: &str,
        page: u32,
    ) -> Result<CatalogPage, RegistryError> {
        let _operation = self.operation_guard().await;
        let source = self.source(registry)?;
        let (source, _) = self.ensure_loaded(&source).await?;
        let path = format!("catalog/page-{}.json", page.max(1));
        let bytes = self.document(&source, &path, KeyRole::Index).await?;
        serde_json::from_slice(&bytes).map_err(|error| RegistryError::document(&path, error))
    }

    pub async fn search(&self, query: &SearchQuery) -> SearchResults {
        let _operation = self.operation_guard().await;
        let words: Vec<String> = query
            .query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let sources: Vec<RegistrySource> = match &query.registry {
            Some(name) => self.source(name).into_iter().collect(),
            None => self.config().ordered().into_iter().cloned().collect(),
        };
        let mut seen = BTreeSet::new();
        let mut entries = Vec::new();
        let mut errors = Vec::new();
        for source in sources {
            match self.registry_entries(&source, query, &words).await {
                Ok(found) => {
                    for entry in found {
                        if seen.insert(entry.id.clone()) {
                            entries.push(SearchEntry {
                                registry: source.name.clone(),
                                entry,
                            });
                        }
                    }
                }
                Err(error) => errors.push(format!("{}: {error}", source.name)),
            }
        }
        let total = entries.len();
        let pages = total.div_ceil(SEARCH_PAGE_SIZE).max(1);
        let page = (query.page.max(1) as usize).min(pages);
        let entries = entries
            .into_iter()
            .skip((page - 1) * SEARCH_PAGE_SIZE)
            .take(SEARCH_PAGE_SIZE)
            .collect();
        SearchResults {
            entries,
            total: total as u32,
            page: page as u32,
            pages: pages as u32,
            errors,
        }
    }

    async fn registry_entries(
        &self,
        source: &RegistrySource,
        query: &SearchQuery,
        words: &[String],
    ) -> Result<Vec<CatalogEntry>, RegistryError> {
        let (source, config) = self.ensure_loaded(source).await?;
        if let Some(api) = config.api.as_deref().filter(|_| !self.fixed_sources) {
            let mut url = url::Url::parse(&join_url(api, "search")?)
                .map_err(|_| RegistryError::InsecureUrl(api.to_owned()))?;
            url.query_pairs_mut()
                .append_pair("q", &query.query)
                .append_pair("category", query.category.as_deref().unwrap_or_default());
            if let Some((bytes, _)) = self.get(url.as_str(), None).await? {
                let page: CatalogPage = serde_json::from_slice(&bytes)
                    .map_err(|error| RegistryError::document("search", error))?;
                return Ok(page.entries);
            }
        }
        let mut entries = Vec::new();
        let pages = config.catalog_pages.clamp(1, MAX_CATALOG_PAGES);
        for page in 1..=pages {
            let path = format!("catalog/page-{}.json", page.max(1));
            let bytes = self.document(&source, &path, KeyRole::Index).await?;
            let catalog: CatalogPage = serde_json::from_slice(&bytes)
                .map_err(|error| RegistryError::document(&path, error))?;
            entries.extend(
                catalog
                    .entries
                    .into_iter()
                    .filter(|entry| entry.matches(words, query.category.as_deref())),
            );
            if page >= catalog.pages {
                break;
            }
        }
        Ok(entries)
    }

    async fn package_from(
        &self,
        source: &RegistrySource,
        id: &str,
    ) -> Result<(RegistrySource, PackageDocument), RegistryError> {
        let (source, _) = self.ensure_loaded(source).await?;
        let path = shard_path(id);
        let bytes = self.document(&source, &path, KeyRole::Index).await?;
        let document: PackageDocument = serde_json::from_slice(&bytes)
            .map_err(|error| RegistryError::document(&path, error))?;
        document
            .validate(id)
            .map_err(|message| RegistryError::document(&path, message))?;
        Ok((source, document))
    }

    async fn ensure_loaded(
        &self,
        source: &RegistrySource,
    ) -> Result<(RegistrySource, RegistryConfigDocument), RegistryError> {
        // Cloud sources re-resolve the active generation before every
        // operation; the returned snapshot pins the whole operation to the
        // generation that was active when it started. Callers must use the
        // returned source (not the one they passed in) for all fetches.
        let owned;
        let source = if source.cloud_origin.is_some() {
            owned = self.ensure_cloud_generation(&source.name).await?;
            owned
        } else {
            source.clone()
        };
        let cached = self.state.lock().ok().and_then(|state| {
            state
                .get(&source.name)
                .filter(|entry| {
                    !self.fixed_sources || entry.source_url.as_deref() == Some(&source.url)
                })
                .and_then(|entry| entry.config.clone())
        });
        if let Some(config) =
            cached.filter(|config| !config.expires.is_some_and(|expires| now() > expires))
        {
            return Ok((source, config));
        }
        // Do not re-resolve `current` here: all metadata in this operation
        // must be loaded from the generation already selected above.
        let config = self.load_registry(&source).await?;
        if let Ok(mut state) = self.state.lock() {
            let entry = state.entry(source.name.clone()).or_default();
            entry.config = Some(config.clone());
            entry.source_url = Some(source.url.clone());
            entry.refreshed_at = Some(now());
            entry.error = None;
        }
        Ok((source, config))
    }

    async fn load_registry(
        &self,
        source: &RegistrySource,
    ) -> Result<RegistryConfigDocument, RegistryError> {
        if source.is_verified() {
            let keys_bytes = self.document(source, "keys.json", KeyRole::Root).await?;
            let keys = KeysDocument::parse(&keys_bytes)?;
            if keys.is_expired(now()) {
                return Err(RegistryError::Expired);
            }
            if let Ok(mut trust) = self.trust.write() {
                trust.remove_source(&source.name);
                for root in &source.root_keys {
                    trust.insert(
                        TrustedKey::new(root.clone(), KeyRole::Root),
                        source.name.clone(),
                    );
                }
                trust.extend_delegations(&keys, &source.name);
            }
        }
        let bytes = self.document(source, "config.json", KeyRole::Index).await?;
        let config: RegistryConfigDocument = serde_json::from_slice(&bytes)
            .map_err(|error| RegistryError::document("config.json", error))?;
        if config.protocol != REGISTRY_PROTOCOL_VERSION {
            return Err(RegistryError::UnsupportedProtocol(config.protocol));
        }
        if config.expires.is_some_and(|expires| now() > expires) {
            return Err(RegistryError::Expired);
        }
        Ok(config)
    }

    async fn document(
        &self,
        source: &RegistrySource,
        path: &str,
        role: KeyRole,
    ) -> Result<Vec<u8>, RegistryError> {
        let bytes = self.cached_get(source, path).await?;
        if source.is_verified() {
            let signature_path = format!("{path}.sig");
            let signature_bytes = self.cached_get(source, &signature_path).await?;
            let signature = DetachedSignature::parse(&signature_bytes).map_err(|error| {
                RegistryError::Signature {
                    path: path.to_owned(),
                    source: error,
                }
            })?;
            let trust = self
                .trust
                .read()
                .map_err(|_| RegistryError::document(path, "trust store is unavailable"))?;
            trust
                .verify_detached(&signature, &bytes, role, now())
                .map_err(|error| RegistryError::Signature {
                    path: path.to_owned(),
                    source: error,
                })?;
        }
        Ok(bytes)
    }

    async fn cached_get(
        &self,
        source: &RegistrySource,
        path: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        let url = join_url(&source.url, path)?;
        let local = if self.fixed_sources {
            self.cache_dir
                .join(&source.name)
                .join(Sha256Digest::of_bytes(source.url.as_bytes()).as_hex())
                .join(path)
        } else {
            self.cache_dir.join(&source.name).join(path)
        };
        let etag_path = local.with_extension(format!(
            "{}.etag",
            local
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
        ));
        let etag = fs::read_to_string(&etag_path)
            .ok()
            .filter(|_| local.is_file());
        match self.get(&url, etag.as_deref()).await {
            Ok(Some((bytes, etag))) => {
                write_cache(&local, &bytes)?;
                match etag {
                    Some(etag) => fs::write(&etag_path, etag)?,
                    None => {
                        let _ = fs::remove_file(&etag_path);
                    }
                }
                Ok(bytes)
            }
            Ok(None) => Ok(fs::read(&local)?),
            Err(error @ RegistryError::Status { status: 404, .. }) => Err(error),
            Err(error) if self.fixed_sources => Err(error),
            Err(error) => match fs::read(&local) {
                Ok(bytes) => {
                    tracing::warn!(registry = %source.name, %path, %error, "using cached registry document");
                    Ok(bytes)
                }
                Err(_) => Err(error),
            },
        }
    }

    async fn get(
        &self,
        url: &str,
        etag: Option<&str>,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, RegistryError> {
        let mut request = self.http.get(url);
        if let Some(etag) = etag {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        let mut response = request
            .send()
            .await
            .map_err(|error| RegistryError::http(url, error))?;
        let status = response.status();
        if status == StatusCode::NOT_MODIFIED && etag.is_some() {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(RegistryError::Status {
                url: url.to_owned(),
                status: status.as_u16(),
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_DOCUMENT_BYTES)
        {
            return Err(RegistryError::TooLarge(MAX_DOCUMENT_BYTES));
        }
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| RegistryError::http(url, error))?
        {
            bytes.extend_from_slice(&chunk);
            if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
                return Err(RegistryError::TooLarge(MAX_DOCUMENT_BYTES));
            }
        }
        Ok(Some((bytes, etag)))
    }

    fn source(&self, name: &str) -> Result<RegistrySource, RegistryError> {
        self.config()
            .get(name)
            .cloned()
            .ok_or_else(|| RegistryError::UnknownRegistry(name.to_owned()))
    }

    async fn operation_guard(&self) -> Option<Arc<RegistryTrustGuard>> {
        if self.fixed_sources {
            Some(Arc::new(RegistryTrustGuard {
                owner: Arc::clone(&self.operation_identity),
                _guard: self.operation.clone().lock_owned().await,
            }))
        } else {
            None
        }
    }

    /// Fetch the active generation advertised by a Cloud origin. A 404 means
    /// no snapshot has been published yet, which is an availability state the
    /// UI surfaces, not a transport failure.
    async fn cloud_generation(&self, origin: &str) -> Result<u64, RegistryError> {
        let url = crate::cloud::current_url(origin)?;
        match self.get(&url, None).await {
            Ok(Some((bytes, _))) => {
                let generation = crate::cloud::parse_generation(&bytes)?.generation;
                if generation == 0 {
                    return Err(RegistryError::document(
                        "registry/current",
                        "invalid generation",
                    ));
                }
                Ok(generation)
            }
            Ok(None) => Err(RegistryError::http(&url, "empty response")),
            Err(RegistryError::Status { status: 404, .. }) => {
                Err(RegistryError::NoPublishedGeneration {
                    origin: origin.to_owned(),
                })
            }
            Err(error) => Err(error),
        }
    }

    /// Resolve the active generation for a Cloud source and pin the stored
    /// URL to its immutable endpoint. The returned snapshot is used for the
    /// whole operation, so a generation change mid-operation cannot mix
    /// snapshots. When the origin is unreachable but a generation was
    /// resolved before, fall back to that URL so cached signed metadata keeps
    /// working offline, mirroring the document cache fallback below.
    async fn ensure_cloud_generation(&self, name: &str) -> Result<RegistrySource, RegistryError> {
        let stored = self.source(name)?;
        let Some(origin) = stored.cloud_origin.clone() else {
            return Ok(stored);
        };
        let generation = match self.cloud_generation(&origin).await {
            Ok(generation) => generation,
            // An authoritative "nothing published" must fail closed: silently
            // serving a stale generation would hide a rollback or takedown.
            Err(RegistryError::NoPublishedGeneration { .. }) => {
                return Err(RegistryError::NoPublishedGeneration { origin });
            }
            Err(error) if !self.fixed_sources && stored.url != origin => {
                tracing::warn!(registry = %name, %error, "cloud generation lookup failed; using last resolved generation");
                return Ok(stored);
            }
            Err(error) => return Err(error),
        };
        let url = crate::cloud::generation_files_url(&origin, generation)?;
        if url == stored.url {
            return Ok(stored);
        }
        self.update_config(|config| {
            let entry = config
                .registries
                .iter_mut()
                .find(|entry| entry.name == name)
                .ok_or_else(|| RegistryError::UnknownRegistry(name.to_owned()))?;
            entry.url = url.clone();
            Ok(())
        })?;
        self.source(name)
    }

    fn describe(&self, source: &RegistrySource) -> RegistryStatus {
        let state = self
            .state
            .lock()
            .ok()
            .and_then(|state| state.get(&source.name).cloned())
            .unwrap_or_default();
        RegistryStatus {
            name: source.name.clone(),
            url: source.url.clone(),
            enabled: source.enabled,
            priority: source.priority,
            scopes: source.scopes.clone(),
            builtin: source.builtin,
            verified: source.is_verified(),
            fingerprints: source
                .root_keys
                .iter()
                .filter_map(|key| key.fingerprint().ok())
                .map(|digest| digest.to_string())
                .collect(),
            description: state.config.and_then(|config| config.description),
            refreshed_at: state.refreshed_at,
            error: state.error,
        }
    }

    fn pin_roots(&self, source: &RegistrySource) {
        if let Ok(mut trust) = self.trust.write() {
            for root in &source.root_keys {
                trust.insert(
                    TrustedKey::new(root.clone(), KeyRole::Root),
                    source.name.clone(),
                );
            }
        }
    }

    fn update_config(
        &self,
        change: impl FnOnce(&mut RegistriesConfig) -> Result<(), RegistryError>,
    ) -> Result<(), RegistryError> {
        let mut config = self.config.write().map_err(|_| {
            RegistryError::document("registries.json", "configuration lock poisoned")
        })?;
        let mut updated = config.clone();
        change(&mut updated)?;
        updated.save(&self.config_path)?;
        *config = updated;
        Ok(())
    }
}

fn fixed_generation_url(value: &str, origin: &str) -> bool {
    if value == origin {
        return true;
    }
    let prefix = format!("{origin}/api/v1/registry/generations/");
    value
        .strip_prefix(&prefix)
        .and_then(|rest| rest.strip_suffix("/files"))
        .and_then(|number| number.parse::<u64>().ok())
        .filter(|generation| *generation > 0)
        .and_then(|generation| crate::cloud::generation_files_url(origin, generation).ok())
        .is_some_and(|expected| expected == value)
}

fn write_cache(path: &Path, bytes: &[u8]) -> Result<(), RegistryError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod defaults_poison_tests {
    use super::*;

    #[test]
    fn poisoned_configuration_never_yields_permissive_security_defaults() {
        let root =
            std::env::temp_dir().join(format!("registry-poison-{}-{}", std::process::id(), now()));
        let client = RegistryClient::new_fixed(
            root.join("cache"),
            root.join("registries.json"),
            Vec::new(),
            Arc::new(RwLock::new(TrustStore::new())),
        )
        .unwrap();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut config = client.config.write().unwrap();
            config.defaults.allow_sideload = false;
            panic!("simulate interrupted settings mutation");
        }))
        .is_err());
        assert!(client.try_defaults().is_err());
    }
}
