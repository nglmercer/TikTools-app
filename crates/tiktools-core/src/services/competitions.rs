//! AppCore-owned competition definitions. Lifecycle and effects are separate
//! from definitions and must be durably integrated before execution.
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
use tiktools_aggregation::model::Definition as RankingDefinition;
use tiktools_competitions::{participants::CommandIndex, Definition};

#[derive(Default)]
pub(crate) struct CompetitionService {
    pub(crate) definitions: Mutex<BTreeMap<String, Definition>>,
    pub(crate) mutations: Mutex<()>,
    pub(crate) command_index: Mutex<CommandIndex>,
    pub(crate) load_failed: bool,
    pub(crate) participation_accepted: AtomicU64,
    pub(crate) participation_duplicate: AtomicU64,
    pub(crate) participation_rejected: AtomicU64,
    pub(crate) participation_failed: AtomicU64,
    pub(crate) timer_started: std::sync::atomic::AtomicBool,
    #[cfg(feature = "persistence")]
    pub(crate) action_worker: Mutex<Option<tokio::task::JoinHandle<()>>>,
    #[cfg(feature = "persistence")]
    pub(crate) action_inflight: Mutex<Option<(String, u64)>>,
}
impl CompetitionService {
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "accepted": self.participation_accepted.load(Ordering::Relaxed),
            "duplicate": self.participation_duplicate.load(Ordering::Relaxed),
            "rejected": self.participation_rejected.load(Ordering::Relaxed),
            "failed": self.participation_failed.load(Ordering::Relaxed),
            "definitionsUnavailable": self.load_failed,
        })
    }
    pub(crate) fn build_index(
        definitions: &BTreeMap<String, Definition>,
        rankings: &[RankingDefinition],
    ) -> Result<CommandIndex, tiktools_competitions::Error> {
        let mut index = CommandIndex::default();
        let mut vote_rankings = std::collections::HashSet::new();
        for definition in definitions.values() {
            let ranking = rankings
                .iter()
                .find(|r| r.id == definition.ranking_id)
                .ok_or(tiktools_competitions::Error::InvalidCandidate)?;
            if ranking
                .sources
                .iter()
                .any(|source| source.source == "competition.vote")
                && !vote_rankings.insert(&ranking.id)
            {
                return Err(tiktools_competitions::Error::InvalidCandidate);
            }
            index.register(definition, &ranking.creator_id, &ranking.room_id)?;
            if ranking
                .sources
                .iter()
                .any(|rule| rule.source == "competition.vote")
            {
                let sources = ["tiktok.like", "tiktok.share", "tiktok.gift"]
                    .into_iter()
                    .filter(|source| {
                        ranking
                            .sources
                            .iter()
                            .any(|rule| rule.enabled && rule.weight > 0 && rule.source == *source)
                    })
                    .collect::<Vec<_>>();
                index.register_sources(
                    &definition.id,
                    &ranking.creator_id,
                    &ranking.room_id,
                    &sources,
                )?;
            }
        }
        Ok(index)
    }
    /// Snapshot only after ingress is stopped, under the same control locks
    /// as competition transitions. Failed writes leave checkpoints open.
    #[cfg(feature = "persistence")]
    pub(crate) fn checkpoint_shutdown(
        core: &crate::AppCore,
    ) -> Result<(), crate::db::DatabaseError> {
        let _ranking = crate::recover_mutex(&core.aggregations.mutations, "ranking mutations");
        let _competition =
            crate::recover_mutex(&core.competitions.mutations, "competition mutations");
        if core.competitions.load_failed {
            return Err(crate::db::DatabaseError::Invalid(
                "competition definitions unavailable".into(),
            ));
        }
        let definitions =
            crate::recover_mutex(&core.competitions.definitions, "competition definitions")
                .values()
                .cloned()
                .collect::<Vec<_>>();
        let mut checkpoints = Vec::new();
        #[cfg(feature = "native-tiktok")]
        let ingress_complete = !core.live_ingress_stop.is_cancelled()
            && core.live_ingress.cancelled.load(Ordering::Relaxed) == 0
            && core.live_ingress.abandoned.load(Ordering::Relaxed) == 0
            && core
                .live_ingress
                .admission_cancelled
                .load(Ordering::Relaxed)
                == 0;
        #[cfg(not(feature = "native-tiktok"))]
        let ingress_complete = true;
        for definition in definitions {
            let state = core
                .db
                .load_competition_state(&definition.id)?
                .ok_or_else(|| {
                    crate::db::DatabaseError::Invalid("competition state missing".into())
                })?;
            if !matches!(
                state.lifecycle,
                tiktools_competitions::Lifecycle::Active
                    | tiktools_competitions::Lifecycle::Paused
                    | tiktools_competitions::Lifecycle::Finalizing
            ) {
                continue;
            }
            let engine = crate::recover_mutex(&core.aggregations.engine, "aggregations");
            let ranking = engine
                .definitions()
                .into_iter()
                .find(|ranking| ranking.id == definition.ranking_id);
            let Some(ranking) = ranking else {
                return Err(crate::db::DatabaseError::Invalid("ranking missing".into()));
            };
            if !ranking
                .sources
                .iter()
                .any(|source| source.source == "competition.vote")
            {
                continue;
            }
            let snapshot = engine
                .snapshot(&definition.ranking_id)
                .ok_or_else(|| crate::db::DatabaseError::Invalid("ranking state missing".into()))?;
            checkpoints.push((
                definition.id,
                state.generation,
                ingress_complete && snapshot.integrity_status != "degraded",
            ));
        }
        let owner = core
            .aggregations
            .checkpoint_owner
            .as_deref()
            .ok_or_else(|| {
                crate::db::DatabaseError::Invalid("competition integrity owner unavailable".into())
            })?;
        core.db.save_competition_integrity(&checkpoints, owner)
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn restore(db: &crate::db::DatabaseManager) -> Self {
        match db.load_competition_definitions() {
            Ok(definitions) => {
                let mut restored = BTreeMap::new();
                for definition in definitions {
                    if definition.validate().is_err()
                        || restored.len() >= 128
                        || restored.contains_key(&definition.id)
                    {
                        tracing::error!(id = %definition.id, "invalid persisted competition definition");
                        return Self {
                            load_failed: true,
                            ..Self::default()
                        };
                    }
                    restored.insert(definition.id.clone(), definition);
                }
                let index = db
                    .load_aggregation_definitions()
                    .ok()
                    .and_then(|rankings| Self::build_index(&restored, &rankings).ok());
                match index {
                    Some(index) => Self {
                        definitions: Mutex::new(restored),
                        command_index: Mutex::new(index),
                        ..Self::default()
                    },
                    None => {
                        tracing::error!("cannot restore competition command index");
                        Self {
                            load_failed: true,
                            ..Self::default()
                        }
                    }
                }
            }
            Err(error) => {
                tracing::error!(%error, "cannot load competition definitions");
                Self {
                    load_failed: true,
                    ..Self::default()
                }
            }
        }
    }
}
