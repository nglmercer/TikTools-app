//! Bounded reconstruction of totals and finalization candidates.
use super::{DatabaseError, DatabaseManager};
use rusqlite::{params, OptionalExtension};

impl DatabaseManager {
    /// Recovery-only bounded scan with checked arithmetic; ignored receipts do
    /// not contribute to the projection.
    pub(crate) fn competition_scored_totals(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Vec<(String, u64)>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT option_id,score FROM competition_scored_contributions WHERE competition_id=?1 AND generation=?2 AND option_id IS NOT NULL LIMIT 500001")?;
        let rows = statement.query_map(params![id, generation], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut totals = std::collections::BTreeMap::<String, u64>::new();
        for (index, row) in rows.enumerate() {
            if index >= 500_000 {
                return Err(DatabaseError::Invalid(
                    "scored contribution capacity exceeded".into(),
                ));
            }
            let (option, score) = row?;
            let score: u64 = score
                .parse()
                .map_err(|_| DatabaseError::Invalid("invalid score".into()))?;
            let total = totals.entry(option).or_default();
            *total = total
                .checked_add(score)
                .ok_or_else(|| DatabaseError::Invalid("score overflow".into()))?;
            if totals.len() > 256 {
                return Err(DatabaseError::Invalid("option capacity exceeded".into()));
            }
        }
        Ok(totals.into_iter().collect())
    }

    /// Bounded restart projection: the option index supports one grouped scan
    /// per generation, never one scan per incoming event.
    pub(crate) fn competition_vote_totals(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Vec<(String, u64)>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT option_id, SUM(votes) FROM competition_participants WHERE competition_id=?1 AND generation=?2 GROUP BY option_id ORDER BY option_id LIMIT 257")?;
        let rows = statement.query_map(params![id, generation], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut totals = Vec::new();
        for row in rows {
            let (option, votes) = row?;
            if totals.len() == 256 || votes <= 0 {
                return Err(DatabaseError::Invalid("invalid vote projection".into()));
            }
            totals.push((option, votes as u64));
        }
        Ok(totals)
    }

    /// Freeze a bounded distinct participant set for a uniform draw. Gift-only
    /// entrants and command voters share one identity space.
    pub(crate) fn competition_draw_participants(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Vec<tiktools_competitions::Candidate>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let mut statement = connection.prepare("SELECT participant_id FROM competition_participants WHERE competition_id=?1 AND generation=?2 UNION SELECT participant_id FROM competition_gift_participants WHERE competition_id=?1 AND generation=?2 ORDER BY participant_id LIMIT 100001")?;
        let rows = statement.query_map(params![id, generation], |row| row.get::<_, String>(0))?;
        let mut candidates = Vec::new();
        for row in rows {
            if candidates.len() >= 100_000 {
                return Err(DatabaseError::Invalid("draw capacity exceeded".into()));
            }
            let key = row?;
            if key.is_empty() || key.len() > 512 {
                return Err(DatabaseError::Invalid("invalid participant".into()));
            }
            candidates.push(tiktools_competitions::Candidate { key, weight: 1 });
        }
        Ok(candidates)
    }

    /// Build ticket weights from committed vote quantities and scored
    /// contributions. This bounded scan happens only at finalization.
    pub(crate) fn competition_weighted_draw_participants(
        &self,
        id: &str,
        generation: u64,
        vote_weight: u64,
    ) -> Result<Vec<tiktools_competitions::Candidate>, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction()?;
        let mut weights = std::collections::BTreeMap::<String, u64>::new();
        {
            let mut statement = tx.prepare("SELECT participant_id,votes FROM competition_participants WHERE competition_id=?1 AND generation=?2 LIMIT 100001")?;
            let rows = statement.query_map(params![id, generation], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (participant, votes) = row?;
                if participant.is_empty()
                    || participant.len() > 512
                    || votes <= 0
                    || weights.len() >= 100_000
                {
                    return Err(DatabaseError::Invalid("invalid draw participants".into()));
                }
                let weight = (votes as u64)
                    .checked_mul(vote_weight)
                    .ok_or_else(|| DatabaseError::Invalid("draw weight overflow".into()))?;
                weights.insert(participant, weight);
            }
        }
        {
            let mut statement = tx.prepare("SELECT participant_id FROM competition_gift_participants WHERE competition_id=?1 AND generation=?2 LIMIT 100001")?;
            let rows =
                statement.query_map(params![id, generation], |row| row.get::<_, String>(0))?;
            for row in rows {
                let participant = row?;
                if participant.is_empty()
                    || participant.len() > 512
                    || (!weights.contains_key(&participant) && weights.len() >= 100_000)
                {
                    return Err(DatabaseError::Invalid("invalid draw participants".into()));
                }
                weights.entry(participant).or_insert(0);
            }
        }
        {
            let mut statement = tx.prepare("SELECT participant_id,score FROM competition_scored_contributions WHERE competition_id=?1 AND generation=?2 AND option_id IS NOT NULL LIMIT 500001")?;
            let rows = statement.query_map(params![id, generation], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for (index, row) in rows.enumerate() {
                if index >= 500_000 {
                    return Err(DatabaseError::Invalid(
                        "draw contribution capacity exceeded".into(),
                    ));
                }
                let (participant, score) = row?;
                let score = score
                    .parse::<u64>()
                    .map_err(|_| DatabaseError::Invalid("invalid contribution score".into()))?;
                let weight = weights
                    .get_mut(&participant)
                    .ok_or_else(|| DatabaseError::Invalid("draw participant missing".into()))?;
                *weight = weight
                    .checked_add(score)
                    .ok_or_else(|| DatabaseError::Invalid("draw weight overflow".into()))?;
            }
        }
        tx.commit()?;
        Ok(weights
            .into_iter()
            .map(|(key, weight)| tiktools_competitions::Candidate { key, weight })
            .collect())
    }

    pub(crate) fn competition_participant_count(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<u64, DatabaseError> {
        let generation = i64::try_from(generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let connection = self.open(&self.automation_path())?;
        let count: Option<i64> = connection.query_row("SELECT participants FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2", params![id, generation], |row| row.get(0)).optional()?;
        u64::try_from(count.unwrap_or(0))
            .map_err(|_| DatabaseError::Invalid("invalid participant count".into()))
    }
}
