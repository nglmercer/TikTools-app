//! Plugin-owned persistence: typed document collections, settings and a
//! bounded file area, all rooted in one private directory.
//!
//! One process owns a store at a time (the plugin process). Every write is
//! atomic (temporary file, `fsync`, rename) and checked against a revision, so
//! a stale writer receives [`StoreError::Conflict`] instead of silently
//! overwriting newer data. Revisions come from a per-collection sequence that
//! never goes backwards, so a record that is deleted and recreated never reuses
//! an old revision.
//!
//! ```no_run
//! # use tiktools_plugin_sdk::store::{Expect, Store};
//! # use serde::{Deserialize, Serialize};
//! #[derive(Serialize, Deserialize, Default)]
//! struct Rule { enabled: bool }
//!
//! let store = Store::open("/tmp/plugin-data/store")?;
//! let rules = store.collection::<Rule>("rules")?;
//! let saved = rules.put("welcome", &Rule { enabled: true }, Expect::Absent)?;
//! rules.put("welcome", &Rule { enabled: false }, Expect::Revision(saved.revision))?;
//! # Ok::<(), tiktools_plugin_sdk::store::StoreError>(())
//! ```

use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    marker::PhantomData,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::{data_dir, PluginError, PluginResult};

/// On-disk format of every collection and settings file.
pub const FORMAT_VERSION: u32 = 1;
/// Largest encoded value of one record.
pub const MAX_RECORD_BYTES: usize = 256 * 1024;
/// Largest collection file, and largest single file in the file area.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Longest accepted record id.
pub const MAX_ID_BYTES: usize = 256;
/// Largest accepted relative path inside the file area.
pub const MAX_PATH_BYTES: usize = 512;

const SETTINGS_ID: &str = "settings";
const FILES_DIR: &str = "files";

/// Errors from the plugin store. Their display text is what plugins see.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("invalid collection name `{0}`: use 1-64 lowercase letters, digits, `-` or `_`, starting with a letter")]
    InvalidName(String),
    #[error("invalid record id: {0}")]
    InvalidId(String),
    #[error("invalid file path `{0}`: use relative paths made of normal components")]
    InvalidPath(String),
    #[error("record `{id}` changed: expected {expected:?}, found revision {found:?}")]
    Conflict {
        id: String,
        expected: Expect,
        found: Option<u64>,
    },
    #[error("{what} exceeds the {limit}-byte limit")]
    TooLarge { what: &'static str, limit: u64 },
    #[error("store format {0} is not supported by this plugin SDK")]
    UnsupportedFormat(u32),
    #[error("store data is damaged: {0}")]
    Corrupt(String),
    #[error("store I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("store value could not be encoded or decoded: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<StoreError> for PluginError {
    fn from(error: StoreError) -> Self {
        PluginError::other(error.to_string())
    }
}

/// Precondition for a write, checked against the record's current revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// Write regardless of the current state.
    Any,
    /// Only if the record does not exist.
    Absent,
    /// Only if the record exists at exactly this revision.
    Revision(u64),
}

/// A stored value together with the revision it was written at.
#[derive(Debug, Clone, PartialEq)]
pub struct Record<T> {
    pub id: String,
    pub revision: u64,
    pub value: T,
}

/// Root of one plugin's private store.
#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    /// Opens (creating if needed) the store rooted at `dir`.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let dir = dir.into();
        create_private_dir(&dir)?;
        Ok(Self { dir })
    }

    /// Opens the store under the plugin's data directory, at `<data>/store`.
    pub fn from_plugin_data() -> PluginResult<Self> {
        let root = data_dir()?;
        Self::open(root.join("store")).map_err(PluginError::from)
    }

    /// Typed collection `name`. The name is validated; the file is created on
    /// the first write.
    pub fn collection<T>(&self, name: &str) -> Result<Collection<T>, StoreError>
    where
        T: Serialize + DeserializeOwned,
    {
        validate_name(name)?;
        Ok(Collection {
            path: self.dir.join(format!("{name}.json")),
            _value: PhantomData,
        })
    }

    /// Settings object `name`: one value with a default, read and written as a
    /// whole.
    pub fn settings<T>(&self, name: &str) -> Result<Settings<T>, StoreError>
    where
        T: Default + Serialize + DeserializeOwned,
    {
        Ok(Settings {
            collection: self.collection::<T>(&format!("settings-{name}"))?,
        })
    }

    /// Bounded area for arbitrary files (binary or text), addressed by relative
    /// path.
    pub fn files(&self) -> Files {
        Files {
            root: self.dir.join(FILES_DIR),
        }
    }
}

