//! Transaction-scoped high-water accounting for gift streaks.
//! Callers must commit this update together with the durable contribution
//! identity and score; a rollback must leave neither side partially applied.
use super::DatabaseError;
use rusqlite::{params, OptionalExtension, Transaction};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct GiftDelta {
    pub quantity: u64,
    /// None means no trustworthy price was supplied, not a zero-price gift.
    pub diamonds: Option<u64>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn advance_combo(
    tx: &Transaction<'_>,
    competition: &str,
    generation: u64,
    participant: &str,
    gift: &str,
    group: &str,
    observed_quantity: u64,
    price: Option<u64>,
) -> Result<GiftDelta, DatabaseError> {
    if competition.is_empty()
        || competition.len() > 128
        || participant.is_empty()
        || participant.len() > 512
        || gift.is_empty()
        || gift.len() > 128
        || group.is_empty()
        || group.len() > 128
        || observed_quantity == 0
    {
        return Err(DatabaseError::Invalid("invalid gift combo identity".into()));
    }
    let generation = i64::try_from(generation)
        .map_err(|_| DatabaseError::Invalid("invalid generation".into()))?;
    let row: Option<(String,String)> = tx.query_row("SELECT quantity,diamonds FROM competition_gift_combos WHERE competition_id=?1 AND generation=?2 AND participant_id=?3 AND gift_id=?4 AND group_id=?5", params![competition,generation,participant,gift,group], |row| Ok((row.get(0)?,row.get(1)?))).optional()?;
    let (previous_quantity, previous_diamonds) = match row {
        Some((quantity, diamonds)) => (
            quantity
                .parse::<u64>()
                .map_err(|_| DatabaseError::Invalid("invalid gift quantity".into()))?,
            diamonds
                .parse::<u64>()
                .map_err(|_| DatabaseError::Invalid("invalid gift diamonds".into()))?,
        ),
        None => {
            let count: Option<i64> = tx.query_row("SELECT count FROM competition_gift_combo_counts WHERE competition_id=?1 AND generation=?2", params![competition,generation], |row| row.get(0)).optional()?;
            if count.unwrap_or(0) >= 100_000 {
                return Err(DatabaseError::Invalid(
                    "gift combo capacity exceeded".into(),
                ));
            }
            tx.execute("INSERT INTO competition_gift_combo_counts(competition_id,generation,count) VALUES (?1,?2,1) ON CONFLICT(competition_id,generation) DO UPDATE SET count=count+1", params![competition,generation])?;
            (0, 0)
        }
    };
    let quantity = previous_quantity.max(observed_quantity);
    let target = price
        .map(|price| {
            quantity
                .checked_mul(price)
                .ok_or_else(|| DatabaseError::Invalid("gift diamonds overflow".into()))
        })
        .transpose()?;
    let diamonds = target.map(|target| target.saturating_sub(previous_diamonds));
    let high_diamonds = target.map_or(previous_diamonds, |target| target.max(previous_diamonds));
    tx.execute("INSERT INTO competition_gift_combos(competition_id,generation,participant_id,gift_id,group_id,quantity,diamonds) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(competition_id,generation,participant_id,gift_id,group_id) DO UPDATE SET quantity=excluded.quantity,diamonds=excluded.diamonds", params![competition,generation,participant,gift,group,quantity.to_string(),high_diamonds.to_string()])?;
    Ok(GiftDelta {
        quantity: quantity - previous_quantity,
        diamonds,
    })
}
