use super::*;
fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}
#[test]
fn command_metadata_flags_are_not_domain_params_and_can_precede_the_method() {
    let command = parse(&args(&[
        "call",
        "--expected-revision",
        "7",
        "workflows.save",
        "{}",
        "--command-id",
        "stable-id",
        "--host-instance-id",
        "instance-id",
        "--include-meta",
    ]))
    .unwrap_or_else(|_| panic!("valid metadata"));
    let Command::Call {
        method,
        params,
        metadata,
        include_meta,
        ..
    } = command
    else {
        panic!("call expected");
    };
    assert_eq!(method, "workflows.save");
    assert_eq!(params, json!({}));
    assert!(include_meta);
    let metadata = metadata.unwrap();
    assert_eq!(metadata.expected_revision.as_deref(), Some("7"));
    assert_eq!(metadata.command_id.as_deref(), Some("stable-id"));
}
#[test]
fn malformed_command_flags_fail_before_a_host_can_execute_anything() {
    for flags in [
        vec!["--command-id", "only-id"],
        vec!["--command-id=ignored-before"],
        vec!["--expected-revision", "01"],
        vec!["--expected-revision"],
        vec!["--expected-revision", "1", "--expected-revision", "1"],
        vec!["--unknown"],
    ] {
        let mut input = vec!["call", "workflows.save", "{}"];
        input.extend(flags);
        assert!(matches!(parse(&args(&input)), Err(CommandError::Usage(_))));
    }
    assert!(parse(&args(&[
        "call",
        "workflows.delete",
        "{}",
        "--expected-revision",
        "3"
    ]))
    .is_ok());
}

#[test]
fn additive_metadata_has_no_compare_and_set_precondition() {
    let command = parse(&args(&[
        "call",
        "points.adjust",
        "{\"uniqueId\":\"alice\",\"delta\":2}",
        "--command-id",
        "once",
        "--host-instance-id",
        "host",
        "--include-meta",
    ]))
    .unwrap_or_else(|_| panic!("valid additive metadata"));
    let Command::Call {
        metadata: Some(metadata),
        ..
    } = command
    else {
        panic!("call metadata expected")
    };
    assert_eq!(metadata.command_id.as_deref(), Some("once"));
    assert!(metadata.expected_revision.is_none());
}
