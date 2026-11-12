//! Stable topic mapping, transport envelopes and lossy-lane classification.
use super::{DomainEvent, EventEnvelopeError};
use serde_json::Value;

impl DomainEvent {
    pub fn topic(&self) -> &'static str {
        match self {
            Self::CloudContextInvalidated {} => "cloud.context.invalidated",
            Self::AppUpdateStateChanged { .. } => "app.update.stateChanged",
            Self::PluginInstalled { .. } => "plugin.installed",
            Self::PluginUninstalled { .. } => "plugin.uninstalled",
            Self::PluginStarted { .. } => "plugin.started",
            Self::PluginStopped { .. } => "plugin.stopped",
            Self::PluginProgress { .. } => "plugin.progress",
            Self::PluginSettingsChanged { .. } => "plugin.settings-changed",
            Self::PluginUpdated { .. } => "plugin.updated",
            Self::PluginJob { .. } => "plugin.job",
            Self::PluginUpdates { .. } => "plugin.updates",
            Self::PluginEvent { .. } => "plugin.event",
            Self::PluginStatus { .. } => "plugin.status",
            Self::AutomationRunCompleted { .. } => "automation.run.completed",
            Self::AutomationRunsChanged { .. } => "automation.runs.changed",
            Self::LiveConnected { .. } => "live.connected",
            Self::LiveDisconnected => "live.disconnected",
            Self::LiveMonitorChanged { .. } => "live.monitor",
            Self::LiveEvent { .. } => "live.event",
            Self::LiveUiEvent { .. } => "live.ui-event",
            Self::LiveLikesRanking { .. } => "live.likes-ranking",
            Self::LiveLikesRankingFull { .. } => "live.likes-ranking-full",
            Self::CompetitionStarted(..) => "competition.started",
            Self::CompetitionPaused(..) => "competition.paused",
            Self::CompetitionResumed(..) => "competition.resumed",
            Self::CompetitionCancelled(..) => "competition.cancelled",
            Self::CompetitionFinished(..) => "competition.finished",
            Self::CompetitionWinnerSelected(..) => "competition.winner.selected",
            Self::CompetitionActionCompleted(..) => "competition.action.completed",
            Self::CompetitionActionFailed(..) => "competition.action.failed",
            Self::CompetitionResult { .. } => "competition.result",
            Self::RankingUpdated { .. } => "ranking.updated",
            Self::RoomStats { .. } => "room.stats",
            Self::GiftsCatalog { .. } => "gifts.catalog",
            Self::LiveReconnecting { .. } => "live.reconnecting",
            Self::LiveError { .. } => "live.error",
            Self::PointsChanged { .. } => "points.changed",
            Self::PointsReset { .. } => "points.reset",
            Self::WorkflowChanged { .. } => "workflow.changed",
            Self::CreatorChanged { .. } => "creator.changed",
            Self::AnalyticsUpdated { .. } => "analytics.updated",
            Self::Shutdown => "shutdown",
        }
    }

    /// Converts this event into its transport-neutral envelope
    /// (`{ topic, data }`), preserving the exact wire shape every client
    /// already receives. Never substitutes `null` on failure.
    pub fn to_envelope(
        &self,
    ) -> Result<tiktools_plugin_api::DomainEventEnvelope, EventEnvelopeError> {
        let value = serde_json::to_value(self)
            .map_err(|error| EventEnvelopeError::Serialization(error.to_string()))?;
        let data = match (value.get("data").cloned(), self) {
            (Some(data), _) => data,
            // Adjacently-tagged unit variants serialize as `{"topic": ...}`
            // with no content payload; their envelope data is null by
            // definition, matching what subscribers already receive.
            (None, Self::LiveDisconnected | Self::Shutdown) => Value::Null,
            (None, _) => return Err(EventEnvelopeError::MissingData),
        };
        Ok(tiktools_plugin_api::DomainEventEnvelope::new(
            self.topic(),
            data,
        ))
    }

    /// High-rate feed and superseding snapshots (live feed, room stats,
    /// analytics, gift catalog, plugin progress) travel the lossy lane:
    /// a lagged subscriber skips them and takes the latest. Every other
    /// event — connection/lifecycle transitions, errors, points changes,
    /// shutdown — travels the reliable lane, so a feed burst can never
    /// overwrite a state transition a subscriber has not read yet.
    /// Mirrors the WebView `Coalescable`/`Droppable` classes.
    pub fn is_lossy(&self) -> bool {
        matches!(
            self,
            Self::LiveEvent { .. }
                | Self::LiveUiEvent { .. }
                | Self::LiveLikesRanking { .. }
                | Self::LiveLikesRankingFull { .. }
                | Self::RankingUpdated { .. }
                | Self::CompetitionResult { .. }
                | Self::RoomStats { .. }
                | Self::AnalyticsUpdated { .. }
                | Self::GiftsCatalog { .. }
                | Self::PluginProgress { .. }
        )
    }
}
