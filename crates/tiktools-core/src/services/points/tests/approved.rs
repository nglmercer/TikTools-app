use super::*;
use crate::db::DatabaseError;

#[test]
fn approved_adjustment_commits_after_earlier_awards_and_before_later_awards() {
    let (database, service, root) = temp_service("approved-order");
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    let award = service
        .adjust_approved("alice", 2.0, |effective, commit| {
            assert_eq!(effective, 2.0);
            commit()
        })
        .unwrap();
    assert_eq!(award.total_points, 3.0);
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        3.0
    );
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    service.shutdown();
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        4.0
    );
    assert_eq!(service.viewer("alice").unwrap()["points"], 4.0);
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn final_admission_can_reject_the_actual_rounded_delta_without_mutating_cache_or_database() {
    let (database, service, root) = temp_service("approved-reject");
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    let result = service.adjust_approved("alice", 0.006, |effective, _commit| {
        assert!((effective - 0.01).abs() < 0.000001);
        Err(DatabaseError::Invalid(
            "final effective delta is outside the grant".into(),
        ))
    });
    assert!(result.is_err());
    service.shutdown();
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        1.0
    );
    assert_eq!(service.viewer("alice").unwrap()["points"], 1.0);
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn approved_adjustment_does_not_fabricate_a_missing_viewer_or_accept_an_uncommitted_callback() {
    let (database, service, root) = temp_service("approved-noop");
    assert!(matches!(
        service.adjust_approved("missing", 1.0, |_, commit| commit()),
        Err(DatabaseError::NotFound)
    ));
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    assert!(service
        .adjust_approved("alice", 1.0, |_, _| Ok(()))
        .is_err());
    service.shutdown();
    assert!(database.load_viewer_by_id("missing").unwrap().is_none());
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        1.0
    );
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}
