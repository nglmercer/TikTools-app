use tiktools_aggregation::{Aggregation, Error};

#[test]
fn score_reassignment_is_atomic_and_keeps_index_consistent() {
    let mut a = Aggregation::new(2);
    a.apply("a", 10, 1).unwrap();
    a.apply("b", 3, 1).unwrap();
    let before = a.snapshot(10);
    assert_eq!(a.reassign("a", "b", 11, 0), Err(Error::InsufficientScore));
    assert_eq!(a.reassign("a", "c", 5, 0), Err(Error::Capacity));
    assert_eq!(a.reassign("a", "b", 1, u64::MAX), Err(Error::Overflow));
    assert_eq!(a.snapshot(10), before);
    a.reassign("a", "b", 4, 1).unwrap();
    let snapshot = a.snapshot(10);
    assert_eq!(snapshot.total_score, 14);
    assert_eq!(
        (snapshot.entries[0].key.as_str(), snapshot.entries[0].score),
        ("b", 8)
    );
    assert_eq!(
        (snapshot.entries[1].key.as_str(), snapshot.entries[1].score),
        ("a", 6)
    );
    a.reassign("a", "c", 6, 0).unwrap();
    assert_eq!(a.snapshot(10).entries[1].key, "c");
}
#[test]
fn ordering_and_atomic_rejection() {
    let mut a = Aggregation::new(2);
    a.apply("b", 2, 3).unwrap();
    a.apply("a", 6, 1).unwrap();
    let before = a.snapshot(10);
    assert_eq!(before.entries[0].key, "a");
    assert_eq!(a.apply("c", 1, 1), Err(Error::Capacity));
    assert_eq!(a.apply("a", u64::MAX, 2), Err(Error::Overflow));
    assert_eq!(a.snapshot(10), before);
}
