//! Trusted admission for durable, exact-definition approvals. Stored JSON is
//! evidence to compare, never a deserializable execution permit.
mod admission;
mod authorization;
mod candidates;
mod competition;
mod definitions;
mod registry;

use admission::execution_chain_current;
pub(super) use candidates::{
    candidate_current, candidate_for_primary, reusable_candidate_for_primary,
};
#[cfg(test)]
use definitions::provenance;
use definitions::{capture, capture_review, fence_current};
pub(super) use registry::{clear, install};

use super::{manifest, *};
use crate::collaboration::{
    AutomationFamily, ExecutionIdentity, ResourceKind, ResourceVersion, Versioned,
};
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Clone)]
struct DefinitionFence {
    family: AutomationFamily,
    id: String,
    authority_key: (String, String),
    definition: Value,
    revision: String,
    generation: u64,
    authority: RuntimeAuthority,
}

/// Captured only under the host index lock. It still supplies no execution
/// authority until the current subjects and closed policy have been checked.
#[derive(Clone)]
pub(crate) struct ApprovalCandidate {
    index_identity: std::sync::Arc<()>,
    approval_key: (String, String),
    approval: Option<crate::db::BehaviorApprovalSnapshot>,
    review: Versioned<Value>,
    definitions: Vec<DefinitionFence>,
    catalog_epoch: u64,
}

/// No public constructor, Serialize or Deserialize: only a trusted saved
/// closure and current saved grants can create this.
#[derive(Clone)]
pub(crate) struct RuntimeApprovalPermit {
    candidate: ApprovalCandidate,
    manifest: manifest::ExecutionManifest,
    configuration_revisions: BTreeMap<(String, String), String>,
    competition: Option<crate::db::CompetitionExecutionFence>,
    competition_winner: Option<(tiktools_competitions::ActionClaim, Value)>,
}

/// Receiving approved scoped input never transfers Owner authority. This
/// private fence pins the saved Owner rule and its entire action closure.
#[derive(Clone)]
pub(crate) struct RuntimeOwnerRuleFence {
    index_identity: std::sync::Arc<()>,
    definitions: Vec<DefinitionFence>,
    catalog_epoch: u64,
}

#[cfg(test)]
mod tests;
