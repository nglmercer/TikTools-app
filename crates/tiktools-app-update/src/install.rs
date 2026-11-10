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
        if self.kind == InstallationType::AppImage {
            preflight_appimage(&self.bytes, &self.target, &self.expected_version)?;
        }
        install_package(&self.update, self.bytes)?;
        if self.kind != InstallationType::AppImage {
            // Packager exits after launching NSIS; a return cannot prove success.
            return Err(InstallError::Restart);
        }
        validate_version(&self.target, &self.expected_version)?;
        appimage_command(&self.target)
            .spawn()
            .map_err(|_| InstallError::Restart)?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn preflight_appimage(
    bytes: &[u8],
    target: &std::path::Path,
    expected: &str,
) -> Result<(), InstallError> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let parent = target.parent().ok_or(InstallError::Unsupported)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".tiktools-update-check-")
        .tempfile_in(parent)
        .map_err(|error| InstallError::Installer(error.to_string()))?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.flush())
        .map_err(|error| InstallError::Installer(error.to_string()))?;
    temporary
        .as_file()
        .set_permissions(std::fs::Permissions::from_mode(0o700))
        .map_err(|error| InstallError::Installer(error.to_string()))?;
    // Close the write handle before executing; TempPath removes the file on drop.
    let path = temporary.into_temp_path();
    validate_version(&path, expected)
}

#[cfg(not(target_os = "linux"))]
fn preflight_appimage(_: &[u8], _: &std::path::Path, _: &str) -> Result<(), InstallError> {
    Err(InstallError::Unsupported)
}

fn install_package(update: &Update, bytes: Vec<u8>) -> Result<(), InstallError> {
    update
        .install(bytes)
        .map_err(|error| InstallError::Installer(error.to_string()))
}

/// AppRun adds paths from its mounted image to the environment. Those paths
/// refer to the old image during replacement and must not leak into the new one.
fn appimage_command(path: &std::path::Path) -> Command {
    #[cfg(target_os = "linux")]
    let mut command = Command::new(path);
    #[cfg(not(target_os = "linux"))]
    let command = Command::new(path);
    #[cfg(target_os = "linux")]
    {
        let mount = std::env::var_os("APPDIR").map(PathBuf::from);
        for name in ["APPDIR", "APPIMAGE", "ARGV0", "OWD"] {
            command.env_remove(name);
        }
        if let Some(mount) = mount {
            for name in [
                "PATH",
                "LD_LIBRARY_PATH",
                "LD_PRELOAD",
                "XDG_DATA_DIRS",
                "GIO_EXTRA_MODULES",
                "GTK_PATH",
                "QT_PLUGIN_PATH",
            ] {
                if let Some(value) = std::env::var_os(name) {
                    match without_mount_paths(&value, &mount) {
                        Some(value) => {
                            command.env(name, value);
                        }
                        None => {
                            command.env_remove(name);
                        }
                    }
                }
            }
            for name in ["GSETTINGS_SCHEMA_DIR", "QT_QPA_PLATFORM_PLUGIN_PATH"] {
                if std::env::var_os(name)
                    .is_some_and(|value| PathBuf::from(value).starts_with(&mount))
                {
                    command.env_remove(name);
                }
            }
        }
    }
    command
}

#[cfg(target_os = "linux")]
fn without_mount_paths(
    value: &std::ffi::OsStr,
    mount: &std::path::Path,
) -> Option<std::ffi::OsString> {
    let paths: Vec<_> = std::env::split_paths(value)
        .filter(|path| !path.starts_with(mount))
        .collect();
    if paths.is_empty() {
        None
    } else {
        std::env::join_paths(paths).ok()
    }
}

