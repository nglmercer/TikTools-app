use super::*;
use crate::collaboration::{Grant, ResourceKind, ResourceVersion, Versioned};
use rusqlite::OptionalExtension;

// Rust's Unicode White_Space trim set, also used by valid_viewer_id. SQL must
// exclude unaddressable legacy IDs before ranking/limiting permitted rows.
const TRIM: &str = "\u{9}\u{a}\u{b}\u{c}\u{d} \u{85}\u{a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}";

pub(super) fn leaderboard_query(scoped: bool) -> String {
    // A scoped grant has at most 256 IDs. Keep that list as the outer loop
    // and use primary-key lookups, instead of scanning a million hidden ranks
    // to locate one permitted viewer near the bottom of the board.
    let (from, scope) = if scoped {
        (
            "json_each(?1) permitted CROSS JOIN viewers",
            "unique_id=permitted.value",
        )
    } else {
        ("viewers", "?1 IS NULL")
    };
    format!("SELECT unique_id FROM {from} WHERE {scope} AND length(CAST(unique_id AS BLOB)) BETWEEN 1 AND 128 AND trim(unique_id,?2)=unique_id AND substr(unique_id,1,1)<>'@' AND instr(unique_id,'*')=0 AND instr(unique_id,char(0))=0 AND unique_id NOT GLOB ('*['||char(1)||'-'||char(31)||char(127)||'-'||char(159)||']*') ORDER BY points DESC,total_coins DESC,total_chats DESC,unique_id ASC LIMIT ?3")
}

fn version(db: &Connection, id: &str, present: bool) -> Result<ResourceVersion, DatabaseError> {
    let revision = commands::revision(db, id)?;
    if present && revision <= 0 {
        return Err(DatabaseError::Invalid(
            "viewer resource version is missing".into(),
        ));
    }
    Ok(ResourceVersion {
        kind: ResourceKind::Viewer,
        scope: "host".into(),
        id: id.into(),
        revision: revision.to_string(),
    })
}

impl DatabaseManager {
    /// A complete, immutable permitted baseline, never a truncated leaderboard.
    /// Loading stops as soon as either byte budget is exceeded.
    pub(crate) fn load_points_snapshot(
        &self,
        grant: &Grant,
        page_bytes: usize,
        total_bytes: usize,
    ) -> Result<Vec<(Value, ResourceVersion)>, DatabaseError> {
        self.load_points_snapshot_limited(grant, page_bytes, total_bytes, None)
    }
    pub(crate) fn load_points_snapshot_limited(
        &self,
        grant: &Grant,
        page_bytes: usize,
        total_bytes: usize,
        limit: Option<u16>,
    ) -> Result<Vec<(Value, ResourceVersion)>, DatabaseError> {
        let permitted = grant
            .resources
            .get(&ResourceKind::Viewer)
            .map(serde_json::to_string)
            .transpose()?;
        let mut db = self.open(&self.points_path())?;
        let tx = db.transaction()?;
        let mut records = Vec::new();
        let mut used = 0usize;
        {
            let mut stmt = tx.prepare(&leaderboard_query(permitted.is_some()))?;
            let mut rows =
                stmt.query(params![permitted, TRIM, limit.map_or(i64::MAX, i64::from)])?;
            while let Some(row) = rows.next()? {
                let id: String = row.get(0)?;
                if !crate::services::valid_viewer_id(&id) {
                    return Err(DatabaseError::PointsStateInvalid);
                }
                let record = if limit.is_some() {
                    display_row(&tx, &id, page_bytes)?.ok_or(DatabaseError::NotFound)?
                } else {
                    commands::bounded_viewer(&tx, &id, page_bytes)?
                        .ok_or(DatabaseError::NotFound)?
                        .to_value()
                };
                let version = version(&tx, &id, true)?;
                let bytes =
                    serde_json::to_vec(&record)?.len() + serde_json::to_vec(&version)?.len() + 2;
                used = used
                    .checked_add(bytes)
                    .ok_or(DatabaseError::SnapshotTooLarge)?;
                if bytes > page_bytes
                    || used > total_bytes.min(crate::collaboration::SNAPSHOT_BYTES)
                {
                    return Err(DatabaseError::SnapshotTooLarge);
                }
                records.push((record, version));
            }
        }
        tx.commit()?;
        Ok(records)
    }

