use super::*;

impl RegistryClient {
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

    pub(super) async fn ensure_loaded(
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

    pub(super) async fn load_registry(
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

    pub(super) async fn document(
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
}
