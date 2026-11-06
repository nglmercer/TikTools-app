//! One bounded, ordered analytics owner. No worker retains AppCore.
use super::{write, AnalyticsEventRecord};
use crate::db::DatabaseManager;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};

const CAPACITY: usize = 1024;
const BATCH_SIZE: usize = 128;
const FLUSH_DEADLINE: Duration = Duration::from_secs(10);
const MAX_RECOVERY_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum AnalyticsWrite {
    Event {
        creator: String,
        record: AnalyticsEventRecord,
        at: i64,
    },
    Viewers {
        creator: String,
        viewers: i64,
        at: i64,
    },
    Session {
        creator: String,
        room: Option<String>,
        open: bool,
        at: i64,
    },
}

#[derive(Default)]
struct Counters {
    pending: AtomicU64,
    batches: AtomicU64,
    failures: AtomicU64,
    overload: AtomicU64,
}

#[derive(Serialize, Deserialize)]
struct RecoveryBatch {
    id: String,
    records: Vec<AnalyticsWrite>,
}

pub(crate) struct AnalyticsWriter {
    sender: mpsc::Sender<AnalyticsWrite>,
    ready: watch::Receiver<bool>,
    stop: Arc<AtomicBool>,
    counters: Arc<Counters>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl AnalyticsWriter {
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "capacity": CAPACITY,
            "batchCapacity": BATCH_SIZE,
            "queued": CAPACITY - self.sender.capacity(),
            "pending": self.counters.pending.load(Ordering::Acquire),
            "batches": self.counters.batches.load(Ordering::Acquire),
            "failures": self.counters.failures.load(Ordering::Acquire),
            "overload": self.counters.overload.load(Ordering::Acquire),
            "ready": *self.ready.borrow(),
            "stopping": self.stop.load(Ordering::Acquire),
        })
    }
    pub(crate) fn new(db: Arc<DatabaseManager>) -> Self {
        let (sender, receiver) = mpsc::channel(CAPACITY);
        let (ready_tx, ready) = watch::channel(false);
        let stop = Arc::new(AtomicBool::new(false));
        let counters = Arc::new(Counters::default());
        let worker_stop = Arc::clone(&stop);
        let worker_counters = Arc::clone(&counters);
        let worker = std::thread::Builder::new()
            .name("tiktools-analytics".into())
            .spawn(move || {
                run_writer(db, receiver, ready_tx, worker_stop, worker_counters);
            })
            .expect("analytics owner thread");
        Self {
            sender,
            ready,
            stop,
            counters,
            worker: Mutex::new(Some(worker)),
        }
    }

    pub(crate) async fn submit(&self, record: AnalyticsWrite) -> Result<(), String> {
        if self.stop.load(Ordering::Acquire) {
            return Err("analytics writer stopped".into());
        }
        let mut ready = self.ready.clone();
        while !*ready.borrow_and_update() {
            ready
                .changed()
                .await
                .map_err(|_| "analytics writer unavailable".to_owned())?;
        }
        if self.sender.capacity() == 0 {
            self.counters.overload.fetch_add(1, Ordering::Relaxed);
        }
        let permit = self
            .sender
            .reserve()
            .await
            .map_err(|_| "analytics writer stopped".to_owned())?;
        if self.stop.load(Ordering::Acquire) {
            return Err("analytics writer stopped".into());
        }
        self.counters.pending.fetch_add(1, Ordering::AcqRel);
        permit.send(record);
        Ok(())
    }

    pub(crate) async fn flush(&self) -> Result<(), String> {
        let start = Instant::now();
        while self.counters.pending.load(Ordering::Acquire) != 0 {
            if start.elapsed() >= FLUSH_DEADLINE {
                return Err(
                    "analytics flush deadline exceeded; writer retains pending records".into(),
                );
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }

    pub(crate) async fn shutdown(&self) -> bool {
        self.stop.store(true, Ordering::Release);
        let worker = crate::recover_mutex(&self.worker, "analytics worker").take();
        if let Some(worker) = worker {
            match tokio::task::spawn_blocking(move || worker.join()).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    tracing::error!("analytics owner panicked");
                    return false;
                }
                Err(error) => {
                    tracing::error!(%error, "analytics owner join failed");
                    return false;
                }
            }
        }
        true
    }
}

