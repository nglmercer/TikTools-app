use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;

/// Detection must fail closed: a portable binary must never be assumed to be
/// an installed application merely because it runs on a supported OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallationType {
    Unknown,
    Portable,
    Nsis,
    AppImage,
    Deb,
    Unsupported,
}

impl InstallationType {
    pub fn automatic_install_supported(self) -> bool {
        matches!(self, Self::Nsis | Self::AppImage)
    }
}

/// A validated on-disk installation target. Paths never originate from RPC.
#[derive(Debug, Clone)]
pub struct DetectedInstallation {
    pub(crate) kind: InstallationType,
    pub(crate) appimage: Option<PathBuf>,
}

impl DetectedInstallation {
    pub fn kind(&self) -> InstallationType {
        self.kind
    }
}

pub fn detect_installation() -> DetectedInstallation {
    #[cfg(target_os = "linux")]
    {
        if let (Ok(executable), Some(appdir), Some(image), Ok(mounts)) = (
            std::env::current_exe(),
            std::env::var_os("APPDIR"),
            std::env::var_os("APPIMAGE"),
            std::fs::read_to_string("/proc/self/mountinfo"),
        ) {
            if let Some(path) =
                validated_appimage(&executable, Path::new(&appdir), Path::new(&image), &mounts)
            {
                return DetectedInstallation {
                    kind: InstallationType::AppImage,
                    appimage: Some(path),
                };
            }
        }
        if std::env::current_exe().ok().as_deref() == Some(Path::new("/usr/bin/tiktools-desktop"))
            && std::fs::read_to_string("/var/lib/dpkg/info/tik-tools.list")
                .ok()
                .is_some_and(|list| list.lines().any(|line| line == "/usr/bin/tiktools-desktop"))
        {
            return DetectedInstallation {
                kind: InstallationType::Deb,
                appimage: None,
            };
        }
        if std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .is_some_and(|dir| dir.join("web/index.html").is_file() && dir.join("plugins").is_dir())
        {
            return DetectedInstallation {
                kind: InstallationType::Portable,
                appimage: None,
            };
        }
    }
    DetectedInstallation {
        kind: if cfg!(target_os = "android") {
            InstallationType::Unsupported
        } else {
            InstallationType::Unknown
        },
        appimage: None,
    }
}

#[cfg(target_os = "linux")]
fn validated_appimage(
    executable: &Path,
    appdir: &Path,
    image: &Path,
    mountinfo: &str,
) -> Option<PathBuf> {
    use std::io::Read;
    if !appdir.file_name()?.to_str()?.starts_with(".mount_") || !image.is_absolute() {
        return None;
    }
    let metadata = std::fs::symlink_metadata(image).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    let appdir = appdir.canonicalize().ok()?;
    let executable = executable.canonicalize().ok()?;
    if appdir
        .join("usr/bin/tiktools-desktop")
        .canonicalize()
        .ok()?
        != executable
    {
        return None;
    }
    let image_name = image.file_name()?.to_str()?;
    let mounted = mountinfo.lines().any(|line| {
        let Some((fields, filesystem)) = line.split_once(" - ") else {
            return false;
        };
        let Some(point) = fields.split_whitespace().nth(4) else {
            return false;
        };
        let point = point
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\");
        let mut fields = filesystem.split_whitespace();
        let kind = fields.next().unwrap_or_default();
        let source = fields
            .next()
            .unwrap_or_default()
            .replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\");
        Path::new(&point) == appdir
            && (kind == "fuse" || kind.starts_with("fuse."))
            && source == image_name
    });
    if !mounted {
        return None;
    }
    let mut header = [0u8; 12];
    std::fs::File::open(image)
        .ok()?
        .read_exact(&mut header)
        .ok()?;
    if &header[..4] != b"\x7fELF" || &header[8..11] != b"AI\x02" {
        return None;
    }
    image.canonicalize().ok()
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn appimage_detection_requires_matching_executable_mount_and_image() {
        let root =
            std::env::temp_dir().join(format!("tiktools-appimage-detect-{}", std::process::id()));
        let appdir = root.join(".mount_tiktools");
        std::fs::create_dir_all(appdir.join("usr/bin")).unwrap();
        let executable = appdir.join("usr/bin/tiktools-desktop");
        let image = root.join("TikTools.AppImage");
        std::fs::write(&executable, b"fixture").unwrap();
        std::fs::write(&image, b"\x7fELF\0\0\0\0AI\x02\0").unwrap();
        let mounts = format!(
            "1 0 0:1 / {} rw - fuse.squashfuse TikTools.AppImage rw",
            appdir.display()
        );
        assert_eq!(
            validated_appimage(&executable, &appdir, &image, &mounts),
            Some(image.clone())
        );
        assert!(validated_appimage(&executable, &appdir, &image, "").is_none());
        assert!(validated_appimage(
            &executable,
            &appdir,
            &image,
            &mounts.replace("TikTools.AppImage", "other.AppImage")
        )
        .is_none());
        assert!(validated_appimage(&image, &appdir, &image, &mounts).is_none());
        std::fs::write(&image, b"not an AppImage").unwrap();
        assert!(validated_appimage(&executable, &appdir, &image, &mounts).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
