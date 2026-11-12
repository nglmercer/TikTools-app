//! Atomic gift receipts, combo high-water marks and scored contributions.
use super::{DatabaseError, DatabaseManager};
use super::{GiftInput, GiftOutcome};
use rusqlite::{params, OptionalExtension};
use tiktools_competitions::{Definition, Lifecycle, LifecycleState};

impl DatabaseManager {
    /// Atomically admit a combo packet, its scoped high-water mark and the
    /// resulting shared-engine score. No contribution is assigned implicitly.
    pub(crate) fn route_competition_gift<F>(
        &self,
        id: &str,
        input: &GiftInput<'_>,
        owner: &str,
        score: F,
    ) -> Result<Option<GiftOutcome>, DatabaseError>
    where
        F: FnOnce(u64, u64, &str) -> Result<u64, DatabaseError>,
    {
        if input.creator_id.is_empty()
            || input.creator_id.len() > 256
            || input.room_id.is_empty()
            || input.room_id.len() > 256
            || input.participant_id.is_empty()
            || input.participant_id.len() > 512
            || input.event_identity.is_empty()
            || input.event_identity.len() > 512
            || input.gift_id.is_empty()
            || input.gift_id.len() > 128
            || input.group_id.is_empty()
            || input.group_id.len() > 128
            || input.observed_quantity == 0
            || input.price == 0
            || input.received_at > i64::MAX as u64
            || owner.is_empty()
        {
            return Err(DatabaseError::Invalid("invalid gift contribution".into()));
        }
        let mut connection = self.open(&self.automation_path())?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let row: Option<(String,String)> = tx.query_row("SELECT d.definition_json,s.state_json FROM competition_state s JOIN competition_definitions d ON d.id=s.competition_id JOIN aggregation_definitions a ON a.id=d.ranking_id WHERE d.id=?1 AND a.creator_id=?2 AND a.room_id=?3", params![id,input.creator_id,input.room_id], |row| Ok((row.get(0)?,row.get(1)?))).optional()?;
        let Some((definition, state)) = row else {
            return Ok(None);
        };
        let state: LifecycleState = serde_json::from_str(&state)?;
        if !matches!(state.lifecycle, Lifecycle::Active | Lifecycle::Paused) {
            return Ok(Some(GiftOutcome::Closed));
        }
        let generation = i64::try_from(state.generation)
            .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
        let current: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_integrity_owners WHERE competition_id=?1 AND generation=?2 AND owner=?3)", params![id,generation,owner], |row| row.get(0))?;
        if !current {
            return Err(DatabaseError::Invalid("competition owner changed".into()));
        }
        let prior: Option<(String,String,String,String,String)> = tx.query_row("SELECT participant_id,gift_id,group_id,observed_quantity,price FROM competition_gift_receipts WHERE competition_id=?1 AND generation=?2 AND event_identity=?3",params![id,generation,input.event_identity],|row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))).optional()?;
        if let Some((participant, gift, group, quantity, price)) = prior {
            return Ok(Some(
                if participant == input.participant_id
                    && gift == input.gift_id
                    && group == input.group_id
                    && quantity == input.observed_quantity.to_string()
                    && price == input.price.to_string()
                {
                    GiftOutcome::Duplicate
                } else {
                    GiftOutcome::IdentityConflict
                },
            ));
        }
        let count: Option<i64> = tx.query_row("SELECT contributions FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2", params![id,generation], |row| row.get(0)).optional()?;
        if count.unwrap_or(0) < 0 {
            return Err(DatabaseError::Invalid(
                "invalid participation counters".into(),
            ));
        }
        if count.unwrap_or(0) >= 500_000 {
            return Ok(Some(GiftOutcome::Capacity));
        }
        let previous_price: Option<String> = tx.query_row("SELECT price FROM competition_gift_combo_prices WHERE competition_id=?1 AND generation=?2 AND participant_id=?3 AND gift_id=?4 AND group_id=?5",params![id,generation,input.participant_id,input.gift_id,input.group_id],|row|row.get(0)).optional()?;
        if previous_price
            .as_ref()
            .is_some_and(|price| price != &input.price.to_string())
        {
            return Err(DatabaseError::Invalid("gift combo price changed".into()));
        }
        if previous_price.is_none() && input.observed_quantity > 1 {
            return Err(DatabaseError::Invalid(
                "gift combo began before observation".into(),
            ));
        }
        if previous_price.is_none() {
            tx.execute("INSERT INTO competition_gift_combo_prices(competition_id,generation,participant_id,gift_id,group_id,price) VALUES (?1,?2,?3,?4,?5,?6)",params![id,generation,input.participant_id,input.gift_id,input.group_id,input.price.to_string()])?;
        }
        let delta = super::super::gift_accounting::advance_combo(
            &tx,
            id,
            state.generation,
            input.participant_id,
            input.gift_id,
            input.group_id,
            input.observed_quantity,
            Some(input.price),
        )?;
        let score = if delta.quantity == 0 {
            0
        } else {
            score(
                delta.quantity,
                delta
                    .diamonds
                    .ok_or_else(|| DatabaseError::Invalid("gift price missing".into()))?,
                input.gift_id,
            )?
        };
        let definition: Definition = serde_json::from_str(&definition)?;
        let option = if state.lifecycle == Lifecycle::Active && !state.is_due(input.received_at) {
            definition.options.iter().find(|option| option.gift_ids.iter().any(|gift| gift == input.gift_id)).map(|option| option.id.clone()).or(tx.query_row("SELECT option_id FROM competition_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3",params![id,generation,input.participant_id],|row| row.get(0)).optional()?)
        } else {
            None
        };
        let outcome = if state.lifecycle != Lifecycle::Active || state.is_due(input.received_at) {
            GiftOutcome::Closed
        } else if score == 0 {
            GiftOutcome::Zero
        } else if let Some(option_id) = &option {
            GiftOutcome::Accepted {
                option_id: option_id.clone(),
                score,
            }
        } else {
            GiftOutcome::Unassigned
        };
        let credited_option = if matches!(outcome, GiftOutcome::Accepted { .. }) {
            option
        } else {
            None
        };
        let new_gift_participant = if credited_option.is_some() {
            let existing: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM competition_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3 UNION ALL SELECT 1 FROM competition_gift_participants WHERE competition_id=?1 AND generation=?2 AND participant_id=?3)",params![id,generation,input.participant_id],|row| row.get(0))?;
            if !existing {
                let participants: Option<i64> = tx.query_row("SELECT participants FROM competition_participation_counts WHERE competition_id=?1 AND generation=?2",params![id,generation],|row| row.get(0)).optional()?;
                if participants.unwrap_or(0) >= 100_000 {
                    return Ok(Some(GiftOutcome::Capacity));
                }
                tx.execute("INSERT INTO competition_gift_participants(competition_id,generation,participant_id) VALUES (?1,?2,?3)",params![id,generation,input.participant_id])?;
            }
            !existing
        } else {
            false
        };
        tx.execute("INSERT INTO competition_participation_counts(competition_id,generation,participants,contributions) VALUES (?1,?2,?3,1) ON CONFLICT(competition_id,generation) DO UPDATE SET participants=participants+excluded.participants,contributions=contributions+1", params![id,generation,i64::from(new_gift_participant)])?;
        tx.execute("INSERT INTO competition_gift_receipts(competition_id,generation,event_identity,participant_id,gift_id,group_id,observed_quantity,price) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",params![id,generation,input.event_identity,input.participant_id,input.gift_id,input.group_id,input.observed_quantity.to_string(),input.price.to_string()])?;
        if let Some(option) = credited_option {
            tx.execute("INSERT INTO competition_scored_contributions(competition_id,generation,source,event_identity,participant_id,option_id,score,created_at) VALUES (?1,?2,'tiktok.gift',?3,?4,?5,?6,?7)",params![id,generation,input.event_identity,input.participant_id,option,score.to_string(),input.received_at as i64])?;
        }
        tx.commit()?;
        Ok(Some(outcome))
    }
}
