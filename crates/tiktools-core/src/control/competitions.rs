//! Competition contracts and bounded read projections.
//!
//! Operation modules share the host's authoritative state. When both mutation
//! gates are needed, ranking remains first. Durable transitions precede publication.

mod definitions;
mod finalization;
mod lifecycle;
mod remote;
mod timer;

use super::OperationError;
use crate::{recover_mutex, AppCore};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Read-only remote projection. Definitions, commands, winner actions,
/// ranking IDs and participant information never leave the host here.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompetitionSummary {
    pub id: String,
    pub name: String,
    pub kind: tiktools_competitions::CompetitionKind,
    pub lifecycle: tiktools_competitions::Lifecycle,
    pub generation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    pub deadline_ms: Option<String>,
    pub remaining_ms: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompetitionSummaryPage {
    pub competitions: Vec<CompetitionSummary>,
    pub next_after: Option<String>,
}

/// Revision-bound remote competition lifecycle transitions.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CompetitionRemoteTransition {
    Start,
    Resume,
    Pause,
    Cancel,
}
impl CompetitionRemoteTransition {
    #[cfg(feature = "persistence")]
    pub(crate) fn method(self) -> &'static str {
        match self {
            Self::Start => "competitions.start",
            Self::Resume => "competitions.resume",
            Self::Pause => "competitions.pause",
            Self::Cancel => "competitions.cancel",
        }
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn transition(self) -> tiktools_competitions::Transition {
        match self {
            Self::Start => tiktools_competitions::Transition::Start,
            Self::Resume => tiktools_competitions::Transition::Resume,
            Self::Pause => tiktools_competitions::Transition::Pause,
            Self::Cancel => tiktools_competitions::Transition::Cancel,
        }
    }
}

impl AppCore {
    /// Coherent bounded status page filtered before serialization. This is a
    /// live keyset view, not a snapshot or authority to issue a command.
    pub fn competition_summary_page(
        &self,
        grant: &crate::collaboration::Grant,
        after: Option<&str>,
        limit: usize,
        byte_budget: usize,
        include_revision: bool,
    ) -> Result<CompetitionSummaryPage, OperationError> {
        use crate::collaboration::{Capability, ResourceKind};
        if !grant.capabilities.contains(&Capability::CompetitionsRead)
            || grant.resources.contains_key(&ResourceKind::Live)
        {
            return Err(OperationError::forbidden(
                "competition status is unavailable",
            ));
        }
        if !(1..=50).contains(&limit)
            || after.is_some_and(|id| {
                id.is_empty() || id.len() > 128 || id.chars().any(char::is_control)
            })
            || byte_budget < 1024
        {
            return Err(OperationError::invalid("invalid competition page"));
        }
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        let definitions = recover_mutex(&self.competitions.definitions, "competition definitions");
        let mut rows = Vec::new();
        // Reserve space for the JSON envelope and a fully escaped cursor.
        let mut bytes = 512usize;
        let mut more = false;
        for (id, definition) in definitions.iter() {
            if after.is_some_and(|cursor| id.as_str() <= cursor)
                || !grant.allows_resource(ResourceKind::Competition, id)
            {
                continue;
            }
            if rows.len() == limit {
                more = true;
                break;
            }
            if id.is_empty()
                || id.len() > 128
                || id.chars().any(char::is_control)
                || definition.name.trim().is_empty()
                || definition.name.len() > 256
                || definition.name.chars().any(char::is_control)
            {
                return Err(OperationError::unavailable("competition status is invalid"));
            }
            #[cfg(feature = "persistence")]
            let (state, revision) = if include_revision {
                let (state, revision) = self
                    .db
                    .competition_state_revision(id)
                    .map_err(|_| OperationError::unavailable("competition revision unavailable"))?
                    .ok_or_else(|| OperationError::unavailable("competition state unavailable"))?;
                (state, Some(revision.to_string()))
            } else {
                (self.competition_state(id)?, None)
            };
            #[cfg(not(feature = "persistence"))]
            let (state, revision) = {
                let _ = include_revision;
                (self.competition_state(id)?, None)
            };
            let row = CompetitionSummary {
                id: id.clone(),
                name: definition.name.clone(),
                kind: definition.kind,
                lifecycle: state.lifecycle,
                generation: state.generation.to_string(),
                revision,
                deadline_ms: state.deadline_ms.map(|value| value.to_string()),
                remaining_ms: state.remaining_ms.map(|value| value.to_string()),
            };
            let row_bytes = serde_json::to_vec(&row)
                .map_err(|_| OperationError::internal("competition status serialization failed"))?
                .len();
            if bytes.saturating_add(row_bytes).saturating_add(1) > byte_budget {
                if rows.is_empty() {
                    return Err(OperationError::unavailable(
                        "competition page exceeds transport limit",
                    ));
                }
                more = true;
                break;
            }
            bytes += row_bytes + 1;
            rows.push(row);
        }
        let next_after = if more {
            rows.last().map(|row| row.id.clone())
        } else {
            None
        };
        Ok(CompetitionSummaryPage {
            competitions: rows,
            next_after,
        })
    }
}

/// Presentation transports have a per-message limit. Preserve accounting
/// totals and order while dropping optional artwork/identity first, then
/// reducing the visible prefix if unusually escaped text remains too large.
pub(crate) fn bound_ranking_presentation(
    mut snapshot: tiktools_aggregation::Snapshot,
) -> tiktools_aggregation::Snapshot {
    const BUDGET: usize = 512 * 1024;
    let fits = |value: &tiktools_aggregation::Snapshot| {
        serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= BUDGET)
    };
    if fits(&snapshot) {
        return snapshot;
    }
    for entry in &mut snapshot.entries {
        entry.metadata = Default::default();
    }
    if fits(&snapshot) {
        return snapshot;
    }
    while !snapshot.entries.is_empty() && !fits(&snapshot) {
        snapshot.entries.truncate(snapshot.entries.len() / 2);
    }
    snapshot
}

impl AppCore {
    /// Read committed result history. Pages are bounded and ordered by
    /// generation, so reconnecting clients can resume without rerunning draws.
    pub fn competition_results(
        &self,
        id: &str,
        before_generation: Option<u64>,
        limit: usize,
    ) -> Result<(Vec<tiktools_competitions::ResultRecord>, Option<u64>), OperationError> {
        if !(1..=20).contains(&limit)
            || before_generation.is_some_and(|value| value > i64::MAX as u64)
        {
            return Err(OperationError::invalid("invalid result page"));
        }
        self.competition_get(id)?;
        #[cfg(feature = "persistence")]
        {
            let mut results = self
                .db
                .list_competition_results(id, before_generation, limit + 1)
                .map_err(|error| {
                    tracing::error!(%error, "competition results read failed");
                    OperationError::unavailable("competition results read failed")
                })?;
            let next = if results.len() > limit {
                results.truncate(limit);
                results.last().map(|result| result.generation)
            } else {
                None
            };
            Ok((results, next))
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::unavailable(
            "competition results require persistence",
        ))
    }
}