/// A typed collection of records keyed by id.
#[derive(Debug)]
pub struct Collection<T> {
    path: PathBuf,
    _value: PhantomData<fn() -> T>,
}

impl<T> Collection<T>
where
    T: Serialize + DeserializeOwned,
{
    /// The record with `id`, if it exists.
    pub fn get(&self, id: &str) -> Result<Option<Record<T>>, StoreError> {
        validate_id(id)?;
        let file = self.read_file()?;
        file.records
            .get(id)
            .map(|stored| decode_record(id, stored))
            .transpose()
    }

    /// Every record, ordered by id.
    pub fn list(&self) -> Result<Vec<Record<T>>, StoreError> {
        let file = self.read_file()?;
        file.records
            .iter()
            .map(|(id, stored)| decode_record(id, stored))
            .collect()
    }

    /// Records whose value satisfies `predicate`, ordered by id.
    pub fn query(
        &self,
        mut predicate: impl FnMut(&T) -> bool,
    ) -> Result<Vec<Record<T>>, StoreError> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|record| predicate(&record.value))
            .collect())
    }

    /// Writes `value` under `id` if `expect` holds. Returns the new record.
    pub fn put(&self, id: &str, value: &T, expect: Expect) -> Result<Record<T>, StoreError> {
        validate_id(id)?;
        let encoded = serde_json::to_value(value)?;
        if serde_json::to_vec(&encoded)?.len() > MAX_RECORD_BYTES {
            return Err(StoreError::TooLarge {
                what: "record",
                limit: MAX_RECORD_BYTES as u64,
            });
        }
        let mut file = self.read_file()?;
        check(
            id,
            expect,
            file.records.get(id).map(|stored| stored.revision),
        )?;
        file.sequence += 1;
        let revision = file.sequence;
        file.records.insert(
            id.to_owned(),
            StoredRecord {
                revision,
                value: encoded.clone(),
            },
        );
        self.write_file(&file)?;
        Ok(Record {
            id: id.to_owned(),
            revision,
            value: serde_json::from_value(encoded)?,
        })
    }

    /// Removes `id` if `expect` holds. Returns whether a record was removed.
    pub fn delete(&self, id: &str, expect: Expect) -> Result<bool, StoreError> {
        validate_id(id)?;
        let mut file = self.read_file()?;
        let current = file.records.get(id).map(|stored| stored.revision);
        check(id, expect, current)?;
        if current.is_none() {
            return Ok(false);
        }
        file.records.remove(id);
        file.sequence += 1;
        self.write_file(&file)?;
        Ok(true)
    }

    fn read_file(&self) -> Result<CollectionFile, StoreError> {
        let Some(bytes) = read_bounded(&self.path, MAX_FILE_BYTES, "collection file")? else {
            return Ok(CollectionFile::empty());
        };
        let file: CollectionFile = serde_json::from_slice(&bytes)
            .map_err(|error| StoreError::Corrupt(error.to_string()))?;
        if file.format_version != FORMAT_VERSION {
            return Err(StoreError::UnsupportedFormat(file.format_version));
        }
        Ok(file)
    }

    fn write_file(&self, file: &CollectionFile) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec(file)?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(StoreError::TooLarge {
                what: "collection",
                limit: MAX_FILE_BYTES,
            });
        }
        write_atomic(&self.path, &bytes)?;
        Ok(())
    }
}

/// One settings object with a default value.
#[derive(Debug)]
pub struct Settings<T> {
    collection: Collection<T>,
}

impl<T> Settings<T>
where
    T: Default + Serialize + DeserializeOwned,
{
    /// The stored settings, or `T::default()` when none were saved yet.
    pub fn get(&self) -> Result<T, StoreError> {
        Ok(self
            .collection
            .get(SETTINGS_ID)?
            .map(|record| record.value)
            .unwrap_or_default())
    }

    /// Replaces the settings if `expect` holds.
    pub fn save(&self, value: &T, expect: Expect) -> Result<Record<T>, StoreError> {
        self.collection.put(SETTINGS_ID, value, expect)
    }

    /// Applies `change` to the current settings and saves the result. Fails
    /// with a conflict if another writer changed the settings in between.
    pub fn update(&self, change: impl FnOnce(&mut T)) -> Result<Record<T>, StoreError> {
        let (mut value, expect) = match self.collection.get(SETTINGS_ID)? {
            Some(record) => (record.value, Expect::Revision(record.revision)),
            None => (T::default(), Expect::Absent),
        };
        change(&mut value);
        self.collection.put(SETTINGS_ID, &value, expect)
    }
}

