//! Speech policy: settings sanitize, comment/user eligibility, voice
//! choice, and replay dedup.
//!
//! This is a faithful Rust port of the TypeScript policy the isolated UI
//! uses for previews (`../shared/tts/`): same defaults, same clamps, same
//! decision order, same reason strings. The two copies are pinned by
//! mirrored unit tests; they cannot share code across the language
//! boundary, so any policy change must land in both.

use std::collections::HashMap;

pub const MAX_COMMENT_LENGTH: usize = 4096;
const MAX_VOICE_LENGTH: usize = 128;
const MAX_HANDLE_LENGTH: usize = 64;
const MAX_COMMAND_LENGTH: usize = 32;
const MAX_LANGUAGE_LENGTH: usize = 16;
const MAX_ALLOWED_USERS: usize = 500;
const MAX_SPECIAL_USERS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentMode {
    Any,
    Dot,
    Slash,
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecialUser {
    pub handle: String,
    pub allowed: bool,
    pub voice: String,
    pub speed: f64,
    pub pitch: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TtsSettings {
    pub enabled: bool,
    pub language: String,
    pub default_voice: String,
    pub random_voice: bool,
    pub default_speed: f64,
    pub default_pitch: f64,
    pub volume: f64,
    pub allow_all_users: bool,
    pub allow_followers: bool,
    pub allow_subscribers: bool,
    pub allow_moderators: bool,
    pub allow_team_members: bool,
    pub min_team_level: i64,
    pub allow_top_gifters: bool,
    pub top_gifter_count: i64,
    pub allow_listed_users: bool,
    pub allowed_users: Vec<String>,
    pub comment_mode: CommentMode,
    pub command: String,
    pub strip_command: bool,
    pub charge_points: bool,
    pub points_cost: i64,
    pub special_users: Vec<SpecialUser>,
}

impl Default for TtsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            language: "en".to_owned(),
            default_voice: String::new(),
            random_voice: false,
            default_speed: 1.0,
            default_pitch: 1.0,
            volume: 1.0,
            allow_all_users: true,
            allow_followers: false,
            allow_subscribers: false,
            allow_moderators: false,
            allow_team_members: false,
            min_team_level: 0,
            allow_top_gifters: false,
            top_gifter_count: 10,
            allow_listed_users: false,
            allowed_users: Vec::new(),
            comment_mode: CommentMode::Any,
            command: "!tts".to_owned(),
            strip_command: true,
            charge_points: false,
            points_cost: 0,
            special_users: Vec::new(),
        }
    }
}

/// Case-folds a handle for comparison: trims, strips `@`, lowercases.
pub fn normalize_handle(handle: &str) -> String {
    handle.trim().trim_start_matches('@').to_lowercase()
}

fn clamp_number(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if !value.is_finite() {
        return fallback;
    }
    value.clamp(min, max)
}

fn to_bool(value: Option<&serde_json::Value>, fallback: bool) -> bool {
    value
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(fallback)
}

fn to_trimmed(value: Option<&serde_json::Value>, max: usize, fallback: &str) -> String {
    let Some(text) = value.and_then(serde_json::Value::as_str) else {
        return fallback.to_owned();
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return fallback.to_owned();
    }
    trimmed.chars().take(max).collect()
}

fn to_voice(value: Option<&serde_json::Value>) -> String {
    let Some(text) = value.and_then(serde_json::Value::as_str) else {
        return String::new();
    };
    text.trim().chars().take(MAX_VOICE_LENGTH).collect()
}

fn to_number(value: Option<&serde_json::Value>, fallback: f64) -> f64 {
    value
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(fallback)
}

fn to_handle_list(value: Option<&serde_json::Value>) -> Vec<String> {
    let Some(list) = value.and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    let mut clean = Vec::new();
    for entry in list {
        let Some(text) = entry.as_str() else { continue };
        let handle = normalize_handle(text);
        if handle.is_empty() || handle.len() > MAX_HANDLE_LENGTH || !seen.insert(handle.clone()) {
            continue;
        }
        clean.push(handle);
        if clean.len() >= MAX_ALLOWED_USERS {
            break;
        }
    }
    clean
}

