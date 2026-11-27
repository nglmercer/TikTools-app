//! Seeds a v2-ready Cloud workspace for the account `alice`: product-guarded
//! workspace state, a v2 consent file and the protocol and product-outbox markers.
use super::*;
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tiktools_cloud_auth::CloudAuth;
use tiktools_cloud_sync::WorkspaceCoordinator;
use tiktools_plugin_package::Sha256Digest;

pub(super) struct V2Seed {
    pub sync: tiktools_cloud_sync::SyncCoordinator,
    pub workspace: WorkspaceCoordinator,
    pub store: cloud_client::WorkspaceStore,
    pub protocol_marker: PathBuf,
    pub product_marker: PathBuf,
}

/// Wires `auth` into `core`'s Cloud sync, enables it and writes the v2 state.
pub(super) fn seed_v2_workspace(
    core: &mut Arc<AppCore>,
    paths: &AppPaths,
    origin: &str,
    auth: &CloudAuth,
) -> V2Seed {
    let origin = origin.to_owned();
    Arc::get_mut(core).unwrap().sync = Arc::new(crate::cloud_sync::CloudSync::new(paths, auth));
    let sync = core.sync.coordinator().unwrap().clone();
    sync.set_enabled(true).unwrap();
    let sync_dir = paths.data.join("cloud-sync");
    let workspace = WorkspaceCoordinator::open(&sync_dir, origin.clone(), "alice".into()).unwrap();
    workspace.bootstrap(&[], None).unwrap();
    let store = cloud_client::WorkspaceStore::open(&sync_dir, &origin, "alice").unwrap();
    let mut snapshot = store.load().unwrap().unwrap();
    snapshot.state.mark_product_guarded();
    store
        .save(Some(snapshot.generation), &snapshot.state)
        .unwrap();
    let identity = Sha256Digest::of_bytes(&serde_json::to_vec(&(&origin, "alice")).unwrap());
    let scope = Sha256Digest::of_bytes(
        &serde_json::to_vec(&("tiktools-v2-consent", &origin, "alice")).unwrap(),
    );
    std::fs::write(
        sync_dir.join("cloud-sync.json"),
        serde_json::to_vec(
            &json!({"version":2,"enabled":true,"generation":42,"scope":scope.as_bytes()}),
        )
        .unwrap(),
    )
    .unwrap();
    let protocol_marker = sync_dir.join(format!("protocol-{identity}.json"));
    std::fs::write(&protocol_marker, serde_json::to_vec(&json!({"formatVersion":3,"origin":origin,"accountId":"alice","mode":"v2","candidateSha256":"a".repeat(64)})).unwrap()).unwrap();
    let product_marker = sync_dir.join(format!("v1-product-outbox-{identity}.json"));
    std::fs::write(
        &product_marker,
        serde_json::to_vec(&json!({"formatVersion":1,"origin":origin,"accountId":"alice"}))
            .unwrap(),
    )
    .unwrap();
    V2Seed {
        sync,
        workspace,
        store,
        protocol_marker,
        product_marker,
    }
}
