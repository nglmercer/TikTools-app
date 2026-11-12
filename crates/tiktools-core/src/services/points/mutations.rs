use super::*;

impl PointsService {
    pub fn reset(&self, unique_id: Option<&str>) {
        self.reset_admitted(unique_id);
    }

    /// The owner emits a reset event only after this admission succeeds.
    /// Retain the historical public `reset` signature for service callers.
    pub(crate) fn reset_admitted(&self, unique_id: Option<&str>) -> bool {
        let _mutation = recover_mutex(&self.mutation, "points mutation");
        let unique_id = match unique_id {
            Some(id) => match clean_unique_id(id) {
                Some(id) => Some(id),
                // Invalid single-viewer input must never become a full reset.
                None => return false,
            },
            None => None,
        };
        // Admission first: a rejected reset must not zero memory the durable
        // store will never match.
        if !self.wait_write_capacity() {
            return false;
        }
        let resolved = self.resolved_seq();
        let mut inner = recover_rwlock_write(&self.inner, "points viewers");
        #[cfg(feature = "persistence")]
        let seq = self.next_seq();
        let PointsInner {
            viewers,
            #[cfg(feature = "persistence")]
            pending,
            #[cfg(feature = "persistence")]
            reset_pending,
            ..
        } = &mut *inner;
        match unique_id.as_deref() {
            Some(unique_id) => {
                if let Some(viewer) = viewers.get_mut(unique_id) {
                    viewer.points = 0.0;
                    viewer.level = 1;
                    #[cfg(feature = "persistence")]
                    if self.database.is_some() {
                        pending.insert(unique_id.to_owned(), seq);
                    }
                }
                // A cold (evicted or never-cached) viewer is already reset
                // durably below; nothing is fabricated in memory.
                #[cfg(feature = "persistence")]
                self.submit_points_op(PointsWriteOp::Reset {
                    seq,
                    unique_id: Some(unique_id.to_owned()),
                });
            }
            None => {
                for viewer in viewers.values_mut() {
                    viewer.points = 0.0;
                    viewer.level = 1;
                }
                // Full reset: markers retire when the reset op resolves (it
                // zeroes every row and is queued behind all earlier writes),
                // so nothing is cleared here. Clearing early would let a
                // concurrent eviction reload a row the reset is about to
                // zero, and the reload would win the FIFO race. The reset
                // sequence itself gates every cold reload until it lands.
                #[cfg(feature = "persistence")]
                if self.database.is_some() {
                    *reset_pending = seq;
                }
                #[cfg(feature = "persistence")]
                self.submit_points_op(PointsWriteOp::Reset {
                    seq,
                    unique_id: None,
                });
            }
        }
        inner.evict_cold(resolved);
        true
    }

    pub fn adjust(&self, unique_id: &str, delta: f64) -> Option<PointAward> {
        let unique_id = clean_unique_id(unique_id)?;
        if !delta.is_finite() {
            return None;
        }
        let _mutation = recover_mutex(&self.mutation, "points mutation");
        // A bounded cache means the viewer may live only in SQLite: reload
        // through the indexed key instead of reporting a durable viewer as
        // unknown.
        let cached = {
            recover_rwlock_read(&self.inner, "points viewers")
                .viewers
                .contains_key(&unique_id)
        };
        if !cached {
            #[cfg(feature = "persistence")]
            if let Some(loaded) = {
                if !self.wait_for_viewer_resolved(&unique_id) {
                    return None;
                }
                self.database
                    .as_ref()
                    .and_then(|database| database.load_viewer_by_id(&unique_id).ok().flatten())
                    .and_then(|value| ViewerState::from_value(&value))
            } {
                let mut inner = recover_rwlock_write(&self.inner, "points viewers");
                // Conditional promotion only: no eviction yet, for the same
                // stale-timestamp reason as in `award_points`. The update
                // below refreshes the row and enforces the bound.
                inner.viewers.entry(unique_id.clone()).or_insert(loaded);
            }
        }
        // Read config outside the viewer lock: one lock at a time.
        let config = self.config();
        let (award_value, seq) = {
            let mut inner = recover_rwlock_write(&self.inner, "points viewers");
            let viewer = inner.viewers.get(&unique_id)?;
            let points = round_points((viewer.points + delta).max(0.0));
            if !points.is_finite() {
                return None;
            }
            let level =
                ((points / config.points_per_level.max(10.0)).floor() as u32).saturating_add(1);
            // Reserve before mutating: a rejected admission leaves the row
            // untouched instead of diverging from the durable store.
            if !self.wait_write_capacity() {
                return None;
            }
            let seq = self.next_seq();
            let viewer = inner
                .viewers
                .get_mut(&unique_id)
                .expect("viewer checked above the admission wait");
            viewer.points = points;
            viewer.level = level;
            viewer.last_seen = now_millis();
            self.track_pending(&mut inner, &unique_id, seq);
            // Bound enforced after the refresh (amortized), never on a
            // stale timestamp.
            let resolved = self.resolved_seq();
            inner.evict_if_over(resolved);
            (
                PointAward {
                    unique_id: unique_id.clone(),
                    delta,
                    total_points: points,
                    level,
                    currency_name: config.currency_name.clone(),
                },
                seq,
            )
        };
        #[cfg(feature = "persistence")]
        self.submit_points_op(PointsWriteOp::Adjust {
            seq,
            unique_id: award_value.unique_id.clone(),
            points: award_value.total_points,
            level: award_value.level,
        });
        #[cfg(not(feature = "persistence"))]
        let _ = seq;
        Some(award_value)
    }
}
