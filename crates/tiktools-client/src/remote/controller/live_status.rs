//! Dedicated read-only LIVE connection status for the controller window.
//! Neither this method nor its projection can start, stop or control a LIVE.
use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HostStatus {
    connected: bool,
    unique_id: Option<String>,
    room_id: Option<String>,
    connection_id: Option<String>,
    native: bool,
}

const FEED_LIMIT: usize = 64;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FeedPage {
    oldest_cursor: u64,
    latest_cursor: u64,
    depth: usize,
    gap: bool,
    truncated: bool,
    events: Vec<FeedEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FeedEntry {
    cursor: u64,
    event: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreatorFeedPage {
    creator_id: String,
    stream_id: Option<String>,
    oldest_cursor: String,
    latest_cursor: String,
    next_cursor: String,
    gap: bool,
    truncated: bool,
    events: Vec<CreatorFeedEvent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreatorFeedEvent {
    cursor: String,
    kind: String,
    author: String,
    author_lookup_id: Option<String>,
    nickname: Option<String>,
    text: String,
}

fn cursor(value: &str) -> Result<u64, ClientError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|n| n.to_string() == value)
        .ok_or_else(protocol)
}
fn valid_stream(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn project_creator_feed(
    page: CreatorFeedPage,
    creator: &str,
    requested_stream: Option<&str>,
    after: u64,
) -> Result<Value, ClientError> {
    let oldest = cursor(&page.oldest_cursor)?;
    let latest = cursor(&page.latest_cursor)?;
    let next = cursor(&page.next_cursor)?;
    if page.creator_id != creator
        || page.events.len() > FEED_LIMIT
        || page
            .stream_id
            .as_deref()
            .is_some_and(|id| !valid_stream(id))
        || (page.stream_id.is_none()
            && (oldest != 0 || latest != 0 || next != 0 || !page.events.is_empty()))
        || (oldest > latest)
        || (page.truncated != (next < latest))
    {
        return Err(protocol());
    }
    let reset = requested_stream.is_some() && requested_stream != page.stream_id.as_deref()
        || after > latest;
    if reset && !page.gap {
        return Err(protocol());
    }
    let mut previous = if reset { 0 } else { after };
    let mut events = Vec::with_capacity(page.events.len());
    for entry in page.events {
        let sequence = cursor(&entry.cursor)?;
        if sequence <= previous
            || sequence > latest
            || sequence < oldest
            || (events.is_empty() && previous == 0 && sequence != oldest)
            || (events.is_empty() && previous == 0 && oldest > 1 && !page.gap)
            || (!events.is_empty() && previous.checked_add(1) != Some(sequence))
            || (events.is_empty()
                && previous != 0
                && !page.gap
                && previous.checked_add(1) != Some(sequence))
            || !matches!(
                entry.kind.as_str(),
                "chat" | "gift" | "like" | "member" | "social"
            )
            || entry.author.chars().count() > 64
            || entry.text.chars().count() > 128
            || entry
                .nickname
                .as_ref()
                .is_some_and(|n| n.chars().count() > 64)
            || entry
                .author_lookup_id
                .as_ref()
                .is_some_and(|id| lookup_author_id(id, &entry.author).as_deref() != Some(id))
        {
            return Err(protocol());
        }
        previous = sequence;
        events.push(json!({"cursor":entry.cursor,"kind":entry.kind,"author":entry.author,"authorLookupId":entry.author_lookup_id,"nickname":entry.nickname,"text":entry.text}));
    }
    if !events.is_empty() && next != previous
        || events.is_empty() && next != previous && !(page.gap && oldest == 0 && next == latest)
    {
        return Err(protocol());
    }
    Ok(
        json!({"creatorId":page.creator_id,"streamId":page.stream_id,"oldestCursor":page.oldest_cursor,"latestCursor":page.latest_cursor,"nextCursor":page.next_cursor,"gap":page.gap,"truncated":page.truncated,"events":events}),
    )
}

fn bounded_text(value: &str, maximum: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .take(maximum)
        .collect()
}

/// A display label can be sanitized or truncated and `viewer` is also the
/// host's fallback for a missing ID. Never turn either into a points target.
fn lookup_author_id(original: &str, displayed: &str) -> Option<String> {
    (original == displayed
        && !original.is_empty()
        && original != "viewer"
        && original.len() <= 128
        && original.trim() == original
        && !original.starts_with('@')
        && !original.contains('*')
        && !original.chars().any(char::is_control))
    .then(|| original.to_owned())
}

fn project_feed(page: FeedPage, after: u64) -> Result<Value, ClientError> {
    if page.depth > 512
        || page.events.len() > FEED_LIMIT
        || (page.depth == 0 && (page.oldest_cursor != 0 || page.latest_cursor != 0))
        || (page.depth != 0 && (page.oldest_cursor == 0 || page.oldest_cursor > page.latest_cursor))
    {
        return Err(protocol());
    }
    let mut previous = after;
    let mut events = Vec::with_capacity(page.events.len());
    for entry in page.events {
        if entry.cursor <= previous
            || entry.cursor < page.oldest_cursor
            || entry.cursor > page.latest_cursor
            || (!events.is_empty() && previous.checked_add(1) != Some(entry.cursor))
            || (events.is_empty() && after == 0 && entry.cursor != page.oldest_cursor)
            || (events.is_empty()
                && after != 0
                && !page.gap
                && previous.checked_add(1) != Some(entry.cursor))
        {
            return Err(protocol());
        }
        previous = entry.cursor;
        let event = entry.event.as_object().ok_or_else(protocol)?;
        let kind = event
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(protocol)?;
        if !matches!(kind, "chat" | "gift" | "like" | "member" | "social") {
            return Err(protocol());
        }
        let author = event
            .get("author")
            .and_then(Value::as_str)
            .ok_or_else(protocol)?;
        let text = event
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(protocol)?;
        let nickname = match event.get("nickname") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) => Some(bounded_text(value, 128)),
            _ => return Err(protocol()),
        };
        let displayed_author = bounded_text(author, 128);
        let author_lookup_id = lookup_author_id(author, &displayed_author);
        events.push(json!({"cursor":entry.cursor.to_string(),"kind":kind,
            "author":displayed_author,"authorLookupId":author_lookup_id,"nickname":nickname,"text":bounded_text(text, 2048)}));
    }
    if (after > page.latest_cursor && (!page.gap || !events.is_empty()))
        || (after < page.latest_cursor && events.is_empty())
        || page.truncated != (previous < page.latest_cursor)
    {
        return Err(protocol());
    }
    let next = if after > page.latest_cursor {
        0
    } else {
        previous
    };
    Ok(
        json!({"oldestCursor":page.oldest_cursor.to_string(),"latestCursor":page.latest_cursor.to_string(),
        "nextCursor":next.to_string(),"gap":page.gap || (after == 0 && page.oldest_cursor > 1),"truncated":page.truncated,"events":events}),
    )
}