fn to_special_users(value: Option<&serde_json::Value>) -> Vec<SpecialUser> {
    let Some(list) = value.and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    let mut clean = Vec::new();
    for entry in list {
        let Some(record) = entry.as_object() else {
            continue;
        };
        let handle = record
            .get("handle")
            .and_then(serde_json::Value::as_str)
            .map(normalize_handle)
            .unwrap_or_default();
        if handle.is_empty() || handle.len() > MAX_HANDLE_LENGTH || !seen.insert(handle.clone()) {
            continue;
        }
        clean.push(SpecialUser {
            handle,
            allowed: to_bool(record.get("allowed"), true),
            voice: to_voice(record.get("voice")),
            speed: clamp_number(to_number(record.get("speed"), 1.0), 0.25, 3.0, 1.0),
            pitch: clamp_number(to_number(record.get("pitch"), 1.0), 0.25, 2.0, 1.0),
        });
        if clean.len() >= MAX_SPECIAL_USERS {
            break;
        }
    }
    clean
}

/// Coerces unknown persisted state into valid settings with clamped numbers.
pub fn sanitize_settings(input: Option<&serde_json::Value>) -> TtsSettings {
    let defaults = TtsSettings::default();
    let Some(record) = input.and_then(serde_json::Value::as_object) else {
        return defaults;
    };
    let comment_mode = match record
        .get("commentMode")
        .and_then(serde_json::Value::as_str)
    {
        Some("dot") => CommentMode::Dot,
        Some("slash") => CommentMode::Slash,
        Some("command") => CommentMode::Command,
        _ => CommentMode::Any,
    };
    TtsSettings {
        enabled: to_bool(record.get("enabled"), defaults.enabled),
        language: to_trimmed(record.get("language"), MAX_LANGUAGE_LENGTH, "en"),
        default_voice: to_voice(record.get("defaultVoice")),
        random_voice: to_bool(record.get("randomVoice"), defaults.random_voice),
        default_speed: clamp_number(to_number(record.get("defaultSpeed"), 1.0), 0.25, 3.0, 1.0),
        default_pitch: clamp_number(to_number(record.get("defaultPitch"), 1.0), 0.25, 2.0, 1.0),
        volume: clamp_number(to_number(record.get("volume"), 1.0), 0.0, 1.0, 1.0),
        allow_all_users: to_bool(record.get("allowAllUsers"), defaults.allow_all_users),
        allow_followers: to_bool(record.get("allowFollowers"), false),
        allow_subscribers: to_bool(record.get("allowSubscribers"), false),
        allow_moderators: to_bool(record.get("allowModerators"), false),
        allow_team_members: to_bool(record.get("allowTeamMembers"), false),
        min_team_level: clamp_number(to_number(record.get("minTeamLevel"), 0.0), 0.0, 100.0, 0.0)
            .round() as i64,
        allow_top_gifters: to_bool(record.get("allowTopGifters"), false),
        top_gifter_count: clamp_number(
            to_number(record.get("topGifterCount"), 10.0),
            1.0,
            100.0,
            10.0,
        )
        .round() as i64,
        allow_listed_users: to_bool(record.get("allowListedUsers"), false),
        allowed_users: to_handle_list(record.get("allowedUsers")),
        comment_mode,
        command: to_trimmed(record.get("command"), MAX_COMMAND_LENGTH, "!tts"),
        strip_command: to_bool(record.get("stripCommand"), true),
        charge_points: to_bool(record.get("chargePoints"), false),
        points_cost: clamp_number(
            to_number(record.get("pointsCost"), 0.0),
            0.0,
            100_000.0,
            0.0,
        )
        .round() as i64,
        special_users: to_special_users(record.get("specialUsers")),
    }
}

