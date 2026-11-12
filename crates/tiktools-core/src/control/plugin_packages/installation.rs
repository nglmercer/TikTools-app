use super::*;

impl AppCore {
    pub async fn plugin_package_inspect(
        self: &Arc<Self>,
        input: PackageSpecInput,
    ) -> Result<PackagePreview, OperationError> {
        let packages = self.packages()?;
        let spec = product_package_spec(input)?;
        let label = spec.to_string();
        let reporter = tiktools_plugin_installer::NoopReporter;
        let (prepared, _trust_guard, _target) = self
            .fetch_prepared(
                &packages,
                &spec,
                &reporter,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .await
            .map_err(OperationError::install)?;
        let preview = packages.installer.preview(&prepared);
        packages.remember_review(&preview.consent_token, &label, prepared);
        Ok(preview)
    }

    pub fn plugin_package_install(
        self: &Arc<Self>,
        input: PackageSpecInput,
        consent_token: String,
        policy: Option<UpdatePolicy>,
    ) -> Result<JobSnapshot, OperationError> {
        let spec = product_package_spec(input)?;
        self.spawn_package_job(
            JobKind::Install,
            spec,
            InstallConsent::Reviewed(consent_token),
            policy,
        )
    }

    pub fn plugin_package_update(
        self: &Arc<Self>,
        plugin_id: &str,
        version: Option<String>,
        consent_token: Option<String>,
    ) -> Result<JobSnapshot, OperationError> {
        let packages = self.packages()?;
        let receipt = packages
            .installer
            .receipt(plugin_id)
            .map_err(OperationError::install)?
            .ok_or_else(|| OperationError::install(InstallError::not_installed(plugin_id)))?;
        let PackageOrigin::Registry { registry, .. } = &receipt.origin else {
            return Err(OperationError::install(InstallError::new(
                InstallErrorCode::Unsupported,
                format!("{plugin_id} was not installed from a registry; install a newer package instead"),
            )));
        };
        if registry != OFFICIAL_REGISTRY_NAME {
            return Err(OperationError::invalid(
                "this plugin was installed from a legacy source; review a local package or the configured Cloud catalog",
            ));
        }
        let spec = PackageSpec::try_from(PackageSpecInput::Registry {
            id: plugin_id.to_owned(),
            version,
            registry: Some(registry.clone()),
        })
        .map_err(OperationError::install)?;
        let consent = match consent_token {
            Some(token) => InstallConsent::Reviewed(token),
            None => InstallConsent::Automatic,
        };
        self.spawn_package_job(JobKind::Update, spec, consent, None)
    }

    pub(super) async fn fetch_prepared(
        &self,
        packages: &Arc<PluginPackages>,
        spec: &PackageSpec,
        reporter: &dyn JobReporter,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<
        (
            PreparedPackage,
            Option<Arc<tiktools_plugin_registry::RegistryTrustGuard>>,
            Option<String>,
        ),
        InstallError,
    > {
        let defaults = packages
            .registry()
            .try_defaults()
            .map_err(InstallError::io)?;
        let channel = match spec.plugin_id() {
            Some(id) => packages
                .installer
                .receipt(id)?
                .map(|receipt| receipt.policy.channel)
                .unwrap_or_else(|| defaults.channel.clone()),
            None => defaults.channel.clone(),
        };
        if !matches!(spec, PackageSpec::Registry { .. }) && !defaults.allow_sideload {
            return Err(InstallError::new(
                InstallErrorCode::Unsupported,
                "installing plugins from local files is disabled in the plugin settings",
            ));
        }
        let resolved = packages
            .sources
            .fetch(spec, &channel, packages.installer.host(), reporter, cancel)
            .await?;
        reporter.stage(JobStage::Verify);
        let installer = packages.installer.clone();
        tokio::task::spawn_blocking(move || {
            // Rust may otherwise capture only individual fields of
            // `resolved`, releasing the fixed-generation trust guard before
            // signature verification completes.
            let trust_guard = resolved.trust_guard;
            let target = resolved.target;
            let prepared = installer.prepare(&resolved.path, resolved.origin, resolved.policy)?;
            if let Some(expected) = &resolved.expected {
                expected.check(&prepared)?;
            }
            Ok((prepared, trust_guard, target))
        })
        .await
        .unwrap_or_else(|error| Err(InstallError::io(error)))
    }
}

pub(super) fn product_package_spec(input: PackageSpecInput) -> Result<PackageSpec, OperationError> {
    let spec = PackageSpec::try_from(input).map_err(OperationError::install)?;
    match &spec {
        PackageSpec::Registry {
            registry: Some(name),
            ..
        } if name != OFFICIAL_REGISTRY_NAME => Err(OperationError::invalid(
            "plugin sources are managed by the application",
        )),
        PackageSpec::Url(_) => Err(OperationError::invalid(
            "install plugins from the configured Cloud instance or a local package",
        )),
        _ => Ok(spec),
    }
}
