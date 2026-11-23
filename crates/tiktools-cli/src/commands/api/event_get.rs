//! Client-side lookup over the bounded feed journal (`ui.feed.backfill`).
//! No new RPC is needed: the host already pages LIVE render events by cursor.
use super::*;
use tiktools_control_api::modules::ui::{FeedBackfill, FeedBackfillParams};

/// Small seam for the paginated journal read so the paging/limit logic can be
/// exercised without a live host. The real client simply forwards the RPC.
pub(super) trait FeedJournal {
    async fn backfill(&self, params: FeedBackfillParams) -> Result<FeedBackfill, ClientError>;
}

impl FeedJournal for TikToolsClient {
    async fn backfill(&self, params: FeedBackfillParams) -> Result<FeedBackfill, ClientError> {
        self.ui_feed_backfill(params).await
    }
}

const PAGE_SIZE: usize = 512;
const MAX_PAGES: usize = 8;

/// True when an event matches the query. Names match any string field, kinds
/// accept both raw `kind` values and the feed's `join`/`follow` sub-kinds.
pub(super) fn event_matches(
    event: &Value,
    name: &str,
    kinds: &[String],
    author: Option<&str>,
) -> bool {
    let Some(object) = event.as_object() else {
        return false;
    };
    if !kinds.is_empty() {
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let sub_kind = object
            .get("i18nKey")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let matches = kinds.iter().any(|candidate| match candidate.as_str() {
            "join" => kind == "member" && sub_kind == "joinedLive",
            "follow" => kind == "social" && sub_kind == "followedCreator",
            other => kind == other,
        });
        if !matches {
            return false;
        }
    }
    if let Some(filter) = author {
        let needle = filter.to_lowercase();
        let matches = ["author", "nickname"]
            .iter()
            .filter_map(|field| object.get(*field).and_then(Value::as_str))
            .any(|value| value.to_lowercase().contains(&needle));
        if !matches {
            return false;
        }
    }
    if name.is_empty() {
        return true;
    }
    let needle = name.to_lowercase();
    object.values().any(|value| match value {
        Value::String(text) => text.to_lowercase().contains(&needle),
        _ => false,
    })
}

/// Pages the journal from oldest to newest, keeps the newest `limit` matches.
pub(super) async fn get_event(
    client: &TikToolsClient,
    name: &str,
    kinds: &[String],
    author: Option<&str>,
    limit: usize,
) -> Result<Output, ClientError> {
    collect_event_matches(client, name, kinds, author, limit).await
}