/// Authoritative role flags for one chat author. Absent means unknown.
#[derive(Debug, Clone, Default)]
pub struct AuthorRoles {
    pub is_subscriber: Option<bool>,
    pub is_follower: Option<bool>,
    pub is_moderator: Option<bool>,
    pub is_team_member: Option<bool>,
    pub team_level: Option<f64>,
    pub is_top_gifter: Option<bool>,
    pub top_gifter_rank: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct Author {
    pub handle: String,
    pub points: Option<f64>,
    pub roles: AuthorRoles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EligibilityVia {
    SpecialUser,
    AllUsers,
    AllowList,
    Subscriber,
    Follower,
    Moderator,
    TeamMember,
    TopGifter,
    None,
}

/// Applies the comment-mode filter and strips the command prefix when asked.
pub fn evaluate_comment_filter(comment: &str, settings: &TtsSettings) -> (bool, String, String) {
    let text: String = comment.chars().take(MAX_COMMENT_LENGTH).collect();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return (
            false,
            String::new(),
            "Empty comments are never spoken.".to_owned(),
        );
    }
    match settings.comment_mode {
        CommentMode::Any => (
            true,
            trimmed.to_owned(),
            "Any comment may be spoken.".to_owned(),
        ),
        CommentMode::Dot | CommentMode::Slash => {
            let prefix = if settings.comment_mode == CommentMode::Dot {
                '.'
            } else {
                '/'
            };
            if !trimmed.starts_with(prefix) || trimmed.len() < 2 {
                return (
                    false,
                    String::new(),
                    format!("Only comments starting with `{prefix}` are spoken."),
                );
            }
            let spoken = if settings.strip_command {
                trimmed[1..].trim().to_owned()
            } else {
                trimmed.to_owned()
            };
            if spoken.is_empty() {
                return (
                    false,
                    String::new(),
                    format!("Nothing remains after stripping `{prefix}`."),
                );
            }
            (
                true,
                spoken,
                if prefix == '.' {
                    "Dot-prefixed comment.".to_owned()
                } else {
                    "Slash-prefixed comment.".to_owned()
                },
            )
        }
        CommentMode::Command => {
            let command = settings.command.trim();
            if command.is_empty() {
                return (
                    false,
                    String::new(),
                    "No TTS command is configured.".to_owned(),
                );
            }
            let lowered = trimmed.to_lowercase();
            let prefix = command.to_lowercase();
            if lowered != prefix && !lowered.starts_with(&format!("{prefix} ")) {
                return (
                    false,
                    String::new(),
                    format!("Only comments starting with `{command}` are spoken."),
                );
            }
            if !settings.strip_command {
                return (
                    true,
                    trimmed.to_owned(),
                    "Command-prefixed comment.".to_owned(),
                );
            }
            let spoken = trimmed[command.len()..].trim().to_owned();
            if spoken.is_empty() {
                return (
                    false,
                    String::new(),
                    "Nothing remains after stripping the command.".to_owned(),
                );
            }
            (true, spoken, "Command-prefixed comment.".to_owned())
        }
    }
}

pub fn find_special_user<'a>(handle: &str, settings: &'a TtsSettings) -> Option<&'a SpecialUser> {
    let clean = normalize_handle(handle);
    if clean.is_empty() {
        return None;
    }
    settings
        .special_users
        .iter()
        .find(|entry| entry.handle == clean)
}

/// Special-user allow/block plus the global allow rules, in priority order.
pub fn evaluate_user_eligibility(
    author: &Author,
    settings: &TtsSettings,
) -> (bool, String, EligibilityVia, Option<SpecialUser>) {
    if let Some(special) = find_special_user(&author.handle, settings) {
        if !special.allowed {
            return (
                false,
                "This user is blocked in Special Users.".to_owned(),
                EligibilityVia::SpecialUser,
                Some(special.clone()),
            );
        }
        return (
            true,
            "Allowed by Special Users.".to_owned(),
            EligibilityVia::SpecialUser,
            Some(special.clone()),
        );
    }
    if settings.allow_all_users {
        return (
            true,
            "All users may use TTS.".to_owned(),
            EligibilityVia::AllUsers,
            None,
        );
    }
    let clean = normalize_handle(&author.handle);
    if settings.allow_listed_users && !clean.is_empty() && settings.allowed_users.contains(&clean) {
        return (
            true,
            "Listed in Allowed Users.".to_owned(),
            EligibilityVia::AllowList,
            None,
        );
    }
    let roles = &author.roles;
    // The remaining roles grant access only with authoritative data. A
    // missing flag means "unknown", which never satisfies the rule.
    if settings.allow_subscribers && roles.is_subscriber == Some(true) {
        return (
            true,
            "Subscribers may use TTS.".to_owned(),
            EligibilityVia::Subscriber,
            None,
        );
    }
    if settings.allow_followers && roles.is_follower == Some(true) {
        return (
            true,
            "Followers may use TTS.".to_owned(),
            EligibilityVia::Follower,
            None,
        );
    }
    if settings.allow_moderators && roles.is_moderator == Some(true) {
        return (
            true,
            "Moderators may use TTS.".to_owned(),
            EligibilityVia::Moderator,
            None,
        );
    }
    if settings.allow_team_members && roles.is_team_member == Some(true) {
        let level = roles.team_level.unwrap_or(0.0);
        if level >= settings.min_team_level as f64 {
            return (
                true,
                "Team members may use TTS.".to_owned(),
                EligibilityVia::TeamMember,
                None,
            );
        }
        return (
            false,
            format!(
                "Team level {} or higher is required.",
                settings.min_team_level
            ),
            EligibilityVia::None,
            None,
        );
    }
    if settings.allow_top_gifters && roles.is_top_gifter == Some(true) {
        let rank = roles.top_gifter_rank.unwrap_or(f64::MAX);
        if rank <= settings.top_gifter_count as f64 {
            return (
                true,
                "Top gifters may use TTS.".to_owned(),
                EligibilityVia::TopGifter,
                None,
            );
        }
        return (
            false,
            format!(
                "Only the top {} gifters may use TTS.",
                settings.top_gifter_count
            ),
            EligibilityVia::None,
            None,
        );
    }
    (
        false,
        "This user is not allowed to use TTS.".to_owned(),
        EligibilityVia::None,
        None,
    )
}

