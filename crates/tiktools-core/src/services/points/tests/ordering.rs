use super::*;
use std::sync::mpsc;

fn paused_award(
    service: &Arc<PointsService>,
) -> (mpsc::SyncSender<()>, std::thread::JoinHandle<()>) {
    let (reached, observed) = mpsc::sync_channel(0);
    let (resume, proceed) = mpsc::sync_channel(0);
    *service.before_submit.lock().unwrap() = Some(AdmissionPause {
        reached,
        resume: proceed,
    });
    let producer = Arc::clone(service);
    let thread = std::thread::spawn(move || {
        producer
            .award_points("racer", PointAction::Chat, AwardOptions::default())
            .expect("admitted award");
    });
    observed
        .recv_timeout(Duration::from_secs(10))
        .expect("producer paused after memory update and before enqueue");
    // This is the precise old race window: `inner` has been released but
    // the absolute row snapshot is not in the queue. A second producer or
    // shutdown must not be admitted in this window.
    assert!(service.inner.try_write().is_ok());
    assert!(matches!(
        service.mutation.try_lock(),
        Err(std::sync::TryLockError::WouldBlock)
    ));
    (resume, thread)
}

#[test]
fn paused_snapshot_cannot_be_overtaken_by_an_adjustment_or_reset() {
    for operation in ["award", "adjust", "reset-one", "reset-all"] {
        let (database, service, root) = temp_service(operation);
        let service = Arc::new(service);
        let (resume, first) = paused_award(&service);
        let (started, observed) = mpsc::sync_channel(0);
        let second_service = Arc::clone(&service);
        let second = std::thread::spawn(move || {
            started.send(()).unwrap();
            match operation {
                "award" => {
                    second_service
                        .award_points("racer", PointAction::Chat, AwardOptions::default())
                        .expect("second award");
                }
                "adjust" => {
                    second_service.adjust("racer", 10.0).expect("adjustment");
                }
                "reset-one" => second_service.reset(Some("racer")),
                "reset-all" => second_service.reset(None),
                _ => unreachable!(),
            }
        });
        observed.recv_timeout(Duration::from_secs(10)).unwrap();
        resume.send(()).unwrap();
        first.join().unwrap();
        second.join().unwrap();
        let expected = match operation {
            "award" => 2.0,
            "adjust" => 11.0,
            _ => 0.0,
        };
        assert_eq!(service.viewer("racer").unwrap()["points"], expected);
        service.shutdown();
        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.write_rejected, 0, "{diagnostics:?}");
        assert_eq!(diagnostics.write_failed, 0, "{diagnostics:?}");
        assert_eq!(diagnostics.committed_seq, 2, "{diagnostics:?}");
        drop(service);
        let restarted = PointsService::new(database);
        let durable = restarted.viewer("racer").unwrap();
        assert_eq!(durable["points"], expected, "{operation}");
        assert_eq!(
            durable["totalChats"],
            if operation == "award" { 2 } else { 1 }
        );
        restarted.shutdown();
        let _ = std::fs::remove_dir_all(root);
    }
}

