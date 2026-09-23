use std::{sync::Arc, sync::RwLock, time::SystemTime};

use serde_json::{Map, Value};

use crate::db::DatabaseManager;
use crate::ipc::messages::{PartialPointsConfig, PointsConfig};
use tiktools_plugin_api::sync::{recover_rwlock_read, recover_rwlock_write};

pub struct PointsService {
    config: RwLock<PointsConfig>,
    viewers: RwLock<Vec<Value>>,
    #[cfg(feature = "persistence")]
    database: Option<Arc<DatabaseManager>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PointAward {
    pub unique_id: String,
    pub delta: f64,
    pub total_points: f64,
    pub level: u32,
    pub currency_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointAction {
    Chat,
    Gift,
    Like,
    Share,
    Follow,
    Join,
    Manual,
}

impl PointAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Gift => "gift",
            Self::Like => "like",
            Self::Share => "share",
            Self::Follow => "follow",
            Self::Join => "join",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AwardOptions {
    pub user_id: Option<String>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub count: Option<f64>,
    pub diamond_count: Option<f64>,
    pub is_subscriber: Option<bool>,
    pub custom_amount: Option<f64>,
}

impl Default for PointsService {
    fn default() -> Self {
        Self {
            config: RwLock::new(PointsConfig::default()),
            viewers: RwLock::new(Vec::new()),
            #[cfg(feature = "persistence")]
            database: None,
        }
    }
}

impl PointsService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let service = Self::default();
        #[cfg(feature = "persistence")]
        {
            // These locks were just created by `Self::default()` above and
            // are still thread-local to this constructor, so poisoning is
            // impossible: expect pins that invariant.
            if let Ok(mut config) = database.load_points_config() {
                config.normalize();
                *service.config.write().expect("points config poisoned") = config;
            }
            if let Ok(viewers) = database.load_point_viewers() {
                *service.viewers.write().expect("points viewers poisoned") = viewers;
            }
            // Keep the connection boundary in the service, while opening a
            // short-lived rusqlite connection per operation.
            let mut service = service;
            service.database = Some(database);
            service
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = database;
            service
        }
    }

    pub fn config(&self) -> PointsConfig {
        recover_rwlock_read(&self.config, "points config").clone()
    }