/// `true` when the viewer can cover the per-message points cost.
pub fn can_afford(points: Option<f64>, settings: &TtsSettings) -> bool {
    if !settings.charge_points || settings.points_cost <= 0 {
        return true;
    }
    matches!(points, Some(total) if total.is_finite() && total >= settings.points_cost as f64)
}

/// Mirrors the TypeScript `TtsDecision`: the observer only reads `speak`,
/// `spoken_text`, `voice`, and `language`, but the denial reasons and
/// attribution travel with the value (and are asserted by tests) so the
/// two policies can't drift silently.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Decision {
    pub speak: bool,
    pub reason: String,
    pub spoken_text: String,
    pub voice: String,
    pub language: String,
    pub points_cost: i64,
    pub via: EligibilityVia,
    pub special_user: Option<SpecialUser>,
}

/// First usable voice from a loaded list, or empty when none is known.
pub fn first_available_voice(available: &[String]) -> String {
    available
        .iter()
        .map(|voice| voice.trim())
        .find(|voice| !voice.is_empty())
        .unwrap_or_default()
        .to_owned()
}

/// Voice priority: special-user voice → random voice → default voice →
/// first available voice. Mirrors the TypeScript `chooseVoice`, including
/// the trailing fallback (SonicBoom 400s on a present-but-empty `voice=`
/// param, so an empty resolution must never be sent while voices are
/// known).
pub fn choose_voice(
    settings: &TtsSettings,
    special_voice: &str,
    available: &[String],
    mut random_shift: impl FnMut(usize) -> usize,
) -> String {
    let special = special_voice.trim();
    if !special.is_empty() {
        return special.to_owned();
    }
    let pool: Vec<&str> = available
        .iter()
        .map(|voice| voice.trim())
        .filter(|voice| !voice.is_empty())
        .collect();
    if settings.random_voice && !pool.is_empty() {
        let index = random_shift(pool.len()).min(pool.len() - 1);
        return pool[index].to_owned();
    }
    let fallback = first_available_voice(available);
    if settings.default_voice.trim().is_empty() {
        fallback
    } else {
        settings.default_voice.trim().to_owned()
    }
}

