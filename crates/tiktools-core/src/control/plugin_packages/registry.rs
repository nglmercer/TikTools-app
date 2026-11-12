use super::*;

impl AppCore {
    pub fn registry_list(&self) -> Result<Vec<RegistryStatus>, OperationError> {
        Ok(self.packages()?.registry().statuses())
    }

    pub async fn registry_root(&self, url: &str) -> Result<RegistryRootPreview, OperationError> {
        let packages = self.packages()?;
        let root = packages
            .registry()
            .fetch_root(url)
            .await
            .map_err(registry_error)?;
        let keys = root
            .keys
            .iter()
            .map(|key| {
                Ok(RegistryRootKey {
                    key_id: key.key_id.to_string(),
                    fingerprint: key
                        .fingerprint()
                        .map_err(registry_package_error)?
                        .to_string(),
                })
            })
            .collect::<Result<_, OperationError>>()?;
        Ok(RegistryRootPreview {
            url: url.to_owned(),
            keys,
        })
    }

    pub async fn registry_add(
        &self,
        input: RegistryAddInput,
    ) -> Result<RegistryStatus, OperationError> {
        let packages = self.packages()?;
        let mut source = RegistrySource::new(&input.name, &input.url).map_err(registry_error)?;
        source.scopes = input.scopes;
        source.priority = input.priority.clamp(-100, 99);
        if !input.fingerprints.is_empty() {
            let root = packages
                .registry()
                .fetch_root(&input.url)
                .await
                .map_err(registry_error)?;
            let mut pinned = Vec::new();
            for key in root.keys {
                let fingerprint = key.fingerprint().map_err(registry_package_error)?;
                if input
                    .fingerprints
                    .iter()
                    .any(|expected| expected.eq_ignore_ascii_case(fingerprint.as_hex()))
                {
                    pinned.push(key);
                }
            }
            if pinned.len() != input.fingerprints.len() {
                return Err(OperationError::conflict(
                    "the registry root keys no longer match the confirmed fingerprints",
                ));
            }
            source.root_keys = pinned;
        }
        let name = source.name.clone();
        packages.registry().add(source).map_err(registry_error)?;
        match packages.registry().refresh(&name).await {
            Ok(status) => Ok(status),
            Err(error) => {
                tracing::warn!(registry = %name, %error, "new registry could not be refreshed");
                packages.registry().status(&name).map_err(registry_error)
            }
        }
    }

    pub fn registry_remove(&self, name: &str) -> Result<(), OperationError> {
        self.packages()?
            .registry()
            .remove(name)
            .map_err(registry_error)
    }

    pub fn registry_set_enabled(
        &self,
        name: &str,
        enabled: bool,
    ) -> Result<RegistryStatus, OperationError> {
        self.packages()?
            .registry()
            .set_enabled(name, enabled)
            .map_err(registry_error)
    }

    pub async fn registry_refresh(
        &self,
        name: Option<&str>,
    ) -> Result<Vec<RegistryStatus>, OperationError> {
        if name.is_some_and(|name| name != OFFICIAL_REGISTRY_NAME) {
            return Err(OperationError::invalid(
                "plugin sources are managed by the application",
            ));
        }
        let packages = self.packages()?;
        match name {
            Some(name) => Ok(vec![packages
                .registry()
                .refresh(name)
                .await
                .map_err(registry_error)?]),
            None => Ok(packages.registry().refresh_all().await),
        }
    }

    pub async fn registry_search(
        &self,
        query: SearchQuery,
    ) -> Result<SearchResults, OperationError> {
        if query.query.len() > 256 {
            return Err(OperationError::invalid(
                "search queries are at most 256 characters",
            ));
        }
        if query
            .registry
            .as_deref()
            .is_some_and(|name| name != OFFICIAL_REGISTRY_NAME)
        {
            return Err(OperationError::invalid(
                "plugin sources are managed by the application",
            ));
        }
        Ok(self.packages()?.registry().search(&query).await)
    }

    pub async fn registry_package(
        &self,
        id: &str,
        registry: Option<&str>,
    ) -> Result<RegistryPackageView, OperationError> {
        if registry.is_some_and(|name| name != OFFICIAL_REGISTRY_NAME) {
            return Err(OperationError::invalid(
                "plugin sources are managed by the application",
            ));
        }
        let packages = self.packages()?;
        let (source, document, live) = registry_package_live(&packages, id, registry).await?;
        Ok(RegistryPackageView {
            registry: source.name,
            document,
            live_statuses: live
                .into_iter()
                .map(|(version, entry)| (version, entry.status))
                .collect(),
        })
    }

    pub fn registry_defaults(&self) -> Result<RegistryDefaults, OperationError> {
        self.packages()?
            .registry()
            .try_defaults()
            .map_err(registry_error)
    }

    pub fn registry_set_defaults(
        &self,
        defaults: RegistryDefaults,
    ) -> Result<RegistryDefaults, OperationError> {
        if !matches!(defaults.channel.as_str(), "stable" | "beta") {
            return Err(OperationError::invalid(
                "channel must be `stable` or `beta`",
            ));
        }
        if !(1..=168).contains(&defaults.check_interval_hours) {
            return Err(OperationError::invalid(
                "update checks run every 1..=168 hours",
            ));
        }
        let packages = self.packages()?;
        let _settings = packages
            .try_sideload_settings()
            .map_err(OperationError::install)?;
        packages
            .registry()
            .set_defaults(defaults.clone())
            .map_err(registry_error)?;
        Ok(defaults)
    }
}
