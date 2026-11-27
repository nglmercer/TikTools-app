//! Value checks shared by the owner journals (adoption, approval, definition
//! and library): identifiers, revisions, digests, scope names and the byte
//! bounds of text columns read back from `control_commands`.

use super::DatabaseError;
use crate::control::AutomationKind;
use crate::wire;
use rusqlite::{types::ValueRef, Connection, Row};

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

/// A new owner journal entry may only start when no owner journal (adoption,
/// approval, definition or library) still has an unresolved entry for `host`.
/// Adding a journal means adding it here once, not in every submitter.
pub(super) fn ensure_no_pending_owner_journal(
    db: &Connection,
    host: &str,
) -> Result<(), DatabaseError> {
    if super::owner_adoption::has_pending(db, host)?
        || super::owner_approval::has_pending(db, host)?
        || super::owner_definition::has_pending(db, host)?
        || super::owner_library::has_pending(db, host)?
    {
        return Err(DatabaseError::OwnerAdoptionPending);
    }
    Ok(())
}

/// Journal entries kept after completion, newest first; older completed rows are pruned.
const COMPLETED_ENTRIES_KEPT: u32 = 16;

/// Persists the completed form of a pending journal entry and prunes old
/// completed rows. Exactly one pending row must match `command_id`, otherwise
/// the journal changed underneath the caller and `invalid()` is returned.
/// `table` is one of the journal table names, never caller-supplied text.
pub(super) fn retire_pending(
    db: &Connection,
    table: &'static str,
    command_id: &str,
    payload_json: &str,
    recorded_at: i64,
    invalid: fn() -> DatabaseError,
) -> Result<(), DatabaseError> {
    let changed = db.execute(
        &format!("UPDATE {table} SET pending=0,payload_json=?2,updated_at=?3 WHERE command_id=?1 AND pending=1"),
        rusqlite::params![command_id, payload_json, recorded_at],
    )?;
    if changed != 1 {
        return Err(invalid());
    }
    db.execute(
        &format!("DELETE FROM {table} WHERE pending=0 AND command_id NOT IN (SELECT command_id FROM {table} WHERE pending=0 ORDER BY rowid DESC LIMIT {COMPLETED_ENTRIES_KEPT})"),
        [],
    )?;
    Ok(())
}

/// How a pending owner journal entry ends.
pub(super) enum Resolution {
    Succeeded,
    Rejected,
    ReviewedUnknown,
}

/// What one `owner_*_resolve` knows about its pending entry.
pub(super) struct PendingAsk<'a> {
    pub command_id: &'a str,
    /// A command ledger row bound to this entry exists.
    pub ledger_bound: bool,
    pub review_unknown: bool,
    pub rejection: Option<&'a str>,
    pub rejection_known: fn(&str) -> bool,
    pub invalid: fn() -> DatabaseError,
}

pub(super) struct Settlement<R> {
    pub resolution: Resolution,
    pub receipt: Option<R>,
    pub error_code: Option<String>,
}

impl<R> Settlement<R> {
    fn of(resolution: Resolution) -> Self {
        Self {
            resolution,
            receipt: None,
            error_code: None,
        }
    }
}

/// The shared decision of every `owner_*_resolve`: a bound command ledger row
/// with a known immutable outcome wins over rejection and unknown assertions;
/// an expired outcome is settled only when the owner reviewed it as unknown;
/// without a ledger row, a recognised rejection or an unknown review settles
/// the entry. Returns `None` when the entry must stay pending. `receipt`
/// builds the journal-specific receipt from the known outcome.
pub(super) fn settle_pending<R>(
    db: &Connection,
    ask: PendingAsk<'_>,
    receipt: impl FnOnce(&crate::collaboration::CommandOutcome) -> Result<R, DatabaseError>,
) -> Result<Option<Settlement<R>>, DatabaseError> {
    if ask.ledger_bound {
        return match super::commands::lookup(db, "local-owner", ask.command_id) {
            Ok(Some(outcome)) => Ok(Some(Settlement {
                resolution: Resolution::Succeeded,
                receipt: Some(receipt(&outcome)?),
                error_code: None,
            })),
            Err(DatabaseError::CommandExpired) if ask.review_unknown => {
                Ok(Some(Settlement::of(Resolution::ReviewedUnknown)))
            }
            Err(DatabaseError::CommandExpired) => Ok(None),
            Err(error) => Err(error),
            Ok(None) => Err((ask.invalid)()),
        };
    }
    if ask.rejection.is_some_and(ask.rejection_known) {
        return Ok(Some(Settlement {
            resolution: Resolution::Rejected,
            receipt: None,
            error_code: ask.rejection.map(str::to_owned),
        }));
    }
    Ok(ask
        .review_unknown
        .then(|| Settlement::of(Resolution::ReviewedUnknown)))
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
