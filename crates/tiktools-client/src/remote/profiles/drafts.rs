//! Explicitly saved local text. Never a host command, credential or auto-send queue.
use super::{commands::Binding, directory, *};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub const TEXT_BYTES: usize = 256 * 1024;
const FILE_BYTES: u64 = 4 * 1024 * 1024;
const RECORDS: usize = 128;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub id: String,
    pub base_revision: String,
    pub text: String,
    pub updated_at: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Data {
    version: u32,
    binding: Binding,
    pub revision: String,
    pub records: BTreeMap<String, Draft>,
}
pub(in crate::remote) fn invalid() -> ClientError {
    fail(
        "draft_invalid",
        "saved drafts are invalid; existing data was preserved",
    )
}
fn unavailable() -> ClientError {
    fail(
        "draft_unavailable",
        "local draft storage is unavailable; keep your unsaved text",
    )
}
fn options() -> OpenOptions {
    let mut o = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    o
}
fn private(f: &File) -> Result<(), ClientError> {
    let m = f.metadata().map_err(|_| unavailable())?;
    if !m.is_file() {
        return Err(invalid());
    }
    directory::private(&m).map_err(|_| invalid())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if m.nlink() != 1 {
            return Err(invalid());
        }
    }
    Ok(())
}
pub(in crate::remote) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.trim() == id
        && !id.contains('*')
        && !id.chars().any(char::is_control)
}
impl Draft {
    fn validate(&self) -> Result<(), ClientError> {
        if !valid_id(&self.id)
            || self.text.len() > TEXT_BYTES
            || tiktools_collaboration::stream_sequence(&self.base_revision).is_err()
            || !(0..=9_007_199_254_740_991).contains(&self.updated_at)
        {
            return Err(invalid());
        }
        Ok(())
    }
}
pub(super) fn read(path: &Path, binding: &Binding) -> Result<Data, ClientError> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Data {
                version: 1,
                binding: binding.clone(),
                revision: "0".into(),
                records: BTreeMap::new(),
            })
        }
        Err(_) => return Err(unavailable()),
        Ok(m) if !m.is_file() => return Err(invalid()),
        _ => {}
    }
    let f = options().read(true).open(path).map_err(|_| invalid())?;
    private(&f)?;
    let mut bytes = Vec::new();
    f.take(FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() as u64 > FILE_BYTES {
        return Err(invalid());
    }
    let data: Data = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if data.version != 1
        || data.binding != *binding
        || data.records.len() > RECORDS
        || tiktools_collaboration::stream_sequence(&data.revision).is_err()
    {
        return Err(invalid());
    }
    for (id, draft) in &data.records {
        draft.validate()?;
        if id != &draft.id {
            return Err(invalid());
        }
    }
    Ok(data)
}
fn write(path: &Path, data: &Data) -> Result<(), ClientError> {
    let directory = path.parent().ok_or_else(invalid)?;
    directory::check_directory(directory, false).map_err(|_| unavailable())?;
    if fs::symlink_metadata(path).is_ok() {
        private(&options().read(true).open(path).map_err(|_| invalid())?)?;
    }
    let bytes = serde_json::to_vec(data).map_err(|_| invalid())?;
    if bytes.len() as u64 > FILE_BYTES {
        return Err(fail(
            "draft_limit",
            "local draft storage limit reached; existing drafts were preserved",
        ));
    }
    // One known private staging file per leased profile bounds leftover text
    // after repeated process death. Reads never promote an unconfirmed staging
    // copy; a subsequent explicit save may replace it.
    let staging = path.with_extension("drafts.tmp");
    let mut temp = options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&staging)
        .map_err(|_| unavailable())?;
    private(&temp)?;
    temp.set_len(0).map_err(|_| unavailable())?;
    temp.write_all(&bytes)
        .and_then(|_| temp.flush())
        .and_then(|_| temp.sync_all())
        .map_err(|_| unavailable())?;
    #[cfg(test)]
    crash("before-rename");
    drop(temp);
    fs::rename(&staging, path).map_err(|_| unavailable())?;
    #[cfg(test)]
    crash("after-rename");
    directory::sync_directory(directory).map_err(|_| unavailable())?;
    #[cfg(test)]
    crash("synced");
    Ok(())
}
/// Caller holds the journal's process lease and write mutex. Global CAS also
/// fences a delayed page save after delete/recreate without an unbounded ID set.
pub(super) fn change(
    path: &Path,
    binding: &Binding,
    expected: &str,
    id: &str,
    draft: Option<Draft>,
) -> Result<String, ClientError> {
    if !valid_id(id) || tiktools_collaboration::stream_sequence(expected).is_err() {
        return Err(invalid());
    }
    let mut data = read(path, binding)?;
    if data.revision != expected {
        return Err(fail(
            "draft_conflict",
            "saved drafts changed; review the saved copy before replacing it",
        ));
    }
    if let Some(draft) = draft {
        draft.validate()?;
        if draft.id != id {
            return Err(invalid());
        }
        if !data.records.contains_key(id) && data.records.len() >= RECORDS {
            return Err(fail(
                "draft_limit",
                "local draft count limit reached; existing drafts were preserved",
            ));
        }
        data.records.insert(id.into(), draft);
    } else {
        data.records.remove(id);
    }
    data.revision = tiktools_collaboration::stream_sequence(&data.revision)
        .map_err(|_| invalid())?
        .checked_add(1)
        .ok_or_else(unavailable)?
        .to_string();
    write(path, &data)?;
    Ok(data.revision)
}
pub(super) fn remove(path: &Path) -> Result<(), ClientError> {
    remove_one(&path.with_extension("drafts.tmp"))?;
    remove_one(path)?;
    directory::sync_directory(path.parent().ok_or_else(invalid)?).map_err(|_| unavailable())
}
fn remove_one(path: &Path) -> Result<(), ClientError> {
    match options().read(true).open(path) {
        Ok(f) => {
            private(&f)?;
            fs::remove_file(path).map_err(|_| unavailable())?;
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(invalid()),
    }
}
#[cfg(test)]
fn crash(stage: &str) {
    if std::env::var("TIKTOOLS_DRAFT_CRASH").as_deref() == Ok(stage) {
        std::process::exit(88);
    }
}
#[cfg(test)]
mod tests;