fn valid_optional(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|value| {
        !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
    })
}

fn bound(
    v: &View,
    epoch: &str,
    connection: &str,
    expected: Option<&RemoteClient>,
) -> Result<RemoteClient, ClientError> {
    let remote = v.remote.as_ref().ok_or_else(network)?;
    let host = v.status.hello.as_ref().ok_or_else(network)?;
    let actual = remote.hello();
    if v.status.phase != "ready"
        || v.status.epoch != epoch
        || connection.is_empty()
        || host.connection_id != connection
        || actual.connection_id != connection
        || host.host_id != actual.host_id
        || host.host_instance_id != actual.host_instance_id
        || host.principal_id != actual.principal_id
        || host.session_id != actual.session_id
        || host.grant_revision != actual.grant_revision
        || host.capabilities != actual.capabilities
        || host.resource_restrictions != actual.resource_restrictions
        || !host.capabilities.contains(&Capability::LiveRead)
        || remote.is_closed()
        || remote.requires_resync()
        || expected.is_some_and(|old| {
            old.is_closed()
                || old.requires_resync()
                || old.hello().connection_id != actual.connection_id
                || old.hello().host_id != actual.host_id
                || old.hello().host_instance_id != actual.host_instance_id
                || old.hello().principal_id != actual.principal_id
                || old.hello().session_id != actual.session_id
                || old.hello().grant_revision != actual.grant_revision
        })
    {
        return Err(fail(
            "stale_view",
            "LIVE status belongs to a previous controller session",
        ));
    }
    Ok(remote.clone())
}

