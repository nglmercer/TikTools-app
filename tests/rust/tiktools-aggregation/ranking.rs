use tiktools_aggregation::ranking::RankingIndex;

#[test]
fn stable_ties_replacement_and_removal() {
    let mut index = RankingIndex::default();
    index.update("z", None, Some(10));
    index.update("a", None, Some(10));
    assert_eq!(index.top(2).collect::<Vec<_>>(), vec![("a", 10), ("z", 10)]);
    index.update("z", Some(10), Some(20));
    assert_eq!(index.top(2).collect::<Vec<_>>(), vec![("z", 20), ("a", 10)]);
    index.update("z", Some(20), None);
    assert_eq!(index.top(2).collect::<Vec<_>>(), vec![("a", 10)]);
}
