//! Writable runtime paths shared by the Rust host and future persistence
//! services. Production paths do not depend on the process working directory.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub root: PathBuf,
    pub data: PathBuf,
    pub plugins: PathBuf,
    pub plugin_data: PathBuf,
    pub builtin_plugins: PathBuf,
    pub development_plugins: Option<PathBuf>,
    pub logs: PathBuf,
    pub temp: PathBuf,
}

impl AppPaths {
    pub fn from_environment() -> Self {
        let default_root = default_data_root().join("TikTools");
        let root = configured_path("TIKTOOLS_HOME", default_root);
        let builtin_plugins = env::var_os("TIKTOOLS_BUILTIN_PLUGINS_DIR")
            .map(PathBuf::from)
            .map(normalize)
            .unwrap_or_else(|| {
                executable_directory()
                    .map(|directory| resource_path(&directory, "plugins"))
                    .unwrap_or_else(|| root.join("builtin-plugins"))
            });
        let development_plugins = env::var_os("TIKTOOLS_DEV_PLUGINS_DIR")
            .map(PathBuf::from)
            .map(normalize)
            .or_else(|| {
                if !cfg!(debug_assertions) {
                    return None;
                }
                let path = env::current_dir().ok()?.join("plugins");
                path.exists().then_some(path)
            });

        Self {
            data: configured_path("TIKTOOLS_DATA_DIR", root.join("data")),
            plugins: configured_path("TIKTOOLS_PLUGINS_DIR", root.join("plugins")),
            plugin_data: configured_path("TIKTOOLS_PLUGIN_DATA_DIR", root.join("plugin-data")),
            logs: configured_path("TIKTOOLS_LOG_DIR", root.join("logs")),
            temp: configured_path("TIKTOOLS_TEMP_DIR", root.join("temp")),
            root,
            builtin_plugins,
            development_plugins,
        }
    }

    pub fn ensure_directories(&self) -> io::Result<()> {
        for path in [
            &self.root,
            &self.data,
            &self.plugins,
            &self.plugin_data,
            &self.logs,
            &self.temp,
        ] {
            if !path.is_absolute() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("application path must be absolute: {}", path.display()),
                ));
            }
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    pub fn plugin_store(&self) -> PathBuf {
        configured_path("TIKTOOLS_PLUGIN_STORE_DIR", self.root.join("plugin-store"))
    }

    pub fn points_database(&self) -> PathBuf {
        self.data.join("tiktok-points.db")
    }

    pub fn automation_database(&self) -> PathBuf {
        self.data.join("tiktok-automation.db")
    }

    /// Aggregate-only analytics store. Counters, never chat text.
    pub fn analytics_database(&self) -> PathBuf {
        self.data.join("tiktok-analytics.db")
    }
}

fn configured_path(variable: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(normalize)
        .unwrap_or_else(|| normalize(fallback))
}

fn normalize(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn executable_directory() -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

/// Locate immutable packaged resources relative to the executable. Portable
/// and Windows packages put them alongside the binary; cargo-packager's DEB
/// and AppImage layouts put them under `<prefix>/lib/tiktools-desktop`.
pub fn resource_path(executable_directory: &Path, name: &str) -> PathBuf {
    let adjacent = executable_directory.join(name);
    if adjacent.exists() {
        return adjacent;
    }
    if executable_directory
        .file_name()
        .is_some_and(|part| part == "bin")
    {
        if let Some(prefix) = executable_directory.parent() {
            let installed = prefix.join("lib").join("tiktools-desktop").join(name);
            if installed.exists() {
                return installed;
            }
        }
    }
    adjacent
}

fn default_data_root() -> PathBuf {
    if cfg!(target_os = "windows") {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_directory().join("AppData").join("Local"))
    } else if cfg!(target_os = "macos") {
        home_directory().join("Library").join("Application Support")
    } else {
        env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_directory().join(".local").join("share"))
    }
}

fn home_directory() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_database_names_are_compatible() {
        let paths = AppPaths {
            root: PathBuf::from("/tmp/TikTools"),
            data: PathBuf::from("/tmp/TikTools/data"),
            plugins: PathBuf::from("/tmp/TikTools/plugins"),
            plugin_data: PathBuf::from("/tmp/TikTools/plugin-data"),
            builtin_plugins: PathBuf::from("/tmp/TikTools/builtin-plugins"),
            development_plugins: None,
            logs: PathBuf::from("/tmp/TikTools/logs"),
            temp: PathBuf::from("/tmp/TikTools/temp"),
        };
        assert!(paths.points_database().ends_with("data/tiktok-points.db"));
        assert!(paths
            .automation_database()
            .ends_with("data/tiktok-automation.db"));
        assert!(paths
            .analytics_database()
            .ends_with("data/tiktok-analytics.db"));
    }

    #[test]
    fn resolves_portable_and_packager_resource_layouts() {
        let root =
            std::env::temp_dir().join(format!("tiktools-resource-test-{}", fastrand::u64(..)));
        let bin = root.join("usr/bin");
        let installed = root.join("usr/lib/tiktools-desktop/web");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&installed).unwrap();
        assert_eq!(resource_path(&bin, "web"), installed);
        std::fs::create_dir_all(bin.join("web")).unwrap();
        assert_eq!(resource_path(&bin, "web"), bin.join("web"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
