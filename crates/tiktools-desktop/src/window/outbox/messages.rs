//! Single-pass message classification, routing and escaped UI payload preparation.
use super::*;

impl QueuedWebviewMessage {
    pub(super) fn bytes(&self) -> usize {
        self.body.len()
            + self
                .javascript_literal
                .as_ref()
                .map_or(0, |value| value.len())
            + self.coalesce_key.as_ref().map_or(0, String::len)
            + self.ranking_revision.as_ref().map_or(0, |(id, _)| id.len())
            + self
                .plugin
                .as_ref()
                .map_or(0, |route| route.plugin_id.len() + route.topic.len())
    }

    pub(super) fn from_value(
        value: &serde_json::Value,
        body: Option<String>,
    ) -> Result<Self, serde_json::Error> {
        let plugin = plugin_route(value);
        let limit = message_limit(value);
        #[derive(serde::Serialize)]
        struct Compatible<'a> {
            #[serde(rename = "apiVersion")]
            version: u32,
            #[serde(flatten)]
            value: &'a serde_json::Value,
        }
        let body = if plugin.is_some() && value.get("apiVersion").is_none() {
            encode_bounded(&Compatible { version: 1, value })?
        } else if let Some(body) = body {
            if body.len() > limit {
                return Err(serde_json::Error::io(std::io::Error::other(
                    "outbound payload exceeds transport limit",
                )));
            }
            body
        } else {
            encode_with_limit(value, limit)?
        };
        let javascript_literal = encode_with_limit(&body, 2 * limit + 2)?;
        Ok(Self {
            body: body.into(),
            javascript_literal: Some(javascript_literal.into()),
            class: classify_value(value),
            coalesce_key: webview_coalesce_key(value),
            coalesce_generation: (value
                .pointer("/params/topic")
                .and_then(serde_json::Value::as_str)
                == Some("competition.result"))
            .then(|| {
                value
                    .pointer("/params/data/result/generation")
                    .and_then(serde_json::Value::as_u64)
            })
            .flatten(),
            ranking_revision: (value
                .pointer("/params/topic")
                .and_then(serde_json::Value::as_str)
                == Some("ranking.updated"))
            .then(|| {
                let id = value
                    .pointer("/params/data/snapshot/instanceId")?
                    .as_str()?;
                if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return None;
                }
                Some((
                    id.to_owned(),
                    value.pointer("/params/data/snapshot/revision")?.as_u64()?,
                ))
            })
            .flatten(),
            plugin,
        })
    }
}

/// Coalesce identity for snapshots, e.g. `legacy:room-stats`,
/// `domain:room.stats`, `domain:plugin.progress:<plugin-id>`.
fn webview_coalesce_key(value: &serde_json::Value) -> Option<String> {
    if classify_value(value) != WebviewMessageClass::Coalescable {
        return None;
    }
    if let Some(kind) = value.get("type").and_then(serde_json::Value::as_str) {
        return Some(format!("legacy:{kind}"));
    }
    let topic = value
        .get("params")
        .and_then(|params| params.get("topic"))
        .and_then(serde_json::Value::as_str)?;
    if topic == "ranking.updated" {
        let ranking = value.pointer("/params/data/snapshot/rankingId")?.as_str()?;
        return Some(format!("domain:{topic}:{ranking}"));
    }
    if topic == "competition.result" {
        let competition = value
            .pointer("/params/data/result/competitionId")?
            .as_str()?;
        return Some(format!("domain:{topic}:{competition}"));
    }
    if topic == "plugin.progress" {
        let plugin = value
            .get("params")
            .and_then(|params| params.get("data"))
            .and_then(|data| data.get("pluginId"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        return Some(format!("domain:{topic}:{plugin}"));
    }
    Some(format!("domain:{topic}"))
}

fn classify_value(value: &serde_json::Value) -> WebviewMessageClass {
    if let Some(kind) = value.get("type").and_then(serde_json::Value::as_str) {
        return match kind {
            // Compat duplicates of authoritative domain twins: safe to shed.
            "live-event" | "points-awarded" => WebviewMessageClass::Droppable,
            "room-stats" | "leaderboard" | "analytics-summary" | "processor-status"
            | "automation-context" | "gift-catalog" => WebviewMessageClass::Coalescable,
            // rpc-response, connection/error/reconnecting transitions, and
            // every other rare state/result message: never evicted.
            _ => WebviewMessageClass::Critical,
        };
    }
    if value.get("method").and_then(serde_json::Value::as_str) == Some("event") {
        let topic = value
            .get("params")
            .and_then(|params| params.get("topic"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        return match topic {
            "room.stats"
            | "analytics.updated"
            | "gifts.catalog"
            | "ranking.updated"
            | "competition.result"
            | "live.likes-ranking"
            | "live.likes-ranking-full"
            | "ui.points.snapshot"
            | "ui.automation.runs"
            | "ui.projection.stats" => WebviewMessageClass::Coalescable,
            "live.event" | "live.ui-event" => WebviewMessageClass::Droppable,
            // Connection/lifecycle/state transitions, errors, shutdown,
            // and unknown topics: never evicted.
            _ => WebviewMessageClass::Critical,
        };
    }
    // Unrecognized shapes default to Critical per the core rule.
    WebviewMessageClass::Critical
}

/// String entry point kept for the classification contract tests; the
/// production paths parse once (`push`) or never (`push_value`).
#[cfg(test)]
pub(crate) fn classify_webview_message(message: &str) -> WebviewMessageClass {
    match serde_json::from_str::<serde_json::Value>(message) {
        Ok(value) => classify_value(&value),
        Err(_) => WebviewMessageClass::Critical,
    }
}

fn plugin_route(value: &serde_json::Value) -> Option<PluginRoute> {
    if value.get("method")?.as_str()? != "event" {
        return None;
    }
    let params = value.get("params")?;
    let topic = params.get("topic")?.as_str()?;
    if !topic.starts_with("plugin.") || topic.len() > 128 {
        return None;
    }
    let plugin_id = params.get("data")?.get("pluginId")?.as_str()?;
    if !crate::plugin_webview::is_valid_ui_id(plugin_id) {
        return None;
    }
    Some(PluginRoute {
        plugin_id: plugin_id.to_owned(),
        topic: topic.to_owned(),
    })
}
