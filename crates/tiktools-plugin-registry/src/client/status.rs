use super::*;

impl RegistryClient {
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

    pub(super) fn describe(&self, source: &RegistrySource) -> RegistryStatus {
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
}
