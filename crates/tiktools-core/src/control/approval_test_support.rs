//! Shared fixtures for Owner approval tests: an isolated core per test,
//! a recording emitter and a baseline reusable grant.
#![cfg(test)]

use super::*;
use crate::{
    collaboration::*,
    control::{ApprovalEnablement, CommandCaller},
    paths::AppPaths,
    services::{AwardOptions, NoopMediaHost, PointAction},
    AppCore, HostEmitter,
};
use serde_json::Value;
use std::sync::Arc;

pub(crate) struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}
pub(crate) struct Fixture {
    pub(crate) core: Option<Arc<AppCore>>,
    pub(crate) paths: AppPaths,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-approval-test-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root,
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        let opened = Self::open(&paths);
        // Approved adjustments retain the ordinary adjustment contract:
        // viewers already exist. Zero-point awards seed private real rows.
        for id in ["alice", "bob"] {
            opened
                .points
                .award_points(
                    id,
                    PointAction::Manual,
                    AwardOptions {
                        custom_amount: Some(0.0),
                        ..AwardOptions::default()
                    },
                )
                .unwrap();
        }
        let core = Some(opened);
        Self { core, paths }
    }
    pub(crate) fn open(paths: &AppPaths) -> Arc<AppCore> {
        Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths.clone(),
        ))
    }
    pub(crate) fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    pub(crate) fn reopen(&mut self) {
        drop(self.core.take());
        self.core = Some(Self::open(&self.paths));
    }
    pub(crate) fn request(&self, expected: &str) -> CommandRequest {
        CommandRequest {
            command_id: format!("owner-approval-{}", fastrand::u64(..)),
            host_instance_id: self.core().collaboration().unwrap().instance_id().into(),
            expected_revision: expected.into(),
        }
    }
    pub(crate) fn pair(&self, grant: Grant) -> Authorization {
        let service = self.core().collaboration().unwrap();
        let mut config = service.configuration().unwrap();
        if !config.enabled {
            config.enabled = true;
            config.advertised_origin = Some("http://127.0.0.1:8788".into());
            service.configure(config).unwrap();
        }
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "e".repeat(64);
        let pending = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Approval runtime author".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "fixture",
            )
            .unwrap();
        service.decide_request(&pending.request_id, true).unwrap();
        let credentials = service
            .collect(
                Collect {
                    request_id: pending.request_id,
                    exchange_secret: exchange,
                },
                "fixture",
            )
            .unwrap();
        service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap()
    }
    pub(crate) fn draft(&self, auth: &Authorization, kind: AutomationKind, mut record: Value) {
        record["enabled"] = false.into();
        self.core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                self.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    pub(crate) fn approval(
        &self,
        kind: AutomationKind,
        id: &str,
        enable_plan: Vec<ApprovalEnablement>,
    ) -> (CommandRequest, ApprovalActivateParams) {
        let preview = self.core().automation_execution_manifest(kind, id).unwrap();
        (
            self.request(&preview.resource_versions[0].revision),
            ApprovalActivateParams {
                kind,
                id: id.into(),
                reviewed_versions: preview.resource_versions,
                manifest_digest: preview.value["manifestDigest"].as_str().unwrap().into(),
                enable_plan,
                acknowledge_effects: true,
            },
        )
    }
    pub(crate) fn approve(
        &self,
        kind: AutomationKind,
        id: &str,
        enable_plan: Vec<ApprovalEnablement>,
    ) -> Versioned<Value> {
        let (request, params) = self.approval(kind, id, enable_plan);
        self.core()
            .automation_approval_activate(request, params)
            .unwrap()
    }
    pub(crate) fn body(&self, kind: AutomationKind, id: &str) -> Value {
        self.core().automation_get(kind, id).unwrap()
    }
    pub(crate) fn points(&self, id: &str) -> f64 {
        self.core()
            .points_viewer(id)
            .map(|v| v["points"].as_f64().unwrap())
            .unwrap_or(0.0)
    }
    pub(crate) fn assert_delegated(&self, kind: AutomationKind, id: &str) {
        assert_eq!(
            self.core().automation_authority(kind, id).unwrap().value["provenance"]["authority"]
                ["kind"],
            "delegated"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.paths.root);
    }
}
pub(crate) fn grant() -> Grant {
    Grant {
        capabilities: [
            Capability::AutomationRead,
            Capability::AutomationEdit,
            Capability::LiveRead,
            Capability::PointsRead,
            Capability::PointsAdjust,
        ]
        .into(),
        ..Grant::default()
    }
}
