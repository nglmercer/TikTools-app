//! Owner review intent DTOs. These carry a saved-version fence and enablement
//! plan; source, subjects, grants and runtime permits are never accepted here.
use super::AutomationKind;
use crate::collaboration::ResourceVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalEnablement {
    pub kind: AutomationKind,
    pub id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalActivateParams {
    pub kind: AutomationKind,
    pub id: String,
    pub reviewed_versions: Vec<ResourceVersion>,
    pub manifest_digest: String,
    pub enable_plan: Vec<ApprovalEnablement>,
    pub acknowledge_effects: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalRevokeParams {
    pub kind: AutomationKind,
    pub id: String,
    pub acknowledge_revocation: bool,
}
