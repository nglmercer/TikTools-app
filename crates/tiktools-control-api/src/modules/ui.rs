//! Transport-neutral read-only UI recovery. Desktop presentation stays host-owned.
use crate::{error::ApiError, router::ControlRouter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use tiktools_core::events::FeedBackfill;
use tiktools_core::AppCore;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FeedBackfillParams {
    pub version: u32,
    pub after: u64,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatorFeedParams {
    pub creator_id: String,
    pub stream_id: Option<String>,
    pub after: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrowserFeedParams {
    pub after: String,
}

fn display_text(value: &str, max: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .take(max)
        .collect()
}

fn browser_feed(
    page: FeedBackfill,
    after: u64,
    budget: usize,
) -> Result<serde_json::Value, ApiError> {
    let mut events = Vec::with_capacity(page.events.len());
    let mut previous = after;
    let mut used = 512usize;
    for entry in page.events {
        let event = entry
            .event
            .as_object()
            .ok_or_else(|| ApiError::new("protocol", "invalid feed event"))?;
        let kind = event
            .get("kind")
            .and_then(|v| v.as_str())
            .filter(|kind| matches!(*kind, "chat" | "gift" | "like" | "member" | "social"))
            .ok_or_else(|| ApiError::new("protocol", "invalid feed event"))?;
        let author = event
            .get("author")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::new("protocol", "invalid feed event"))?;
        let text = event
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::new("protocol", "invalid feed event"))?;
        let nickname = match event.get("nickname") {
            None | Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(v)) => Some(display_text(v, 128)),
            _ => return Err(ApiError::new("protocol", "invalid feed event")),
        };
        let displayed = display_text(author, 128);
        let lookup = (author == displayed
            && !author.is_empty()
            && author != "viewer"
            && author.len() <= 128
            && author.trim() == author
            && !author.starts_with('@')
            && !author.contains('*')
            && !author.chars().any(char::is_control))
        .then_some(author);
        let mut projected = serde_json::json!({"cursor":entry.cursor.to_string(),"kind":kind,"author":displayed,"authorLookupId":lookup,"nickname":nickname,"text":display_text(text,2048)});
        let mut size = serde_json::to_vec(&projected)
            .map_err(|_| ApiError::new("protocol", "invalid feed event"))?
            .len()
            + 1;
        if used.saturating_add(size) > budget {
            if !events.is_empty() {
                break;
            }
            // A single large event must not permanently prevent the cursor
            // from advancing. Reduce display fields only, preserving UTF-8;
            // a shortened author is never usable as an action target.
            for field in ["text", "nickname", "author"] {
                while used.saturating_add(size) > budget {
                    let Some(value) = projected[field].as_str() else {
                        break;
                    };
                    let count = value.chars().count();
                    if count == 0 {
                        break;
                    }
                    let shortened = if count > 1 {
                        format!(
                            "{}…",
                            value.chars().take((count - 1) / 2).collect::<String>()
                        )
                    } else {
                        String::new()
                    };
                    projected[field] = shortened.into();
                    if field == "author" {
                        projected["authorLookupId"] = serde_json::Value::Null;
                    }
                    size = serde_json::to_vec(&projected)
                        .map_err(|_| ApiError::new("protocol", "invalid feed event"))?
                        .len()
                        + 1;
                }
            }
            if used.saturating_add(size) > budget {
                return Err(ApiError::new(
                    "too_large",
                    "feed event exceeds connection budget",
                ));
            }
        }
        used += size;
        previous = entry.cursor;
        events.push(projected);
    }
    let next = if after > page.latest_cursor {
        0
    } else {
        previous
    };
    Ok(
        serde_json::json!({"oldestCursor":page.oldest_cursor.to_string(),"latestCursor":page.latest_cursor.to_string(),"nextCursor":next.to_string(),"gap":page.gap || (after == 0 && page.oldest_cursor > 1),"truncated":next != 0 && next < page.latest_cursor,"events":events}),
    )
}