fn validate_version(path: &std::path::Path, expected: &str) -> Result<(), InstallError> {
    // A concurrent fork can briefly hold the just-written file open, so exec
    // reports ETXTBSY ("text file busy"). Retry only that transient error.
    let mut child = {
        let mut attempts = 0;
        loop {
            match appimage_command(path)
                .arg("--version")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => break child,
                Err(error) if error.raw_os_error() == Some(26) && attempts < 50 => {
                    attempts += 1;
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return Err(InstallError::Restart),
            }
        }
    };
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
    fn restart_environment_removes_only_paths_from_the_old_mount() {
        let value =
            std::ffi::OsStr::new("/old/.mount_tik/usr/lib:/usr/lib:/old/.mount_tik-extra/lib");
        let cleaned = without_mount_paths(value, std::path::Path::new("/old/.mount_tik")).unwrap();
        assert_eq!(cleaned, "/usr/lib:/old/.mount_tik-extra/lib");
        assert!(without_mount_paths(
            std::ffi::OsStr::new("/old/.mount_tik/usr/lib"),
            std::path::Path::new("/old/.mount_tik")
        )
        .is_none());
    }

    #[test]
    #[ignore = "requires a real AppImage and FUSE; set TIKTOOLS_TEST_APPIMAGE"]
    fn preflight_runs_a_real_appimage() {
        let source = PathBuf::from(
            std::env::var_os("TIKTOOLS_TEST_APPIMAGE").expect("set TIKTOOLS_TEST_APPIMAGE"),
        );
        let bytes = std::fs::read(&source).unwrap();
        let root = tempfile::tempdir_in(source.parent().unwrap()).unwrap();
        let current = root.path().join("current.AppImage");
        std::fs::write(&current, b"old image").unwrap();
        assert!(preflight_appimage(&bytes, &current, "4.0.0").is_ok());
        assert!(preflight_appimage(&bytes, &current, "4.0.1").is_err());
        assert_eq!(std::fs::read(&current).unwrap(), b"old image");
    }

    #[test]
    #[ignore = "requires a real AppImage, GUI display and FUSE; set TIKTOOLS_TEST_APPIMAGE"]
    fn packager_replaces_a_running_disposable_appimage_without_touching_its_profile() {
        use std::os::unix::fs::MetadataExt;
        struct Running(std::process::Child);
        impl Drop for Running {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let source = PathBuf::from(
            std::env::var_os("TIKTOOLS_TEST_APPIMAGE").expect("set TIKTOOLS_TEST_APPIMAGE"),
        );
        let root = tempfile::tempdir_in(source.parent().unwrap()).unwrap();
        let image = root.path().join("TikTools.AppImage");
        let profile_root = tempfile::Builder::new()
            .prefix("tik-smoke-")
            .tempdir()
            .unwrap();
        let profile = profile_root.path().join("profile");
        let empty_plugins = root.path().join("empty-plugins");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::create_dir_all(&empty_plugins).unwrap();
        std::fs::copy(&source, &image).unwrap();
        let marker = profile.join("user-data-marker");
        std::fs::write(&marker, b"preserve").unwrap();
        let original_inode = std::fs::metadata(&image).unwrap().ino();
        let mut command = Command::new(&image);
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("TIKTOOLS_") {
                command.env_remove(name);
            }
        }
        let mut running = Running(
            command
                .env("TIKTOOLS_HOME", &profile)
                .env("TIKTOOLS_BUILTIN_PLUGINS_DIR", &empty_plugins)
                .env("XDG_CONFIG_HOME", root.path().join("config"))
                .env("XDG_DATA_HOME", root.path().join("data"))
                .env("XDG_CACHE_HOME", root.path().join("cache"))
                .env("XDG_STATE_HOME", root.path().join("state"))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            assert!(
                running.0.try_wait().unwrap().is_none(),
                "AppImage exited before frontend was ready"
            );
            let log =
                std::fs::read_to_string(profile.join("logs/tiktools.log")).unwrap_or_default();
            if log.contains("WebView frontend-ready") {
                break;
            }
            assert!(Instant::now() < deadline, "frontend did not become ready");
            std::thread::sleep(Duration::from_millis(100));
        }
        // Test-only key; no production trust material or network is involved.
        let key_path = root.path().join("fixture.key");
        let password = "ephemeral-test-password";
        let generated = Command::new("cargo")
            .args(["packager", "signer", "generate", "--ci", "--path"])
            .arg(&key_path)
            .env("CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD", password)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(generated.success(), "cargo-packager signer is required");
        let private_key = std::fs::read_to_string(&key_path).unwrap();
        let public_key = std::fs::read_to_string(root.path().join("fixture.key.pub")).unwrap();
        let candidate = root.path().join("candidate.AppImage");
        std::fs::copy(&source, &candidate).unwrap();
        let sign = |path: &std::path::Path| {
            let result = Command::new("cargo")
                .args(["packager", "signer", "sign"])
                .arg(path)
                .env("CARGO_PACKAGER_SIGN_PRIVATE_KEY", &private_key)
                .env("CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD", password)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(result.success(), "fixture signing failed");
        };
        sign(&candidate);
        let signature =
            std::fs::read_to_string(root.path().join("candidate.AppImage.sig")).unwrap();
        let manifest = serde_json::json!({"version":"4.0.0", "platforms": {"linux-x86_64": {
            "url":"https://github.com/nglmercer/TikTools-app/releases/download/v4.0.0/TikTools-4.0.0-linux-x86_64-installer.AppImage",
            "signature":signature.trim(), "format":"appimage"
        }}}).to_string();
        let manifest_path = root.path().join("manifest.json");
        std::fs::write(&manifest_path, &manifest).unwrap();
        sign(&manifest_path);
        let manifest_signature = std::fs::read(root.path().join("manifest.json.sig")).unwrap();
        let release = crate::authenticate_manifest(
            manifest.as_bytes(),
            &manifest_signature,
            crate::Channel::Stable,
            "linux-x86_64",
            &semver::Version::parse("3.9.0").unwrap(),
            &public_key,
        )
        .unwrap();
        let altered_manifest = format!("{manifest} ");
        assert!(crate::authenticate_manifest(
            altered_manifest.as_bytes(),
            &manifest_signature,
            crate::Channel::Stable,
            "linux-x86_64",
            &semver::Version::parse("3.9.0").unwrap(),
            &public_key,
        )
        .is_err());
        let bytes = std::fs::read(&candidate).unwrap();
        let mut corrupted = bytes.clone();
        corrupted[0] ^= 1;
        assert!(crate::download::verified_package(corrupted, &signature, &public_key).is_err());
        // Only this test uses loopback HTTP; production still enforces HTTPS and fixed hosts.
        let size = bytes.len() as u64;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            stream.write_all(&bytes).unwrap();
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut received = 0;
        let package = runtime.block_on(async {
            let response = reqwest::get(format!("http://{address}/fixture"))
                .await
                .unwrap();
            crate::download::verify_response(
                response,
                crate::MAX_UPDATE_BYTES,
                &mut |downloaded, _| received = downloaded,
                || {},
                &signature,
                &public_key,
            )
            .await
            .unwrap()
        });
        server.join().unwrap();
        assert_eq!(received, size);
        let plan = InstallPlan::new(
            Arc::new(release),
            package,
            image.clone(),
            "3.9.0".into(),
            InstallationType::AppImage,
        )
        .unwrap();
        preflight_appimage(&plan.bytes, &image, &plan.expected_version).unwrap();
        install_package(&plan.update, plan.bytes).unwrap();
        assert_ne!(std::fs::metadata(&image).unwrap().ino(), original_inode);
        assert!(
            running.0.try_wait().unwrap().is_none(),
            "old process exited during replacement"
        );
        assert_eq!(std::fs::read(&marker).unwrap(), b"preserve");
        validate_version(&image, "4.0.0").unwrap();
    }

    #[test]
    fn preflight_rejects_mismatched_binary_without_replacing_current_image() {
        let root = tempfile::tempdir().unwrap();
        let current = root.path().join("current.AppImage");
        std::fs::write(&current, b"old image").unwrap();
        let candidate = b"#!/bin/sh\nprintf '4.1.0\\n'\n";
        assert!(preflight_appimage(candidate, &current, "4.2.0").is_err());
        assert_eq!(std::fs::read(&current).unwrap(), b"old image");
        assert!(preflight_appimage(candidate, &current, "4.1.0").is_ok());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

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
