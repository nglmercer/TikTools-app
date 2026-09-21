//! Topic subscription parsing and matching.

use url::form_urlencoded;

pub(crate) fn query_topics(query: &str) -> Vec<String> {
    let topics = form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == "topics")
        .map(|(_, value)| value.into_owned())
        .unwrap_or_else(|| "*".to_owned());
    let mut parsed = Vec::new();
    for topic in topics
        .split(',')
        .map(str::trim)
        .filter(|topic| !topic.is_empty())
    {
        if !tiktools_plugin_sdk::tiktools_plugin_api::manifest::is_valid_event_subscription(topic) {
            return Vec::new();
        }
        parsed.push(topic.to_owned());
    }
    if parsed.is_empty() {
        vec!["*".to_owned()]
    } else {
        parsed
    }
}

pub(crate) fn matches_topics(topics: &[String], topic: &str) -> bool {
    topics.iter().any(|subscription| {
        tiktools_plugin_sdk::tiktools_plugin_api::event_subscription_matches(subscription, topic)
    })
}
