//! Definition validation, source indexing and ranking lifecycle changes.
use super::*;

impl Engine {
    pub fn create(&mut self, definition: Definition) -> Result<(), DefinitionError> {
        if self.rankings.len() >= 128 {
            return Err(DefinitionError::Capacity);
        }
        if definition.schema_version != 1
            || definition.id.is_empty()
            || definition.creator_id.is_empty()
            || definition.room_id.is_empty()
            || definition.id.len() > 128
            || definition.creator_id.len() > 256
            || definition.room_id.len() > 256
            || definition.sources.len() > 64
            || definition.capacity > 100_000
            || definition.capacity == 0
            || definition.limit == 0
            || matches!(&definition.window, Window::Rolling { duration_ms, max_events } if *duration_ms == 0 || *max_events == 0 || *max_events > 100_000)
            || matches!(&definition.window, Window::Daily { utc_offset_minutes } if !(-840..=840).contains(utc_offset_minutes))
            || definition.user_limits.as_ref().is_some_and(|limits| {
                limits.max_users == 0
                    || limits.max_users > 100_000
                    || limits.max_contributions == Some(0)
                    || limits.cooldown_ms == Some(0)
                    || (limits.max_contributions.is_none() && limits.cooldown_ms.is_none())
            })
            || definition.sources.is_empty()
            || (definition
                .sources
                .iter()
                .any(|s| s.source == "competition.vote")
                && Self::assignment_weight(&definition).is_err())
            || definition.sources.iter().any(|s| {
                s.source.is_empty()
                    || s.weight > 1_000_000_000
                    || s.gift_multipliers.len() > 512
                    || s.gift_multipliers.iter().any(|(gift, multiplier)| {
                        gift.is_empty() || gift.len() > 128 || *multiplier > 1_000_000_000
                    })
                    || s.source.len() > 128
                    || s.metric.is_empty()
                    || s.metric.len() > 128
            })
        {
            return Err(DefinitionError::Invalid);
        }
        if self.rankings.contains_key(&definition.id) {
            return Err(DefinitionError::Duplicate);
        }
        let mut identity = [0u8; 16];
        getrandom::fill(&mut identity).map_err(|_| DefinitionError::IdentityUnavailable)?;
        let instance_id = identity
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        for source in &definition.sources {
            let ids = self.by_source.entry(source.source.clone()).or_default();
            if !ids.contains(&definition.id) {
                ids.push(definition.id.clone());
            }
        }
        let instance = self
            .next_instance
            .checked_add(1)
            .ok_or(DefinitionError::RevisionOverflow)?;
        self.next_instance = instance;
        let state = Aggregation::new(definition.capacity);
        self.rankings.insert(
            definition.id.clone(),
            Ranking {
                definition,
                instance,
                instance_id,
                state,
                paused: false,
                rejected: 0,
                dropped: 0,
                window_events: VecDeque::new(),
                last_time: 0,
                users: HashMap::new(),
                limited: 0,
                cooldown_rejected: 0,
            },
        );
        Ok(())
    }

    /// Configuration changes require a paused ranking; changing accounting
    /// semantics also requires an explicit reset of the accumulated state.
    pub fn validate_update(
        &self,
        definition: &Definition,
        reset: bool,
    ) -> Result<(), DefinitionError> {
        let mut validator = Engine::default();
        validator.create(definition.clone())?;
        let current = self
            .rankings
            .get(&definition.id)
            .ok_or(DefinitionError::NotFound)?;
        if !current.paused {
            return Err(DefinitionError::NotPaused);
        }
        let mut comparable = current.definition.clone();
        comparable.limit = definition.limit;
        if comparable != *definition && !reset {
            return Err(DefinitionError::ResetRequired);
        }
        if current.state.revision() == u64::MAX {
            return Err(DefinitionError::RevisionOverflow);
        }
        Ok(())
    }

    pub fn update(&mut self, definition: Definition, reset: bool) -> Result<(), DefinitionError> {
        self.validate_update(&definition, reset)?;
        self.by_source.retain(|_, ids| {
            ids.retain(|id| id != &definition.id);
            !ids.is_empty()
        });
        for source in &definition.sources {
            let ids = self.by_source.entry(source.source.clone()).or_default();
            if !ids.contains(&definition.id) {
                ids.push(definition.id.clone());
            }
        }
        let ranking = self
            .rankings
            .get_mut(&definition.id)
            .expect("validated ranking");
        if reset {
            ranking
                .state
                .reset()
                .map_err(|_| DefinitionError::RevisionOverflow)?;
            ranking.state.set_capacity(definition.capacity);
            ranking.rejected = 0;
            ranking.dropped = 0;
            ranking.window_events.clear();
            ranking.last_time = 0;
            ranking.users.clear();
            ranking.limited = 0;
            ranking.cooldown_rejected = 0;
        } else {
            ranking
                .state
                .touch()
                .map_err(|_| DefinitionError::RevisionOverflow)?;
        }
        ranking.definition = definition;
        Ok(())
    }

    pub fn definitions(&self) -> Vec<Definition> {
        let mut definitions: Vec<_> = self
            .rankings
            .values()
            .map(|r| r.definition.clone())
            .collect();
        definitions.sort_unstable_by(|a, b| a.id.cmp(&b.id));
        definitions
    }

    pub fn reset(&mut self, id: &str) -> Option<Result<(), Error>> {
        self.rankings.get_mut(id).map(|r| {
            let result = r.state.reset();
            if result.is_ok() {
                r.rejected = 0;
                r.dropped = 0;
                r.window_events.clear();
                r.last_time = 0;
                r.users.clear();
                r.limited = 0;
                r.cooldown_rejected = 0;
            }
            result
        })
    }

    pub fn set_paused(&mut self, id: &str, paused: bool) -> Option<Result<(), Error>> {
        self.rankings.get_mut(id).map(|r| {
            if r.paused != paused {
                r.state.touch()?;
                r.paused = paused;
            }
            Ok(())
        })
    }

    pub fn remove(&mut self, id: &str) -> bool {
        if self.rankings.remove(id).is_none() {
            return false;
        }
        self.by_source.retain(|_, ids| {
            ids.retain(|item| item != id);
            !ids.is_empty()
        });
        true
    }
}
