//! Transfer a previously verified update to Packager after desktop shutdown.
use crate::{detect_installation, AuthenticatedRelease, InstallationType, VerifiedPackage};
use cargo_packager_updater::{Config, Update, UpdateFormat};
use std::{
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("the installation source is no longer supported or has changed")]
    SourceChanged,
    #[error("the verified update is incompatible with this installation")]
    Unsupported,
    #[error("the installer failed: {0}")]
    Installer(String),
    #[error("the updated application could not be validated or restarted")]
    Restart,
}

/// A one-use handoff containing only authenticated metadata and verified bytes.
pub struct InstallPlan {
    update: Update,
    bytes: Vec<u8>,
    target: PathBuf,
    expected_version: String,
    kind: InstallationType,
}

impl InstallPlan {
    pub(crate) fn new(
        release: Arc<AuthenticatedRelease>,
        package: VerifiedPackage,
        target: PathBuf,
        current: String,
        kind: InstallationType,
    ) -> Result<Self, InstallError> {
        let validated = &release.release;
        if !validated.available {
            return Err(InstallError::Unsupported);
        }
        let format = validated
            .release
            .format(&validated.target)
            .map_err(|_| InstallError::Unsupported)?;
        let target_os = match (kind, validated.target.as_str(), &format) {
            (InstallationType::AppImage, "linux-x86_64", UpdateFormat::AppImage)
                if cfg!(target_os = "linux") =>
            {
                "linux"
            }
            (InstallationType::Nsis, "windows-x86_64", UpdateFormat::Nsis)
                if cfg!(target_os = "windows") =>
            {
                "windows"
            }
            _ => return Err(InstallError::Unsupported),
        };
        let version = validated.release.version.to_string();
        let update = Update {
            config: Config {
                endpoints: vec![],
                pubkey: release.public_key.clone(),
                windows: None,
            },
            body: validated.release.notes.clone(),
            current_version: current,
            version: version.clone(),
            date: validated.release.pub_date,
            target: target_os.to_owned(),
            extract_path: target.clone(),
            download_url: validated
                .release
                .download_url(&validated.target)
                .map_err(|_| InstallError::Unsupported)?
                .clone(),
            signature: validated
                .release
                .signature(&validated.target)
                .map_err(|_| InstallError::Unsupported)?
                .clone(),
            timeout: None,
            headers: Default::default(),
            format,
        };
        Ok(Self {
            update,
            bytes: package.into_bytes(),
            target,
            expected_version: version,
            kind,
        })
    }

    /// Must be called only after the desktop, runtime, and instance lock have
    /// been dropped. Packager atomically moves the old image before writing.
    pub fn apply(self) -> Result<(), InstallError> {
        let detected = detect_installation();
        if detected.kind != self.kind || detected.target.as_ref() != Some(&self.target) {
            return Err(InstallError::SourceChanged);
        }
        install_package(&self.update, self.bytes)?;
        if self.kind != InstallationType::AppImage {
            // Packager exits after launching NSIS; a return cannot prove success.
            return Err(InstallError::Restart);
        }
        validate_version(&self.target, &self.expected_version)?;
        Command::new(&self.target)
            .spawn()
            .map_err(|_| InstallError::Restart)?;
        Ok(())
    }
}

fn install_package(update: &Update, bytes: Vec<u8>) -> Result<(), InstallError> {
    update
        .install(bytes)
        .map_err(|error| InstallError::Installer(error.to_string()))
}

fn validate_version(path: &std::path::Path, expected: &str) -> Result<(), InstallError> {
    let mut child = Command::new(path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| InstallError::Restart)?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let mut output = String::new();
                child
                    .stdout
                    .take()
                    .ok_or(InstallError::Restart)?
                    .take(1024)
                    .read_to_string(&mut output)
                    .map_err(|_| InstallError::Restart)?;
                return if output.trim() == expected {
                    Ok(())
                } else {
                    Err(InstallError::Restart)
                };
            }
            Ok(Some(_)) | Err(_) => return Err(InstallError::Restart),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(InstallError::Restart);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn packager_replaces_a_disposable_image_without_touching_user_data() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-updater-install-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("TikTools.AppImage");
        let data = root.join("user.db");
        std::fs::write(&image, b"old image").unwrap();
        std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(&data, b"user data").unwrap();
        let update = Update {
            config: Config {
                endpoints: vec![],
                pubkey: String::new(),
                windows: None,
            },
            body: None,
            current_version: "4.0.0".into(),
            version: "4.1.0".into(),
            date: None,
            target: "linux".into(),
            extract_path: image.clone(),
            download_url: "https://example.com/app".parse().unwrap(),
            signature: String::new(),
            timeout: None,
            headers: Default::default(),
            format: UpdateFormat::AppImage,
        };
        install_package(&update, b"new image".to_vec()).unwrap();
        assert_eq!(std::fs::read(&image).unwrap(), b"new image");
        assert_eq!(std::fs::read(&data).unwrap(), b"user data");
        assert_eq!(
            std::fs::metadata(&image).unwrap().permissions().mode() & 0o777,
            0o755
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn handoff_refuses_an_unrelated_installation_source() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-update-source-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("TikTools.AppImage");
        std::fs::write(&image, b"original").unwrap();
        let update = Update {
            config: Config {
                endpoints: vec![],
                pubkey: String::new(),
                windows: None,
            },
            body: None,
            current_version: "4.0.0".into(),
            version: "4.1.0".into(),
            date: None,
            target: "linux".into(),
            extract_path: image.clone(),
            download_url: "https://example.com/app".parse().unwrap(),
            signature: String::new(),
            timeout: None,
            headers: Default::default(),
            format: UpdateFormat::AppImage,
        };
        let plan = InstallPlan {
            update,
            bytes: b"replacement".to_vec(),
            target: image.clone(),
            expected_version: "4.1.0".into(),
            kind: InstallationType::AppImage,
        };
        assert!(matches!(plan.apply(), Err(InstallError::SourceChanged)));
        assert_eq!(std::fs::read(&image).unwrap(), b"original");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn version_validation_rejects_a_different_binary_version() {
        let root = std::env::temp_dir().join(format!(
            "tiktools-updater-version-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let binary = root.join("app");
        std::fs::write(&binary, "#!/bin/sh\nprintf '4.1.0\\n'\n").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(validate_version(&binary, "4.1.0").is_ok());
        assert!(validate_version(&binary, "4.2.0").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
