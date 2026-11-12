//! Frozen candidate validation and winner selection.
use crate::Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMode {
    HighestScore,
    UniformRandom,
    WeightedRandom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TiePolicy {
    Manual,
    Stable,
    Random,
    NoWinner,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub key: String,
    pub weight: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub mode: SelectionMode,
    pub tie_policy: TiePolicy,
    pub eligible: Vec<Candidate>,
    pub winner: Option<String>,
}

impl Selection {
    /// Validate a record before persistence or recovery; the eligible set is
    /// sorted, unique, bounded, and the winner must belong to it.
    pub fn validate(&self) -> Result<(), Error> {
        // An empty eligible set records an explicit no-winner outcome, such
        // as a leaderboard challenge whose target nobody reached.
        if self.eligible.is_empty() {
            return if self.winner.is_none() {
                Ok(())
            } else {
                Err(Error::Empty)
            };
        }
        if self.eligible.len() > 100_000 {
            return Err(Error::Empty);
        }
        if self
            .eligible
            .iter()
            .any(|c| c.key.is_empty() || c.key.len() > 512)
        {
            return Err(Error::InvalidCandidate);
        }
        if self
            .eligible
            .windows(2)
            .any(|pair| pair[0].key >= pair[1].key)
        {
            return Err(Error::DuplicateCandidate);
        }
        let chosen = match &self.winner {
            Some(key) => Some(
                self.eligible
                    .iter()
                    .find(|c| &c.key == key)
                    .ok_or(Error::InvalidCandidate)?,
            ),
            None => None,
        };
        match self.mode {
            SelectionMode::HighestScore => {
                let highest = self.eligible.iter().map(|c| c.weight).max().unwrap();
                let mut leaders = self.eligible.iter().filter(|c| c.weight == highest);
                let first = leaders.next().expect("nonempty candidates");
                let tied = leaders.next().is_some();
                if chosen.is_none() && (self.tie_policy != TiePolicy::NoWinner || !tied) {
                    return Err(Error::InvalidCandidate);
                }
                if let Some(chosen) = chosen {
                    if chosen.weight != highest
                        || (tied && self.tie_policy == TiePolicy::NoWinner)
                        || (tied && self.tie_policy == TiePolicy::Stable && chosen.key != first.key)
                    {
                        return Err(Error::InvalidCandidate);
                    }
                }
            }
            SelectionMode::UniformRandom => {
                if chosen.is_none() {
                    return Err(Error::InvalidCandidate);
                }
            }
            SelectionMode::WeightedRandom => {
                let total = self
                    .eligible
                    .iter()
                    .try_fold(0u64, |sum, c| sum.checked_add(c.weight))
                    .ok_or(Error::Overflow)?;
                if total == 0 || chosen.is_none_or(|c| c.weight == 0) {
                    return Err(Error::ZeroWeight);
                }
            }
        }
        Ok(())
    }
}

/// Resolve a manual highest-score tie against the frozen eligible set.
/// Only a tied leader can be chosen; no new random draw is performed.
pub fn select_manual(mut eligible: Vec<Candidate>, winner: &str) -> Result<Selection, Error> {
    if eligible.is_empty() || eligible.len() > 100_000 || winner.is_empty() {
        return Err(Error::InvalidCandidate);
    }
    eligible.sort_unstable_by(|a, b| a.key.cmp(&b.key));
    let highest = eligible
        .iter()
        .map(|candidate| candidate.weight)
        .max()
        .ok_or(Error::Empty)?;
    if eligible
        .iter()
        .filter(|candidate| candidate.weight == highest)
        .count()
        < 2
        || !eligible
            .iter()
            .any(|candidate| candidate.key == winner && candidate.weight == highest)
    {
        return Err(Error::InvalidCandidate);
    }
    let selection = Selection {
        mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Manual,
        eligible,
        winner: Some(winner.to_owned()),
    };
    selection.validate()?;
    Ok(selection)
}

/// Cryptographically secure rejection sampling, with no modulo bias.
fn random_below(bound: u64) -> Result<u64, Error> {
    if bound == 0 {
        return Err(Error::ZeroWeight);
    }
    let threshold = bound.wrapping_neg() % bound;
    loop {
        let mut bytes = [0; 8];
        getrandom::fill(&mut bytes).map_err(|_| Error::RandomUnavailable)?;
        let value = u64::from_le_bytes(bytes);
        if value >= threshold {
            return Ok(value % bound);
        }
    }
}

/// Freezes a sorted candidate list and selects once. Store the returned value
/// durably before revealing it; this function does not execute any effects.
pub fn select(
    mut eligible: Vec<Candidate>,
    mode: SelectionMode,
    tie: TiePolicy,
) -> Result<Selection, Error> {
    if eligible.is_empty() || eligible.len() > 100_000 {
        return Err(Error::Empty);
    }
    if eligible
        .iter()
        .any(|c| c.key.is_empty() || c.key.len() > 512)
    {
        return Err(Error::InvalidCandidate);
    }
    eligible.sort_unstable_by(|a, b| a.key.cmp(&b.key));
    if eligible.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err(Error::DuplicateCandidate);
    }
    let winner = match mode {
        SelectionMode::HighestScore => {
            let max = eligible.iter().map(|c| c.weight).max().unwrap();
            let tied: Vec<_> = eligible.iter().filter(|c| c.weight == max).collect();
            if tied.len() == 1 {
                Some(tied[0].key.clone())
            } else {
                match tie {
                    TiePolicy::Manual => return Err(Error::ManualTie),
                    TiePolicy::NoWinner => None,
                    TiePolicy::Stable => Some(tied[0].key.clone()),
                    TiePolicy::Random => {
                        Some(tied[random_below(tied.len() as u64)? as usize].key.clone())
                    }
                }
            }
        }
        SelectionMode::UniformRandom => Some(
            eligible[random_below(eligible.len() as u64)? as usize]
                .key
                .clone(),
        ),
        SelectionMode::WeightedRandom => {
            let total = eligible
                .iter()
                .try_fold(0u64, |sum, c| sum.checked_add(c.weight))
                .ok_or(Error::Overflow)?;
            let mut ticket = random_below(total)?;
            let mut winner = None;
            for candidate in &eligible {
                if ticket < candidate.weight {
                    winner = Some(candidate.key.clone());
                    break;
                }
                ticket -= candidate.weight;
            }
            winner
        }
    };
    Ok(Selection {
        mode,
        tie_policy: tie,
        eligible,
        winner,
    })
}
