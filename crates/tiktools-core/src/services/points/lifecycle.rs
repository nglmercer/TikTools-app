use super::*;

impl Default for PointsService {
    fn default() -> Self {
        Self {
            mutation: Mutex::new(()),
            config: RwLock::new(PointsConfig::default()),
            inner: RwLock::new(PointsInner::with_capacity(DEFAULT_HOT_VIEWER_CAPACITY)),
            next_op: AtomicU64::new(1),
            #[cfg(feature = "persistence")]
            database: None,
            #[cfg(feature = "persistence")]
            writer: Arc::new(PointsWriterShared::new(false)),
            #[cfg(feature = "persistence")]
            writer_thread: Mutex::new(None),
            #[cfg(all(test, feature = "persistence"))]
            before_submit: Mutex::new(None),
            #[cfg(all(test, feature = "persistence"))]
            after_command_admission: Mutex::new(None),
        }
    }
}

impl PointsService {
    pub fn new(database: Arc<DatabaseManager>) -> Self {
        let capacity = std::env::var("TIKTOOLS_POINTS_CACHE_CAPACITY")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_HOT_VIEWER_CAPACITY);
        let service = Self {
            mutation: Mutex::new(()),
            config: RwLock::new(PointsConfig::default()),
            inner: RwLock::new(PointsInner::with_capacity(capacity)),
            next_op: AtomicU64::new(1),
            #[cfg(feature = "persistence")]
            database: None,
            #[cfg(feature = "persistence")]
            writer: Arc::new(PointsWriterShared::new(false)),
            #[cfg(feature = "persistence")]
            writer_thread: Mutex::new(None),
            #[cfg(all(test, feature = "persistence"))]
            before_submit: Mutex::new(None),
            #[cfg(all(test, feature = "persistence"))]
            after_command_admission: Mutex::new(None),
        };
        #[cfg(feature = "persistence")]
        {
            // These locks were just created by construction above and are
            // still thread-local to this constructor, so poisoning is
            // impossible: expect pins that invariant.
            if let Ok(mut config) = database.load_points_config() {
                config.normalize();
                *service.config.write().expect("points config poisoned") = config;
            }
            // Warm only the leaderboard head: the hot cache is bounded, and
            // the database stays the source of truth for everyone else.
            // Cold viewers reload on demand through the indexed primary key.
            match database.load_top_viewers(capacity) {
                Ok(viewers) => {
                    let mut inner = service.inner.write().expect("points viewers poisoned");
                    for viewer in viewers.into_iter().filter_map(|value| {
                        ViewerState::from_value(&value)
                            .map(|state| (state.unique_id.clone(), state))
                    }) {
                        inner.viewers.insert(viewer.0, viewer.1);
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "could not warm points hot cache; cold viewers load on demand");
                }
            }
            let shared = Arc::new(PointsWriterShared::new(true));
            let thread = std::thread::Builder::new()
                .name("points-writer".to_owned())
                .spawn({
                    let database = Arc::clone(&database);
                    let shared = Arc::clone(&shared);
                    move || run_points_writer(database, shared)
                })
                .ok();
            if thread.is_none() {
                // No thread means nothing will ever drain the queue: close
                // admission now so awards reject loudly instead of queueing
                // writes nobody applies.
                tracing::error!("points writer thread failed to start; durable writes are rejected until it is restarted");
                let mut state = shared.lock_queue();
                state.closed = true;
            }
            *service
                .writer_thread
                .lock()
                .expect("points writer thread poisoned") = thread;
            // Keep the connection boundary in the service while the writer
            // owns its long-lived connection.
            let mut service = service;
            service.database = Some(database);
            service.writer = shared;
            service
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = database;
            service
        }
    }

    /// Closes admission, joins the writer thread and flushes every admitted
    /// op. Ordinary shutdown never loses accepted-intent work: when this
    /// returns the queue is empty and every admitted sequence is resolved.
    /// Safe to call more than once.
    ///
    /// Bounded by construction: closing admission lets the writer drain and
    /// exit, and a writer with no connection drains without applying.
    /// [`AppCore::shutdown`](crate::AppCore::shutdown) still wraps this in
    /// its own deadline and reports a stuck writer explicitly.
    pub fn shutdown(&self) {
        let _mutation = recover_mutex(&self.mutation, "points mutation");
        #[cfg(feature = "persistence")]
        {
            {
                let mut state = self.writer.lock_queue();
                state.closed = true;
            }
            self.writer.wake.notify_all();
            let thread = recover_mutex(&self.writer_thread, "points writer thread").take();
            if let Some(thread) = thread {
                if let Err(error) = thread.join() {
                    tracing::warn!(?error, "points writer thread panicked during shutdown");
                }
            }
            // Post-shutdown reloads must never stall on the wait above: the
            // writer drained, so every admitted sequence is resolved. This
            // never stores a sentinel maximum — a permanently failed write
            // stays visible as a gap between `committed_seq` and
            // `resolved_seq` instead of being papered over.
            let admitted = self.writer.admitted.load(Ordering::Acquire);
            self.writer
                .resolved_seq
                .fetch_max(admitted, Ordering::Release);
        }
    }
}
