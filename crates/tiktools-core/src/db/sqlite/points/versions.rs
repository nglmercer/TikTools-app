//! Viewer versions and invalidations belong to the balance transaction. The
//! cache is optimistic; a projection always describes committed SQLite state.
use super::*;
use rusqlite::OptionalExtension;

// Exhaustion deliberately violates NOT NULL, rolling back the entire
// business transaction rather than permitting SQLite to promote an integer
// overflow to an imprecise floating revision.
const ADVANCE: &str =
    "INSERT INTO control_resource_versions(kind,scope,resource_id,revision,deleted)
    VALUES ('viewer','host',?1,1,0) ON CONFLICT(kind,scope,resource_id) DO UPDATE SET
    revision=CASE WHEN control_resource_versions.revision<9223372036854775807
    THEN control_resource_versions.revision+1 ELSE NULL END,deleted=0 RETURNING revision";
const RESERVE: &str =
    "INSERT INTO control_resource_outbox(kind,scope,resource_id,revision,change,committed_at)
    VALUES ('viewer','host',?1,?2,?3,?4)";

pub(super) struct Reservations<'db> {
    advance: rusqlite::Statement<'db>,
    outbox: rusqlite::Statement<'db>,
    exists: rusqlite::Statement<'db>,
}

impl<'db> Reservations<'db> {
    pub(super) fn new(db: &'db Connection) -> Result<Self, DatabaseError> {
        Ok(Self {
            advance: db.prepare(ADVANCE)?,
            outbox: db.prepare(RESERVE)?,
            exists: db.prepare("SELECT 1 FROM viewers WHERE unique_id=?1")?,
        })
    }

    fn advance(&mut self, id: &str, change: &str) -> Result<(), DatabaseError> {
        // Old databases can contain identities that today's transport cannot
        // address. Keep their local balances usable without publishing malformed
        // or unbounded identifiers. Remote reads must use this same validator.
        if !crate::services::valid_viewer_id(id) {
            return Ok(());
        }
        let next: i64 = self.advance.query_row([id], |row| row.get(0))?;
        self.outbox.execute(params![id, next, change, now()])?;
        Ok(())
    }

    pub(super) fn reserve(
        &mut self,
        db: &Connection,
        op: &PointsWriteOp,
    ) -> Result<(), DatabaseError> {
        match op {
            PointsWriteOp::Save { viewer, reason, .. } => self.advance(
                &viewer.unique_id,
                if *reason == "manual" {
                    "adjusted"
                } else {
                    "awarded"
                },
            ),
            PointsWriteOp::Adjust { unique_id, .. } => {
                // Legacy automation adjustments never fabricate missing viewers.
                if self
                    .exists
                    .query_row([unique_id], |_| Ok(()))
                    .optional()?
                    .is_some()
                {
                    self.advance(unique_id, "automation-adjusted")?;
                }
                Ok(())
            }
            PointsWriteOp::Reset {
                unique_id: Some(id),
                ..
            } => {
                if self
                    .exists
                    .query_row([id], |_| Ok(()))
                    .optional()?
                    .is_some()
                {
                    self.advance(id, "reset")?;
                }
                Ok(())
            }
            PointsWriteOp::Reset {
                unique_id: None, ..
            } => {
                // Stream one bounded identity at a time. A full reset covers cold
                // viewers as well as the hot cache and does not allocate all rows.
                let mut stmt = db.prepare("SELECT unique_id FROM viewers WHERE length(CAST(unique_id AS BLOB)) BETWEEN 1 AND 128 ORDER BY unique_id")?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    self.advance(&row.get::<_, String>(0)?, "reset")?;
                }
                Ok(())
            }
        }
    }
}