#[test]
fn shutdown_drains_a_producer_paused_after_admission_before_closing_the_queue() {
    let (database, service, root) = temp_service("shutdown-admission");
    let service = Arc::new(service);
    let (resume, producer) = paused_award(&service);
    let (started, observed) = mpsc::sync_channel(0);
    let shutdown_service = Arc::clone(&service);
    let shutdown = std::thread::spawn(move || {
        started.send(()).unwrap();
        shutdown_service.shutdown();
    });
    observed.recv_timeout(Duration::from_secs(10)).unwrap();
    resume.send(()).unwrap();
    producer.join().unwrap();
    shutdown.join().unwrap();
    assert!(service
        .award_points("racer", PointAction::Chat, AwardOptions::default())
        .is_none());
    assert_eq!(service.viewer("racer").unwrap()["points"], 1.0);
    let diagnostics = service.diagnostics();
    assert_eq!(diagnostics.write_rejected, 0, "{diagnostics:?}");
    assert_eq!(diagnostics.write_failed, 0, "{diagnostics:?}");
    assert_eq!(diagnostics.committed_seq, 1, "{diagnostics:?}");
    drop(service);
    let restarted = PointsService::new(database);
    assert_eq!(restarted.viewer("racer").unwrap()["points"], 1.0);
    restarted.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn large_valid_awards_saturate_level_instead_of_overflowing() {
    let service = PointsService::default();
    let award = service
        .award_points(
            "big-balance",
            PointAction::Manual,
            AwardOptions {
                custom_amount: Some(1_000_000_000_000.0),
                ..AwardOptions::default()
            },
        )
        .unwrap();
    assert_eq!(award.level, u32::MAX);
    assert_eq!(
        service.viewer("big-balance").unwrap()["points"],
        1_000_000_000_000.0
    );
}

#[test]
fn timed_out_cold_reload_cannot_resurrect_points_from_before_a_pending_full_reset() {
    let (database, service, root) = temp_service("cold-reset-timeout");
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    service
        .award_points("bob", PointAction::Chat, AwardOptions::default())
        .unwrap();
    wait_for_writer_idle(&service);
    let (reached, observed) = mpsc::sync_channel(0);
    let (resume, proceed) = mpsc::sync_channel(0);
    *service.writer.before_apply.lock().unwrap() = Some(AdmissionPause {
        reached,
        resume: proceed,
    });
    service
        .writer
        .admission_timeout_ms
        .store(20, Ordering::Relaxed);
    service.reset(None);
    observed.recv_timeout(Duration::from_secs(10)).unwrap();
    service.adjust("bob", 0.0).unwrap();
    service.set_cache_capacity(1);
    assert!(!service.inner.read().unwrap().viewers.contains_key("alice"));
    assert!(
        service.viewer("alice").is_none(),
        "a stalled reset cannot serve pre-reset state"
    );
    assert!(service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .is_none());
    assert!(service.adjust("alice", 1.0).is_none());
    assert!(!service.inner.read().unwrap().viewers.contains_key("alice"));
    resume.send(()).unwrap();
    service.shutdown();
    drop(service);
    let restarted = PointsService::new(database);
    assert_eq!(restarted.viewer("alice").unwrap()["points"], 0.0);
    restarted.shutdown();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn later_success_cannot_hide_an_earlier_permanent_failure_in_the_durable_watermark() {
    let (database, service, root) = temp_service("durability-gap");
    let db = database.open_points_writer().unwrap();
    db.execute_batch("CREATE TRIGGER fail_point_version BEFORE INSERT ON control_resource_outbox WHEN NEW.resource_id='bad' BEGIN SELECT RAISE(ABORT,'test'); END").unwrap();
    service
        .award_points("bad", PointAction::Chat, AwardOptions::default())
        .unwrap();
    wait_for_writer_idle(&service);
    let failed = service.diagnostics();
    assert_eq!(failed.write_failed, 1, "{failed:?}");
    assert_eq!(failed.committed_seq, 0, "{failed:?}");
    assert_eq!(failed.resolved_seq, 1, "{failed:?}");
    db.execute_batch("DROP TRIGGER fail_point_version").unwrap();
    service
        .award_points("good", PointAction::Chat, AwardOptions::default())
        .unwrap();
    service.shutdown();
    let diagnostics = service.diagnostics();
    assert_eq!(diagnostics.resolved_seq, 2, "{diagnostics:?}");
    assert_eq!(
        diagnostics.committed_seq, 0,
        "a later success cannot close the failure gap: {diagnostics:?}"
    );
    assert_eq!(diagnostics.write_applied, 1, "{diagnostics:?}");
    assert_eq!(diagnostics.write_failed, 1, "{diagnostics:?}");
    assert!(database.load_viewer_by_id("bad").unwrap().is_none());
    assert_eq!(
        database.load_viewer_by_id("good").unwrap().unwrap()["points"],
        1.0
    );
    drop(db);
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn partial_batch_fallback_counts_each_committed_award_once_and_rolls_back_the_failed_neighbor() {
    let (database, service, root) = temp_service("partial-fallback");
    let db = database.open_points_writer().unwrap();
    db.execute_batch("CREATE TRIGGER fail_point_version BEFORE INSERT ON control_resource_outbox WHEN NEW.resource_id='bad' BEGIN SELECT RAISE(ABORT,'test'); END").unwrap();
    let (reached, observed) = mpsc::sync_channel(0);
    let (resume, proceed) = mpsc::sync_channel(0);
    *service.writer.before_apply.lock().unwrap() = Some(AdmissionPause {
        reached,
        resume: proceed,
    });
    service
        .award_points("warm", PointAction::Chat, AwardOptions::default())
        .unwrap();
    observed.recv_timeout(Duration::from_secs(10)).unwrap();
    // Both operations are already queued before the first writer batch
    // resumes. The next batch must contain this failing/succeeding pair.
    service
        .award_points("bad", PointAction::Chat, AwardOptions::default())
        .unwrap();
    service
        .award_points("good", PointAction::Chat, AwardOptions::default())
        .unwrap();
    resume.send(()).unwrap();
    service.shutdown();
    let diagnostics = service.diagnostics();
    assert_eq!(diagnostics.write_applied, 2, "{diagnostics:?}");
    assert_eq!(diagnostics.write_failed, 1, "{diagnostics:?}");
    assert_eq!(diagnostics.committed_seq, 1, "{diagnostics:?}");
    assert_eq!(diagnostics.resolved_seq, 3, "{diagnostics:?}");
    assert_eq!(diagnostics.write_batches, 2, "{diagnostics:?}");
    assert!(database.load_viewer_by_id("bad").unwrap().is_none());
    assert_eq!(
        database.load_viewer_by_id("good").unwrap().unwrap()["points"],
        1.0
    );
    for id in ["warm", "good"] {
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM points_transactions WHERE unique_id=?1",
                [id],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(db.query_row("SELECT revision FROM control_resource_versions WHERE kind='viewer' AND scope='host' AND resource_id=?1", [id], |r|r.get::<_,i64>(0)).unwrap(), 1);
    }
    drop(db);
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}
