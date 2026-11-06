//! Optional end-to-end check against an independently pinned staging registry.
use std::sync::{atomic::AtomicBool, Arc, RwLock};
use tiktools_plugin_api::HostCompat;
use tiktools_plugin_installer::{
    CommitOptions, Consent, Installer, InstallerConfig, NoopHooks, NoopReporter, PackageOrigin,
    SignaturePolicy,
};
use tiktools_plugin_package::{PublicKey, TrustStore};
use tiktools_plugin_registry::{
    join_url, DownloadRequest, Downloader, RegistryClient, RegistrySource,
};

#[tokio::test]
#[ignore = "requires a published disposable staging registry and independent root pin"]
async fn current_client_installs_all_official_staging_packages() {
    let base = std::env::var("CLOUD_STAGING_ORIGIN").unwrap();
    let generation: u64 = std::env::var("CLOUD_STAGING_GENERATION")
        .unwrap()
        .parse()
        .unwrap();
    let root: PublicKey = serde_json::from_slice(
        &std::fs::read(std::env::var("CLOUD_STAGING_ROOT_PIN").unwrap()).unwrap(),
    )
    .unwrap();
    let work =
        std::env::temp_dir().join(format!("tiktools-staging-install-{}", std::process::id()));
    std::fs::create_dir(&work).unwrap();
    let mut source = RegistrySource::new(
        "staging",
        format!("{base}/api/v1/registry/generations/{generation}/files"),
    )
    .unwrap();
    source.root_keys = vec![root];
    let trust = Arc::new(RwLock::new(TrustStore::new()));
    let client = RegistryClient::new(
        work.join("cache"),
        work.join("state.json"),
        vec![source.clone()],
        trust.clone(),
    )
    .unwrap();
    assert!(client.refresh("staging").await.unwrap().verified);
    let installer = Installer::new(
        InstallerConfig {
            plugins_dir: work.join("plugins"),
            store_dir: work.join("store"),
            host: HostCompat::new(semver::Version::new(0, 3, 3)),
            retain_versions: 1,
            provider_endpoints: None,
        },
        trust,
    );
    for id in [
        "audio.play.process",
        "hotkeys",
        "minecraft.server",
        "peak.game",
        "repo.game",
        "sonicboom.server",
        "textintel",
        "tiktools.event-gateway",
    ] {
        let (_, package) = client.package(id, Some("staging")).await.unwrap();
        let artifact = &package.releases.first().unwrap().artifacts[0];
        let downloaded = Downloader::new(work.join("downloads"))
            .unwrap()
            .fetch(
                DownloadRequest {
                    url: &join_url(&source.url, &artifact.url).unwrap(),
                    expected_digest: Some(&artifact.sha256),
                    expected_size: Some(artifact.size),
                },
                &|_, _| {},
                &AtomicBool::new(false),
            )
            .await
            .unwrap();
        let prepared = installer
            .prepare(
                &downloaded.path,
                PackageOrigin::Registry {
                    registry: "staging".into(),
                    url: source.url.clone(),
                },
                SignaturePolicy::Require,
            )
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        let preview = installer.preview(&prepared);
        let result = installer
            .commit(
                &prepared,
                &CommitOptions {
                    consent: Consent::Token(preview.consent_token),
                    policy: None,
                },
                &NoopHooks,
                &NoopReporter,
            )
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(result.receipt.id, id);
    }
    std::fs::remove_dir_all(&work).unwrap();
}
