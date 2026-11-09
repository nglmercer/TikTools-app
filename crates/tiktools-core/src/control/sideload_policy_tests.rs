use super::*;
use crate::{paths::AppPaths, HostEmitter, NoopMediaHost};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tiktools_plugin_installer::{InstallErrorCode, JobStatus, PackageSpecInput};
use tiktools_plugin_package::{pack, PackRequest};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}

struct Fixture {
    core: Arc<AppCore>,
    root: PathBuf,
    archive: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tiktools-sideload-policy-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("no-builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let manifest = root.join("plugin.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": 3,
                "id": "acme.sideload",
                "name": "Sideload Fixture",
                "version": "1.0.0",
                "runtime": "declarative"
            }))
            .unwrap(),
        )
        .unwrap();
        let archive = root.join("sideload.plugin");
        pack(&PackRequest {
            manifest,
            entry: None,
            output: archive.clone(),
            target: None,
        })
        .unwrap();
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths,
        ));
        Self {
            core,
            root,
            archive,
        }
    }

    fn spec(&self) -> PackageSpecInput {
        PackageSpecInput::File {
            path: self.archive.to_string_lossy().into_owned(),
        }
    }

    fn second_version(&self) -> PathBuf {
        let directory = self.root.join("second-version");
        std::fs::create_dir_all(&directory).unwrap();
        let manifest = directory.join("plugin.json");
        std::fs::write(
            &manifest,
            serde_json::to_vec(&serde_json::json!({
                "schemaVersion": 3,
                "id": "acme.sideload",
                "name": "Sideload Fixture",
                "version": "1.1.0",
                "runtime": "declarative"
            }))
            .unwrap(),
        )
        .unwrap();
        let archive = self.root.join("sideload-1.1.0.plugin");
        pack(&PackRequest {
            manifest,
            entry: None,
            output: archive.clone(),
            target: None,
        })
        .unwrap();
        archive
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

async fn finished(core: &AppCore, id: &str) -> JobSnapshot {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let snapshot = core.plugin_job(id).unwrap();
            if snapshot.status.is_finished() {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("package operation must finish")
}

#[tokio::test]
async fn update_checks_reject_overlap_before_they_can_publish_out_of_order() {
    let fixture = Fixture::new();
    let packages = fixture.core.packages().unwrap();
    let slow_check = packages.updates_gate.lock().await;
    assert!(fixture.core.plugin_updates_check().await.is_err());
    drop(slow_check);
    assert!(fixture.core.plugin_updates_check().await.is_ok());
}

async fn install_both_versions(fixture: &Fixture) {
    let archives = [fixture.archive.clone(), fixture.second_version()];
    for archive in &archives {
        let spec = || PackageSpecInput::File {
            path: archive.to_string_lossy().into_owned(),
        };
        let preview = fixture.core.plugin_package_inspect(spec()).await.unwrap();
        let job = fixture
            .core
            .plugin_package_install(spec(), preview.consent_token, None)
            .unwrap();
        let result = finished(&fixture.core, &job.id).await;
        assert_eq!(result.status, JobStatus::Succeeded, "{:?}", result.error);
    }
    let receipt = fixture
        .core
        .packages()
        .unwrap()
        .installer
        .receipt("acme.sideload")
        .unwrap()
        .unwrap();
    assert_eq!(receipt.version, "1.1.0");
}

#[cfg(all(feature = "persistence", feature = "cloud-auth"))]
#[tokio::test]
async fn package_jobs_preserve_installed_files_while_product_emission_is_active() {
    let fixture = Fixture::new();
    let core = &fixture.core;
    let fence = tiktools_cloud_sync::ProductEmissionFence::open(
        core.db.paths().data.join("plugin-sync-emission"),
    )
    .unwrap();
    let preview = core.plugin_package_inspect(fixture.spec()).await.unwrap();
    let emission = fence.try_emission().unwrap();
    let job = core
        .plugin_package_install(fixture.spec(), preview.consent_token, None)
        .unwrap();
    let result = finished(core, &job.id).await;
    assert_eq!(result.status, JobStatus::Failed);
    assert_eq!(result.error.unwrap().code, InstallErrorCode::Busy);
    assert!(core
        .packages()
        .unwrap()
        .installer
        .receipt("acme.sideload")
        .unwrap()
        .is_none());
    drop(emission);
    install_both_versions(&fixture).await;

    let emission = fence.try_emission().unwrap();
    let job = core
        .plugin_package_rollback("acme.sideload", Some("1.0.0".into()))
        .unwrap();
    let result = finished(core, &job.id).await;
    assert_eq!(result.status, JobStatus::Failed);
    assert_eq!(result.error.unwrap().code, InstallErrorCode::Busy);
    assert_eq!(
        core.packages()
            .unwrap()
            .installer
            .receipt("acme.sideload")
            .unwrap()
            .unwrap()
            .version,
        "1.1.0"
    );
    drop(emission);
    let job = core
        .plugin_package_rollback("acme.sideload", Some("1.0.0".into()))
        .unwrap();
    let result = finished(core, &job.id).await;
    assert_eq!(result.status, JobStatus::Succeeded, "{:?}", result.error);
    core.shutdown().await;
}

#[tokio::test]
async fn disabling_sideload_after_inspect_rejects_reviewed_and_direct_install() {
    let fixture = Fixture::new();
    let core = &fixture.core;
    let preview = core.plugin_package_inspect(fixture.spec()).await.unwrap();
    assert!(preview.requires_sideload_consent);
    let mut settings = core.registry_defaults().unwrap();
    assert!(settings.allow_sideload);
    settings.allow_sideload = false;
    core.registry_set_defaults(settings.clone()).unwrap();

    // The retained review contains verified bytes; it must not bypass a
    // later change to the product's local-install preference.
    let job = core
        .plugin_package_install(fixture.spec(), preview.consent_token.clone(), None)
        .unwrap();
    let result = finished(core, &job.id).await;
    assert_eq!(result.status, JobStatus::Failed);
    assert_eq!(result.error.unwrap().code, InstallErrorCode::Unsupported);
    assert!(core
        .packages()
        .unwrap()
        .installer
        .receipt("acme.sideload")
        .unwrap()
        .is_none());
    assert_eq!(
        core.install_plugin(&fixture.archive, false)
            .unwrap_err()
            .code,
        InstallErrorCode::Unsupported
    );

    // Enabling again restores local installation through the reviewed path.
    settings.allow_sideload = true;
    core.registry_set_defaults(settings).unwrap();
    let job = core
        .plugin_package_install(fixture.spec(), preview.consent_token, None)
        .unwrap();
    let result = finished(core, &job.id).await;
    assert_eq!(result.status, JobStatus::Succeeded, "{:?}", result.error);
    assert!(core
        .packages()
        .unwrap()
        .installer
        .receipt("acme.sideload")
        .unwrap()
        .is_some());
    core.shutdown().await;
}

#[tokio::test]
async fn changing_sideload_while_commit_holds_lease_is_busy_then_succeeds() {
    let fixture = Fixture::new();
    let core = &fixture.core;
    let preview = core.plugin_package_inspect(fixture.spec()).await.unwrap();
    let packages = core.packages().unwrap();
    let mut settings = core.registry_defaults().unwrap();
    settings.allow_sideload = false;

    // The package worker acquires the sideload read lease immediately before
    // the invocation gate. Holding that gate pauses the real commit without a
    // test-only hook in the production path.
    let invocation = core.plugin_state.invocation_gate.lock().unwrap();
    let job = core
        .plugin_package_install(fixture.spec(), preview.consent_token, None)
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if packages.try_sideload_settings().is_err() {
                break;
            }
            assert!(
                !core.plugin_job(&job.id).unwrap().status.is_finished(),
                "installation ended before acquiring the sideload lease"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("commit must acquire its sideload lease");
    let error = core.registry_set_defaults(settings.clone()).unwrap_err();
    assert_eq!(error.code, "install_failed");
    assert_eq!(error.data.unwrap()["code"], "busy");
    assert!(core.registry_defaults().unwrap().allow_sideload);
    drop(invocation);

    let result = finished(core, &job.id).await;
    assert_eq!(result.status, JobStatus::Succeeded, "{:?}", result.error);
    assert!(!core.registry_set_defaults(settings).unwrap().allow_sideload);
    assert!(!core.registry_defaults().unwrap().allow_sideload);
    core.shutdown().await;
}

#[tokio::test]
async fn disabling_sideload_blocks_local_rollback_until_enabled_again() {
    let fixture = Fixture::new();
    install_both_versions(&fixture).await;
    let core = &fixture.core;
    let mut settings = core.registry_defaults().unwrap();
    settings.allow_sideload = false;
    core.registry_set_defaults(settings.clone()).unwrap();

    let job = core
        .plugin_package_rollback("acme.sideload", Some("1.0.0".into()))
        .unwrap();
    let rejected = finished(core, &job.id).await;
    assert_eq!(rejected.status, JobStatus::Failed);
    assert_eq!(rejected.error.unwrap().code, InstallErrorCode::Unsupported);
    assert_eq!(
        core.packages()
            .unwrap()
            .installer
            .receipt("acme.sideload")
            .unwrap()
            .unwrap()
            .version,
        "1.1.0"
    );

    settings.allow_sideload = true;
    core.registry_set_defaults(settings).unwrap();
    let job = core
        .plugin_package_rollback("acme.sideload", Some("1.0.0".into()))
        .unwrap();
    let restored = finished(core, &job.id).await;
    assert_eq!(
        restored.status,
        JobStatus::Succeeded,
        "{:?}",
        restored.error
    );
    assert_eq!(
        core.packages()
            .unwrap()
            .installer
            .receipt("acme.sideload")
            .unwrap()
            .unwrap()
            .version,
        "1.0.0"
    );
    core.shutdown().await;
}

#[tokio::test]
async fn changing_sideload_while_rollback_holds_lease_is_busy_then_succeeds() {
    let fixture = Fixture::new();
    install_both_versions(&fixture).await;
    let core = &fixture.core;
    let packages = core.packages().unwrap();
    let mut settings = core.registry_defaults().unwrap();
    settings.allow_sideload = false;
    let invocation = core.plugin_state.invocation_gate.lock().unwrap();
    let job = core
        .plugin_package_rollback("acme.sideload", Some("1.0.0".into()))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if packages.try_sideload_settings().is_err() {
                break;
            }
            assert!(
                !core.plugin_job(&job.id).unwrap().status.is_finished(),
                "rollback ended before acquiring its sideload lease"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("rollback must acquire its sideload lease");
    let error = core.registry_set_defaults(settings.clone()).unwrap_err();
    assert_eq!(error.code, "install_failed");
    assert_eq!(error.data.unwrap()["code"], "busy");
    assert!(core.registry_defaults().unwrap().allow_sideload);
    drop(invocation);

    let result = finished(core, &job.id).await;
    assert_eq!(result.status, JobStatus::Succeeded, "{:?}", result.error);
    assert!(!core.registry_set_defaults(settings).unwrap().allow_sideload);
    assert_eq!(
        packages
            .installer
            .receipt("acme.sideload")
            .unwrap()
            .unwrap()
            .version,
        "1.0.0"
    );
    core.shutdown().await;
}
