//! Anonymous, bounded discovery of the v2 workspace contract.
use super::*;
use serde::Deserialize;

/// Validated limits from the configured Cloud instance. This does not grant
/// authorization, approve a cutover, or establish package trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSyncCapabilities {
    pub max_records: u32,
    pub max_bytes: u32,
    pub max_document_bytes: u32,
    /// True only when the configured instance advertises the exact fixed
    /// write-barrier contract understood by this SDK.
    pub write_epoch_barrier: bool,
    /// True only if the irreversible account-wide enforcement endpoint is advertised.
    pub write_epoch_enforcement: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityDocument {
    api_version: u32,
    instance_mode: String,
    user_configurable_registries: bool,
    sync: SyncDocument,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncDocument {
    base_path: String,
    schema_version: u32,
    collections: Vec<String>,
    max_records: u32,
    max_bytes: u32,
    max_document_bytes: u32,
    conflict_status: u16,
    expired_cursor_status: u16,
    preconditions_required: bool,
    #[serde(default)]
    write_epoch_barrier: Option<WriteEpochBarrierDocument>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteEpochBarrierDocument {
    epoch_path: String,
    barrier_path: String,
    header: String,
    stale_status: u16,
    #[serde(default)]
    require_epoch_path: Option<String>,
    #[serde(default)]
    missing_epoch_status: Option<u16>,
}

impl CloudCatalog {
    /// Discover only the fixed endpoint on the provisioned origin. No token,
    /// account identifier, redirect, or advertised path is used for requests.
    pub async fn workspace_sync_capabilities(
        &self,
    ) -> Result<WorkspaceSyncCapabilities, SyncError> {
        let url = self
            .origin
            .join("api/v2/capabilities")
            .map_err(|_| SyncError::Configuration)?;
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        let document: CapabilityDocument = decode_with_limit(response, 64 * 1024).await?;
        let sync = document.sync;
        let collections: std::collections::BTreeSet<_> =
            sync.collections.iter().map(String::as_str).collect();
        let expected: std::collections::BTreeSet<_> =
            portable::COLLECTIONS.iter().copied().collect();
        if document.api_version != 2
            || document.instance_mode != "single"
            || document.user_configurable_registries
            || sync.base_path != "/api/v2/sync"
            || sync.schema_version != 2
            || sync.collections.len() != expected.len()
            || collections != expected
            || !(1..=1000).contains(&sync.max_records)
            || !(1..=8 * 1024 * 1024).contains(&sync.max_bytes)
            || !(1..=portable::MAX_DOCUMENT_BYTES as u32).contains(&sync.max_document_bytes)
            || sync.max_document_bytes > sync.max_bytes
            || sync.conflict_status != 409
            || sync.expired_cursor_status != 410
            || !sync.preconditions_required
            || sync.write_epoch_barrier.as_ref().is_some_and(|barrier| {
                barrier.epoch_path != "/api/v2/sync/write-epoch"
                    || barrier.barrier_path != "/api/v2/sync/write-barrier"
                    || barrier.header != "X-Workspace-Write-Epoch"
                    || barrier.stale_status != 412
                    || !matches!(
                        (&barrier.require_epoch_path, barrier.missing_epoch_status),
                        (None, None) | (Some(_), Some(428))
                    )
                    || barrier
                        .require_epoch_path
                        .as_deref()
                        .is_some_and(|path| path != "/api/v2/sync/require-write-epoch")
            })
        {
            return Err(SyncError::Protocol);
        }
        Ok(WorkspaceSyncCapabilities {
            max_records: sync.max_records,
            max_bytes: sync.max_bytes,
            max_document_bytes: sync.max_document_bytes,
            write_epoch_barrier: sync.write_epoch_barrier.is_some(),
            write_epoch_enforcement: sync
                .write_epoch_barrier
                .as_ref()
                .is_some_and(|barrier| barrier.require_epoch_path.is_some()),
        })
    }
}
