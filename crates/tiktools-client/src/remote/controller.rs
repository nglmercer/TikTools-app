//! Native controller projection. Never constructs a host or interprets legacy IPC.
//! WebViews receive nonsecret state; credentials stay in ProfileManager.
use super::{profiles::ProfileManager, *};
use crate::{CallResult, RequestMetadata};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use tiktools_collaboration::{
    Capability, ResourceKind, ResourceVersion, SnapshotPage, SNAPSHOT_BYTES,
};
use tiktools_control_api::modules::collaboration::HelloResult;
use tokio::sync::{watch, Mutex};
#[cfg(feature = "cloud-peer")]
mod cloud;
mod connection;
mod management;
mod projection;
mod read_only;
mod worker;
mod workflows;

use projection::journal_projection;
use worker::run;
use workflows::apply;

mod automations;
mod commands;
mod competitions;
mod drafts;
mod live_status;
mod points;
mod proposals;
pub use automations::AutomationRow;
pub use commands::{AutomationCommand, CompetitionCommand, PointsCommand, WorkflowCommand};
use profiles::commands::Journal;
pub use profiles::commands::Operation as CommandOperation;
pub use profiles::commands::{
    Binding as CommandBinding, Intent as PendingCommand, JournalView,
    Operation as WorkflowCommandOperation, Outcome as LocalCommandOutcome, PointsResult,
    Record as CommandRecord,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerStatus {
    pub drafts_supported: bool,
    pub cloud_supported: bool,
    pub cloud_pairing: Option<CloudPairing>,
    pub proposals_supported: bool,
    pub epoch: String,
    pub view_revision: String,
    pub points_revision: String,
    pub automation_revision: String,
    pub competition_revision: String,
    pub points_loading: bool,
    pub points_error_code: Option<String>,
    pub phase: String,
    /// Actual selected native path; absent for the HTTPS/browser transport.
    pub transport_route: Option<String>,
    pub profile: Option<String>,
    pub origin: Option<String>,
    pub hello: Option<HelloResult>,
    pub error_code: Option<String>,
    pub pairing_request_id: Option<String>,
    pub command_journal: Option<JournalView>,
    pub command_busy: bool,
    pub command_error_code: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudPairing {
    pub profile: String,
    pub host_id: String,
    pub request_id: String,
}
#[derive(Clone, Serialize)]
pub struct WorkflowRow {
    pub graph: Value,
    pub version: ResourceVersion,
}
struct View {
    status: ControllerStatus,
    remote: Option<RemoteClient>,
    rows: BTreeMap<String, WorkflowRow>,
    journal: Option<Arc<Journal>>,
    points: points::State,
    automation: automations::State,
    competition_observation: Option<competitions::Observation>,
    proposal_inspection: Option<proposals::Inspection>,
}

struct Shared {
    view: RwLock<View>,
    changed: watch::Sender<u64>,
}

struct Pairing {
    profile: String,
    endpoint: RemoteEndpoint,
    pending: PendingPairing,
}
#[derive(Default)]
struct Lifecycle {
    worker: Option<tokio::task::JoinHandle<()>>,
    pairing: Option<Pairing>,
    #[cfg(feature = "cloud-peer")]
    cloud_pairing: Option<CloudPairing>,
}
impl Drop for Lifecycle {
    fn drop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.abort();
        }
    }
}
pub struct Controller {
    profiles: ProfileManager,
    #[cfg(feature = "cloud-peer")]
    cloud: Option<Arc<super::cloud_peer::CloudPeerController>>,
    #[cfg(feature = "cloud-peer")]
    cloud_login: std::sync::Mutex<Option<cloud::LoginJob>>,
    #[cfg(feature = "cloud-peer")]
    cloud_network: std::sync::Mutex<cloud::NetworkState>,
    shared: Arc<Shared>,
    lifecycle: Mutex<Lifecycle>,
    commands: Arc<Mutex<()>>,
    journals: std::sync::Mutex<BTreeMap<String, std::sync::Weak<Journal>>>,
}

impl Drop for Controller {
    fn drop(&mut self) {
        #[cfg(feature = "cloud-peer")]
        cloud::stop_login(&self.cloud_login, None);
        let v = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
        if let Some(r) = &v.remote {
            r.close();
        }
    }
}

impl Controller {
    #[cfg(feature = "cloud-peer")]
    pub fn with_cloud_environment(profiles: ProfileManager) -> Result<Self, ClientError> {
        let cloud = super::cloud_peer::CloudPeerController::from_environment(
            profiles.directory().join("cloud-account"),
            profiles.clone(),
            ClientKind::Desktop,
        )?;
        let mut controller = Self::new(profiles);
        controller.cloud = Some(Arc::new(cloud));
        Ok(controller)
    }
    pub fn new(profiles: ProfileManager) -> Self {
        let (changed, _) = watch::channel(0);
        Self {
            profiles,
            #[cfg(feature = "cloud-peer")]
            cloud: None,
            #[cfg(feature = "cloud-peer")]
            cloud_login: std::sync::Mutex::new(None),
            #[cfg(feature = "cloud-peer")]
            cloud_network: std::sync::Mutex::new(cloud::NetworkState::default()),
            lifecycle: Mutex::new(Lifecycle::default()),
            commands: Arc::new(Mutex::new(())),
            journals: std::sync::Mutex::new(BTreeMap::new()),
            shared: Arc::new(Shared {
                changed,
                view: RwLock::new(View {
                    status: ControllerStatus {
                        drafts_supported: true,
                        cloud_supported: false,
                        cloud_pairing: None,
                        proposals_supported: true,
                        epoch: "0".into(),
                        view_revision: "0".into(),
                        points_revision: "0".into(),
                        automation_revision: "0".into(),
                        competition_revision: "0".into(),
                        points_loading: true,
                        points_error_code: None,
                        phase: "idle".into(),
                        transport_route: None,
                        profile: None,
                        origin: None,
                        hello: None,
                        error_code: None,
                        pairing_request_id: None,
                        command_journal: None,
                        command_busy: false,
                        command_error_code: None,
                    },
                    remote: None,
                    rows: BTreeMap::new(),
                    journal: None,
                    points: points::State::default(),
                    automation: automations::State::default(),
                    competition_observation: None,
                    proposal_inspection: None,
                }),
            }),
        }
    }
}

#[cfg(test)]
mod tests;
