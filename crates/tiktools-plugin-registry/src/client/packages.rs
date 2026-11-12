use super::*;

impl RegistryClient {
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
}
