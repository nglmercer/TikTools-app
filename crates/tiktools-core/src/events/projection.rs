//! Allowlisted creator-feed projection without raw JSON or global cursor data.
use super::CreatorFeedEvent;
use serde_json::Value;

pub(super) fn project_creator_event(cursor: u64, value: &Value) -> Option<CreatorFeedEvent> {
    let kind = value.get("kind")?.as_str()?;
    if !matches!(kind, "chat" | "gift" | "like" | "member" | "social") {
        return None;
    }
    let clean = |value: &str, max| {
        value
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
            .take(max)
            .collect::<String>()
    };
    let original_author = value.get("author")?.as_str()?;
    let author = clean(original_author, 64);
    let author_lookup_id = (original_author == author
        && !author.is_empty()
        && author != "viewer"
        && author.len() <= 128
        && author.trim() == author
        && !author.starts_with('@')
        && !author.contains('*')
        && !author.chars().any(char::is_control))
    .then(|| author.clone());
    let text = clean(value.get("text")?.as_str()?, 128);
    let nickname = match value.get("nickname") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(clean(value, 64)),
        _ => return None,
    };
    Some(CreatorFeedEvent {
        cursor: cursor.to_string(),
        kind: kind.to_owned(),
        author,
        author_lookup_id,
        nickname,
        text,
    })
}
