//! Minimal, local equivalent of the `cargo-packager-updater` pieces the
//! updater actually uses: the static manifest schema and the final install
//! handoff for AppImage / NSIS.
//!
//! The upstream crate hard-depends on `reqwest 0.12` (and an older
//! `minisign-verify`, `base64`, `webpki-roots`, ...). The workspace standardises
//! on `reqwest 0.13`; download and signature verification already live in
//! `download.rs`, so keeping the upstream crate only for its data types and
//! `Update::install` duplicated half the HTTP stack. This module keeps the same
//! on-disk JSON format and the same atomic replace / installer-launch semantics.

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use semver::Version;
use serde::{de::Error as _, Deserialize};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFormat {
    Nsis,
    Wix,
    AppImage,
    App,
}

impl<'de> Deserialize<'de> for UpdateFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.to_lowercase().as_str() {
            "nsis" => Ok(Self::Nsis),
            "wix" => Ok(Self::Wix),
            "appimage" => Ok(Self::AppImage),
            "app" => Ok(Self::App),
            other => Err(D::Error::custom(format!("unknown updater format: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseManifestPlatform {
    pub url: Url,
    pub signature: String,
    pub format: UpdateFormat,
}

#[derive(Debug, Clone)]
pub enum RemoteReleaseData {
    Dynamic(ReleaseManifestPlatform),
    Static {
        platforms: HashMap<String, ReleaseManifestPlatform>,
    },
}

#[derive(Debug, Clone)]
pub struct RemoteRelease {
    pub version: Version,
    pub notes: Option<String>,
    pub data: RemoteReleaseData,
}

impl RemoteRelease {
    pub fn download_url(&self, target: &str) -> Option<&Url> {
        match &self.data {
            RemoteReleaseData::Dynamic(platform) => Some(&platform.url),
            RemoteReleaseData::Static { platforms } => platforms.get(target).map(|p| &p.url),
        }
    }

    pub fn signature(&self, target: &str) -> Option<&String> {
        match &self.data {
            RemoteReleaseData::Dynamic(platform) => Some(&platform.signature),
            RemoteReleaseData::Static { platforms } => platforms.get(target).map(|p| &p.signature),
        }
    }

    pub fn format(&self, target: &str) -> Option<UpdateFormat> {
        match &self.data {
            RemoteReleaseData::Dynamic(platform) => Some(platform.format),
            RemoteReleaseData::Static { platforms } => platforms.get(target).map(|p| p.format),
        }
    }
}

impl<'de> Deserialize<'de> for RemoteRelease {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Inner {
            #[serde(alias = "name")]
            version: String,
            notes: Option<String>,
            platforms: Option<HashMap<String, ReleaseManifestPlatform>>,
            url: Option<Url>,
            signature: Option<String>,
            format: Option<UpdateFormat>,
        }

        let inner = Inner::deserialize(deserializer)?;
        let version =
            Version::from_str(inner.version.trim_start_matches('v')).map_err(D::Error::custom)?;
        let data = match inner.platforms {
            Some(platforms) => RemoteReleaseData::Static { platforms },
            None => RemoteReleaseData::Dynamic(ReleaseManifestPlatform {
                url: inner.url.ok_or_else(|| D::Error::custom("missing `url`"))?,
                signature: inner
                    .signature
                    .ok_or_else(|| D::Error::custom("missing `signature`"))?,
                format: inner
                    .format
                    .ok_or_else(|| D::Error::custom("missing `format`"))?,
            }),
        };
        Ok(Self {
            version,
            notes: inner.notes,
            data,
        })
    }
}

/// Only the two fields needed for the final platform handoff. Metadata and
/// signature verification live in `check.rs` / `download.rs` and are not
/// duplicated here.
#[derive(Debug, Clone)]
pub struct Update {
    pub extract_path: PathBuf,
    pub format: UpdateFormat,
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("unsupported update format")]
    UnsupportedFormat,
    #[error("no temporary directory on the same mount point as the application")]
    NoSameMountTempDir,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Update {
    /// Hand the already verified bytes to the platform installer. Download
    /// and signature verification happen in `download.rs` before this point.
    pub fn install(&self, bytes: Vec<u8>) -> Result<(), InstallError> {
        match self.format {
            UpdateFormat::AppImage => self.install_appimage(bytes),
            UpdateFormat::Nsis => self.install_nsis(bytes),
            _ => Err(InstallError::UnsupportedFormat),
        }
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    fn install_appimage(&self, bytes: Vec<u8>) -> Result<(), InstallError> {
        use std::{
            fs,
            os::unix::fs::{MetadataExt, PermissionsExt},
        };

        let metadata = self.extract_path.metadata()?;
        let mut roots = vec![std::env::temp_dir()];
        if let Some(parent) = self.extract_path.parent() {
            roots.push(parent.to_path_buf());
        }

        for root in roots {
            let dir = tempfile::Builder::new()
                .prefix("current_app")
                .tempdir_in(root)?;
            if metadata.dev() != dir.path().metadata()?.dev() {
                continue;
            }
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700))?;
            let backup = dir.path().join("current_app.AppImage");
            fs::rename(&self.extract_path, &backup)?;
            if let Err(error) = fs::write(&self.extract_path, bytes)
                .and_then(|_| fs::set_permissions(&self.extract_path, metadata.permissions()))
            {
                fs::rename(backup, &self.extract_path)?;
                return Err(error.into());
            }
            return Ok(());
        }
        Err(InstallError::NoSameMountTempDir)
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    fn install_appimage(&self, _bytes: Vec<u8>) -> Result<(), InstallError> {
        Err(InstallError::UnsupportedFormat)
    }

    #[cfg(target_os = "windows")]
    fn install_nsis(&self, bytes: Vec<u8>) -> Result<(), InstallError> {
        use std::{io::Write, os::windows::process::CommandExt, process::Command};

        let mut file = tempfile::Builder::new().suffix(".exe").tempfile()?;
        file.write_all(&bytes)?;
        let (_, path) = file.keep().map_err(std::io::Error::from)?;

        let mut installer = std::ffi::OsString::from("\"");
        installer.push(&path);
        installer.push("\"");

        let system_root = std::env::var("SYSTEMROOT").unwrap_or_default();
        let powershell = if system_root.is_empty() {
            "powershell.exe".to_owned()
        } else {
            format!("{system_root}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe")
        };

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = Command::new(powershell);
        command
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-NoProfile", "-WindowStyle", "Hidden", "Start-Process"])
            .arg(installer)
            .args(["-ArgumentList", "/P, /R"])
            .spawn()?;
        // NSIS exits after launching the installer; returning would let the
        // old desktop instance keep the instance lock.
        std::process::exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    fn install_nsis(&self, _bytes: Vec<u8>) -> Result<(), InstallError> {
        Err(InstallError::UnsupportedFormat)
    }
}
