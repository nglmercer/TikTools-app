//! Explicitly retained proposal text. Never an execution grant or an auto-send queue.
use super::{commands::Binding, directory, *};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use tiktools_collaboration::AutomationFamily;

pub const TEXT_BYTES: usize = 256 * 1024;
const FILE_BYTES: u64 = 4 * 1024 * 1024;
const RECORDS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProposalMode {
    Create,
    Update,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProposalDraft {
    pub kind: AutomationFamily,
    pub id: String,
    pub mode: ProposalMode,
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
    pub records: BTreeMap<String, ProposalDraft>,
}

pub(in crate::remote) fn invalid() -> ClientError {
    fail(
        "proposal_invalid",
        "saved proposals are invalid; existing text was preserved",
    )
}
fn unavailable() -> ClientError {
    fail(
        "proposal_unavailable",
        "local proposal storage is unavailable; keep your unsaved text",
    )
}
fn limit() -> ClientError {
    fail(
        "proposal_limit",
        "local proposal storage limit reached; existing text was preserved",
    )
}
fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    options
}
fn parent(path: &Path) -> Result<&Path, ClientError> {
    let directory = path.parent().ok_or_else(invalid)?;
    directory::check_directory(directory, false).map_err(|error| {
        if error.code == "profile_invalid" {
            invalid()
        } else {
            unavailable()
        }
    })?;
    Ok(directory)
}
fn private(file: &File) -> Result<(), ClientError> {
    let metadata = file.metadata().map_err(|_| unavailable())?;
    if !metadata.is_file() {
        return Err(invalid());
    }
    directory::private(&metadata).map_err(|_| invalid())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(invalid());
        }
    }
    Ok(())
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.trim() == id
        && !id.contains('*')
        && !id.chars().any(char::is_control)
}
fn key(kind: AutomationFamily, id: &str) -> String {
    kind.grant_id(id)
}

impl ProposalDraft {
    fn validate(&self) -> Result<(), ClientError> {
        let revision =
            tiktools_collaboration::stream_sequence(&self.base_revision).map_err(|_| invalid())?;
        if !valid_id(&self.id)
            || (self.mode == ProposalMode::Update && revision == 0)
            || revision > i64::MAX as u64
            || !(0..=9_007_199_254_740_991).contains(&self.updated_at)
        {
            return Err(invalid());
        }
        if self.text.len() > TEXT_BYTES {
            return Err(limit());
        }
        Ok(())
    }
}

pub(super) fn read(path: &Path, binding: &Binding) -> Result<Data, ClientError> {
    parent(path)?;
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Data {
                version: 1,
                binding: binding.clone(),
                revision: "0".into(),
                records: BTreeMap::new(),
            })
        }
        Err(_) => return Err(unavailable()),
        Ok(metadata) if !metadata.is_file() => return Err(invalid()),
        _ => {}
    }
    let file = options().read(true).open(path).map_err(|_| invalid())?;
    private(&file)?;
    let mut bytes = Vec::new();
    file.take(FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() as u64 > FILE_BYTES {
        return Err(invalid());
    }
    let data: Data = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if data.version != 1
        || data.binding != *binding
        || data.records.len() > RECORDS
        || tiktools_collaboration::stream_sequence(&data.revision)
            .ok()
            .is_none_or(|revision| revision > i64::MAX as u64)
    {
        return Err(invalid());
    }
    for (stored_key, proposal) in &data.records {
        proposal.validate().map_err(|_| invalid())?;
        if stored_key != &key(proposal.kind, &proposal.id) {
            return Err(invalid());
        }
    }
    Ok(data)
}

fn write(path: &Path, data: &Data) -> Result<(), ClientError> {
    let directory = parent(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => return Err(invalid()),
        Ok(_) => private(&options().read(true).open(path).map_err(|_| invalid())?)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(unavailable()),
    }
    let bytes = serde_json::to_vec(data).map_err(|_| invalid())?;
    if bytes.len() as u64 > FILE_BYTES {
        return Err(limit());
    }
    // A single known private sibling bounds leftover text after repeated death.
    // Reads never promote it; only another explicit save may replace it.
    let staging = path.with_extension("proposals.tmp");
    let mut temporary = options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&staging)
        .map_err(|_| unavailable())?;
    private(&temporary)?;
    temporary.set_len(0).map_err(|_| unavailable())?;
    temporary
        .write_all(&bytes)
        .and_then(|_| temporary.flush())
        .and_then(|_| temporary.sync_all())
        .map_err(|_| unavailable())?;
    #[cfg(test)]
    crash("before-rename");
    drop(temporary);
    fs::rename(&staging, path).map_err(|_| unavailable())?;
    #[cfg(test)]
    crash("after-rename");
    directory::sync_directory(directory).map_err(|_| unavailable())?;
    #[cfg(test)]
    crash("synced");
    Ok(())
}

/// The caller holds the profile process lease and journal write mutex. Global
/// CAS fences delayed editor saves, including deletion and same-ID recreation.
pub(super) fn change(
    path: &Path,
    binding: &Binding,
    expected: &str,
    kind: AutomationFamily,
    id: &str,
    proposal: Option<ProposalDraft>,
) -> Result<String, ClientError> {
    if !valid_id(id) || tiktools_collaboration::stream_sequence(expected).is_err() {
        return Err(invalid());
    }
    let mut data = read(path, binding)?;
    if data.revision != expected {
        return Err(fail(
            "proposal_conflict",
            "saved proposals changed; review the saved copy before replacing it",
        ));
    }
    let key = key(kind, id);
    if let Some(proposal) = proposal {
        proposal.validate()?;
        if proposal.kind != kind || proposal.id != id {
            return Err(invalid());
        }
        if !data.records.contains_key(&key) && data.records.len() >= RECORDS {
            return Err(limit());
        }
        data.records.insert(key, proposal);
    } else {
        data.records.remove(&key);
    }
    data.revision = tiktools_collaboration::stream_sequence(&data.revision)
        .map_err(|_| invalid())?
        .checked_add(1)
        .filter(|next| *next <= i64::MAX as u64)
        .ok_or_else(unavailable)?
        .to_string();
    write(path, &data)?;
    Ok(data.revision)
}

pub(super) fn remove(path: &Path) -> Result<(), ClientError> {
    let directory = parent(path)?;
    remove_one(&path.with_extension("proposals.tmp"))?;
    remove_one(path)?;
    directory::sync_directory(directory).map_err(|_| unavailable())
}
fn remove_one(path: &Path) -> Result<(), ClientError> {
    match options().read(true).open(path) {
        Ok(file) => {
            private(&file)?;
            fs::remove_file(path).map_err(|_| unavailable())?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(invalid()),
    }
}

#[cfg(test)]
fn crash(stage: &str) {
    if std::env::var("TIKTOOLS_PROPOSAL_CRASH").as_deref() == Ok(stage) {
        std::process::exit(89);
    }
}
#[cfg(test)]
mod tests;