impl Controller {
    /// Bounded, read-only activity; this is polling, not a durable event feed.
    /// Unfiltered history is denied for creator-scoped grants by the host and
    /// here as well. A session/host/grant change invalidates the response.
    pub(super) async fn live_feed(
        &self,
        epoch: &str,
        connection: &str,
        after: &str,
        creator: Option<&str>,
        stream: Option<&str>,
    ) -> Result<Value, ClientError> {
        let cursor = after
            .parse::<u64>()
            .map_err(|_| fail("invalid_params", "invalid LIVE cursor"))?;
        if cursor.to_string() != after {
            return Err(fail("invalid_params", "invalid LIVE cursor"));
        }
        let remote = {
            let view = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            let remote = bound(&view, epoch, connection, None)?;
            if creator.is_none()
                && remote
                    .hello()
                    .resource_restrictions
                    .contains_key(&ResourceKind::Live)
            {
                return Err(fail(
                    "forbidden",
                    "LIVE feed is not available for this grant",
                ));
            }
            remote
        };
        if let Some(creator) = creator {
            if remote.hello().features.get("creatorFeed") != Some(&true) {
                return Err(fail(
                    "unsupported",
                    "host does not support creator-scoped LIVE feed",
                ));
            }
            let allowed = |remote: &RemoteClient| {
                remote
                    .hello()
                    .resource_restrictions
                    .get(&ResourceKind::Live)
                    .is_none_or(|ids| ids.contains(creator))
            };
            if creator.is_empty()
                || creator.len() > 256
                || creator.trim() != creator
                || creator.chars().any(char::is_control)
                || stream.is_some_and(|id| !valid_stream(id))
                || stream.is_none() && cursor != 0
                || !allowed(&remote)
            {
                return Err(fail(
                    "forbidden",
                    "LIVE feed is not available for this grant",
                ));
            }
            let response = remote
                .call_value_with_meta(
                    "ui.feed.creator",
                    json!({"creatorId":creator,"streamId":stream,"after":after,"limit":FEED_LIMIT}),
                    None,
                )
                .await?;
            let page: CreatorFeedPage =
                serde_json::from_value(response.result).map_err(|_| protocol())?;
            let result = project_creator_feed(page, creator, stream, cursor)?;
            let view = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            let current = bound(&view, epoch, connection, Some(&remote))?;
            if !allowed(&current) {
                return Err(fail(
                    "forbidden",
                    "LIVE feed is not available for this grant",
                ));
            }
            let mut result = result.as_object().cloned().ok_or_else(protocol)?;
            result.insert("epoch".into(), json!(epoch));
            result.insert("connectionId".into(), json!(connection));
            return Ok(Value::Object(result));
        }
        if stream.is_some() {
            return Err(fail("invalid_params", "unexpected feed stream"));
        }
        let response = remote
            .call_value_with_meta(
                "ui.feed.backfill",
                json!({"version":1,"after":cursor,"limit":FEED_LIMIT}),
                None,
            )
            .await?;
        let page: FeedPage = serde_json::from_value(response.result).map_err(|_| protocol())?;
        let result = project_feed(page, cursor)?;
        {
            let view = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            let current = bound(&view, epoch, connection, Some(&remote))?;
            if current
                .hello()
                .resource_restrictions
                .contains_key(&ResourceKind::Live)
            {
                return Err(fail(
                    "forbidden",
                    "LIVE feed is not available for this grant",
                ));
            }
        }
        let mut result = result.as_object().cloned().ok_or_else(protocol)?;
        result.insert("epoch".into(), json!(epoch));
        result.insert("connectionId".into(), json!(connection));
        Ok(Value::Object(result))
    }
    pub(super) async fn live_status(
        &self,
        epoch: &str,
        connection: &str,
    ) -> Result<Value, ClientError> {
        let remote = {
            let view = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            bound(&view, epoch, connection, None)?
        };
        let result = remote
            .call_value_with_meta("live.status", json!({}), None)
            .await?;
        let result: HostStatus = serde_json::from_value(result.result).map_err(|_| protocol())?;
        if !valid_optional(&result.unique_id)
            || !valid_optional(&result.room_id)
            || !valid_optional(&result.connection_id)
        {
            return Err(protocol());
        }
        {
            let view = self.shared.view.read().unwrap_or_else(|p| p.into_inner());
            let current = bound(&view, epoch, connection, Some(&remote))?;
            if current
                .hello()
                .resource_restrictions
                .get(&ResourceKind::Live)
                .is_some_and(|allowed| {
                    !result
                        .unique_id
                        .as_ref()
                        .is_some_and(|id| allowed.contains(id))
                })
            {
                return Err(fail("forbidden", "LIVE creator is not permitted"));
            }
        }
        Ok(
            json!({"epoch":epoch,"connectionId":connection,"connected":result.connected,
            "uniqueId":result.unique_id,"native":result.native}),
        )
    }
}