    pub(crate) fn points_viewer_snapshot(
        &self,
        id: &str,
        bytes: usize,
    ) -> Result<Versioned<Option<Value>>, DatabaseError> {
        let mut db = self.open(&self.points_path())?;
        let tx = db.transaction()?;
        let record = commands::bounded_viewer(&tx, id, bytes)?.map(|v| v.to_value());
        let result = Versioned::new(record.clone(), vec![version(&tx, id, record.is_some())?]);
        if serde_json::to_vec(&result)?.len() > bytes {
            return Err(DatabaseError::SnapshotTooLarge);
        }
        tx.commit()?;
        Ok(result)
    }

    pub(crate) fn points_leaderboard_snapshot(
        &self,
        grant: &Grant,
        limit: usize,
        bytes: usize,
    ) -> Result<Versioned<Vec<Value>>, DatabaseError> {
        let mut db = self.open(&self.points_path())?;
        let tx = db.transaction()?;
        let result = leaderboard_rows(&tx, grant, limit, bytes, false)?;
        tx.commit()?;
        Ok(result)
    }
    pub(crate) fn points_display_snapshot(
        &self,
        grant: &Grant,
        limit: usize,
        id: Option<&str>,
        currency: &str,
        bytes: usize,
    ) -> Result<Versioned<Value>, DatabaseError> {
        let mut db = self.open(&self.points_path())?;
        let tx = db.transaction()?;
        let mut board = leaderboard_rows(&tx, grant, limit, bytes, true)?;
        let selected = if let Some(id) = id {
            let row = display_row(&tx, id, bytes)?;
            if !board.resource_versions.iter().any(|v| v.id == id) {
                board
                    .resource_versions
                    .push(version(&tx, id, row.is_some())?);
            }
            row
        } else {
            None
        };
        let result = Versioned::new(
            serde_json::json!({"currencyName":currency,"viewers":board.value,"selectedId":id,"selected":selected}),
            board.resource_versions,
        );
        if serde_json::to_vec(&result)?.len() > bytes {
            return Err(DatabaseError::SnapshotTooLarge);
        }
        tx.commit()?;
        Ok(result)
    }
}
fn leaderboard_rows(
    tx: &Connection,
    grant: &Grant,
    limit: usize,
    bytes: usize,
    display: bool,
) -> Result<Versioned<Vec<Value>>, DatabaseError> {
    if bytes < 64 {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let permitted = grant
        .resources
        .get(&ResourceKind::Viewer)
        .map(serde_json::to_string)
        .transpose()?;
    let mut value = Vec::new();
    let mut versions = Vec::new();
    let mut used = 64usize;
    {
        let mut stmt = tx.prepare(&leaderboard_query(permitted.is_some()))?;
        let mut rows = stmt.query(params![permitted, TRIM, limit.min(1_000) as i64])?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            if !crate::services::valid_viewer_id(&id) {
                return Err(DatabaseError::Invalid(
                    "unaddressable viewer in projection".into(),
                ));
            }
            let record = if display {
                display_row(tx, &id, bytes)?.ok_or(DatabaseError::NotFound)?
            } else {
                commands::bounded_viewer(tx, &id, bytes)?
                    .ok_or(DatabaseError::NotFound)?
                    .to_value()
            };
            let v = version(tx, &id, true)?;
            used = used
                .checked_add(serde_json::to_vec(&record)?.len() + serde_json::to_vec(&v)?.len() + 2)
                .ok_or(DatabaseError::SnapshotTooLarge)?;
            if used > bytes {
                return Err(DatabaseError::SnapshotTooLarge);
            }
            value.push(record);
            versions.push(v);
        }
    }
    Ok(Versioned::new(value, versions))
}

fn display_row(db: &Connection, id: &str, bytes: usize) -> Result<Option<Value>, DatabaseError> {
    let size:Option<i64>=db.query_row("SELECT length(CAST(unique_id AS BLOB))+length(CAST(coalesce(nickname,unique_id) AS BLOB)) FROM viewers WHERE unique_id=?1",[id],|r|r.get(0)).optional()?;
    let Some(size) = size else {
        return Ok(None);
    };
    if size < 0 || size as u64 > bytes as u64 {
        return Err(DatabaseError::SnapshotTooLarge);
    }
    let (name,nickname,points,level,subscriber):(String,String,f64,i64,bool)=db.query_row("SELECT unique_id,coalesce(nickname,unique_id),points,level,is_subscriber FROM viewers WHERE unique_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
    if !points.is_finite() || points < 0.0 || level <= 0 || level > i64::from(u32::MAX) {
        return Err(DatabaseError::PointsStateInvalid);
    }
    Ok(Some(
        serde_json::json!({"uniqueId":name,"nickname":nickname,"points":points,"level":level,"isSubscriber":subscriber}),
    ))
}
