//! Ordered shutdown, durable writer draining and update-safety reporting.
use super::*;

impl AppCore {
    pub async fn shutdown(self: &Arc<Self>) {
        if self.shutdown_started.swap(true, Ordering::AcqRel) {
            loop {
                let finished = self.shutdown_complete.notified();
                tokio::pin!(finished);
                finished.as_mut().enable();
                if self.shutdown_finished.load(Ordering::Acquire) {
                    return;
                }
                finished.await;
            }
        }
        self.plugins.close_start_admission();
        self.automation_write_settled.notify_waiters();
        #[cfg(feature = "persistence")]
        {
            #[cfg(feature = "cloud-auth")]
            self.stop_cloud_sync_worker().await;
            self.stop_coordination().await;
            self.stop_competition_action_worker().await;
        }
        {
            let _operation = self.live_monitor.operation.lock().await;
            self.stop_live_monitor().await;
        }
        // Keep a notification queued if the polling task is in a plugin call
        // when shutdown begins; `notify_waiters` alone would be lost before
        // the task reaches its select point.
        self.plugin_state.poll_shutdown.notify_one();
        // LIVE ingestion stops first and in its own order: admission closes,
        // the pump exits (dropping the queue sender), and the worker drains
        // every event already accepted before it is joined. The observer
        // token stays open during that drain so drained events keep their
        // automation side effects instead of being silently dropped here.
        #[cfg(feature = "native-tiktok")]
        self.stop_live_ingress().await;
        // Persistent cancellation: every supervisor and worker observes
        // shutdown no matter when it reaches its select point, and no
        // worker can steal the supervisor's wakeup.
        self.plugin_state.observer_shutdown.cancel();
        self.events
            .publish_domain(crate::events::DomainEvent::Shutdown);
        self.publish_disconnected_event().await;
        self.live.disconnect().await;
        let task = recover_mutex(&self.plugin_state.poll_task, "plugin poll task").take();
        if let Some(mut task) = task {
            match tokio::time::timeout(std::time::Duration::from_secs(2), &mut task).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    self.shutdown_update_safe.store(false, Ordering::Release);
                    tracing::error!(%error, "plugin poll task failed; application update is unsafe");
                }
                Err(_) => {
                    task.abort();
                    self.shutdown_update_safe.store(false, Ordering::Release);
                    tracing::error!(
                        "plugin poll did not stop within two seconds; application update is unsafe"
                    );
                }
            }
        }
        let push_task = recover_mutex(&self.plugin_state.push_task, "plugin push task").take();
        if let Some(task) = push_task {
            let _ = task.await;
        }
        let observer_task = recover_mutex(
            &self.plugin_state.observer_task,
            "plugin event observer task",
        )
        .take();
        if let Some(task) = observer_task {
            let _ = task.await;
        }
        if !self.plugins.stop_all_checked() {
            self.shutdown_update_safe.store(false, Ordering::Release);
            tracing::error!("plugin shutdown was incomplete; application update is unsafe");
        }
        #[cfg(feature = "persistence")]
        if !self.analytics_writer.shutdown().await {
            self.shutdown_update_safe.store(false, Ordering::Release);
        }
        // Join the owned points writer last: every earlier shutdown step may
        // still award points (disconnect handling included), and accepted
        // writes must reach SQLite before the process exits.
        #[cfg(feature = "native-tiktok")]
        if !self.live_points_work.shutdown().await {
            self.shutdown_update_safe.store(false, Ordering::Release);
        }
        // Bounded: a wedged SQLite must not block process shutdown
        // forever. The deadline is an explicit failure policy — the error
        // below is the report that durable work may remain unresolved, and
        // the writer thread is joined (or abandoned) with that on record.
        let points = Arc::clone(&self.points);
        let flush = tokio::task::spawn_blocking(move || points.shutdown());
        match tokio::time::timeout(std::time::Duration::from_secs(30), flush).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                self.shutdown_update_safe.store(false, Ordering::Release);
                tracing::error!(%error, "points writer shutdown failed");
            }
            Err(_) => {
                self.shutdown_update_safe.store(false, Ordering::Release);
                tracing::error!("points writer failed to stop within 30s; durable points work may remain unresolved");
            }
        }
        #[cfg(feature = "persistence")]
        {
            let core = Arc::clone(self);
            let checkpoint = tokio::task::spawn_blocking(move || {
                services::competitions::CompetitionService::checkpoint_shutdown(&core)?;
                services::aggregations::AggregationService::checkpoint_shutdown(&core)
            });
            // The checkpoint owner fences a late write after a newer host starts.
            match tokio::time::timeout(std::time::Duration::from_secs(5), checkpoint).await {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(error))) => {
                    self.shutdown_update_safe.store(false, Ordering::Release);
                    tracing::error!(%error, "ranking or competition checkpoint failed")
                }
                Ok(Err(error)) => {
                    self.shutdown_update_safe.store(false, Ordering::Release);
                    tracing::error!(%error, "checkpoint worker failed")
                }
                Err(_) => {
                    self.shutdown_update_safe.store(false, Ordering::Release);
                    tracing::error!("checkpoint timed out; restart may be degraded");
                }
            }
        }
        self.shutdown_finished.store(true, Ordering::Release);
        self.shutdown_complete.notify_waiters();
    }

    /// Updates must not proceed if shutdown left plugin or durable work unresolved.
    pub fn update_shutdown_safe(&self) -> bool {
        self.shutdown_finished.load(Ordering::Acquire)
            && self.shutdown_update_safe.load(Ordering::Acquire)
    }
}
