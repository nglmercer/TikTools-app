use super::*;

impl RegistryClient {
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

    /// Fetch the active generation advertised by a Cloud origin. A 404 means
    /// no snapshot has been published yet, which is an availability state the
    /// UI surfaces, not a transport failure.
    pub(super) async fn cloud_generation(&self, origin: &str) -> Result<u64, RegistryError> {
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
    pub(super) async fn ensure_cloud_generation(
        &self,
        name: &str,
    ) -> Result<RegistrySource, RegistryError> {
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
}
