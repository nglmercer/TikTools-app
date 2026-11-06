use serde::{Deserialize, Serialize};
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
    pub(crate) target: Option<PathBuf>,
}

impl DetectedInstallation {
    pub fn kind(&self) -> InstallationType {
        self.kind
    }
}

pub fn detect_installation() -> DetectedInstallation {
    #[cfg(target_os = "windows")]
    {
        if let (Ok(executable), Some(directory)) =
            (std::env::current_exe(), nsis_install_directory())
        {
            if let Some(target) = validated_nsis_install(&executable, &directory) {
                return DetectedInstallation {
                    kind: InstallationType::Nsis,
                    target: Some(target),
                };
            }
        }
        if std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .is_some_and(|dir| dir.join("web/index.html").is_file() && dir.join("plugins").is_dir())
        {
            return DetectedInstallation {
                kind: InstallationType::Portable,
                target: None,
            };
        }
    }
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
                    target: Some(path),
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
                target: None,
            };
        }
        if std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .is_some_and(|dir| dir.join("web/index.html").is_file() && dir.join("plugins").is_dir())
        {
            return DetectedInstallation {
                kind: InstallationType::Portable,
                target: None,
            };
        }
    }
    DetectedInstallation {
        kind: if cfg!(target_os = "android") {
            InstallationType::Unsupported
        } else {
            InstallationType::Unknown
        },
        target: None,
    }
}

#[cfg(target_os = "windows")]
fn nsis_install_directory() -> Option<PathBuf> {
    read_install_directory(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\TikTools")
}

#[cfg(target_os = "windows")]
fn read_install_directory(registry_key: &str) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::{
        Foundation::ERROR_SUCCESS,
        System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ},
    };
    let key: Vec<u16> = registry_key.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = "InstallLocation".encode_utf16().chain(Some(0)).collect();
    let mut buffer = [0u16; 32768];
    let mut size = (buffer.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if result != ERROR_SUCCESS || size == 0 || size as usize > buffer.len() * 2 || size % 2 != 0 {
        return None;
    }
    let mut value = &buffer[..size as usize / 2];
    while value.last() == Some(&0) {
        value = &value[..value.len() - 1];
    }
    if value.len() >= 2 && value[0] == 34 && value[value.len() - 1] == 34 {
        value = &value[1..value.len() - 1];
    }
    if value.is_empty() || value.contains(&0) {
        return None;
    }
    let path = PathBuf::from(std::ffi::OsString::from_wide(value));
    path.is_absolute().then_some(path)
}

#[cfg(any(target_os = "windows", test))]
fn validated_nsis_install(executable: &Path, directory: &Path) -> Option<PathBuf> {
    if !directory.is_absolute()
        || !directory.join("uninstall.exe").is_file()
        || !directory.join("web/index.html").is_file()
        || !directory.join("plugins").is_dir()
    {
        return None;
    }
    let executable = executable.canonicalize().ok()?;
    (directory.join("tiktools-desktop.exe").canonicalize().ok()? == executable)
        .then_some(executable)
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

#[cfg(test)]
mod nsis_tests {
    use super::*;

    #[test]
    fn nsis_detection_requires_the_registered_executable_and_resources() {
        let root =
            std::env::temp_dir().join(format!("tiktools-nsis-detection-{}", std::process::id()));
        std::fs::create_dir_all(root.join("web")).unwrap();
        std::fs::create_dir_all(root.join("plugins")).unwrap();
        let executable = root.join("tiktools-desktop.exe");
        std::fs::write(&executable, b"binary").unwrap();
        std::fs::write(root.join("web/index.html"), b"web").unwrap();
        assert!(validated_nsis_install(&executable, &root).is_none());
        std::fs::write(root.join("uninstall.exe"), b"uninstaller").unwrap();
        assert_eq!(
            validated_nsis_install(&executable, &root),
            executable.canonicalize().ok()
        );
        let other = root.join("other.exe");
        std::fs::write(&other, b"portable").unwrap();
        assert!(validated_nsis_install(&other, &root).is_none());
        std::fs::remove_file(root.join("web/index.html")).unwrap();
        assert!(validated_nsis_install(&executable, &root).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, target_os = "windows"))]
mod registry_tests {
    use super::*;
    use windows_sys::Win32::{
        Foundation::ERROR_SUCCESS,
        System::Registry::{
            RegCloseKey, RegCreateKeyW, RegDeleteKeyW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
            REG_SZ,
        },
    };

    #[test]
    fn reads_quoted_unicode_install_location_from_registry() {
        let key_name = format!("Software\\TikToolsUpdaterTest_{}", std::process::id());
        let wide_key: Vec<u16> = key_name.encode_utf16().chain(Some(0)).collect();
        let value_name: Vec<u16> = "InstallLocation".encode_utf16().chain(Some(0)).collect();
        let value: Vec<u16> = "\"C:\\Users\\Tester\\Tik Tools ñ\""
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut handle: HKEY = std::ptr::null_mut();
        unsafe {
            assert_eq!(
                RegCreateKeyW(HKEY_CURRENT_USER, wide_key.as_ptr(), &mut handle),
                ERROR_SUCCESS
            );
            assert_eq!(
                RegSetValueExW(
                    handle,
                    value_name.as_ptr(),
                    0,
                    REG_SZ,
                    value.as_ptr().cast(),
                    (value.len() * 2) as u32
                ),
                ERROR_SUCCESS
            );
            assert_eq!(RegCloseKey(handle), ERROR_SUCCESS);
        }
        assert_eq!(
            read_install_directory(&key_name),
            Some(PathBuf::from("C:\\Users\\Tester\\Tik Tools ñ"))
        );
        unsafe {
            assert_eq!(
                RegDeleteKeyW(HKEY_CURRENT_USER, wide_key.as_ptr()),
                ERROR_SUCCESS
            );
        }
        assert!(read_install_directory(&key_name).is_none());
    }
}