/// Single full-pipeline decision. Callers must check `speak` before
/// acting: computing eligibility never speaks by itself.
pub fn decide(
    comment: &str,
    author: &Author,
    settings: &TtsSettings,
    available_voices: &[String],
    random_shift: impl FnMut(usize) -> usize,
) -> Decision {
    let idle = |reason: String| Decision {
        speak: false,
        reason,
        spoken_text: String::new(),
        voice: String::new(),
        language: settings.language.clone(),
        points_cost: 0,
        via: EligibilityVia::None,
        special_user: None,
    };
    if !settings.enabled {
        return idle("TTS is disabled.".to_owned());
    }
    let (comment_allowed, spoken_text, comment_reason) = evaluate_comment_filter(comment, settings);
    if !comment_allowed {
        return idle(comment_reason);
    }
    let (user_allowed, user_reason, via, special_user) =
        evaluate_user_eligibility(author, settings);
    if !user_allowed {
        return Decision {
            speak: false,
            reason: user_reason,
            spoken_text: String::new(),
            voice: String::new(),
            language: settings.language.clone(),
            points_cost: 0,
            via,
            special_user,
        };
    }
    let points_cost = if settings.charge_points {
        settings.points_cost.max(0)
    } else {
        0
    };
    if points_cost > 0 && !can_afford(author.points, settings) {
        return Decision {
            speak: false,
            reason: format!("Needs {points_cost} points to use TTS."),
            spoken_text: String::new(),
            voice: String::new(),
            language: settings.language.clone(),
            points_cost,
            via: via.clone(),
            special_user,
        };
    }
    let voice = choose_voice(
        settings,
        special_user
            .as_ref()
            .map(|user| user.voice.as_str())
            .unwrap_or_default(),
        available_voices,
        random_shift,
    );
    Decision {
        speak: true,
        reason: user_reason,
        spoken_text,
        voice,
        language: settings.language.clone(),
        points_cost,
        via,
        special_user,
    }
}

/// Fingerprint for one speakable chat line, used for de-duplication.
pub fn fingerprint(handle: &str, spoken_text: &str) -> String {
    format!(
        "{}\n{}",
        normalize_handle(handle),
        spoken_text.trim().to_lowercase()
    )
}

/// Drops repeated deliveries of the same handle+text within a short
/// window. Guards against transport replays and double event emission
/// speaking twice.
pub struct Deduper {
    window_ms: u64,
    max_entries: usize,
    seen: HashMap<String, u64>,
    /// Insertion sequence per fingerprint, so eviction breaks timestamp
    /// ties by insertion order (mirrors the TypeScript stable order).
    sequence: HashMap<String, u64>,
    next_sequence: u64,
}

impl Deduper {
    pub fn new(window_ms: u64, max_entries: usize) -> Self {
        Self {
            window_ms,
            max_entries: max_entries.max(1),
            seen: HashMap::new(),
            sequence: HashMap::new(),
            next_sequence: 0,
        }
    }

