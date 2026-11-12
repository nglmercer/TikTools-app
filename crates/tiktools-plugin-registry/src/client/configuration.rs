use super::*;

impl RegistryClient {
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

    pub(super) fn source(&self, name: &str) -> Result<RegistrySource, RegistryError> {
        self.config()
            .get(name)
            .cloned()
            .ok_or_else(|| RegistryError::UnknownRegistry(name.to_owned()))
    }

    pub(super) fn update_config(
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
