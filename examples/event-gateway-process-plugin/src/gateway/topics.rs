//! Topic subscription parsing and matching.

use std::fmt;
use url::form_urlencoded;

/// Malformed `topics` query input. Callers must reject the request
/// (HTTP 400) or control message; the input is never widened to a
/// wildcard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TopicParseError {
    reason: &'static str,
}

impl fmt::Display for TopicParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid topics query: {}", self.reason)
    }
}

impl std::error::Error for TopicParseError {}

/// Parses the `topics` query parameter:
/// - missing parameter → default wildcard (`*`), the documented default;
/// - valid comma-separated list → parsed, deduplicated topics;
/// - any malformed entry, or explicit-but-empty input → error, so a
///   client bug can never silently widen into full access.
pub(crate) fn query_topics(query: &str) -> Result<Vec<String>, TopicParseError> {
    let Some(raw) = form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == "topics")
        .map(|(_, value)| value.into_owned())
    else {
        return Ok(vec!["*".to_owned()]);
    };
    let mut parsed: Vec<String> = Vec::new();
    for topic in raw
        .split(',')
        .map(str::trim)
        .filter(|topic| !topic.is_empty())
    {
        if !tiktools_plugin_sdk::tiktools_plugin_api::manifest::is_valid_event_subscription(topic) {
            return Err(TopicParseError {
                reason: "malformed topic subscription",
            });
        }
        if !parsed.iter().any(|existing| existing == topic) {
            parsed.push(topic.to_owned());
        }
    }
    if parsed.is_empty() {
        return Err(TopicParseError {
            reason: "topics must not be empty",
        });
    }
    Ok(parsed)
}

/// Gateway topic matching: exact topics, the `*` match-all
/// wildcard, and `prefix.*` namespace wildcards. A namespace
/// wildcard matches the bare `prefix` topic itself and every topic
/// starting with `prefix.`, so subscribing to `live.*` also covers
/// a summary event published directly on `live`.
pub(crate) fn matches_topics(topics: &[String], topic: &str) -> bool {
    topics
        .iter()
        .any(|subscription| subscription_matches(subscription, topic))
}

fn subscription_matches(subscription: &str, topic: &str) -> bool {
    if subscription == "*" || subscription == topic {
        return true;
    }
    subscription.strip_suffix(".*").is_some_and(|prefix| {
        !prefix.is_empty()
            && (topic == prefix
                || topic
                    .strip_prefix(prefix)
                    .is_some_and(|remainder| remainder.starts_with('.')))
    })
}
