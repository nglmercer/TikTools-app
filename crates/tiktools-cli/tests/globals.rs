//! `globals` round trip over the real binary: each step spawns
//! `tiktools --standalone` under an isolated `TIKTOOLS_HOME`, so values
//! persist across processes through sqlite, and a dry-run fetch proves
//! the two-phase URL render end to end without touching the network.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn isolated_home(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "tiktools-test-{}-{label}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("create isolated home");
    path
}

fn run(home: &std::path::Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_tiktools"))
        .args(args)
        .env("TIKTOOLS_HOME", home)
        .output()
        .expect("spawn tiktools");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn run_json(home: &std::path::Path, args: &[&str]) -> Value {
    let mut full = vec!["--standalone", "--json"];
    full.extend(args.iter());
    let (ok, stdout, stderr) = run(home, &full);
    assert!(
        ok,
        "command failed: {args:?}\nstderr: {stderr}\nstdout: {stdout}"
    );
    serde_json::from_str(&stdout).expect("stdout is JSON")
}

#[test]
fn globals_round_trip_persists_across_processes() {
    let home = isolated_home("globals-roundtrip");

    let set = run_json(&home, &["globals", "set", "commandPort", "46665"]);
    assert_eq!(set.get("key").and_then(Value::as_str), Some("commandPort"));
    assert_eq!(set.get("value").and_then(Value::as_str), Some("46665"));

    let quoted = run_json(&home, &["globals", "set", "commandHost", "127.0.0.1"]);
    assert_eq!(
        quoted.get("value").and_then(Value::as_str),
        Some("127.0.0.1")
    );

    // Fresh process, same home: values survived through sqlite.
    let got = run_json(&home, &["globals", "get", "commandPort"]);
    assert_eq!(got.get("value").and_then(Value::as_str), Some("46665"));

    let list = run_json(&home, &["globals", "list"]);
    let globals = list.get("globals").expect("globals map");
    assert_eq!(
        globals.get("commandPort").and_then(Value::as_str),
        Some("46665")
    );
    assert_eq!(
        globals.get("commandHost").and_then(Value::as_str),
        Some("127.0.0.1")
    );

    let deleted = run_json(&home, &["globals", "delete", "commandPort"]);
    assert_eq!(deleted.get("deleted"), Some(&Value::Bool(true)));
    let missing = run_json(&home, &["globals", "delete", "commandPort"]);
    assert_eq!(missing.get("deleted"), Some(&Value::Bool(false)));

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn globals_reject_bad_keys_and_missing_reads() {
    let home = isolated_home("globals-invalid");

    // JSON mode reports errors on stdout as a JSON-RPC envelope.
    let (ok, stdout, _) = run(
        &home,
        &["--standalone", "--json", "globals", "set", "has space", "x"],
    );
    assert!(!ok, "bad key must fail");
    assert!(stdout.contains("global keys"), "clear error: {stdout}");

    let (ok, stdout, _) = run(&home, &["--standalone", "--json", "globals", "get", "nope"]);
    assert!(!ok, "missing key must fail");
    assert!(stdout.contains("unknown global"), "clear error: {stdout}");

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn fetch_dry_run_renders_globals_in_the_url_host() {
    let home = isolated_home("globals-render");
    let record = serde_json::json!({
        "name": "Global command host",
        "typeId": "core.fetch",
        "config": {
            "method": "POST",
            "url": "http://{{ globals.commandHost }}/api/chat",
            "allowPrivateNetwork": true,
        },
    })
    .to_string();

    // No global yet: the host renders empty, so the dry run fails closed
    // instead of sending anywhere.
    let before = run_json(
        &home,
        &[
            "automation",
            "test",
            "--record",
            record.as_str(),
            "--kind",
            "action",
        ],
    );
    assert_eq!(before.get("status").and_then(Value::as_str), Some("error"));

    run_json(&home, &["globals", "set", "commandHost", "127.0.0.1"]);

    let after = run_json(
        &home,
        &[
            "automation",
            "test",
            "--record",
            record.as_str(),
            "--kind",
            "action",
        ],
    );
    assert_eq!(after.get("status").and_then(Value::as_str), Some("ok"));
    let summary = after
        .get("summary")
        .and_then(Value::as_str)
        .expect("summary");
    assert!(
        summary.contains("127.0.0.1"),
        "host rendered from globals: {summary}"
    );

    let _ = std::fs::remove_dir_all(&home);
}
