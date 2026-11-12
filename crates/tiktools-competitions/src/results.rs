//! Persisted selection records and bounded reveal projections.
use crate::{Candidate, Error, Selection, SelectionMode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A finalized generation's immutable selection. The persisted eligible set
/// is the authority for replay and audit, not a widget animation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResultRecord {
    pub schema_version: u32,
    pub competition_id: String,
    pub generation: u64,
    pub ranking_id: String,
    pub ranking_revision: u64,
    pub finalized_at: u64,
    pub selection: Selection,
}

/// Bounded presentation projection of a committed result. The full frozen
/// eligible set remains in ResultRecord and is never altered by rendering.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealSnapshot {
    pub schema_version: u32,
    pub competition_id: String,
    pub ranking_id: String,
    pub generation: u64,
    pub selection: RevealSelection,
    pub eligible_count: usize,
    pub other_weight: u64,
}

/// Presentation-only subset; the complete selection remains in the result record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevealSelection {
    pub mode: SelectionMode,
    pub eligible: Vec<Candidate>,
    pub winner: Option<String>,
}

impl ResultRecord {
    pub fn reveal_snapshot(&self) -> Result<RevealSnapshot, Error> {
        self.selection.validate()?;
        const LIMIT: usize = 256;
        let mut eligible = self
            .selection
            .eligible
            .iter()
            .take(LIMIT)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(winner) = &self.selection.winner {
            if !eligible.iter().any(|entry| &entry.key == winner) {
                let winner = self
                    .selection
                    .eligible
                    .iter()
                    .find(|entry| &entry.key == winner)
                    .ok_or(Error::InvalidCandidate)?
                    .clone();
                eligible.pop();
                eligible.push(winner);
            }
        }
        let included: std::collections::HashSet<&str> =
            eligible.iter().map(|entry| entry.key.as_str()).collect();
        let other_weight = self
            .selection
            .eligible
            .iter()
            .filter(|entry| !included.contains(entry.key.as_str()))
            .try_fold(0u64, |sum, entry| {
                let weight = if self.selection.mode == SelectionMode::WeightedRandom {
                    entry.weight
                } else {
                    1
                };
                sum.checked_add(weight).ok_or(Error::Overflow)
            })?;
        Ok(RevealSnapshot {
            schema_version: self.schema_version,
            competition_id: self.competition_id.clone(),
            ranking_id: self.ranking_id.clone(),
            generation: self.generation,
            selection: RevealSelection {
                mode: self.selection.mode,
                eligible,
                winner: self.selection.winner.clone(),
            },
            eligible_count: self.selection.eligible.len(),
            other_weight,
        })
    }
}