#[cfg(test)]
mod feed_tests {
    use super::*;

    #[test]
    fn feed_projection_preserves_cursor_precision_and_bounds_untrusted_content() {
        let page: FeedPage = serde_json::from_value(json!({"oldestCursor":9007199254740993u64,
            "latestCursor":9007199254740994u64,"depth":2,"gap":true,"truncated":false,
            "events":[{"cursor":9007199254740994u64,"event":{"kind":"chat","author":"x".repeat(300),
                "text":"hello\0world", "secret":"not projected"}}]}))
        .unwrap();
        let result = project_feed(page, 9007199254740992).unwrap();
        assert_eq!(result["nextCursor"], "9007199254740994");
        assert_eq!(result["events"][0]["author"].as_str().unwrap().len(), 128);
        assert_eq!(result["events"][0]["authorLookupId"], Value::Null);
        assert_eq!(result["events"][0]["text"], "helloworld");
        assert!(result["events"][0].get("secret").is_none());
    }

    #[test]
    fn feed_lookup_author_requires_an_exact_unambiguous_id() {
        for (author, expected) in [
            ("alice", Some("alice")),
            ("viewer", None),
            ("@alice", None),
            (" alice", None),
            ("ali\0ce", None),
            ("a*b", None),
        ] {
            let page: FeedPage = serde_json::from_value(json!({"oldestCursor":1,"latestCursor":1,
                "depth":1,"gap":false,"truncated":false,"events":[
                    {"cursor":1,"event":{"kind":"chat","author":author,"text":"hello"}}]}))
            .unwrap();
            assert_eq!(
                project_feed(page, 0).unwrap()["events"][0]["authorLookupId"],
                json!(expected)
            );
        }
        let long = "é".repeat(65);
        assert_eq!(lookup_author_id(&long, &long), None);
    }

    #[test]
    fn feed_detects_invalid_order_and_recovers_cursor_after_host_restart() {
        let page: FeedPage = serde_json::from_value(json!({"oldestCursor":0,"latestCursor":0,
            "depth":0,"gap":true,"truncated":false,"events":[]}))
        .unwrap();
        assert_eq!(project_feed(page, 50).unwrap()["nextCursor"], "0");
        let page: FeedPage = serde_json::from_value(json!({"oldestCursor":1,"latestCursor":2,
            "depth":2,"gap":false,"truncated":false,"events":[
                {"cursor":2,"event":{"kind":"chat","author":"a","text":"a"}},
                {"cursor":1,"event":{"kind":"chat","author":"b","text":"b"}}]}))
        .unwrap();
        assert!(project_feed(page, 0).is_err());
        let missing_first: FeedPage =
            serde_json::from_value(json!({"oldestCursor":1,"latestCursor":2,
            "depth":2,"gap":false,"truncated":false,"events":[
                {"cursor":2,"event":{"kind":"chat","author":"b","text":"b"}}]}))
            .unwrap();
        assert!(project_feed(missing_first, 0).is_err());
        let retained_window: FeedPage =
            serde_json::from_value(json!({"oldestCursor":2,"latestCursor":2,
            "depth":1,"gap":false,"truncated":false,"events":[
                {"cursor":2,"event":{"kind":"chat","author":"b","text":"b"}}]}))
            .unwrap();
        assert_eq!(project_feed(retained_window, 0).unwrap()["gap"], true);
    }
}
