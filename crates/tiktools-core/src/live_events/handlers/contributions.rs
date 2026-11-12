//! Ranking contributions from already admitted native packets and committed point awards. No
//! event identities, streak increments or deltas are coalesced here.

use crate::{
    helpers::{clean_unique_id, native_user},
    AppCore, LiveContext, NativeLiveEvent,
};
use serde_json::Value;

impl AppCore {
    /// Observe only accepted original deltas after canonical dedup and totals.
    pub(super) fn observe_native_ranking(
        &self,
        event: &NativeLiveEvent,
        context: &LiveContext,
        totals: &Value,
    ) {
        use tiktools_tiktok::events::CanonicalLiveEvent;
        let (source, user, gift, metrics) = match &event.base {
            CanonicalLiveEvent::Like(like) => {
                let user = if like.user.id != 0 {
                    like.user.id.to_string()
                } else {
                    like.user.unique_id.clone()
                };
                (
                    "tiktok.like",
                    user,
                    None,
                    vec![("like.count", like.count), ("event.count", 1)],
                )
            }
            CanonicalLiveEvent::Gift(gift)
                if !totals.is_null()
                    && ((gift.group_id != 0 && (gift.user.id == 0 || gift.gift_id == 0))
                        || (gift.group_id == 0
                            && (event.gift_streakable()
                                || gift.repeat_count > 1
                                || gift.combo_count > 1))) =>
            {
                self.aggregations
                    .mark_gap(&context.unique_id, &context.room_id, "tiktok.gift");
                tracing::warn!(msg_id = %event.msg_id(), "ambiguous gift streak identity; ranking contribution rejected");
                ("", String::new(), None, Vec::new())
            }
            CanonicalLiveEvent::Gift(gift) if !totals.is_null() => {
                let user = if gift.user.id != 0 {
                    gift.user.id.to_string()
                } else {
                    gift.user.unique_id.clone()
                };
                let increment = totals["increment"].as_u64().unwrap_or(0);
                let diamonds = totals["diamonds"].as_u64().unwrap_or(0);
                let price = event.gift_diamond_count().unwrap_or(0);
                let count = gift.repeat_count.max(gift.combo_count).max(1);
                let mut metrics = vec![
                    ("gift.count", increment),
                    ("event.count", u64::from(increment > 0)),
                ];
                // A newly discovered price can make LiveTotals' cumulative
                // diamond high-water jump for quantities already observed.
                // Do not credit that retroactive amount or a zero-delta final.
                let max_incremental = increment.checked_mul(price);
                if price > 0
                    && count.checked_mul(price).is_some()
                    && max_incremental.is_some_and(|maximum| diamonds <= maximum)
                    && increment > 0
                {
                    metrics.push(("gift.diamonds", diamonds));
                } else if increment > 0 || diamonds > 0 || count.checked_mul(price).is_none() {
                    self.aggregations.metric_gap(
                        &context.unique_id,
                        &context.room_id,
                        "tiktok.gift",
                        "gift.diamonds",
                    );
                }
                (
                    "tiktok.gift",
                    user,
                    (gift.gift_id != 0).then(|| gift.gift_id.to_string()),
                    metrics,
                )
            }
            CanonicalLiveEvent::Social(social) => {
                use tiktools_tiktok::events::SocialKind;
                let user = native_user(event)
                    .map(|u| {
                        if u.id != 0 {
                            u.id.to_string()
                        } else {
                            u.unique_id.clone()
                        }
                    })
                    .unwrap_or_default();
                match tiktools_tiktok::events::classify_social_action(social.action) {
                    SocialKind::Share => (
                        "tiktok.share",
                        user,
                        None,
                        vec![("share.count", 1), ("event.count", 1)],
                    ),
                    SocialKind::Follow => ("tiktok.follow", user, None, vec![("event.count", 1)]),
                    _ => ("tiktok.social", user, None, vec![("event.count", 1)]),
                }
            }
            CanonicalLiveEvent::Chat(_)
            | CanonicalLiveEvent::EmoteChat(_)
            | CanonicalLiveEvent::Member(_) => {
                let source = if matches!(&event.base, CanonicalLiveEvent::Member(_)) {
                    "tiktok.join"
                } else {
                    "tiktok.chat"
                };
                let user = native_user(event)
                    .map(|u| {
                        if u.id != 0 {
                            u.id.to_string()
                        } else {
                            u.unique_id.clone()
                        }
                    })
                    .unwrap_or_default();
                (source, user, None, vec![("event.count", 1)])
            }
            _ => ("", String::new(), None, Vec::new()),
        };
        if !source.is_empty() && (event.msg_id() == 0 || event.metadata.is_history) {
            // Missing identities and history beyond the bounded replay guard
            // cannot be proven new. Reject ranking accounting and expose
            // the gap while leaving existing consumers unchanged.
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, source);
        } else if !source.is_empty() {
            let user_metadata = native_user(event).map(|user| {
                let nickname = (!user.nickname.is_empty()).then(|| user.nickname.clone());
                let unique_id = clean_unique_id(&user.unique_id);
                let avatar_url = user
                    .avatar_url
                    .as_ref()
                    .filter(|value| {
                        value.len() <= 2048
                            && url::Url::parse(value).is_ok_and(|url| {
                                matches!(url.scheme(), "http" | "https")
                                    && url.host_str().is_some()
                                    && url.username().is_empty()
                                    && url.password().is_none()
                            })
                    })
                    .cloned();
                tiktools_aggregation::EntryMetadata {
                    label: nickname.clone().or_else(|| unique_id.clone()),
                    user_id: (user.id != 0).then(|| user.id.to_string()),
                    unique_id,
                    nickname,
                    avatar_url,
                    ..Default::default()
                }
            });
            let gift_metadata = match &event.base {
                CanonicalLiveEvent::Gift(item) if item.gift_id != 0 => {
                    let label = (!item.gift_name.is_empty())
                        .then(|| item.gift_name.clone())
                        .or_else(|| {
                            event
                                .gift
                                .as_ref()
                                .and_then(|gift| (!gift.name.is_empty()).then(|| gift.name.clone()))
                        });
                    Some(tiktools_aggregation::EntryMetadata {
                        label,
                        gift_id: Some(item.gift_id.to_string()),
                        icon_url: event.gift_icon_url().map(str::to_owned),
                        ..Default::default()
                    })
                }
                _ => None,
            };
            self.aggregations.observe(
                tiktools_aggregation::model::Contribution {
                    creator_id: &context.unique_id,
                    room_id: &context.room_id,
                    source,
                    user: (!user.is_empty()).then_some(user.as_str()),
                    gift: gift.as_deref(),
                    option: None,
                    team: None,
                    custom_key: None,
                    metrics: &metrics,
                },
                user_metadata.as_ref(),
                gift_metadata.as_ref(),
            );
        }
    }
    /// `points.awarded` / `points.delta`: whole points granted by one
    /// committed award, keyed like native user sources so it can share a
    /// ranking with likes or gifts. Fractions are truncated, never rounded up.
    pub(crate) fn observe_points_award(
        &self,
        event: &NativeLiveEvent,
        context: &LiveContext,
        delta: f64,
    ) {
        if !(delta.is_finite() && delta >= 1.0) {
            return;
        }
        let Some(user) = crate::helpers::native_user(event) else {
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, "points.awarded");
            return;
        };
        let key = if user.id != 0 {
            user.id.to_string()
        } else {
            user.unique_id.clone()
        };
        if key.is_empty() || event.msg_id() == 0 || event.metadata.is_history {
            self.aggregations
                .mark_gap(&context.unique_id, &context.room_id, "points.awarded");
            return;
        }
        let nickname = (!user.nickname.is_empty()).then(|| user.nickname.clone());
        let unique_id = crate::helpers::clean_unique_id(&user.unique_id);
        let metadata = tiktools_aggregation::EntryMetadata {
            label: nickname.clone().or_else(|| unique_id.clone()),
            user_id: (user.id != 0).then(|| user.id.to_string()),
            unique_id,
            nickname,
            ..Default::default()
        };
        let metrics = [("points.delta", delta.min(u64::MAX as f64) as u64)];
        self.aggregations.observe(
            tiktools_aggregation::model::Contribution {
                creator_id: &context.unique_id,
                room_id: &context.room_id,
                source: "points.awarded",
                user: Some(&key),
                gift: None,
                option: None,
                team: None,
                custom_key: None,
                metrics: &metrics,
            },
            Some(&metadata),
            None,
        );
    }
}
