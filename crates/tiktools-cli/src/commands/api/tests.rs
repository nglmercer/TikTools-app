use super::topic_allowed;

#[test]
fn topics_filter_exact_wildcard_and_namespaces() {
    assert!(topic_allowed(&[], "anything"));
    assert!(topic_allowed(&["*".to_owned()], "anything.at.all"));
    assert!(topic_allowed(&["live.*".to_owned()], "live"));
    assert!(topic_allowed(&["live.*".to_owned()], "live.event"));
    assert!(!topic_allowed(&["live.*".to_owned()], "lively"));
    assert!(!topic_allowed(&["points.changed".to_owned()], "points"));
}
