use semver::Version;
use thiserror::Error;
use url::Url;

use crate::{
    packager::{RemoteRelease, RemoteReleaseData, UpdateFormat},
    Channel,
};

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

/// Decode the packager-compatible static manifest format, then apply
/// TikTools' policy. Callers must still authenticate the manifest and verify
/// the payload signature before installation.
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
    let mut expected_prefix =
        Url::parse("https://github.com/nglmercer/TikTools-app/releases/download/")
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
    // Linux AppImage assets omit "linux": every AppImage targets Linux, so
    // the URL carries only the architecture. The platform key stays full.
    let asset_target = if target == "linux-x86_64" {
        "x86_64"
    } else {
        target
    };
    let expected_file = format!(
        "TikTools-{}-{asset_target}-installer.{extension}",
        release.version
    );
    let prefix = expected_prefix.path();
    let mut expected_paths = vec![format!("{prefix}{expected_file}")];
    if target == "linux-x86_64" {
        // Published channel heads before the asset-name fix used the full
        // target in the AppImage file. Keep verifying those exact historical
        // URLs so old feeds remain readable; new manifests still use the
        // canonical name above.
        expected_paths.push(format!(
            "{prefix}TikTools-{}-linux-x86_64-installer.{extension}",
            release.version
        ));
    }
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
        || !expected_paths
            .iter()
            .any(|expected| path == expected.as_str())
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
