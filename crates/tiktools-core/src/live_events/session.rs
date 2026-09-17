use crate::*;

impl AppCore {
    /// Fire-and-forget analytics write. Slow or failed writes only log; live
    /// delivery never waits for the analytics database.
    #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
    pub(crate) fn record_analytics_event(self: &Arc<Self>, event: &NativeLiveEvent) {
        use crate::db::AnalyticsEventRecord;

        let Some(record) = AnalyticsEventRecord::from_canonical_event(event) else {
            return;
        };
        let Some(creator) = self.current_creator_unique_id() else {
            return;
        };
        self.publish_analytics_updated(&creator);
        let db = std::sync::Arc::clone(&self.db);
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);
        tokio::spawn(async move {
            let outcome = tokio::task::spawn_blocking(move || {
                db.record_analytics_event(&creator, &record, now_unix)
            })
            .await;
            if let Err(error) = outcome {
                tracing::warn!(%error, "analytics event write failed");
            } else if let Ok(Err(error)) = outcome {
                tracing::warn!(%error, "analytics event write failed");
            }
        });
    }
    #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
    pub(crate) fn record_analytics_viewers(self: &Arc<Self>, viewers: u64) {
        let Some(creator) = self.current_creator_unique_id() else {
            return;
        };
        self.publish_analytics_updated(&creator);
        let db = std::sync::Arc::clone(&self.db);
        let viewers = i64::try_from(viewers).unwrap_or(i64::MAX);
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);
        tokio::spawn(async move {
            let outcome = tokio::task::spawn_blocking(move || {
                db.record_analytics_viewers(&creator, viewers, now_unix)
            })
            .await;
            if let Err(error) = outcome {
                tracing::warn!(%error, "analytics viewers write failed");
            } else if let Ok(Err(error)) = outcome {
                tracing::warn!(%error, "analytics viewers write failed");
            }
        });
    }
    /// Publishes `analytics.updated` at a bounded rate. Live events are
    /// high-rate; subscribers only need a periodic refresh signal.
    #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
    pub(crate) fn publish_analytics_updated(&self, creator: &str) {
        const MIN_INTERVAL_MS: u64 = 5_000;
        let now = crate::helpers::now_millis();
        let last = self
            .last_analytics_emit_at
            .load(std::sync::atomic::Ordering::Acquire);
        if last != 0 && now.saturating_sub(last) < MIN_INTERVAL_MS {
            return;
        }
        if self
            .last_analytics_emit_at
            .compare_exchange(
                last,
                now,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
        {
            self.events.publish_domain(
                crate::events::DomainEvent::AnalyticsUpdated {
                    creator_unique_id: creator.to_owned(),
                },
            );
        }
    }
    #[cfg(feature = "persistence")]
    pub(crate) fn write_live_session(&self, creator: &str, room_id: Option<&str>, open: bool) {
        let db = std::sync::Arc::clone(&self.db);
        let creator = creator.to_owned();
        let room_id = room_id.map(str::to_owned);
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);
        tokio::spawn(async move {
            let outcome = tokio::task::spawn_blocking(move || {
                if open {
                    db.open_live_session(&creator, room_id.as_deref(), now_unix)
                } else {
                    db.close_live_sessions(&creator, now_unix)
                }
            })
            .await;
            if let Err(error) = outcome {
                tracing::warn!(%error, "live session write failed");
            } else if let Ok(Err(error)) = outcome {
                tracing::warn!(%error, "live session write failed");
            }
        });
    }
}