impl Drop for AnalyticsWriter {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

fn apply(
    connection: &mut rusqlite::Connection,
    batch: &RecoveryBatch,
    retain_id: bool,
) -> Result<(), crate::db::DatabaseError> {
    let transaction = connection.transaction()?;
    let fresh = transaction.execute(
        "INSERT OR IGNORE INTO analytics_writer_batches(id) VALUES (?1)",
        [&batch.id],
    )?;
    if fresh != 0 {
        for record in &batch.records {
            match record {
                AnalyticsWrite::Event {
                    creator,
                    record,
                    at,
                } => write::record_analytics_event(&transaction, creator, record, *at)?,
                AnalyticsWrite::Viewers {
                    creator,
                    viewers,
                    at,
                } => write::record_analytics_viewers(&transaction, creator, *viewers, *at)?,
                AnalyticsWrite::Session {
                    creator,
                    room,
                    open,
                    at,
                } => {
                    if *open {
                        write::open_live_session(&transaction, creator, room.as_deref(), *at)?;
                    } else {
                        write::close_live_sessions(&transaction, creator, *at)?;
                    }
                }
            }
        }
    }
    transaction.commit()?;
    if !retain_id {
        connection.execute(
            "DELETE FROM analytics_writer_batches WHERE id = ?1",
            [&batch.id],
        )?;
    }
    Ok(())
}

fn batch(records: Vec<AnalyticsWrite>) -> RecoveryBatch {
    RecoveryBatch {
        id: format!("{}-{}", std::process::id(), fastrand::u128(..)),
        records,
    }
}

fn run_writer(
    db: Arc<DatabaseManager>,
    mut receiver: mpsc::Receiver<AnalyticsWrite>,
    ready: watch::Sender<bool>,
    stop: Arc<AtomicBool>,
    counters: Arc<Counters>,
) {
    let recovery_path = db.analytics_path().with_extension("pending.json");
    let recovery: Vec<RecoveryBatch> = if recovery_path.exists() {
        let loaded = std::fs::metadata(&recovery_path)
            .map_err(|e| e.to_string())
            .and_then(|meta| {
                if meta.len() > MAX_RECOVERY_BYTES {
                    return Err("analytics recovery exceeds safety limit".into());
                }
                let bytes = std::fs::read(&recovery_path).map_err(|e| e.to_string())?;
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())
            });
        match loaded {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(%error, "analytics recovery requires repair; admission disabled");
                return;
            }
        }
    } else {
        Vec::new()
    };
    let mut connection = loop {
        let opened = rusqlite::Connection::open(db.analytics_path()).and_then(|connection| {
            connection.busy_timeout(Duration::from_secs(2))?;
            connection.execute_batch(
                "CREATE TABLE IF NOT EXISTS analytics_writer_batches(id TEXT PRIMARY KEY)",
            )?;
            Ok(connection)
        });
        match opened {
            Ok(connection) => break connection,
            Err(error) => {
                counters.failures.fetch_add(1, Ordering::Relaxed);
                tracing::error!(%error, "analytics database unavailable; admission waiting");
                if stop.load(Ordering::Acquire) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    };
    for pending in &recovery {
        loop {
            match apply(&mut connection, pending, true) {
                Ok(()) => break,
                Err(error) => {
                    counters.failures.fetch_add(1, Ordering::Relaxed);
                    tracing::error!(%error, "analytics recovery waiting for database; admission disabled");
                    if stop.load(Ordering::Acquire) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
    if !recovery.is_empty() {
        if let Err(error) = std::fs::remove_file(&recovery_path) {
            tracing::error!(%error, "could not retire analytics recovery file; admission disabled");
            return;
        }
    }
    if let Err(error) = connection.execute("DELETE FROM analytics_writer_batches", []) {
        tracing::error!(%error, "analytics recovery bookkeeping cleanup failed");
        return;
    }
    let _ = ready.send(true);
    let mut stopping_at = None;
    loop {
        if stop.load(Ordering::Acquire) {
            receiver.close();
            stopping_at.get_or_insert_with(Instant::now);
        }
        let first = match receiver.try_recv() {
            Ok(record) => record,
            Err(mpsc::error::TryRecvError::Disconnected) => break,
            Err(mpsc::error::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            }
        };
        let mut records = vec![first];
        while records.len() < BATCH_SIZE {
            match receiver.try_recv() {
                Ok(record) => records.push(record),
                Err(_) => break,
            }
        }
        let pending = batch(records);
        loop {
            match apply(&mut connection, &pending, false) {
                Ok(()) => {
                    counters
                        .pending
                        .fetch_sub(pending.records.len() as u64, Ordering::AcqRel);
                    counters.batches.fetch_add(1, Ordering::Relaxed);
                    break;
                }
                Err(error) => {
                    counters.failures.fetch_add(1, Ordering::Relaxed);
                    tracing::error!(%error, "analytics batch failed; retaining records and applying backpressure");
                    if stop.load(Ordering::Acquire) {
                        receiver.close();
                        let began = stopping_at.get_or_insert_with(Instant::now);
                        if began.elapsed() >= FLUSH_DEADLINE {
                            let mut remainder = vec![pending];
                            let mut queued = Vec::new();
                            while let Ok(record) = receiver.try_recv() {
                                queued.push(record);
                            }
                            if !queued.is_empty() {
                                remainder.push(batch(queued));
                            }
                            let saved = serde_json::to_vec(&remainder)
                                .map_err(|e| e.to_string())
                                .and_then(|bytes| {
                                    if bytes.len() as u64 > MAX_RECOVERY_BYTES {
                                        return Err(
                                            "analytics recovery exceeds safety limit".into()
                                        );
                                    }
                                    let temporary = recovery_path.with_extension("pending.tmp");
                                    use std::io::Write;
                                    let mut file = std::fs::File::create(&temporary)
                                        .map_err(|e| e.to_string())?;
                                    file.write_all(&bytes)
                                        .and_then(|()| file.sync_all())
                                        .map_err(|e| e.to_string())?;
                                    std::fs::rename(temporary, &recovery_path)
                                        .map_err(|e| e.to_string())
                                });
                            match saved {
                                Ok(()) => tracing::error!(
                                    pending = counters.pending.load(Ordering::Acquire),
                                    "analytics flush deadline reached; records saved for recovery"
                                ),
                                Err(error) => {
                                    tracing::error!(%error, pending = counters.pending.load(Ordering::Acquire), "analytics recovery write failed; records could not be preserved")
                                }
                            }
                            return;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chat(creator: &str) -> AnalyticsWrite {
        AnalyticsWrite::Event {
            creator: creator.into(),
            record: AnalyticsEventRecord {
                unique_id: "viewer".into(),
                chats: 1,
                likes: 3,
                ..Default::default()
            },
            at: 86_400,
        }
    }

    async fn wait_ready(writer: &AnalyticsWriter) {
        let mut ready = writer.ready.clone();
        while !*ready.borrow_and_update() {
            ready.changed().await.unwrap();
        }
    }

    #[cfg(all(feature = "native-tiktok", target_os = "linux"))]
    #[tokio::test]
    #[ignore = "opt-in Linux RSS stress profiling; run alone with --nocapture"]
    async fn synthetic_canonical_analytics_memory_cycles() {
        fn rss_kib() -> u64 {
            std::fs::read_to_string("/proc/self/status")
                .unwrap()
                .lines()
                .find(|line| line.starts_with("VmRSS:"))
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse()
                .unwrap()
        }
        let db = Arc::new(super::super::tests::test_manager());
        let event = crate::NativeLiveEvent {
            base: tiktools_tiktok::events::CanonicalLiveEvent::Chat(
                tiktools_tiktok::events::ChatEvent {
                    user: tiktools_tiktok::events::EventUser {
                        id: 1,
                        unique_id: "synthetic".into(),
                        nickname: "Synthetic".into(),
                        sec_uid: String::new(),
                        avatar_url: None,
                    },
                    comment: "synthetic fixture".into(),
                    ..Default::default()
                },
            ),
            metadata: tiktools_tiktok::events::EventMetadata {
                method: "WebcastChatMessage".into(),
                msg_id: 1,
                is_history: false,
            },
            gift: None,
        };
        let record = AnalyticsEventRecord::from_canonical_event(&event).unwrap();
        let mut warmed_rss = 0;
        println!("cycle,rss_kib,pending,batches");
        for cycle in 0..24 {
            let writer = AnalyticsWriter::new(Arc::clone(&db));
            for _ in 0..4096 {
                writer
                    .submit(AnalyticsWrite::Event {
                        creator: "synthetic".into(),
                        record: record.clone(),
                        at: 86400,
                    })
                    .await
                    .unwrap();
            }
            writer.shutdown().await;
            assert_eq!(writer.counters.pending.load(Ordering::Acquire), 0);
            let batches = writer.counters.batches.load(Ordering::Acquire);
            drop(writer);
            let rss = rss_kib();
            println!("{cycle},{rss},0,{batches}");
            if cycle == 3 {
                warmed_rss = rss;
            }
            if cycle > 3 {
                assert!(
                    rss <= warmed_rss + 16 * 1024,
                    "persistent RSS growth exceeds 16 MiB allowance"
                );
            }
        }
        assert_eq!(
            db.analytics_summary("synthetic", 1, 1, 10, 0)
                .unwrap()
                .totals
                .chats,
            24 * 4096
        );
    }

    #[tokio::test]
    async fn sustained_batched_traffic_flushes_exact_isolated_totals_on_shutdown() {
        let db = Arc::new(super::super::tests::test_manager());
        let writer = AnalyticsWriter::new(Arc::clone(&db));
        for _ in 0..4096 {
            writer.submit(chat("one")).await.unwrap();
            writer.submit(chat("two")).await.unwrap();
        }
        writer.shutdown().await;
        assert_eq!(writer.counters.pending.load(Ordering::Acquire), 0);
        for creator in ["one", "two"] {
            let summary = db.analytics_summary(creator, 1, 1, 10, 0).unwrap();
            assert_eq!(summary.totals.chats, 4096);
            assert_eq!(summary.totals.likes, 4096 * 3);
        }
        assert!(writer.counters.batches.load(Ordering::Acquire) < 8192);
        assert!(writer.submit(chat("one")).await.is_err());
        let diagnostics = writer.diagnostics();
        assert_eq!(diagnostics["pending"], 0);
        assert_eq!(diagnostics["queued"], 0);
        assert_eq!(diagnostics["capacity"], CAPACITY);
        assert_eq!(diagnostics["stopping"], true);
    }

    #[tokio::test]
    async fn locked_database_applies_bounded_backpressure_without_losing_accepted_records() {
        let db = Arc::new(super::super::tests::test_manager());
        let writer = AnalyticsWriter::new(Arc::clone(&db));
        wait_ready(&writer).await;
        let lock = rusqlite::Connection::open(db.analytics_path()).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let mut accepted = 0;
        for _ in 0..CAPACITY + BATCH_SIZE + 1 {
            if tokio::time::timeout(Duration::from_millis(100), writer.submit(chat("one")))
                .await
                .is_err()
            {
                break;
            }
            accepted += 1;
        }
        assert!(accepted <= CAPACITY + BATCH_SIZE);
        assert_eq!(
            writer.counters.pending.load(Ordering::Acquire),
            accepted as u64
        );
        assert!(writer.counters.overload.load(Ordering::Acquire) > 0);
        lock.execute_batch("COMMIT").unwrap();
        writer.shutdown().await;
        assert_eq!(writer.counters.pending.load(Ordering::Acquire), 0);
        assert_eq!(
            db.analytics_summary("one", 1, 1, 10, 0)
                .unwrap()
                .totals
                .chats,
            accepted as i64
        );
    }

    #[tokio::test]
    async fn shutdown_with_locked_database_saves_and_recovers_accepted_records() {
        let db = Arc::new(super::super::tests::test_manager());
        let writer = AnalyticsWriter::new(Arc::clone(&db));
        wait_ready(&writer).await;
        let lock = rusqlite::Connection::open(db.analytics_path()).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();
        for _ in 0..32 {
            writer.submit(chat("one")).await.unwrap();
        }
        writer.shutdown().await;
        let path = db.analytics_path().with_extension("pending.json");
        assert!(path.exists());
        lock.execute_batch("COMMIT").unwrap();
        let recovered = AnalyticsWriter::new(Arc::clone(&db));
        wait_ready(&recovered).await;
        recovered.shutdown().await;
        assert!(!path.exists());
        assert_eq!(
            db.analytics_summary("one", 1, 1, 10, 0)
                .unwrap()
                .totals
                .chats,
            32
        );
    }

    #[tokio::test]
    async fn recovery_replays_once_even_when_a_previous_recovery_commit_already_succeeded() {
        let db = Arc::new(super::super::tests::test_manager());
        let pending = batch(vec![chat("one"), chat("one")]);
        let mut connection = rusqlite::Connection::open(db.analytics_path()).unwrap();
        connection
            .execute_batch("CREATE TABLE analytics_writer_batches(id TEXT PRIMARY KEY)")
            .unwrap();
        apply(&mut connection, &pending, true).unwrap();
        let path = db.analytics_path().with_extension("pending.json");
        std::fs::write(&path, serde_json::to_vec(&vec![pending]).unwrap()).unwrap();
        drop(connection);
        let writer = AnalyticsWriter::new(Arc::clone(&db));
        wait_ready(&writer).await;
        writer.submit(chat("one")).await.unwrap();
        writer.shutdown().await;
        assert!(!path.exists());
        assert_eq!(
            db.analytics_summary("one", 1, 1, 10, 0)
                .unwrap()
                .totals
                .chats,
            3
        );
    }
}
