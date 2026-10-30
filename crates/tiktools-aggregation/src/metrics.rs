//! Descriptors for the native metrics currently emitted by the ingress adapter.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MetricDescriptor {
    pub source: String,
    pub metric: String,
    pub description: String,
    pub unit: String,
}

/// These describe accepted increments, not TikTok's room cumulative totals.
pub fn descriptors() -> Vec<MetricDescriptor> {
    [
        (
            "competition.vote",
            "event.count",
            "Durable command votes assigned to an option",
            "votes",
        ),
        (
            "tiktok.like",
            "like.count",
            "Accepted like packet count delta",
            "likes",
        ),
        (
            "tiktok.like",
            "event.count",
            "Accepted like packets",
            "packets",
        ),
        (
            "tiktok.gift",
            "gift.count",
            "Accepted incremental gift quantity",
            "gifts",
        ),
        (
            "tiktok.gift",
            "gift.diamonds",
            "Accepted incremental known diamond value",
            "diamonds",
        ),
        (
            "tiktok.gift",
            "event.count",
            "Gift packets with a positive accepted increment",
            "packets",
        ),
        (
            "tiktok.share",
            "share.count",
            "Classified individual share occurrences",
            "shares",
        ),
        (
            "tiktok.share",
            "event.count",
            "Classified individual share occurrences",
            "events",
        ),
        (
            "tiktok.follow",
            "event.count",
            "Classified follow occurrences",
            "events",
        ),
        (
            "tiktok.social",
            "event.count",
            "Other or unknown social events",
            "events",
        ),
        (
            "tiktok.chat",
            "event.count",
            "Accepted chat and emote-only events",
            "events",
        ),
        (
            "points.awarded",
            "points.delta",
            "Whole points granted by committed LIVE awards",
            "points",
        ),
        (
            "plugin.event",
            "event.count",
            "Plugin-reported ranking contributions (rankings.contribute)",
            "events",
        ),
        (
            "plugin.event",
            "value",
            "Plugin-reported integer contribution value (rankings.contribute)",
            "units",
        ),
        (
            "tiktok.join",
            "event.count",
            "Accepted member join events",
            "events",
        ),
    ]
    .into_iter()
    .map(|(source, metric, description, unit)| MetricDescriptor {
        source: source.into(),
        metric: metric.into(),
        description: description.into(),
        unit: unit.into(),
    })
    .collect()
}
