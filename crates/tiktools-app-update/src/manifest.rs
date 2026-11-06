use cargo_packager_updater::{RemoteRelease, RemoteReleaseData, UpdateFormat};
use semver::Version;
use thiserror::Error;

use crate::Channel;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("invalid update metadata")]
    InvalidMetadata,
    #[error("no update for this platform")]
    UnsupportedPlatform,
    #[error("unsupported update format")]
    UnsupportedFormat,
    #[error("update URL is not a trusted HTTPS release asset")]
    UntrustedUrl,
    #[error("update signature is missing")]
    MissingSignature,
}

#[derive(Debug)]
pub struct ValidatedRelease {
    pub(crate) release: RemoteRelease,
    pub(crate) target: String,
    pub(crate) available: bool,
}

impl ValidatedRelease {
    pub fn release(&self) -> &RemoteRelease {
        &self.release
    }
    pub fn available(&self) -> bool {
        self.available
    }
}

/// Decode the upstream static manifest format, then apply TikTools' policy.
/// Version and URL metadata are not themselves signed by updater 0.2.3;
/// callers must still verify the payload before installation.
pub fn validate_manifest(
    bytes: &[u8],
    target: &str,
    channel: Channel,
    current: &Version,
) -> Result<ValidatedRelease, ManifestError> {
    if bytes.len() > 256 * 1024 {
        return Err(ManifestError::InvalidMetadata);
    }
    let release: RemoteRelease =
        serde_json::from_slice(bytes).map_err(|_| ManifestError::InvalidMetadata)?;
    let RemoteReleaseData::Static { ref platforms } = release.data else {
        return Err(ManifestError::InvalidMetadata);
    };
    let platform = platforms
        .get(target)
        .ok_or(ManifestError::UnsupportedPlatform)?;
    let valid_format = matches!(
        (target, platform.format),
        ("windows-x86_64", UpdateFormat::Nsis) | ("linux-x86_64", UpdateFormat::AppImage)
    );
    if !valid_format {
        return Err(ManifestError::UnsupportedFormat);
    }
    let url = &platform.url;
    let mut expected_prefix = cargo_packager_updater::url::Url::parse(
        "https://github.com/nglmercer/TikTools-app/releases/download/",
    )
    .expect("static release origin is valid");
    expected_prefix
        .path_segments_mut()
        .expect("static release URL is a base")
        .pop_if_empty()
        .push(&format!("v{}", release.version))
        .push("");
    let extension = match target {
        "windows-x86_64" => "exe",
        "linux-x86_64" => "AppImage",
        _ => return Err(ManifestError::UnsupportedPlatform),
    };
    let expected_file = format!(
        "TikTools-{}-{target}-installer.{extension}",
        release.version
    );
    let expected_path = format!("{}{expected_file}", expected_prefix.path());
    // `+` is legal in a path segment, but release URLs commonly percent-encode
    // it. Normalize only that byte; encoded slashes must not alter boundaries.
    let path = url.path().replace("%2B", "+").replace("%2b", "+");
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || path != expected_path
    {
        return Err(ManifestError::UntrustedUrl);
    }
    if platform.signature.trim().is_empty() {
        return Err(ManifestError::MissingSignature);
    }
    let available = channel.permits(current, &release.version);
    Ok(ValidatedRelease {
        release,
        target: target.to_owned(),
        available,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest(format: &str, url: &str) -> String {
        format!(
            r#"{{"version":"4.1.0","platforms":{{"windows-x86_64":{{"url":"{url}","signature":"signed","format":"{format}"}}}}}}"#
        )
    }
    #[test]
    fn validates_upstream_shape_and_rejects_untrusted_metadata() {
        let current = Version::parse("4.0.0").unwrap();
        let url = "https://github.com/nglmercer/TikTools-app/releases/download/v4.1.0/TikTools-4.1.0-windows-x86_64-installer.exe";
        assert!(
            validate_manifest(
                manifest("nsis", url).as_bytes(),
                "windows-x86_64",
                Channel::Stable,
                &current
            )
            .unwrap()
            .available
        );
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
    fn accepts_encoded_semver_build_metadata_in_release_urls() {
        let json = br#"{"version":"4.1.0+build.1","platforms":{"linux-x86_64":{"url":"https://github.com/nglmercer/TikTools-app/releases/download/v4.1.0%2Bbuild.1/TikTools-4.1.0%2Bbuild.1-linux-x86_64-installer.AppImage","signature":"signed","format":"appimage"}}}"#;
        assert!(validate_manifest(
            json,
            "linux-x86_64",
            Channel::Stable,
            &Version::parse("4.0.0").unwrap()
        )
        .is_ok());
    }
}
