use semver::Version;
use tiktools_app_update::{validate_manifest, Channel};

fn manifest(format: &str, url: &str) -> String {
    format!(
        r#"{{"version":"4.1.0","platforms":{{"windows-x86_64":{{"url":"{url}","signature":"signed","format":"{format}"}}}}}}"#
    )
}
#[test]
fn validates_upstream_shape_and_rejects_untrusted_metadata() {
    let current = Version::parse("4.0.0").unwrap();
    let url = "https://github.com/nglmercer/TikTools-app/releases/download/v4.1.0/TikTools-4.1.0-windows-x86_64-installer.exe";
    assert!(validate_manifest(
        manifest("nsis", url).as_bytes(),
        "windows-x86_64",
        Channel::Stable,
        &current
    )
    .unwrap()
    .available());
    assert!(validate_manifest(
        manifest("nsis", &url.replace("installer.exe", "installer.zip")).as_bytes(),
        "windows-x86_64",
        Channel::Stable,
        &current
    )
    .is_err());
    assert!(validate_manifest(
        manifest("nsis", &url.replace("installer.exe", "installer.exe/extra")).as_bytes(),
        "windows-x86_64",
        Channel::Stable,
        &current
    )
    .is_err());
    assert!(validate_manifest(
        manifest("wix", url).as_bytes(),
        "windows-x86_64",
        Channel::Stable,
        &current
    )
    .is_err());
    assert!(validate_manifest(
        manifest(
            "nsis",
            "https://github.com/nglmercer/TikTools-app/releases/download/v4.0.0/TikTools-4.1.0-windows-x86_64-installer.exe"
        )
        .as_bytes(),
        "windows-x86_64",
        Channel::Stable,
        &current
    )
    .is_err());
    assert!(validate_manifest(
        manifest(
            "nsis",
            "http://github.com/nglmercer/TikTools-app/releases/download/a"
        )
        .as_bytes(),
        "windows-x86_64",
        Channel::Stable,
        &current
    )
    .is_err());
    assert!(validate_manifest(b"{}", "windows-x86_64", Channel::Stable, &current).is_err());
    assert!(validate_manifest(
        manifest("nsis", url).as_bytes(),
        "linux-x86_64",
        Channel::Stable,
        &current
    )
    .is_err());
}

#[test]
fn accepts_legacy_appimage_asset_name_for_published_feed_heads() {
    let json = br#"{"version":"4.2.3","platforms":{"linux-x86_64":{"url":"https://github.com/nglmercer/TikTools-app/releases/download/v4.2.3/TikTools-4.2.3-linux-x86_64-installer.AppImage","signature":"signed","format":"appimage"}}}"#;
    assert!(validate_manifest(
        json,
        "linux-x86_64",
        Channel::Stable,
        &Version::parse("4.0.0").unwrap()
    )
    .is_ok());
}

#[test]
fn accepts_encoded_semver_build_metadata_in_release_urls() {
    let json = br#"{"version":"4.1.0+build.1","platforms":{"linux-x86_64":{"url":"https://github.com/nglmercer/TikTools-app/releases/download/v4.1.0%2Bbuild.1/TikTools-4.1.0%2Bbuild.1-x86_64-installer.AppImage","signature":"signed","format":"appimage"}}}"#;
    assert!(validate_manifest(
        json,
        "linux-x86_64",
        Channel::Stable,
        &Version::parse("4.0.0").unwrap()
    )
    .is_ok());
}
