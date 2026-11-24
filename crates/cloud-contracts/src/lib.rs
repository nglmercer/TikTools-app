//! Backend-owned, transport-independent v2 cloud contracts.
#![forbid(unsafe_code)]
pub mod portable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceRecord {
    pub collection: String,
    pub key: String,
    pub revision: i64,
    pub deleted: bool,
    pub value: Option<Value>,
}
impl WorkspaceRecord {
    /// Identity and revision are immutable; JSONB may change only the decimal
    /// representation of a numeric value, without changing its exact value.
    pub fn equivalent(&self, other: &Self) -> bool {
        self.collection == other.collection
            && self.key == other.key
            && self.revision == other.revision
            && self.deleted == other.deleted
            && match (&self.value, &other.value) {
                (Some(left), Some(right)) => portable::values_equivalent(left, right),
                (None, None) => true,
                _ => false,
            }
    }
    pub fn valid(&self, cursor: i64) -> bool {
        portable::valid_key(&self.collection, &self.key)
            && self.revision > 0
            && self.revision <= cursor
            && if self.deleted {
                self.value.is_none()
            } else {
                self.value
                    .as_ref()
                    .is_some_and(|v| portable::validate(&self.collection, &self.key, v))
            }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSnapshot {
    pub schema_version: u8,
    pub cursor: i64,
    pub records: Vec<WorkspaceRecord>,
}
impl WorkspaceSnapshot {
    pub fn valid(&self) -> bool {
        let mut keys = BTreeSet::new();
        self.schema_version == 2
            && self.cursor >= 0
            && self.records.len() <= 1000
            && self
                .records
                .iter()
                .all(|r| r.valid(self.cursor) && keys.insert((&r.collection, &r.key)))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceChanges {
    pub changes: Vec<WorkspaceRecord>,
    pub cursor: i64,
    pub has_more: bool,
}
impl WorkspaceChanges {
    pub fn valid_after(&self, after: i64) -> bool {
        if after < 0 || self.cursor < after || self.changes.len() > 50 {
            return false;
        }
        let mut last = after;
        for record in &self.changes {
            if !record.valid(self.cursor) || record.revision <= last {
                return false;
            }
            last = record.revision;
        }
        last == self.cursor && (!self.has_more || self.cursor > after)
    }
}
