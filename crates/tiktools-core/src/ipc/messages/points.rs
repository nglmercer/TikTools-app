use serde::{Deserialize, Serialize};

use super::validation::IpcMessageError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PartialPointsConfig {
    #[serde(default)]
    pub currency_name: Option<String>,
    #[serde(default)]
    pub points_per_coin: Option<f64>,
    #[serde(default)]
    pub points_per_coin_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_share: Option<f64>,
    #[serde(default)]
    pub points_per_share_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_chat: Option<f64>,
    #[serde(default)]
    pub points_per_chat_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_like: Option<f64>,
    #[serde(default)]
    pub points_per_like_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_follow: Option<f64>,
    #[serde(default)]
    pub points_per_follow_enabled: Option<bool>,
    #[serde(default)]
    pub points_per_join: Option<f64>,
    #[serde(default)]
    pub points_per_join_enabled: Option<bool>,
    #[serde(default)]
    pub sub_bonus_multiplier: Option<f64>,
    #[serde(default)]
    pub points_per_level: Option<f64>,
}

impl PartialPointsConfig {
    pub fn apply(self, config: &mut PointsConfig) {
        if let Some(value) = self.currency_name {
            config.currency_name = value;
        }
        if let Some(value) = self.points_per_coin {
            config.points_per_coin = value;
        }
        if let Some(value) = self.points_per_coin_enabled {
            config.points_per_coin_enabled = value;
        }
        if let Some(value) = self.points_per_share {
            config.points_per_share = value;
        }
        if let Some(value) = self.points_per_share_enabled {
            config.points_per_share_enabled = value;
        }
        if let Some(value) = self.points_per_chat {
            config.points_per_chat = value;
        }
        if let Some(value) = self.points_per_chat_enabled {
            config.points_per_chat_enabled = value;
        }
        if let Some(value) = self.points_per_like {
            config.points_per_like = value;
        }
        if let Some(value) = self.points_per_like_enabled {
            config.points_per_like_enabled = value;
        }
        if let Some(value) = self.points_per_follow {
            config.points_per_follow = value;
        }
        if let Some(value) = self.points_per_follow_enabled {
            config.points_per_follow_enabled = value;
        }
        if let Some(value) = self.points_per_join {
            config.points_per_join = value;
        }
        if let Some(value) = self.points_per_join_enabled {
            config.points_per_join_enabled = value;
        }
        if let Some(value) = self.sub_bonus_multiplier {
            config.sub_bonus_multiplier = value;
        }
        if let Some(value) = self.points_per_level {
            config.points_per_level = value;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointsConfig {
    pub currency_name: String,
    pub points_per_coin: f64,
    pub points_per_coin_enabled: bool,
    pub points_per_share: f64,
    pub points_per_share_enabled: bool,
    pub points_per_chat: f64,
    pub points_per_chat_enabled: bool,
    pub points_per_like: f64,
    pub points_per_like_enabled: bool,
    pub points_per_follow: f64,
    pub points_per_follow_enabled: bool,
    pub points_per_join: f64,
    pub points_per_join_enabled: bool,
    pub sub_bonus_multiplier: f64,
    pub points_per_level: f64,
}

impl PointsConfig {
    /// Normalizes values at the service boundary so legacy databases and IPC
    /// updates use the same safe ranges. Manual adjustments can still be
    /// negative; per-event rates and the level threshold cannot.
    pub fn normalize(&mut self) {
        const MAX_RATE: f64 = 1_000_000_000.0;
        const MAX_MULTIPLIER: f64 = 10_000.0;
        const MIN_POINTS_PER_LEVEL: f64 = 10.0;

        if self.currency_name.trim().is_empty() {
            self.currency_name = "Points".to_owned();
        } else {
            self.currency_name = self.currency_name.trim().chars().take(64).collect();
        }
        self.points_per_coin = finite_nonnegative(self.points_per_coin, 1.0, MAX_RATE);
        self.points_per_share = finite_nonnegative(self.points_per_share, 3.0, MAX_RATE);
        self.points_per_chat = finite_nonnegative(self.points_per_chat, 1.0, MAX_RATE);
        self.points_per_like = finite_nonnegative(self.points_per_like, 0.1, MAX_RATE);
        self.points_per_follow = finite_nonnegative(self.points_per_follow, 5.0, MAX_RATE);
        self.points_per_join = finite_nonnegative(self.points_per_join, 0.5, MAX_RATE);
        self.sub_bonus_multiplier =
            finite_nonnegative(self.sub_bonus_multiplier, 0.0, MAX_MULTIPLIER);
        self.points_per_level = if self.points_per_level.is_finite() {
            self.points_per_level.clamp(MIN_POINTS_PER_LEVEL, MAX_RATE)
        } else {
            100.0
        };
    }
}

fn finite_nonnegative(value: f64, fallback: f64, maximum: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, maximum)
    } else {
        fallback
    }
}

impl Default for PointsConfig {
    fn default() -> Self {
        Self {
            currency_name: "Points".to_owned(),
            points_per_coin: 1.0,
            points_per_coin_enabled: true,
            points_per_share: 3.0,
            points_per_share_enabled: true,
            points_per_chat: 1.0,
            points_per_chat_enabled: true,
            points_per_like: 0.1,
            points_per_like_enabled: true,
            points_per_follow: 5.0,
            points_per_follow_enabled: true,
            points_per_join: 0.5,
            points_per_join_enabled: false,
            sub_bonus_multiplier: 0.0,
            points_per_level: 100.0,
        }
    }
}
impl PartialPointsConfig {
    pub(crate) fn validate(&self) -> Result<(), IpcMessageError> {
        for value in [
            self.points_per_coin,
            self.points_per_share,
            self.points_per_chat,
            self.points_per_like,
            self.points_per_follow,
            self.points_per_join,
            self.sub_bonus_multiplier,
            self.points_per_level,
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(IpcMessageError::InvalidField("points config"));
            }
        }
        Ok(())
    }
}
