use super::*;

impl PointsService {
    pub fn award_points(
        &self,
        unique_id: &str,
        action: PointAction,
        options: AwardOptions,
    ) -> Option<PointAward> {
        let unique_id = clean_unique_id(unique_id)?;
        let _mutation = recover_mutex(&self.mutation, "points mutation");
        let config = self.config();
        let base_points = match action {
            PointAction::Manual => options.custom_amount.unwrap_or_default(),
            PointAction::Chat if config.points_per_chat_enabled => config.points_per_chat,
            PointAction::Gift if config.points_per_coin_enabled => {
                nonnegative_or_one(options.diamond_count.or(options.count)) * config.points_per_coin
            }
            PointAction::Like if config.points_per_like_enabled => {
                nonnegative_or_one(options.count) * config.points_per_like
            }
            PointAction::Share if config.points_per_share_enabled => config.points_per_share,
            PointAction::Follow if config.points_per_follow_enabled => config.points_per_follow,
            PointAction::Join if config.points_per_join_enabled => config.points_per_join,
            _ => 0.0,
        };
        if !base_points.is_finite() {
            return None;
        }

        // O(1) lookup; on a cold miss fall back to one indexed SQLite read
        // (outside the lock) and promote the row. Unknown viewers are
        // created, exactly as before.
        let is_subscriber_hint = options.is_subscriber;
        let now = now_millis();
        let state_snapshot = {
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            let cached = inner.viewers.contains_key(&unique_id);
            if cached {
                inner.hits += 1;
            } else {
                inner.misses += 1;
            }
            cached
        };
        if !state_snapshot {
            #[cfg(feature = "persistence")]
            if let Some(loaded) = {
                // Never base an award on state older than our own queued work.
                if !self.wait_for_viewer_resolved(&unique_id) {
                    return None;
                }
                self.database
                    .as_ref()
                    .and_then(|database| database.load_viewer_by_id(&unique_id).ok().flatten())
                    .and_then(|value| ViewerState::from_value(&value))
            } {
                let mut inner = recover_rwlock_write(&self.inner, "points viewers");
                // Conditional promotion: a concurrent award may have
                // created a newer row while the database read was in
                // flight. The update below always accumulates on top of
                // whatever is resident, so no increment is ever lost.
                // No eviction here: the promoted row still carries its
                // durable timestamp and would be misjudged as cold. The
                // update section refreshes it and enforces the bound.
                inner.viewers.entry(unique_id.clone()).or_insert(loaded);
            }
        }

        let (award, persisted) = {
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            let existing_subscriber = inner
                .viewers
                .get(&unique_id)
                .map(|viewer| viewer.is_subscriber)
                .unwrap_or(false);
            let is_subscriber = is_subscriber_hint.unwrap_or(existing_subscriber);
            let mut awarded = base_points;
            if is_subscriber && config.sub_bonus_multiplier > 0.0 && awarded > 0.0 {
                awarded += awarded * (config.sub_bonus_multiplier / 100.0);
            }
            awarded = round_points(awarded);
            let current_points = inner
                .viewers
                .get(&unique_id)
                .map(|viewer| viewer.points)
                .unwrap_or_default();
            let total_points = round_points((current_points + awarded).max(0.0));
            if !awarded.is_finite() || !total_points.is_finite() {
                return None;
            }
            let points_per_level = config.points_per_level.max(10.0);
            let level = ((total_points / points_per_level).floor() as u32).saturating_add(1);

            // Reserve durable-queue space *before* mutating anything. A
            // rejected admission must leave memory exactly as it was, so the
            // hot cache never claims an award SQLite will never hold. This
            // wait also pins sequence order to queue order: the mutation
            // lock remains held until the resulting snapshot is enqueued.
            if !self.wait_write_capacity() {
                return None;
            }
            let seq = self.next_seq();
            let snapshot = {
                let state = inner
                    .viewers
                    .entry(unique_id.clone())
                    .or_insert_with(|| new_viewer_state(&unique_id, &options, now));
                update_existing_viewer(
                    state,
                    &options,
                    total_points,
                    level,
                    is_subscriber,
                    action,
                    now,
                );
                state.clone()
            };
            self.track_pending(&mut inner, &unique_id, seq);
            // Enforce the memory bound on the insert path only, amortized:
            // steady-state updates never scan. Rows with unresolved writes
            // are exempt from eviction.
            let resolved = self.resolved_seq();
            inner.evict_if_over(resolved);
            (
                PointAward {
                    unique_id,
                    delta: awarded,
                    total_points,
                    level,
                    currency_name: config.currency_name,
                },
                (seq, snapshot),
            )
        };

        #[cfg(feature = "persistence")]
        {
            #[cfg(test)]
            self.pause_before_submit();
            let (seq, snapshot) = persisted;
            self.submit_points_op(PointsWriteOp::Save {
                seq,
                viewer: snapshot,
                reason: action.as_str(),
                awarded: award.delta,
            });
        }
        #[cfg(not(feature = "persistence"))]
        let _ = persisted;

        Some(award)
    }
}
