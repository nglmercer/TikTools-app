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

pub fn register(router: &mut ControlRouter) {
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
