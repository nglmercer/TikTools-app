use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const STAGING_DIRECTORY: &str = ".staging";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreLayout {
    pub plugins: PathBuf,
    pub store: PathBuf,
}

impl StoreLayout {
    pub fn new(plugins: impl Into<PathBuf>, store: impl Into<PathBuf>) -> Self {
        Self {
            plugins: plugins.into(),
            store: store.into(),
        }
    }

    pub fn package(&self, plugin_id: &str) -> PathBuf {
        self.plugins.join(plugin_id)
    }

    pub fn staging(&self) -> PathBuf {
        self.plugins.join(STAGING_DIRECTORY)
    }

    pub fn receipts(&self) -> PathBuf {
        self.store.join("receipts")
    }

    pub fn receipt(&self, plugin_id: &str) -> PathBuf {
        self.receipts().join(format!("{plugin_id}.json"))
    }

    pub fn versions(&self, plugin_id: &str) -> PathBuf {
        self.store.join("versions").join(plugin_id)
    }

    pub fn retained(&self, plugin_id: &str, version: &str) -> PathBuf {
        self.versions(plugin_id).join(sanitize_version(version))
    }

    pub fn retained_receipt(&self, plugin_id: &str, version: &str) -> PathBuf {
        self.versions(plugin_id)
            .join(format!("{}.json", sanitize_version(version)))
    }

    pub fn cache(&self) -> PathBuf {
        self.store.join("cache")
    }

    pub fn registries(&self) -> PathBuf {
        self.store.join("registries")
    }

    pub fn registries_config(&self) -> PathBuf {
        self.store.join("registries.json")
    }

    pub fn new_staging_dir(&self, label: &str) -> io::Result<PathBuf> {
        let directory = self.staging().join(format!(
            "{label}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&directory)?;
        Ok(directory)
    }
}

pub(crate) fn sanitize_version(version: &str) -> String {
    version
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '+' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub(crate) fn move_dir(source: &Path, destination: &Path) -> io::Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            if destination.exists() {
                return Err(rename_error);
            }
            copy_dir(source, destination).inspect_err(|_| {
                let _ = fs::remove_dir_all(destination);
            })?;
            fs::remove_dir_all(source)
        }
    }
}

fn copy_dir(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "plugin packages cannot contain symbolic links",
            ));
        }
        if kind.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