pub fn register(router: &mut ControlRouter) {
    router.register_contextual::<BrowserFeedParams, serde_json::Value, _, _>(
        "ui.feed.browser",
        "Bounded browser-safe LIVE history",
        |core: Arc<AppCore>, context, params: BrowserFeedParams| async move {
            let after = params
                .after
                .parse::<u64>()
                .ok()
                .filter(|n| n.to_string() == params.after)
                .ok_or_else(|| ApiError::invalid_params("invalid feed cursor"))?;
            let budget = match context {
                crate::RequestContext::Remote(session) => session
                    .connection_limits()
                    .map_or(512 * 1024, |limits| limits.frame_bytes / 2),
                _ => 512 * 1024,
            };
            browser_feed(core.events.feed_backfill(after, 64), after, budget)
        },
    );
    router.register_contextual::<CreatorFeedParams, tiktools_core::events::CreatorFeedPage, _, _>(
        "ui.feed.creator",
        "Creator-scoped bounded LIVE history",
        |core, context, params| async move {
            let after = params
                .after
                .parse::<u64>()
                .ok()
                .filter(|v| v.to_string() == params.after)
                .ok_or_else(|| ApiError::invalid_params("invalid feed cursor"))?;
            if !tiktools_core::events::valid_creator_id(&params.creator_id)
                || params.stream_id.as_ref().is_some_and(|id| {
                    id.len() != 32
                        || !id
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
                || (params.stream_id.is_none() && after != 0)
            {
                return Err(ApiError::invalid_params("invalid creator feed request"));
            }
            let budget = match context {
                crate::RequestContext::Remote(session) => {
                    if !session.grant()?.allows_resource(
                        tiktools_core::collaboration::ResourceKind::Live,
                        &params.creator_id,
                    ) {
                        return Err(crate::context::forbidden());
                    }
                    session
                        .connection_limits()
                        .map_or(512 * 1024, |limits| limits.frame_bytes / 2)
                }
                _ => 512 * 1024,
            };
            core.events
                .creator_feed(
                    &params.creator_id,
                    params.stream_id.as_deref(),
                    after,
                    params.limit.unwrap_or(64),
                    budget,
                )
                .map_err(|_| ApiError::new("unavailable", "creator feed is unavailable"))
        },
    );
    router.register_typed::<FeedBackfillParams, FeedBackfill, _, _>(
        "ui.feed.backfill",
        "Bounded read-only LIVE render history",
        false,
        |core: Arc<AppCore>, params: FeedBackfillParams| async move {
            if params.version != 1 {
                return Err(ApiError::invalid_params(
                    "ui.feed.backfill requires version 1",
                ));
            }
            Ok(core
                .events
                .feed_backfill(params.after, params.limit.unwrap_or(512)))
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_feed_preserves_large_cursors_and_sanitizes_display() {
        let page = FeedBackfill {
            oldest_cursor: 9_007_199_254_740_993,
            latest_cursor: 9_007_199_254_740_993,
            depth: 1,
            gap: false,
            truncated: false,
            events: vec![tiktools_core::events::FeedEntry {
                cursor: 9_007_199_254_740_993,
                event: serde_json::json!({"kind":"chat", "author":"ali\u{0}ce", "text":"hello\u{0}world", "secret":"hidden"}),
            }],
        };
        let result = browser_feed(page, 0, 4096).unwrap();
        assert_eq!(result["latestCursor"], "9007199254740993");
        assert_eq!(result["events"][0]["author"], "alice");
        assert!(result["events"][0]["authorLookupId"].is_null());
        assert_eq!(result["events"][0]["text"], "helloworld");
        assert!(result["events"][0].get("secret").is_none());
    }
    #[test]
    fn browser_feed_pages_to_connection_budget_without_skipping() {
        let entries = (1..=4).map(|cursor| tiktools_core::events::FeedEntry { cursor,
            event: serde_json::json!({"kind":"chat", "author":"viewer1", "text":"x".repeat(400)}) }).collect();
        let page = FeedBackfill {
            oldest_cursor: 1,
            latest_cursor: 4,
            depth: 4,
            gap: false,
            truncated: false,
            events: entries,
        };
        let result = browser_feed(page, 0, 1600).unwrap();
        assert_eq!(result["events"].as_array().unwrap().len(), 2);
        assert_eq!(result["nextCursor"], "2");
        assert_eq!(result["truncated"], true);
    }
    #[test]
    fn oversized_event_is_shortened_and_does_not_block_following_cursors() {
        let author = "😀".repeat(32);
        let page = FeedBackfill {
            oldest_cursor: 1,
            latest_cursor: 2,
            depth: 2,
            gap: false,
            truncated: false,
            events: vec![
                tiktools_core::events::FeedEntry {
                    cursor: 1,
                    event: serde_json::json!({"kind":"chat", "author":author, "nickname":"🎵".repeat(128), "text":"🔥".repeat(2048)}),
                },
                tiktools_core::events::FeedEntry {
                    cursor: 2,
                    event: serde_json::json!({"kind":"chat", "author":"alice", "text":"next"}),
                },
            ],
        };
        let result = browser_feed(page, 0, 2048).unwrap();
        assert!(serde_json::to_vec(&result).unwrap().len() < 2048);
        assert_eq!(result["events"][0]["cursor"], "1");
        assert!(result["events"][0]["text"].as_str().unwrap().ends_with('…'));
        assert!(
            result["events"][0]["text"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                < 2048
        );
        assert_eq!(result["events"][0]["authorLookupId"], author);
        assert_eq!(
            result["nextCursor"],
            result["events"].as_array().unwrap().last().unwrap()["cursor"]
        );
    }

    #[test]
    fn shortened_author_is_not_an_action_target() {
        let author = "a".repeat(128);
        let page = FeedBackfill {
            oldest_cursor: 1,
            latest_cursor: 1,
            depth: 1,
            gap: false,
            truncated: false,
            events: vec![tiktools_core::events::FeedEntry {
                cursor: 1,
                event: serde_json::json!({"kind":"chat", "author":author, "text":""}),
            }],
        };
        let result = browser_feed(page, 0, 640).unwrap();
        assert!(result["events"][0]["author"].as_str().unwrap().len() < 128);
        assert!(result["events"][0]["authorLookupId"].is_null());
    }
}
