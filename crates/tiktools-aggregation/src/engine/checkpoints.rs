//! Complete checkpoint export and atomic recovery of bounded accounting state.
use super::*;

impl Engine {
    /// Export all bounded accounting state, including rolling expiry and limits.
    pub fn session_checkpoint(&self, id: &str) -> Result<SessionCheckpoint, Error> {
        let ranking = self.rankings.get(id).ok_or(Error::NotFound)?;
        Ok(SessionCheckpoint {
            schema_version: 1,
            definition: ranking.definition.clone(),
            snapshot: self.selection_snapshot(id).ok_or(Error::NotFound)?,
            window_events: ranking.window_events.iter().cloned().collect(),
            last_time: ranking.last_time,
            users: ranking.users.clone(),
        })
    }

    /// Restore an entire checkpoint atomically into a fresh ranking instance.
    /// A caller must separately prove the checkpoint includes all accepted
    /// contributions before relying on its integrity for winner selection.
    pub fn restore_session_checkpoint(
        &mut self,
        checkpoint: &SessionCheckpoint,
    ) -> Result<(), Error> {
        let ranking = self
            .rankings
            .get_mut(&checkpoint.definition.id)
            .ok_or(Error::NotFound)?;
        let snapshot = &checkpoint.snapshot;
        if checkpoint.schema_version != 1
            || ranking.definition != checkpoint.definition
            || ranking.state.revision() != 0
            || snapshot.schema_version != 1
            || snapshot.ranking_id != ranking.definition.id
            || snapshot.creator_id != ranking.definition.creator_id
            || snapshot.room_id != ranking.definition.room_id
            || snapshot.window != ranking.definition.window
            || snapshot.instance_id.as_ref().is_some_and(|id| {
                id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            || snapshot.entries.len() != snapshot.total_participants
            || snapshot.entries.len() > ranking.definition.capacity
            || !matches!(snapshot.integrity_status.as_str(), "unknown" | "degraded")
            || (snapshot.integrity_status == "unknown"
                && (snapshot.rejected != 0 || snapshot.dropped != 0))
            || (snapshot.integrity_status == "degraded"
                && snapshot.rejected == 0
                && snapshot.dropped == 0)
        {
            return Err(Error::IncompatibleDefinition);
        }
        let mut state = Aggregation::new(ranking.definition.capacity);
        let mut seen = std::collections::HashSet::new();
        for entry in &snapshot.entries {
            if entry.score == 0 || !seen.insert(&entry.key) {
                return Err(Error::InvalidKey);
            }
            state.apply(&entry.key, entry.score, 1)?;
            let mut metadata = entry.metadata.clone();
            metadata.label = Some(entry.label.clone());
            state.update_metadata(&entry.key, &metadata);
        }
        if state.total != snapshot.total_score || snapshot.revision < state.revision() {
            return Err(Error::IncompatibleDefinition);
        }
        match ranking.definition.window {
            Window::Session | Window::Daily { .. } if !checkpoint.window_events.is_empty() => {
                return Err(Error::IncompatibleDefinition)
            }
            Window::Rolling { max_events, .. } => {
                if checkpoint.window_events.len() > max_events {
                    return Err(Error::IncompatibleDefinition);
                }
                let mut totals = HashMap::<&str, u64>::new();
                let mut last = 0;
                for (time, key, score) in &checkpoint.window_events {
                    if *time < last || *time > checkpoint.last_time || *score == 0 {
                        return Err(Error::IncompatibleDefinition);
                    }
                    last = *time;
                    let total = totals.entry(key).or_default();
                    *total = total.checked_add(*score).ok_or(Error::Overflow)?;
                }
                if totals.len() != state.scores.len()
                    || state
                        .scores
                        .iter()
                        .any(|(key, score)| totals.get(key.as_str()) != Some(score))
                {
                    return Err(Error::IncompatibleDefinition);
                }
            }
            _ => {}
        }
        match &ranking.definition.user_limits {
            None if !checkpoint.users.is_empty() => return Err(Error::IncompatibleDefinition),
            Some(limits)
                if checkpoint.users.len() > limits.max_users
                    || (!state.scores.is_empty() && checkpoint.users.is_empty())
                    || checkpoint.users.iter().any(|(key, user)| {
                        key.is_empty()
                            || key.len() > 512
                            || user.count == 0
                            || user.last_time > checkpoint.last_time
                            || limits.max_contributions.is_some_and(|max| user.count > max)
                    }) =>
            {
                return Err(Error::IncompatibleDefinition)
            }
            _ => {}
        }
        state.revision = snapshot.revision;
        ranking.state = state;
        ranking.paused = snapshot.paused;
        ranking.rejected = snapshot.rejected;
        ranking.dropped = snapshot.dropped;
        ranking.limited = snapshot.limited;
        ranking.cooldown_rejected = snapshot.cooldown_rejected;
        ranking.window_events = checkpoint.window_events.iter().cloned().collect();
        ranking.last_time = checkpoint.last_time;
        ranking.users = checkpoint.users.clone();
        if let Some(instance_id) = &snapshot.instance_id {
            ranking.instance_id = instance_id.clone();
        }
        Ok(())
    }
}
