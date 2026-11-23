use super::{parse, topic_allowed, Command};

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

#[test]
fn event_get_parses_name_kinds_author_and_limit() {
    let command = parse(&args(&[
        "event",
        "get",
        "rose",
        "--type",
        "gift,chat",
        "--type",
        "join",
        "--author",
        "mango",
        "--limit",
        "5",
    ]))
    .unwrap_or_else(|_| panic!("valid event get"));
    let Command::GetEvent {
        name,
        kinds,
        author,
        limit,
    } = command
    else {
        panic!("event get expected");
    };
    assert_eq!(name, "rose");
    assert_eq!(kinds, ["gift", "chat", "join"]);
    assert_eq!(author.as_deref(), Some("mango"));
    assert_eq!(limit, 5);
}

#[test]
fn event_get_rejects_bad_flags_and_limits() {
    for input in [
        vec!["event", "get"],
        vec!["event", "list", "rose"],
        vec!["event", "get", "rose", "--limit", "0"],
        vec!["event", "get", "rose", "--limit", "201"],
        vec!["event", "get", "rose", "--limit", "abc"],
        vec!["event", "get", "rose", "--unknown"],
        vec!["event", "get", "rose", "extra"],
    ] {
        assert!(parse(&args(&input)).is_err(), "{input:?}");
    }
}

#[test]
fn topics_filter_exact_wildcard_and_namespaces() {
    assert!(topic_allowed(&[], "anything"));
    assert!(topic_allowed(&["*".to_owned()], "anything.at.all"));
    assert!(topic_allowed(&["live.*".to_owned()], "live"));
    assert!(topic_allowed(&["live.*".to_owned()], "live.event"));
    assert!(!topic_allowed(&["live.*".to_owned()], "lively"));
    assert!(!topic_allowed(&["points.changed".to_owned()], "points"));
}
