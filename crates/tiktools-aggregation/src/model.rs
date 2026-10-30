use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An integer multiplier. Scores are in whole units; no rounding is performed.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceRule {
    pub source: String,
    pub metric: String,
    pub weight: u64,
    pub enabled: bool,
    /// Per-gift integer multiplier, defaulting to 1 when no override exists.
    /// A value of 0 disables this metric for that gift.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub gift_multipliers: std::collections::BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub schema_version: u32,
    pub id: String,
    pub creator_id: String,
    pub room_id: String,
    #[serde(default)]
    pub group_by: GroupBy,
    #[serde(default)]
    pub window: Window,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_limits: Option<UserLimits>,
    pub sources: Vec<SourceRule>,
    pub limit: usize,
    pub capacity: usize,
}

/// Limits apply to scored contributions per user until the ranking is reset.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserLimits {
    #[serde(default)]
    pub max_contributions: Option<u64>,
    #[serde(default)]
    pub cooldown_ms: Option<u64>,
    pub max_users: usize,
}

/// Rolling windows retain at most `max_events` accepted scored contributions.
/// Entries expire at `accepted_at + duration_ms`, inclusive.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum Window {
    #[default]
    Session,
    Rolling {
        duration_ms: u64,
        max_events: usize,
    },
    /// Accumulates for one calendar day at `utc_offset_minutes` (-840..=840)
    /// and resets scores, limits and integrity counters at local midnight.
    Daily {
        #[serde(default)]
        utc_offset_minutes: i32,
    },
}

/// Available grouping keys. Missing keys are rejected, never guessed.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupBy {
    #[default]
    User,
    Gift,
    EventType,
    Option,
    Team,
    CustomKey,
}

/// A single accepted event with all of its incremental metrics.
#[derive(Clone, Debug)]
pub struct Contribution<'a> {
    pub creator_id: &'a str,
    pub room_id: &'a str,
    pub source: &'a str,
    pub user: Option<&'a str>,
    pub gift: Option<&'a str>,
    pub option: Option<&'a str>,
    pub team: Option<&'a str>,
    pub custom_key: Option<&'a str>,
    pub metrics: &'a [(&'a str, u64)],
}