async fn collect_event_matches<S: FeedJournal>(
    source: &S,
    name: &str,
    kinds: &[String],
    author: Option<&str>,
    limit: usize,
) -> Result<Output, ClientError> {
    let mut after = 0u64;
    let mut matches: Vec<Value> = Vec::new();
    let mut scanned = 0usize;
    for _ in 0..MAX_PAGES {
        let page = source
            .backfill(FeedBackfillParams {
                version: 1,
                after,
                limit: Some(PAGE_SIZE),
            })
            .await?;
        scanned += page.events.len();
        for entry in &page.events {
            if event_matches(&entry.event, name, kinds, author) {
                matches.push(json!({ "cursor": entry.cursor, "event": entry.event }));
            }
        }
        if page.events.is_empty() || !page.truncated || after >= page.latest_cursor {
            break;
        }
        after = page
            .events
            .last()
            .map(|entry| entry.cursor)
            .unwrap_or(after);
    }
    if matches.len() > limit {
        matches.drain(..matches.len() - limit);
    }
    Ok(Output::new(
        "api-event-get",
        json!({
            "query": { "name": name, "kinds": kinds, "author": author, "limit": limit },
            "scanned": scanned,
            "matches": matches,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn event(kind: &str, author: &str, text: &str, i18n_key: Option<&str>) -> Value {
        let mut value = json!({ "kind": kind, "author": author, "text": text });
        if let Some(key) = i18n_key {
            value["i18nKey"] = json!(key);
        }
        value
    }

    #[test]
    fn name_matches_any_string_field_case_insensitively() {
        let gift = event("gift", "mango", "Rose", None);
        assert!(event_matches(&gift, "rose", &[], None));
        assert!(event_matches(&gift, "MANGO", &[], None));
        assert!(!event_matches(&gift, "bubblegum", &[], None));
    }

    #[test]
    fn kinds_cover_raw_kinds_and_join_follow_subkinds() {
        let join = event("member", "kiwi", "joined", Some("joinedLive"));
        let follow = event("social", "ana", "followed", Some("followedCreator"));
        let chat = event("chat", "luna", "hola", None);
        assert!(event_matches(&join, "", &["join".to_owned()], None));
        assert!(event_matches(&follow, "", &["follow".to_owned()], None));
        assert!(event_matches(&chat, "", &["chat".to_owned()], None));
        assert!(!event_matches(&chat, "", &["gift".to_owned()], None));
    }

    #[test]
    fn author_filter_is_case_insensitive() {
        let chat = event("chat", "Luna", "hola", None);
        assert!(event_matches(&chat, "", &[], Some("lun")));
        assert!(!event_matches(&chat, "", &[], Some("mango")));
    }

    fn page(cursor: u64, latest: u64, events: Vec<Value>, truncated: bool) -> FeedBackfill {
        let events = events
            .into_iter()
            .enumerate()
            .map(|(index, event)| json!({ "cursor": cursor + index as u64, "event": event }))
            .collect::<Vec<_>>();
        serde_json::from_value(json!({
            "oldestCursor": cursor,
            "latestCursor": latest,
            "depth": 0,
            "gap": false,
            "truncated": truncated,
            "events": events,
        }))
        .unwrap()
    }

    struct FakeJournal {
        pages: Mutex<Vec<FeedBackfill>>,
    }

    impl FeedJournal for FakeJournal {
        async fn backfill(&self, _params: FeedBackfillParams) -> Result<FeedBackfill, ClientError> {
            let mut pages = self.pages.lock().unwrap();
            Ok(if pages.is_empty() {
                page(0, 0, Vec::new(), false)
            } else {
                pages.remove(0)
            })
        }
    }

    #[tokio::test]
    async fn keeps_newest_matches_across_pages_and_counts_scanned_events() {
        let journal = FakeJournal {
            pages: Mutex::new(vec![
                page(
                    1,
                    2,
                    vec![
                        event("chat", "mango", "rose", None),
                        event("gift", "kiwi", "rose", None),
                    ],
                    true,
                ),
                page(3, 3, vec![event("gift", "mango", "ROSE", None)], false),
            ]),
        };
        let output = collect_event_matches(&journal, "rose", &[], None, 2)
            .await
            .unwrap();
        assert_eq!(output.command, "api-event-get");
        assert_eq!(output.value["scanned"], json!(3));
        let cursors = output.value["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["cursor"].as_u64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(cursors, vec![2, 3]);
    }

    #[tokio::test]
    async fn stops_after_the_page_budget_even_when_every_page_is_truncated() {
        let pages = (1..=8)
            .map(|cursor| {
                page(
                    cursor,
                    cursor,
                    vec![event("chat", "mango", "rose", None)],
                    true,
                )
            })
            .collect();
        let journal = FakeJournal {
            pages: Mutex::new(pages),
        };
        let output = collect_event_matches(&journal, "rose", &[], None, 10)
            .await
            .unwrap();
        assert_eq!(output.value["scanned"], json!(8));
        assert_eq!(output.value["matches"].as_array().unwrap().len(), 8);
    }

    #[tokio::test]
    async fn returns_no_matches_when_nothing_in_the_journal_matches() {
        let journal = FakeJournal {
            pages: Mutex::new(vec![page(
                1,
                1,
                vec![event("chat", "mango", "hola", None)],
                false,
            )]),
        };
        let output = collect_event_matches(&journal, "rose", &[], None, 5)
            .await
            .unwrap();
        assert_eq!(output.value["scanned"], json!(1));
        assert_eq!(output.value["matches"], json!([]));
    }
}
