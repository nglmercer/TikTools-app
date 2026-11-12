use super::*;

impl ViewerState {
    /// Deterministic leaderboard order: points descending, then coins,
    /// then chats, then identity. The final `unique_id` tie-break keeps
    /// the board stable when every counter matches.
    pub(super) fn compare_rank(left: &Self, right: &Self) -> std::cmp::Ordering {
        right
            .points
            .total_cmp(&left.points)
            .then_with(|| right.total_coins.cmp(&left.total_coins))
            .then_with(|| right.total_chats.cmp(&left.total_chats))
            .then_with(|| left.unique_id.cmp(&right.unique_id))
    }

    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "uniqueId": self.unique_id,
            "userId": self.user_id,
            "nickname": self.nickname,
            "avatarUrl": self.avatar_url,
            "points": self.points,
            "level": self.level,
            "isSubscriber": self.is_subscriber,
            "totalChats": self.total_chats,
            "totalCoins": self.total_coins,
            "totalLikes": self.total_likes,
            "totalShares": self.total_shares,
            "firstSeen": self.first_seen,
            "lastSeen": self.last_seen,
        })
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        Some(Self {
            unique_id: object.get("uniqueId")?.as_str()?.to_owned(),
            user_id: object
                .get("userId")
                .and_then(Value::as_str)
                .map(str::to_owned),
            nickname: object
                .get("nickname")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_default(),
            avatar_url: object
                .get("avatarUrl")
                .and_then(Value::as_str)
                .map(str::to_owned),
            points: object
                .get("points")
                .and_then(Value::as_f64)
                .unwrap_or_default(),
            level: object
                .get("level")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .min(u64::from(u32::MAX)) as u32,
            is_subscriber: object
                .get("isSubscriber")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            total_chats: object
                .get("totalChats")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            total_coins: object
                .get("totalCoins")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            total_likes: object
                .get("totalLikes")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            total_shares: object
                .get("totalShares")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            first_seen: object
                .get("firstSeen")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            last_seen: object
                .get("lastSeen")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
        })
    }
}