    pub fn claim(&mut self, fingerprint: &str, now_ms: u64) -> bool {
        if let Some(last) = self.seen.get(fingerprint) {
            if now_ms.saturating_sub(*last) < self.window_ms {
                return false;
            }
        }
        self.seen.insert(fingerprint.to_owned(), now_ms);
        self.next_sequence += 1;
        self.sequence
            .insert(fingerprint.to_owned(), self.next_sequence);
        if self.seen.len() > self.max_entries {
            let oldest = self
                .seen
                .iter()
                .min_by_key(|(key, at)| (**at, *self.sequence.get(*key).unwrap_or(&0)))
                .map(|(key, _)| key.clone());
            if let Some(key) = oldest {
                self.seen.remove(&key);
                self.sequence.remove(&key);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn enabled() -> TtsSettings {
        TtsSettings {
            enabled: true,
            ..TtsSettings::default()
        }
    }

    #[test]
    fn sanitize_clamps_and_normalizes() {
        let settings = sanitize_settings(Some(&json!({
            "enabled": true,
            "language": "  es  ",
            "defaultSpeed": 99.0,
            "pointsCost": -5.0,
            "commentMode": "bogus",
            "allowedUsers": ["@Ada", "ada", 42, ""],
            "specialUsers": [{"handle": "@Bo", "allowed": false, "voice": " X "}],
        })));
        assert!(settings.enabled);
        assert_eq!(settings.language, "es");
        assert_eq!(settings.default_speed, 3.0);
        assert_eq!(settings.points_cost, 0);
        assert_eq!(settings.comment_mode, CommentMode::Any);
        assert_eq!(settings.allowed_users, vec!["ada".to_owned()]);
        assert_eq!(settings.special_users.len(), 1);
        assert_eq!(settings.special_users[0].handle, "bo");
        assert!(!settings.special_users[0].allowed);
        assert_eq!(settings.special_users[0].voice, "X");
    }

    #[test]
    fn comment_modes_filter_and_strip() {
        let settings = enabled();
        assert!(evaluate_comment_filter("hello", &settings).0);
        let dot = TtsSettings {
            comment_mode: CommentMode::Dot,
            ..enabled()
        };
        let (allowed, spoken, _) = evaluate_comment_filter(".hello", &dot);
        assert!(allowed);
        assert_eq!(spoken, "hello");
        assert!(!evaluate_comment_filter("hello", &dot).0);
        assert!(!evaluate_comment_filter(".", &dot).0);
        let command = TtsSettings {
            comment_mode: CommentMode::Command,
            command: "!speak".to_owned(),
            ..enabled()
        };
        let (allowed, spoken, _) = evaluate_comment_filter("!speak hello", &command);
        assert!(allowed);
        assert_eq!(spoken, "hello");
        assert!(!evaluate_comment_filter("!other hello", &command).0);
    }

    #[test]
    fn special_users_override_global_rules() {
        let settings = TtsSettings {
            allow_all_users: false,
            special_users: vec![SpecialUser {
                handle: "ada".to_owned(),
                allowed: true,
                voice: String::new(),
                speed: 1.0,
                pitch: 1.0,
            }],
            ..enabled()
        };
        let author = Author {
            handle: "@ADA".to_owned(),
            ..Author::default()
        };
        let (allowed, _, via, _) = evaluate_user_eligibility(&author, &settings);
        assert!(allowed);
        assert_eq!(via, EligibilityVia::SpecialUser);
        let stranger = Author {
            handle: "mallory".to_owned(),
            ..Author::default()
        };
        assert!(!evaluate_user_eligibility(&stranger, &settings).0);
    }

    #[test]
    fn roles_need_authoritative_data() {
        let settings = TtsSettings {
            allow_all_users: false,
            allow_subscribers: true,
            ..enabled()
        };
        // Unknown roles never satisfy the rule.
        let unknown = Author {
            handle: "ada".to_owned(),
            ..Author::default()
        };
        assert!(!evaluate_user_eligibility(&unknown, &settings).0);
        let subscriber = Author {
            handle: "ada".to_owned(),
            roles: AuthorRoles {
                is_subscriber: Some(true),
                ..AuthorRoles::default()
            },
            ..Author::default()
        };
        let (allowed, _, via, _) = evaluate_user_eligibility(&subscriber, &settings);
        assert!(allowed);
        assert_eq!(via, EligibilityVia::Subscriber);
    }

    #[test]
    fn affordability_gates_charged_speech() {
        let settings = TtsSettings {
            charge_points: true,
            points_cost: 50,
            ..enabled()
        };
        let broke = Author {
            handle: "ada".to_owned(),
            points: Some(10.0),
            ..Author::default()
        };
        let decision = decide("hi", &broke, &settings, &[], |_| 0);
        assert!(!decision.speak);
        assert_eq!(decision.points_cost, 50);
        let rich = Author {
            points: Some(60.0),
            ..broke
        };
        assert!(decide("hi", &rich, &settings, &[], |_| 0).speak);
    }

    #[test]
    fn voice_priority_matches_the_typescript_policy() {
        let settings = enabled();
        let available = vec!["M1".to_owned(), "F1".to_owned()];
        assert_eq!(
            choose_voice(&settings, "Custom", &available, |_| 0),
            "Custom"
        );
        assert_eq!(choose_voice(&settings, "", &available, |_| 0), "M1");
        let random = TtsSettings {
            random_voice: true,
            ..enabled()
        };
        assert_eq!(choose_voice(&random, "", &available, |_| 1), "F1");
        // Empty default with known voices falls back instead of sending ''.
        assert_eq!(choose_voice(&settings, "", &[], |_| 0), "");
    }

    #[test]
    fn deduper_drops_replays_and_evicts_oldest() {
        let mut deduper = Deduper::new(1500, 2);
        assert!(deduper.claim("a", 1000));
        assert!(!deduper.claim("a", 2000));
        assert!(deduper.claim("a", 2600));
        assert!(deduper.claim("b", 2600));
        assert!(deduper.claim("c", 2600));
        // Capacity is 2: the oldest fingerprint ("a") was evicted, so it
        // claims again even inside the window.
        assert!(deduper.claim("a", 2700));
    }
}
