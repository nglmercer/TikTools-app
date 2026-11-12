//! Competition definition creation, replacement, deletion and inventory. Persisted
//! definitions commit before the in-memory definitions and command index are published.

use crate::{control::OperationError, recover_mutex, AppCore};
use tiktools_competitions::Definition;

impl AppCore {
    pub fn competition_create(&self, definition: Definition) -> Result<(), OperationError> {
        definition
            .validate()
            .map_err(|_| OperationError::invalid("invalid or ambiguous competition definition"))?;
        // The shared lock order prevents a ranking being deleted while a
        // competition referencing it is being created.
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        let ranking = self
            .ranking_get(&definition.ranking_id)
            .map_err(|_| OperationError::invalid("ranking not found"))?;
        if definition.kind == tiktools_competitions::CompetitionKind::LeaderboardChallenge
            && ranking
                .sources
                .iter()
                .any(|source| source.source == "competition.vote")
        {
            return Err(OperationError::invalid(
                "leaderboard challenges require an ordinary ranking",
            ));
        }
        if ranking
            .sources
            .iter()
            .any(|source| source.source == "competition.vote")
            && self.ranking_snapshot(&ranking.id)?.total_score != 0
        {
            return Err(OperationError::conflict(
                "reset the vote ranking before creating a competition",
            ));
        }
        {
            let definitions =
                recover_mutex(&self.competitions.definitions, "competition definitions");
            if definitions.contains_key(&definition.id) {
                return Err(OperationError::conflict("competition already exists"));
            }
            if definitions.len() >= 128 {
                return Err(OperationError::unavailable(
                    "competition capacity exhausted",
                ));
            }
        }
        let mut candidate =
            recover_mutex(&self.competitions.definitions, "competition definitions").clone();
        candidate.insert(definition.id.clone(), definition.clone());
        let index = crate::services::competitions::CompetitionService::build_index(
            &candidate,
            &self.ranking_list(),
        )
        .map_err(|_| OperationError::invalid("competition command index capacity exceeded"))?;
        #[cfg(feature = "persistence")]
        self.db
            .insert_competition_definition(&definition)
            .map_err(|error| {
                tracing::error!(%error, "competition persistence failed");
                OperationError::unavailable("competition persistence failed")
            })?;
        #[cfg(not(feature = "persistence"))]
        {
            let _ = index;
            return Err(OperationError::unavailable(
                "competition creation requires persistence",
            ));
        }
        #[cfg(feature = "persistence")]
        {
            *recover_mutex(&self.competitions.definitions, "competition definitions") = candidate;
            *recover_mutex(
                &self.competitions.command_index,
                "competition command index",
            ) = index;
            Ok(())
        }
    }
    pub fn competition_update(&self, definition: Definition) -> Result<(), OperationError> {
        definition
            .validate()
            .map_err(|_| OperationError::invalid("invalid or ambiguous competition definition"))?;
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        let ranking = self
            .ranking_get(&definition.ranking_id)
            .map_err(|_| OperationError::invalid("ranking not found"))?;
        if ranking
            .sources
            .iter()
            .any(|source| source.source == "competition.vote")
            && self.ranking_snapshot(&ranking.id)?.total_score != 0
        {
            return Err(OperationError::conflict(
                "reset the vote ranking before updating a competition",
            ));
        }
        if !recover_mutex(&self.competitions.definitions, "competition definitions")
            .contains_key(&definition.id)
        {
            return Err(OperationError::not_found("competition not found"));
        }
        let mut candidate =
            recover_mutex(&self.competitions.definitions, "competition definitions").clone();
        candidate.insert(definition.id.clone(), definition.clone());
        let index = crate::services::competitions::CompetitionService::build_index(
            &candidate,
            &self.ranking_list(),
        )
        .map_err(|_| OperationError::invalid("competition command index capacity exceeded"))?;
        #[cfg(feature = "persistence")]
        match self.db.update_competition_definition(&definition) {
            Ok(true) => {}
            Ok(false) => {
                return Err(OperationError::conflict(
                    "competition has results or storage changed",
                ))
            }
            Err(error) => {
                tracing::error!(%error, "competition update failed");
                return Err(OperationError::unavailable("competition update failed"));
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = index;
            return Err(OperationError::unavailable(
                "competition update requires persistence",
            ));
        }
        #[cfg(feature = "persistence")]
        {
            *recover_mutex(&self.competitions.definitions, "competition definitions") = candidate;
            *recover_mutex(
                &self.competitions.command_index,
                "competition command index",
            ) = index;
            Ok(())
        }
    }
    pub fn competition_delete(&self, id: &str) -> Result<(), OperationError> {
        let _ranking = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _mutation = recover_mutex(&self.competitions.mutations, "competition mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        if !recover_mutex(&self.competitions.definitions, "competition definitions")
            .contains_key(id)
        {
            return Err(OperationError::not_found("competition not found"));
        }
        #[cfg(feature = "persistence")]
        match self.db.delete_competition_definition(id) {
            Ok(true) => {}
            Ok(false) => {
                return Err(OperationError::conflict(
                    "competition has results or storage changed",
                ))
            }
            Err(error) => {
                tracing::error!(%error, "competition deletion failed");
                return Err(OperationError::unavailable("competition deletion failed"));
            }
        }
        #[cfg(not(feature = "persistence"))]
        return Err(OperationError::unavailable(
            "competition deletion requires persistence",
        ));
        #[cfg(feature = "persistence")]
        {
            recover_mutex(&self.competitions.definitions, "competition definitions").remove(id);
            recover_mutex(
                &self.competitions.command_index,
                "competition command index",
            )
            .remove(id);
            Ok(())
        }
    }
    pub fn competition_list(&self) -> Result<Vec<Definition>, OperationError> {
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        Ok(
            recover_mutex(&self.competitions.definitions, "competition definitions")
                .values()
                .cloned()
                .collect(),
        )
    }
    pub fn competition_get(&self, id: &str) -> Result<Definition, OperationError> {
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        recover_mutex(&self.competitions.definitions, "competition definitions")
            .get(id)
            .cloned()
            .ok_or_else(|| OperationError::not_found("competition not found"))
    }
}
