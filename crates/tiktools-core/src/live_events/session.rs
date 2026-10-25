use crate::*;

impl AppCore {
    #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
    pub(crate) async fn record_analytics_event(
        self: &Arc<Self>,
        event: &NativeLiveEvent,
        totals: &Value,
        creator: Option<String>,
    ) {
        let Some(mut record) = crate::db::AnalyticsEventRecord::from_canonical_event(event) else {
            return;
        };
        if matches!(
            event.base,
            tiktools_tiktok::events::CanonicalLiveEvent::Gift(_)
        ) {
            record.gifts =
                i64::try_from(totals["increment"].as_u64().unwrap_or(0)).unwrap_or(i64::MAX);
            record.diamonds =
                i64::try_from(totals["diamonds"].as_u64().unwrap_or(0)).unwrap_or(i64::MAX);
        }
        let Some(creator) = creator else {
            return;
        };
        self.submit_analytics(crate::db::AnalyticsWrite::Event {
            creator: creator.clone(),
            record,
            at: analytics_now(),
        })
        .await;
        self.publish_analytics_updated(&creator);
    }

    #[cfg(all(feature = "persistence", feature = "native-tiktok"))]
    pub(crate) async fn record_analytics_viewers_for(
        self: &Arc<Self>,
        viewers: u64,
        creator: Option<String>,
    ) {
        let Some(creator) = creator else {
            return;
        };
        self.submit_analytics(crate::db::AnalyticsWrite::Viewers {
            creator: creator.clone(),
            viewers: i64::try_from(viewers).unwrap_or(i64::MAX),
            at: analytics_now(),
        })
        .await;
        self.publish_analytics_updated(&creator);
    }

    #[cfg(feature = "persistence")]
    async fn submit_analytics(&self, record: crate::db::AnalyticsWrite) {
        if let Err(error) = self.analytics_writer.submit(record).await {
            tracing::error!(%error, "analytics record was not admitted");
            self.record_event_gap();
        }
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
            self.events
                .publish_domain(crate::events::DomainEvent::AnalyticsUpdated {
                    creator_unique_id: creator.to_owned(),
                });
        }
    }
    #[cfg(feature = "persistence")]
    pub(crate) async fn write_live_session(
        &self,
        creator: &str,
        room_id: Option<&str>,
        open: bool,
    ) {
        let record = crate::db::AnalyticsWrite::Session {
            creator: creator.to_owned(),
            room: room_id.map(str::to_owned),
            open,
            at: analytics_now(),
        };
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.analytics_writer.submit(record),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::error!(%error, "analytics session write not admitted");
                self.record_event_gap();
            }
            Err(_) => {
                tracing::error!(
                    "analytics session admission deadline exceeded; session state incomplete"
                );
                self.record_event_gap();
            }
        }
        if !open {
            if let Err(error) = self.analytics_writer.flush().await {
                tracing::error!(%error, "analytics disconnect flush incomplete");
            }
        }
    }
}

#[cfg(feature = "persistence")]
fn analytics_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