/// Bounded file area. Paths are relative and cannot leave the area, including
/// through symbolic links.
#[derive(Debug, Clone)]
pub struct Files {
    root: PathBuf,
}

impl Files {
    /// Writes `bytes` to `relative`, creating parent directories as needed.
    pub fn write(&self, relative: &str, bytes: &[u8]) -> Result<(), StoreError> {
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(StoreError::TooLarge {
                what: "file",
                limit: MAX_FILE_BYTES,
            });
        }
        let path = self.resolve(relative)?;
        // Check before creating anything: creating directories through a symlink
        // would otherwise write outside the area even if the file is refused.
        self.ensure_inside(&path, relative)?;
        create_private_dir(path.parent().ok_or_else(|| invalid_path(relative))?)?;
        write_atomic(&path, bytes)?;
        Ok(())
    }

    /// Reads `relative`, or `None` when it does not exist.
    pub fn read(&self, relative: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let path = self.resolve(relative)?;
        self.ensure_inside(&path, relative)?;
        if !path.exists() {
            return Ok(None);
        }
        read_bounded(&path, MAX_FILE_BYTES, "file")
    }

    /// Removes `relative`. Returns whether a file was removed.
    pub fn remove(&self, relative: &str) -> Result<bool, StoreError> {
        let path = self.resolve(relative)?;
        self.ensure_inside(&path, relative)?;
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(&path)?;
        Ok(true)
    }

    /// Relative paths of every regular file in the area, sorted, using `/`.
    pub fn list(&self) -> Result<Vec<String>, StoreError> {
        let mut found = Vec::new();
        if self.root.exists() {
            collect_files(&self.root, &self.root, &mut found)?;
        }
        found.sort();
        Ok(found)
    }

    fn resolve(&self, relative: &str) -> Result<PathBuf, StoreError> {
        if relative.is_empty() || relative.len() > MAX_PATH_BYTES || relative.contains(['\\', '\0'])
        {
            return Err(invalid_path(relative));
        }
        let mut path = self.root.clone();
        for component in Path::new(relative).components() {
            match component {
                Component::Normal(part) => path.push(part),
                _ => return Err(invalid_path(relative)),
            }
        }
        if path == self.root {
            return Err(invalid_path(relative));
        }
        Ok(path)
    }

    /// Rejects a path whose existing ancestors resolve outside the area.
    fn ensure_inside(&self, path: &Path, relative: &str) -> Result<(), StoreError> {
        let root = match fs::canonicalize(&self.root) {
            Ok(root) => root,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let mut probe = path.to_path_buf();
        loop {
            match fs::canonicalize(&probe) {
                Ok(real) => {
                    return if real.starts_with(&root) {
                        Ok(())
                    } else {
                        Err(invalid_path(relative))
                    };
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    if !probe.pop() {
                        return Err(invalid_path(relative));
                    }
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CollectionFile {
    format_version: u32,
    sequence: u64,
    records: BTreeMap<String, StoredRecord>,
}

impl CollectionFile {
    fn empty() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            sequence: 0,
            records: BTreeMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredRecord {
    revision: u64,
    value: serde_json::Value,
}

fn decode_record<T: DeserializeOwned>(
    id: &str,
    stored: &StoredRecord,
) -> Result<Record<T>, StoreError> {
    Ok(Record {
        id: id.to_owned(),
        revision: stored.revision,
        value: serde_json::from_value(stored.value.clone())?,
    })
}

fn check(id: &str, expect: Expect, current: Option<u64>) -> Result<(), StoreError> {
    let satisfied = match expect {
        Expect::Any => true,
        Expect::Absent => current.is_none(),
        Expect::Revision(revision) => current == Some(revision),
    };
    if satisfied {
        Ok(())
    } else {
        Err(StoreError::Conflict {
            id: id.to_owned(),
            expected: expect,
            found: current,
        })
    }
}

fn validate_name(name: &str) -> Result<(), StoreError> {
    let valid = (1..=64).contains(&name.len())
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidName(name.to_owned()))
    }
}

fn validate_id(id: &str) -> Result<(), StoreError> {
    if id.is_empty() {
        return Err(StoreError::InvalidId("must not be empty".into()));
    }
    if id.len() > MAX_ID_BYTES {
        return Err(StoreError::InvalidId(format!(
            "must be at most {MAX_ID_BYTES} bytes"
        )));
    }
    if id.contains('\0') {
        return Err(StoreError::InvalidId("must not contain NUL".into()));
    }
    Ok(())
}

fn invalid_path(relative: &str) -> StoreError {
    StoreError::InvalidPath(relative.to_owned())
}

fn read_bounded(
    path: &Path,
    limit: u64,
    what: &'static str,
) -> Result<Option<Vec<u8>>, StoreError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(StoreError::TooLarge { what, limit });
    }
    Ok(Some(bytes))
}

/// Writes through a uniquely named temporary file, then renames it over `path`.
/// A crash leaves either the old or the new file, never a partial one.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let temporary = parent.join(format!(".{name}.{}.{nonce}.tmp", std::process::id()));
    let result = (|| {
        let mut file = private_file_options()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        sync_directory(parent);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn collect_files(root: &Path, dir: &Path, found: &mut Vec<String>) -> Result<(), StoreError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_files(root, &path, found)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| io::Error::other("entry outside file area"))?;
            let text = relative
                .components()
                .filter_map(|part| part.as_os_str().to_str())
                .collect::<Vec<_>>()
                .join("/");
            found.push(text);
        }
    }
    Ok(())
}

