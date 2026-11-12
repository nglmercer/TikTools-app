//! Local lifecycle transitions, persisted state and winner-action recovery. The worker
//! admission and uncertain-action checks retain their existing guards.

use crate::{control::OperationError, recover_mutex, AppCore};

impl AppCore {
    #[cfg(feature = "persistence")]
    pub fn competition_state(
        &self,
        id: &str,
    ) -> Result<tiktools_competitions::LifecycleState, OperationError> {
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        self.db
            .load_competition_state(id)
            .map_err(|error| {
                tracing::error!(%error, "competition state read failed");
                OperationError::unavailable("competition state read failed")
            })?
            .ok_or_else(|| OperationError::not_found("competition not found"))
    }
    #[cfg(not(feature = "persistence"))]
    pub fn competition_state(
        &self,
        _id: &str,
    ) -> Result<tiktools_competitions::LifecycleState, OperationError> {
        Err(OperationError::unavailable(
            "competition state requires persistence",
        ))
    }
    pub fn competition_transition(
        self: &std::sync::Arc<Self>,
        id: &str,
        transition: tiktools_competitions::Transition,
    ) -> Result<tiktools_competitions::LifecycleState, OperationError> {
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        if matches!(
            transition,
            tiktools_competitions::Transition::Start | tiktools_competitions::Transition::Resume
        ) {
            let definition = self.competition_get(id)?;
            if self.ranking_snapshot(&definition.ranking_id)?.paused {
                return Err(OperationError::conflict(
                    "resume the competition ranking first",
                ));
            }
        }
        #[cfg(feature = "persistence")]
        {
            if matches!(transition, tiktools_competitions::Transition::Start)
                && self.aggregations.checkpoint_owner.is_none()
            {
                return Err(OperationError::unavailable(
                    "competition integrity owner unavailable",
                ));
            }
            let state = self
                .db
                .transition_competition_owned(
                    id,
                    transition,
                    crate::now_millis(),
                    self.aggregations.checkpoint_owner.as_deref(),
                )
                .map_err(|error| match error {
                    crate::db::DatabaseError::Invalid(_) => {
                        OperationError::conflict("invalid competition transition")
                    }
                    error => {
                        tracing::error!(%error, "competition transition failed");
                        OperationError::unavailable("competition transition failed")
                    }
                })?
                .ok_or_else(|| OperationError::not_found("competition not found"))?;
            let event_type = match transition {
                tiktools_competitions::Transition::Start => Some("competition.started"),
                tiktools_competitions::Transition::Pause => Some("competition.paused"),
                tiktools_competitions::Transition::Resume => Some("competition.resumed"),
                tiktools_competitions::Transition::Cancel => Some("competition.cancelled"),
                _ => None,
            };
            if let (Some(kind), Some(data)) =
                (event_type, self.competition_context(id, state.generation))
            {
                self.publish_competition_event(kind, data);
            }
            Ok(state)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (id, transition);
            Err(OperationError::unavailable(
                "competition state requires persistence",
            ))
        }
    }
    pub fn competition_action(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<Option<tiktools_competitions::ActionExecution>, OperationError> {
        self.competition_get(id)?;
        if generation > i64::MAX as u64 {
            return Err(OperationError::invalid("invalid generation"));
        }
        #[cfg(feature = "persistence")]
        return self.db.competition_action(id, generation).map_err(|error| {
            tracing::error!(%error, "competition action status read failed");
            OperationError::unavailable("competition action status read failed")
        });
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::unavailable(
            "competition actions require persistence",
        ))
    }
    /// Explicitly mark a stranded running action uncertain; never requeue it.
    pub fn competition_mark_action_uncertain(
        &self,
        id: &str,
        generation: u64,
    ) -> Result<(), OperationError> {
        self.competition_get(id)?;
        if generation > i64::MAX as u64 {
            return Err(OperationError::invalid("invalid generation"));
        }
        #[cfg(feature = "persistence")]
        {
            // Hold the same guard used by the worker from before claiming
            // until execution ends, preventing recovery of a live invocation.
            let in_flight = recover_mutex(
                &self.competitions.action_inflight,
                "competition action in flight",
            );
            if in_flight
                .as_ref()
                .is_some_and(|(active_id, active_generation)| {
                    active_id == id && *active_generation == generation
                })
            {
                return Err(OperationError::conflict(
                    "competition action is still executing",
                ));
            }
            let changed = self
                .db
                .mark_competition_action_uncertain(
                    id,
                    generation,
                    crate::now_millis().min(i64::MAX as u64) as i64,
                )
                .map_err(|_| OperationError::unavailable("competition action recovery failed"))?;
            if !changed {
                return Err(OperationError::conflict("action is not running"));
            }
            Ok(())
        }
        #[cfg(not(feature = "persistence"))]
        Err(OperationError::unavailable(
            "competition actions require persistence",
        ))
    }
    pub fn competition_resolve_action(
        &self,
        id: &str,
        generation: u64,
        resolution: tiktools_competitions::ActionResolution,
    ) -> Result<(), OperationError> {
        self.competition_get(id)?;
        if generation > i64::MAX as u64 {
            return Err(OperationError::invalid("invalid generation"));
        }
        #[cfg(feature = "persistence")]
        {
            let updated = self
                .db
                .resolve_uncertain_competition_action(
                    id,
                    generation,
                    resolution,
                    crate::now_millis(),
                )
                .map_err(|error| {
                    tracing::error!(%error, "competition action resolution failed");
                    OperationError::unavailable("competition action resolution failed")
                })?;
            if !updated {
                return Err(OperationError::conflict("action is not uncertain"));
            }
            Ok(())
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = resolution;
            Err(OperationError::unavailable(
                "competition actions require persistence",
            ))
        }
    }
}
