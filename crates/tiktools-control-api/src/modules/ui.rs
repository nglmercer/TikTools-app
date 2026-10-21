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

pub fn register(router: &mut ControlRouter) {
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
