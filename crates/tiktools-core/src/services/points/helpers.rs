use super::*;

pub(super) fn clean_unique_id(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('@');
    valid_viewer_id(value).then_some(value.to_owned())
}

pub(crate) fn valid_viewer_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && !value.starts_with('@')
        && !value.contains('*')
        && !value.chars().any(char::is_control)
}

pub(super) fn nonnegative_or_one(value: Option<f64>) -> f64 {
    value
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(1.0)
}

pub(super) fn round_points(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub(super) fn update_existing_viewer(
    state: &mut ViewerState,
    options: &AwardOptions,
    points: f64,
    level: u32,
    is_subscriber: bool,
    action: PointAction,
    now: i64,
) {
    if let Some(value) = options.user_id.as_ref() {
        state.user_id = Some(value.clone());
    }
    if let Some(value) = options.nickname.as_ref() {
        state.nickname = value.clone();
    }
    if let Some(value) = options.avatar_url.as_ref() {
        state.avatar_url = Some(value.clone());
    }
    state.points = points;
    state.level = level;
    state.is_subscriber = is_subscriber;
    // Counters and metadata advance even when the award itself is zero:
    // a chat that earns nothing is still a chat.
    if action == PointAction::Chat {
        state.total_chats = state.total_chats.saturating_add(1);
    }
    if action == PointAction::Gift {
        state.total_coins = state
            .total_coins
            .saturating_add(nonnegative_or_one(options.diamond_count).round() as i64);
    }
    if action == PointAction::Like {
        state.total_likes = state
            .total_likes
            .saturating_add(nonnegative_or_one(options.count).round() as i64);
    }
    if action == PointAction::Share {
        state.total_shares = state.total_shares.saturating_add(1);
    }
    state.last_seen = now;
}

pub(super) fn new_viewer_state(unique_id: &str, options: &AwardOptions, now: i64) -> ViewerState {
    ViewerState {
        unique_id: unique_id.to_owned(),
        user_id: options.user_id.clone(),
        nickname: options
            .nickname
            .clone()
            .unwrap_or_else(|| unique_id.to_owned()),
        avatar_url: options.avatar_url.clone(),
        points: 0.0,
        level: 1,
        is_subscriber: false,
        total_chats: 0,
        total_coins: 0,
        total_likes: 0,
        total_shares: 0,
        first_seen: now,
        last_seen: now,
    }
}

pub(super) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}