    pub fn award_points(
        &self,
        unique_id: &str,
        action: PointAction,
        options: AwardOptions,
    ) -> Option<PointAward> {
        let unique_id = clean_unique_id(unique_id)?;
        let config = self.config();
        let base_points = match action {
            PointAction::Manual => options.custom_amount.unwrap_or_default(),
            PointAction::Chat if config.points_per_chat_enabled => config.points_per_chat,
            PointAction::Gift if config.points_per_coin_enabled => {
                positive_or_one(options.diamond_count.or(options.count)) * config.points_per_coin
            }
            PointAction::Like if config.points_per_like_enabled => {
                positive_or_one(options.count) * config.points_per_like
            }
            PointAction::Share if config.points_per_share_enabled => config.points_per_share,
            PointAction::Follow if config.points_per_follow_enabled => config.points_per_follow,
            PointAction::Join if config.points_per_join_enabled => config.points_per_join,
            _ => 0.0,
        };

        let mut viewers = recover_rwlock_write(&self.viewers, "points viewers");
        let existing_index = viewers.iter().position(|viewer| {
            viewer.get("uniqueId").and_then(Value::as_str) == Some(unique_id.as_str())
        });
        let existing_subscriber = existing_index
            .and_then(|index| viewers.get(index))
            .and_then(|viewer| viewer.get("isSubscriber"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let is_subscriber = options.is_subscriber.unwrap_or(existing_subscriber);
        let mut awarded = base_points;
        if is_subscriber && config.sub_bonus_multiplier > 0.0 && awarded > 0.0 {
            awarded += awarded * (config.sub_bonus_multiplier / 100.0);
        }
        awarded = round_points(awarded);
        let current_points = existing_index
            .and_then(|index| viewers.get(index))
            .and_then(|viewer| viewer.get("points"))
            .and_then(Value::as_f64)
            .unwrap_or_default();
        let total_points = round_points((current_points + awarded).max(0.0));
        let points_per_level = config.points_per_level.max(10.0);
        let level = (total_points / points_per_level).floor() as u32 + 1;
        let now = now_millis();

        if let Some(index) = existing_index {
            if let Some(object) = viewers.get_mut(index).and_then(Value::as_object_mut) {
                update_existing_viewer(
                    object,
                    &options,
                    total_points,
                    level,
                    is_subscriber,
                    action,
                    now,
                );
            }
        } else {
            viewers.push(new_viewer(
                &unique_id,
                &options,
                total_points,
                level,
                is_subscriber,
                action,
                now,
            ));
        }
        viewers.sort_by(|left, right| {
            right
                .get("points")
                .and_then(Value::as_f64)
                .unwrap_or_default()
                .total_cmp(
                    &left
                        .get("points")
                        .and_then(Value::as_f64)
                        .unwrap_or_default(),
                )
        });
        let viewer_snapshot = viewers
            .iter()
            .find(|viewer| {
                viewer.get("uniqueId").and_then(Value::as_str) == Some(unique_id.as_str())
            })
            .cloned();
        drop(viewers);

        #[cfg(not(feature = "persistence"))]
        let _ = &viewer_snapshot;
        #[cfg(feature = "persistence")]
        if let (Some(database), Some(viewer)) = (&self.database, viewer_snapshot.as_ref()) {
            if let Err(error) = database.save_viewer(viewer, action.as_str(), awarded) {
                tracing::warn!(%error, "could not persist points award");
            }
        }

        Some(PointAward {
            unique_id,
            delta: awarded,
            total_points,
            level,
            currency_name: config.currency_name,
        })
    }

    pub fn update_config(&self, update: PartialPointsConfig) -> PointsConfig {
        let updated = {
            let mut config = recover_rwlock_write(&self.config, "points config");
            update.apply(&mut config);
            config.normalize();
            config.clone()
        };
        #[cfg(feature = "persistence")]
        if let Some(database) = &self.database {
            if let Err(error) = database.save_points_config(&updated) {
                tracing::warn!(%error, "could not persist points configuration");
            }
        }
        updated
    }

    pub fn leaderboard(&self, limit: Option<i64>) -> Vec<Value> {
        let limit = limit.unwrap_or(100).clamp(0, 1_000) as usize;
        recover_rwlock_read(&self.viewers, "points viewers")
            .iter()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn reset(&self, unique_id: Option<&str>) {
        let unique_id = unique_id.and_then(clean_unique_id);
        #[cfg(feature = "persistence")]
        if let Some(database) = &self.database {
            if let Err(error) = database.reset_viewers(unique_id.as_deref()) {
                tracing::warn!(%error, "could not persist points reset");
            }
        }
        let mut viewers = recover_rwlock_write(&self.viewers, "points viewers");
        if let Some(unique_id) = unique_id.as_deref() {
            if let Some(viewer) = viewers
                .iter_mut()
                .find(|viewer| viewer.get("uniqueId").and_then(Value::as_str) == Some(unique_id))
            {
                if let Some(object) = viewer.as_object_mut() {
                    object.insert("points".to_owned(), Value::from(0.0));
                    object.insert("level".to_owned(), Value::from(1));
                }
            }
        } else {
            for viewer in &mut *viewers {
                if let Some(object) = viewer.as_object_mut() {
                    object.insert("points".to_owned(), Value::from(0.0));
                    object.insert("level".to_owned(), Value::from(1));
                }
            }
        }
    }

    pub fn adjust(&self, unique_id: &str, delta: f64) -> Option<PointAward> {
        let unique_id = clean_unique_id(unique_id)?;
        if !delta.is_finite() {
            return None;
        }
        let award = {
            let mut viewers = recover_rwlock_write(&self.viewers, "points viewers");
            let viewer = viewers.iter_mut().find(|viewer| {
                viewer.get("uniqueId").and_then(Value::as_str) == Some(unique_id.as_str())
            })?;
            let points = viewer.get("points").and_then(Value::as_f64).unwrap_or(0.0) + delta;
            let points = points.max(0.0);
            let level = ((points / self.config().points_per_level.max(10.0)).floor() as u32)
                .saturating_add(1);
            if let Some(object) = viewer.as_object_mut() {
                object.insert("points".to_owned(), Value::from(points));
                object.insert("level".to_owned(), Value::from(level));
            }
            viewers.sort_by(|left, right| {
                right
                    .get("points")
                    .and_then(Value::as_f64)
                    .unwrap_or_default()
                    .total_cmp(
                        &left
                            .get("points")
                            .and_then(Value::as_f64)
                            .unwrap_or_default(),
                    )
            });
            PointAward {
                unique_id: unique_id.clone(),
                delta,
                total_points: points,
                level,
                currency_name: self.config().currency_name,
            }
        };
        #[cfg(feature = "persistence")]
        if let Some(database) = &self.database {
            if let Err(error) =
                database.update_viewer_points(&award.unique_id, award.total_points, award.level)
            {
                tracing::warn!(%error, "could not persist point adjustment");
            }
        }
        Some(award)
    }
}

fn clean_unique_id(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('@');
    (!value.is_empty()).then_some(value.to_owned())
}

fn positive_or_one(value: Option<f64>) -> f64 {
    value
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0)
}

fn round_points(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn update_existing_viewer(
    object: &mut Map<String, Value>,
    options: &AwardOptions,
    points: f64,
    level: u32,
    is_subscriber: bool,
    action: PointAction,
    now: i64,
) {
    if let Some(value) = options.user_id.as_ref() {
        object.insert("userId".to_owned(), Value::String(value.clone()));
    }
    if let Some(value) = options.nickname.as_ref() {
        object.insert("nickname".to_owned(), Value::String(value.clone()));
    }
    if let Some(value) = options.avatar_url.as_ref() {
        object.insert("avatarUrl".to_owned(), Value::String(value.clone()));
    }
    object.insert("points".to_owned(), Value::from(points));
    object.insert("level".to_owned(), Value::from(level));
    object.insert("isSubscriber".to_owned(), Value::from(is_subscriber));
    increment_counter(object, "totalChats", action == PointAction::Chat, 1);
    increment_counter(
        object,
        "totalCoins",
        action == PointAction::Gift,
        positive_or_one(options.diamond_count).round() as i64,
    );
    increment_counter(
        object,
        "totalLikes",
        action == PointAction::Like,
        positive_or_one(options.count).round() as i64,
    );
    increment_counter(object, "totalShares", action == PointAction::Share, 1);
    object.insert("lastSeen".to_owned(), Value::from(now));
}

fn new_viewer(
    unique_id: &str,
    options: &AwardOptions,
    points: f64,
    level: u32,
    is_subscriber: bool,
    action: PointAction,
    now: i64,
) -> Value {
    let mut object = Map::new();
    object.insert("uniqueId".to_owned(), Value::String(unique_id.to_owned()));
    if let Some(value) = options.user_id.as_ref() {
        object.insert("userId".to_owned(), Value::String(value.clone()));
    }
    object.insert(
        "nickname".to_owned(),
        Value::String(
            options
                .nickname
                .clone()
                .unwrap_or_else(|| unique_id.to_owned()),
        ),
    );
    if let Some(value) = options.avatar_url.as_ref() {
        object.insert("avatarUrl".to_owned(), Value::String(value.clone()));
    }
    object.insert("points".to_owned(), Value::from(points));
    object.insert("level".to_owned(), Value::from(level));
    object.insert("isSubscriber".to_owned(), Value::from(is_subscriber));
    object.insert(
        "totalChats".to_owned(),
        Value::from(i64::from(action == PointAction::Chat)),
    );
    object.insert(
        "totalCoins".to_owned(),
        Value::from(if action == PointAction::Gift {
            positive_or_one(options.diamond_count).round() as i64
        } else {
            0
        }),
    );
    object.insert(
        "totalLikes".to_owned(),
        Value::from(if action == PointAction::Like {
            positive_or_one(options.count).round() as i64
        } else {
            0
        }),
    );
    object.insert(
        "totalShares".to_owned(),
        Value::from(i64::from(action == PointAction::Share)),
    );
    object.insert("firstSeen".to_owned(), Value::from(now));
    object.insert("lastSeen".to_owned(), Value::from(now));
    Value::Object(object)
}

fn increment_counter(object: &mut Map<String, Value>, key: &str, enabled: bool, amount: i64) {
    if !enabled {
        return;
    }
    let current = object.get(key).and_then(Value::as_i64).unwrap_or_default();
    object.insert(key.to_owned(), Value::from(current.saturating_add(amount)));
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn award_points_matches_existing_defaults() {
        let service = PointsService::default();
        let award = service.award_points("@alice", PointAction::Chat, AwardOptions::default());
        assert_eq!(
            award.as_ref().map(|award| award.unique_id.as_str()),
            Some("alice")
        );
        assert_eq!(award.as_ref().map(|award| award.total_points), Some(1.0));
        assert_eq!(service.leaderboard(Some(1))[0]["totalChats"], 1);
    }

    #[test]
    fn reset_keeps_viewers_and_counters_but_zeroes_points() {
        let service = PointsService::default();
        service.award_points("alice", PointAction::Chat, AwardOptions::default());
        service.reset(None);
        let viewer = &service.leaderboard(Some(1))[0];
        assert_eq!(viewer["points"], 0.0);
        assert_eq!(viewer["level"], 1);
        assert_eq!(viewer["totalChats"], 1);
    }

    #[test]
    fn negative_adjustments_clamp_at_zero() {
        // The `core.points` execution path (moderation penalties included)
        // adjusts through here: viewers can never drop below zero.
        let service = PointsService::default();
        service.award_points(
            "alice",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(5.0),
                ..AwardOptions::default()
            },
        );
        let award = service.adjust("alice", -10.0).expect("viewer exists");
        assert_eq!(award.total_points, 0.0);
        assert_eq!(award.delta, -10.0);
        assert_eq!(service.leaderboard(Some(1))[0]["points"], 0.0);
        // Unknown viewers stay unknown: no negative phantom rows.
        assert!(service.adjust("nobody", -10.0).is_none());
        assert!(service.adjust("alice", f64::NAN).is_none());
    }

    #[test]
    fn point_configuration_normalizes_unsafe_ranges() {
        let mut config = PointsConfig {
            points_per_coin: -10.0,
            points_per_share: f64::NAN,
            points_per_chat: 2.0,
            points_per_like: f64::INFINITY,
            points_per_follow: 2.0,
            points_per_join: 2.0,
            sub_bonus_multiplier: -1.0,
            points_per_level: 1.0,
            currency_name: "   ".to_owned(),
            ..PointsConfig::default()
        };
        config.normalize();
        assert_eq!(config.points_per_coin, 0.0);
        assert_eq!(config.points_per_share, 3.0);
        assert_eq!(config.points_per_like, 0.1);
        assert_eq!(config.sub_bonus_multiplier, 0.0);
        assert_eq!(config.points_per_level, 10.0);
        assert_eq!(config.currency_name, "Points");
    }
}
