use super::*;

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
}