fn create_private_dir(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn private_file_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

fn sync_directory(dir: &Path) {
    #[cfg(unix)]
    {
        if let Ok(handle) = File::open(dir) {
            let _ = handle.sync_all();
        }
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
    #[serde(deny_unknown_fields)]
    struct Rule {
        enabled: bool,
        label: String,
    }

    fn scratch(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!(
            "tiktools-store-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn rule(enabled: bool) -> Rule {
        Rule {
            enabled,
            label: "welcome".into(),
        }
    }

    #[test]
    fn put_get_list_round_trip_with_increasing_revisions() {
        let dir = scratch("roundtrip");
        let store = Store::open(&dir).unwrap();
        let rules = store.collection::<Rule>("rules").unwrap();
        let first = rules.put("b", &rule(true), Expect::Absent).unwrap();
        let second = rules.put("a", &rule(false), Expect::Any).unwrap();
        assert!(second.revision > first.revision);
        assert_eq!(rules.get("b").unwrap().unwrap().value, rule(true));
        let ids: Vec<_> = rules.list().unwrap().into_iter().map(|r| r.id).collect();
        assert_eq!(ids, ["a", "b"]);
        let enabled = rules.query(|rule| rule.enabled).unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "b");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stale_and_duplicate_writers_get_conflicts() {
        let dir = scratch("conflicts");
        let store = Store::open(&dir).unwrap();
        let rules = store.collection::<Rule>("rules").unwrap();
        let saved = rules.put("r", &rule(true), Expect::Absent).unwrap();
        assert!(matches!(
            rules.put("r", &rule(false), Expect::Absent),
            Err(StoreError::Conflict { .. })
        ));
        assert!(matches!(
            rules.put("r", &rule(false), Expect::Revision(saved.revision + 1)),
            Err(StoreError::Conflict { .. })
        ));
        let updated = rules
            .put("r", &rule(false), Expect::Revision(saved.revision))
            .unwrap();
        assert_eq!(rules.get("r").unwrap().unwrap().revision, updated.revision);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn deleted_then_recreated_records_never_reuse_a_revision() {
        let dir = scratch("aba");
        let store = Store::open(&dir).unwrap();
        let rules = store.collection::<Rule>("rules").unwrap();
        let old = rules.put("r", &rule(true), Expect::Absent).unwrap();
        assert!(rules.delete("r", Expect::Revision(old.revision)).unwrap());
        let recreated = rules.put("r", &rule(true), Expect::Absent).unwrap();
        assert!(recreated.revision > old.revision);
        assert!(matches!(
            rules.delete("r", Expect::Revision(old.revision)),
            Err(StoreError::Conflict { .. })
        ));
        assert!(!rules.delete("missing", Expect::Any).unwrap());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn data_survives_reopening_the_store() {
        let dir = scratch("reopen");
        {
            let store = Store::open(&dir).unwrap();
            store
                .collection::<Rule>("rules")
                .unwrap()
                .put("kept", &rule(true), Expect::Absent)
                .unwrap();
        }
        let reopened = Store::open(&dir).unwrap();
        let kept = reopened
            .collection::<Rule>("rules")
            .unwrap()
            .get("kept")
            .unwrap();
        assert_eq!(kept.unwrap().value, rule(true));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_default_then_update_with_optimistic_check() {
        let dir = scratch("settings");
        let store = Store::open(&dir).unwrap();
        let settings = store.settings::<Rule>("overlay").unwrap();
        assert_eq!(settings.get().unwrap(), Rule::default());
        settings.update(|value| value.enabled = true).unwrap();
        let first = settings.get().unwrap();
        assert!(first.enabled);
        settings.save(&rule(false), Expect::Any).unwrap();
        assert!(!settings.get().unwrap().enabled);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_bad_collection_names_and_record_ids() {
        let dir = scratch("names");
        let store = Store::open(&dir).unwrap();
        for name in ["", "Rules", "../x", "1abc", "has space", &"a".repeat(65)] {
            assert!(
                matches!(
                    store.collection::<Rule>(name),
                    Err(StoreError::InvalidName(_))
                ),
                "accepted name {name:?}"
            );
        }
        let rules = store.collection::<Rule>("rules").unwrap();
        assert!(matches!(
            rules.put("", &rule(true), Expect::Any),
            Err(StoreError::InvalidId(_))
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn oversized_records_are_refused_before_writing() {
        let dir = scratch("limits");
        let store = Store::open(&dir).unwrap();
        let rules = store.collection::<Rule>("rules").unwrap();
        let huge = Rule {
            enabled: true,
            label: "x".repeat(MAX_RECORD_BYTES),
        };
        assert!(matches!(
            rules.put("big", &huge, Expect::Any),
            Err(StoreError::TooLarge { what: "record", .. })
        ));
        assert!(rules.list().unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_or_future_files_are_errors_not_silent_resets() {
        let dir = scratch("damaged");
        let store = Store::open(&dir).unwrap();
        let rules = store.collection::<Rule>("rules").unwrap();
        fs::write(dir.join("rules.json"), b"{not json").unwrap();
        assert!(matches!(rules.list(), Err(StoreError::Corrupt(_))));
        fs::write(
            dir.join("rules.json"),
            br#"{"formatVersion":2,"sequence":0,"records":{}}"#,
        )
        .unwrap();
        assert!(matches!(
            rules.list(),
            Err(StoreError::UnsupportedFormat(2))
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn file_area_round_trips_and_rejects_escaping_paths() {
        let dir = scratch("files");
        let store = Store::open(&dir).unwrap();
        let files = store.files();
        files.write("exports/one.bin", &[1, 2, 3]).unwrap();
        assert_eq!(files.read("exports/one.bin").unwrap(), Some(vec![1, 2, 3]));
        assert_eq!(files.list().unwrap(), ["exports/one.bin"]);
        assert!(files.remove("exports/one.bin").unwrap());
        assert_eq!(files.read("exports/one.bin").unwrap(), None);
        for bad in [
            "",
            "/etc/passwd",
            "../outside",
            "a/../b",
            "./x",
            "a\\b",
            "a/\0b",
        ] {
            assert!(
                matches!(files.write(bad, b"x"), Err(StoreError::InvalidPath(_))),
                "accepted path {bad:?}"
            );
            assert!(files.read(bad).is_err(), "read accepted {bad:?}");
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn file_area_refuses_symlinks_that_point_outside() {
        let dir = scratch("symlink");
        let outside = scratch("symlink-target");
        let store = Store::open(&dir).unwrap();
        let files = store.files();
        fs::create_dir_all(dir.join(FILES_DIR)).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join(FILES_DIR).join("escape")).unwrap();
        assert!(matches!(
            files.write("escape/planted.txt", b"x"),
            Err(StoreError::InvalidPath(_))
        ));
        assert!(!outside.join("planted.txt").exists());
        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(outside);
    }
}
