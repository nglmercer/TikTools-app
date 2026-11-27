//! Value checks shared by the owner journals (adoption, approval, definition
//! and library): identifiers, revisions, digests, scope names and the byte
//! bounds of text columns read back from `control_commands`.

use crate::control::AutomationKind;
use crate::wire;
use rusqlite::{types::ValueRef, Row};

/// Byte bounds for the text columns of a `control_commands` row, in column
/// order: payload hash, session, method, resource kind, resource scope,
/// resource id, host instance, and state.
pub(super) const CONTROL_COMMAND_TEXT_LIMITS: [(usize, usize); 8] = [
    (0, 64),
    (1, 128),
    (2, 64),
    (3, 32),
    (4, 6),
    (5, 128),
    (6, 128),
    (7, 16),
];

/// True when every `(column, max_bytes)` pair holds text within its bound.
pub(super) fn text_columns_within(
    row: &Row<'_>,
    limits: &[(usize, usize)],
) -> rusqlite::Result<bool> {
    for &(column, max) in limits {
        match row.get_ref(column)? {
            ValueRef::Text(bytes) if bytes.len() <= max => {}
            _ => return Ok(false),
        }
    }
    Ok(true)
}

pub(super) fn id(value: &str) -> bool {
    wire::canonical_id(value, 128)
}

pub(super) fn revision(value: &str) -> Option<i64> {
    wire::decimal(value, false)
}

pub(super) use crate::wire::digest;

pub(super) fn scope(kind: AutomationKind) -> &'static str {
    match kind {
        AutomationKind::Event => "event",
        AutomationKind::Action => "action",
    }
}
