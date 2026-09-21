//! Analytics summaries, gift catalog, and creator history.

use crate::events::DomainEvent;
use crate::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GiftDebugResult {
    pub gift_id: Option<String>,
    pub icon_url: Option<String>,
    pub has_icon: bool,
    pub total_gifts: u64,
}

impl AppCore {
    pub fn creator_get(&self, unique_id: Option<&str>) -> Option<Value> {
        #[cfg(feature = "persistence")]
        {
            match self.db.load_creator(unique_id) {
                Ok(creator) => creator,
                Err(error) => {
                    tracing::warn!(%error, "could not load creator state");
                    None
                }
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = unique_id;
            None
        }
    }

    pub fn creator_recent(&self, limit: Option<i64>) -> Vec<Value> {
        #[cfg(feature = "persistence")]
        {
            match self
                .db
                .load_recent_creators(limit.unwrap_or(10).clamp(0, 1000))
            {
                Ok(creators) => creators,
                Err(error) => {
                    tracing::warn!(%error, "could not load creator history");
                    Vec::new()
                }
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = limit;
            Vec::new()
        }
    }

    pub fn creator_history_clear(&self) {
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.clear_creator_history() {
            tracing::warn!(%error, "could not clear creator history");
        }
        self.events
            .publish_domain(DomainEvent::CreatorChanged { unique_id: None });
    }

    pub fn analytics_summary(
        &self,
        creator_unique_id: Option<String>,
        start_day: Option<i64>,
        end_day: Option<i64>,
        limit: Option<i64>,
        tz_offset_secs: Option<i64>,
    ) -> Option<Value> {
        #[cfg(feature = "persistence")]
        {
            let now_unix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0);
            let today = crate::db::utc_day(now_unix);
            let end = end_day.unwrap_or(today);
            let start = start_day.unwrap_or(end - 6).min(end);
            let creator = creator_unique_id
                .filter(|value| !value.trim().is_empty())
                .or_else(|| self.current_creator_unique_id())
                .unwrap_or_default();
            if creator.is_empty() {
                return None;
            }
            match self.db.analytics_summary(
                &creator,
                start,
                end,
                limit.unwrap_or(10),
                tz_offset_secs.unwrap_or(0),
            ) {
                Ok(summary) => match serde_json::to_value(summary) {
                    Ok(summary) => Some(summary),
                    Err(error) => {
                        tracing::warn!(%error, "could not serialize analytics summary");
                        None
                    }
                },
                Err(error) => {
                    tracing::warn!(%error, "could not load analytics summary");
                    None
                }
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = (creator_unique_id, start_day, end_day, limit, tz_offset_secs);
            None
        }
    }

    pub fn gift_catalog(&self) -> Vec<Value> {
        #[cfg(feature = "persistence")]
        {
            match self.db.load_gift_catalog() {
                Ok(gifts) => gifts,
                Err(error) => {
                    tracing::warn!(%error, "could not load gift catalog");
                    Vec::new()
                }
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            Vec::new()
        }
    }

    pub fn gift_debug(&self, gift_id: Option<&str>) -> GiftDebugResult {
        let catalog = self.gift_catalog();
        let found = gift_id.and_then(|id| {
            let id = id.trim();
            catalog
                .iter()
                .find(|gift| gift.get("id").and_then(Value::as_str) == Some(id))
        });
        let icon_url = found
            .and_then(|gift| gift.get("iconUrl").and_then(Value::as_str))
            .map(str::to_owned);
        GiftDebugResult {
            gift_id: gift_id.map(str::to_owned),
            icon_url: icon_url.clone(),
            has_icon: icon_url.is_some(),
            total_gifts: catalog.len() as u64,
        }
    }
}
