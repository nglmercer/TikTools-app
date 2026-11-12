use super::*;

impl RegistryClient {
    pub fn trust(&self) -> Arc<RwLock<TrustStore>> {
        self.trust.clone()
    }

    /// Whether this client restricts discovery and trust to provisioned sources.
    pub fn enforces_fixed_sources(&self) -> bool {
        self.fixed_sources
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

    pub(super) async fn operation_guard(&self) -> Option<Arc<RegistryTrustGuard>> {
        if self.fixed_sources {
            Some(Arc::new(RegistryTrustGuard {
                owner: Arc::clone(&self.operation_identity),
                _guard: self.operation.clone().lock_owned().await,
            }))
        } else {
            None
        }
    }

    pub(super) fn pin_roots(&self, source: &RegistrySource) {
        if let Ok(mut trust) = self.trust.write() {
            for root in &source.root_keys {
                trust.insert(
                    TrustedKey::new(root.clone(), KeyRole::Root),
                    source.name.clone(),
                );
            }
        }
    }
}
