//! Headless access to in-memory aggregation definitions and snapshots.
use super::OperationError;
use crate::{recover_mutex, AppCore};
use tiktools_aggregation::{model::Definition, Snapshot};

/// One externally reported contribution (plugin or agent). `eventId` is a
/// caller-chosen idempotency key; replays are acknowledged without scoring.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalContribution {
    pub event_id: String,
    pub creator_id: String,
    pub room_id: String,
    /// Namespaced source such as `plugin.event` or `plugin.<id>.<name>`.
    pub source: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub custom_key: Option<String>,
    #[serde(default)]
    pub option: Option<String>,
    pub metrics: std::collections::BTreeMap<String, u64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContributionOutcome {
    pub duplicate: bool,
}

impl AppCore {
    /// Called while ranking and competition mutation locks are held in that
    /// order. A live competition must not lose or change its scoring source.
    fn ensure_ranking_not_in_use(&self, id: &str) -> Result<(), OperationError> {
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        let ids: Vec<String> =
            recover_mutex(&self.competitions.definitions, "competition definitions")
                .values()
                .filter(|definition| definition.ranking_id == id)
                .map(|definition| definition.id.clone())
                .collect();
        for competition in ids {
            let state = self.competition_state(&competition)?;
            if matches!(
                state.lifecycle,
                tiktools_competitions::Lifecycle::Active
                    | tiktools_competitions::Lifecycle::Paused
                    | tiktools_competitions::Lifecycle::Finalizing
            ) {
                return Err(OperationError::conflict(
                    "ranking is used by an active competition",
                ));
            }
        }
        Ok(())
    }
    pub fn ranking_create(&self, definition: Definition) -> Result<(), OperationError> {
        let _mutation = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        // Validate and check capacity before writing, without holding the hot-path
        // engine mutex during synchronous storage operations.
        let mut validation = tiktools_aggregation::engine::Engine::default();
        validation
            .create(definition.clone())
            .map_err(|_| OperationError::invalid("invalid ranking definition"))?;
        {
            let engine = recover_mutex(&self.aggregations.engine, "aggregations");
            let existing = engine.definitions();
            if existing.iter().any(|item| item.id == definition.id) {
                return Err(OperationError::conflict("ranking already exists"));
            }
            if existing.len() >= 128 {
                return Err(OperationError::unavailable("ranking capacity exhausted"));
            }
        }
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.insert_aggregation_definition(&definition) {
            tracing::error!(%error, "ranking definition persistence failed");
            return Err(OperationError::unavailable(
                "ranking definition persistence failed",
            ));
        }
        recover_mutex(&self.aggregations.engine, "aggregations")
            .create(definition)
            .map_err(|_| OperationError::unavailable("ranking creation failed"))
    }
    /// Updates are serialized with control mutations and require the ranking
    /// to be paused; score-affecting changes require an explicit reset.
    pub fn ranking_update(
        &self,
        definition: Definition,
        reset: bool,
    ) -> Result<(), OperationError> {
        let _mutation = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _competition = recover_mutex(&self.competitions.mutations, "competition mutations");
        if reset {
            self.ensure_ranking_not_in_use(&definition.id)?;
        }
        recover_mutex(&self.aggregations.engine, "aggregations")
            .validate_update(&definition, reset)
            .map_err(|error| match error {
                tiktools_aggregation::engine::DefinitionError::NotFound => {
                    OperationError::not_found("ranking not found")
                }
                tiktools_aggregation::engine::DefinitionError::NotPaused => {
                    OperationError::conflict("pause ranking before updating")
                }
                tiktools_aggregation::engine::DefinitionError::ResetRequired => {
                    OperationError::conflict("score-affecting changes require reset")
                }
                _ => OperationError::invalid("invalid ranking update"),
            })?;
        let mut rankings = self.ranking_list();
        if let Some(existing) = rankings
            .iter_mut()
            .find(|ranking| ranking.id == definition.id)
        {
            *existing = definition.clone();
        }
        let index = crate::services::competitions::CompetitionService::build_index(
            &recover_mutex(&self.competitions.definitions, "competition definitions"),
            &rankings,
        )
        .map_err(|_| OperationError::unavailable("competition command index cannot be rebuilt"))?;
        #[cfg(feature = "persistence")]
        match self.db.update_aggregation_definition(&definition) {
            Ok(true) => {}
            Ok(false) => {
                return Err(OperationError::unavailable(
                    "ranking definition missing from storage",
                ))
            }
            Err(error) => {
                tracing::error!(%error, "ranking update failed");
                return Err(OperationError::unavailable("ranking update failed"));
            }
        }
        recover_mutex(&self.aggregations.engine, "aggregations")
            .update(definition, reset)
            .map_err(|_| OperationError::unavailable("ranking update failed"))?;
        *recover_mutex(
            &self.competitions.command_index,
            "competition command index",
        ) = index;
        Ok(())
    }
    pub fn ranking_list(&self) -> Vec<Definition> {
        recover_mutex(&self.aggregations.engine, "aggregations").definitions()
    }
    pub fn ranking_get(&self, id: &str) -> Result<Definition, OperationError> {
        self.ranking_list()
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| OperationError::not_found("ranking not found"))
    }
    pub fn ranking_snapshot(&self, id: &str) -> Result<Snapshot, OperationError> {
        let (snapshot, limit) = {
            let mut engine = recover_mutex(&self.aggregations.engine, "aggregations");
            engine.expire_at(crate::now_millis());
            engine
                .snapshot_unranked(id)
                .ok_or_else(|| OperationError::not_found("ranking not found"))?
        };
        Ok(snapshot.rank(limit))
    }
    pub fn ranking_reset(&self, id: &str) -> Result<(), OperationError> {
        let _mutation = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _competition = recover_mutex(&self.competitions.mutations, "competition mutations");
        self.ensure_ranking_not_in_use(id)?;
        if !recover_mutex(&self.aggregations.engine, "aggregations")
            .definitions()
            .iter()
            .any(|definition| definition.id == id)
        {
            return Err(OperationError::not_found("ranking not found"));
        }
        // Clear durable state first: a crash after the in-memory reset must
        // not restore the discarded scores from an older runtime checkpoint.
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.clear_aggregation_checkpoint(id) {
            tracing::error!(%error, "ranking checkpoint clear failed");
            return Err(OperationError::unavailable("ranking reset failed"));
        }
        recover_mutex(&self.aggregations.engine, "aggregations")
            .reset(id)
            .ok_or_else(|| OperationError::not_found("ranking not found"))?
            .map_err(|_| OperationError::unavailable("ranking revision overflow"))
    }
    pub fn ranking_pause(&self, id: &str, paused: bool) -> Result<(), OperationError> {
        let _mutation = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        let _competition = recover_mutex(&self.competitions.mutations, "competition mutations");
        if paused {
            self.ensure_ranking_not_in_use(id)?;
        }
        recover_mutex(&self.aggregations.engine, "aggregations")
            .set_paused(id, paused)
            .ok_or_else(|| OperationError::not_found("ranking not found"))?
            .map_err(|_| OperationError::unavailable("ranking revision overflow"))
    }
    pub fn ranking_delete(&self, id: &str) -> Result<(), OperationError> {
        let _mutation = recover_mutex(&self.aggregations.mutations, "ranking mutations");
        if self.competitions.load_failed {
            return Err(OperationError::unavailable(
                "competition definitions unavailable",
            ));
        }
        if recover_mutex(&self.competitions.definitions, "competition definitions")
            .values()
            .any(|definition| definition.ranking_id == id)
        {
            return Err(OperationError::conflict("ranking is used by a competition"));
        }
        if !recover_mutex(&self.aggregations.engine, "aggregations")
            .definitions()
            .iter()
            .any(|definition| definition.id == id)
        {
            return Err(OperationError::not_found("ranking not found"));
        }
        #[cfg(feature = "persistence")]
        match self.db.delete_aggregation_definition(id) {
            Ok(true) => {}
            Ok(false) => {
                return Err(OperationError::unavailable(
                    "ranking definition missing from storage",
                ))
            }
            Err(error) => {
                tracing::error!(%error, "ranking deletion failed");
                return Err(OperationError::unavailable("ranking deletion failed"));
            }
        }
        recover_mutex(&self.aggregations.engine, "aggregations").remove(id);
        Ok(())
    }
    /// Apply an external contribution once. Only `plugin.` and `custom.`
    /// sources are accepted so native and ledger-backed sources keep their
    /// own authoritative adapters.
    pub fn ranking_contribute(
        &self,
        input: ExternalContribution,
    ) -> Result<ContributionOutcome, OperationError> {
        let bounded = |value: &Option<String>| {
            value
                .as_ref()
                .is_none_or(|v| !v.is_empty() && v.len() <= 512)
        };
        if input.event_id.is_empty()
            || input.event_id.len() > 256
            || !(input.source.starts_with("plugin.") || input.source.starts_with("custom."))
            || input.source.len() > 128
            || input.creator_id.is_empty()
            || input.room_id.is_empty()
            || input.metrics.is_empty()
            || input.metrics.len() > 16
            || input
                .metrics
                .keys()
                .any(|metric| metric.is_empty() || metric.len() > 128)
            || !bounded(&input.user)
            || !bounded(&input.label)
            || !bounded(&input.team)
            || !bounded(&input.custom_key)
            || !bounded(&input.option)
        {
            return Err(OperationError::invalid("invalid ranking contribution"));
        }
        // No consumer: nothing to account, and no idempotency row to write.
        if !recover_mutex(&self.aggregations.engine, "aggregations").has_source(&input.source) {
            return Ok(ContributionOutcome { duplicate: false });
        }
        let event_id = format!("{}:{}", input.source, input.event_id);
        #[cfg(feature = "persistence")]
        let admitted = self
            .db
            .admit_external_contribution(&event_id, crate::now_millis().min(i64::MAX as u64) as i64)
            .map_err(|error| {
                tracing::error!(%error, "external contribution admission failed");
                OperationError::unavailable("contribution admission failed")
            })?;
        #[cfg(not(feature = "persistence"))]
        let admitted = recover_mutex(&self.aggregations.external_seen, "external contributions")
            .admit(event_id);
        if !admitted {
            return Ok(ContributionOutcome { duplicate: true });
        }
        let metrics: Vec<(&str, u64)> = input
            .metrics
            .iter()
            .map(|(metric, value)| (metric.as_str(), *value))
            .collect();
        let metadata = input
            .label
            .as_ref()
            .map(|label| tiktools_aggregation::EntryMetadata {
                label: Some(label.clone()),
                ..Default::default()
            });
        let contribution = tiktools_aggregation::model::Contribution {
            creator_id: &input.creator_id,
            room_id: &input.room_id,
            source: &input.source,
            user: input.user.as_deref(),
            gift: None,
            option: input.option.as_deref(),
            team: input.team.as_deref(),
            custom_key: input.custom_key.as_deref(),
            metrics: &metrics,
        };
        let failures = recover_mutex(&self.aggregations.engine, "aggregations")
            .apply_at_with_metadata(&contribution, crate::now_millis(), metadata.as_ref(), None);
        for (ranking, error) in failures {
            tracing::warn!(%ranking, ?error, "external ranking contribution rejected");
        }
        Ok(ContributionOutcome { duplicate: false })
    }
}
