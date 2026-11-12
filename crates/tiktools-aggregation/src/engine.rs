mod assignments;
mod checkpoints;
mod contributions;
mod definitions;
mod integrity;
mod snapshots;
mod windows;

use crate::model::{Contribution, Definition, GroupBy, Window};
use crate::{Aggregation, EntryMetadata, Error, Snapshot};
use std::collections::{HashMap, VecDeque};

/// Complete session state for persistence. The definition is included so a
/// checkpoint cannot be restored against changed scoring or scope settings.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCheckpoint {
    pub schema_version: u32,
    pub definition: Definition,
    pub snapshot: Snapshot,
    #[serde(default)]
    window_events: Vec<(u64, String, u64)>,
    #[serde(default)]
    last_time: u64,
    #[serde(default)]
    users: HashMap<String, UserState>,
}

/// Bounded aggregate health counters; no participant identities are exposed.
#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub rankings: usize,
    pub paused: usize,
    pub degraded: usize,
    pub rejected: u64,
    pub dropped: u64,
    pub limited: u64,
    pub cooldown_rejected: u64,
}

/// Indexed definitions and bounded ranking state. Callers serialize updates
/// and supply only events already admitted and deduplicated by ingress.
#[derive(Default)]
pub struct Engine {
    rankings: HashMap<String, Ranking>,
    by_source: HashMap<String, Vec<String>>,
    next_instance: u64,
}
struct Ranking {
    definition: Definition,
    instance: u64,
    instance_id: String,
    state: Aggregation,
    paused: bool,
    rejected: u64,
    dropped: u64,
    window_events: VecDeque<(u64, String, u64)>,
    last_time: u64,
    users: HashMap<String, UserState>,
    limited: u64,
    cooldown_rejected: u64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct UserState {
    count: u64,
    last_time: u64,
}
#[derive(Clone, Copy)]
enum UserDecision {
    Allowed,
    Limited,
    Cooldown,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DefinitionError {
    Invalid,
    Duplicate,
    Capacity,
    NotFound,
    NotPaused,
    ResetRequired,
    RevisionOverflow,
    IdentityUnavailable,
}
