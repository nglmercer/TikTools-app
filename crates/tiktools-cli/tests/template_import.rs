//! Template/profile import over the real binary: each test spawns
//! `tiktools --standalone` under an isolated `TIKTOOLS_HOME`, so the full
//! path (argv parsing, template engine, record creation, gallery) is
//! exercised without touching real user data or a running host.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(relative)
}

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
fn template_import_creates_records_and_gallery_entry() {
    let home = isolated_home("template-import");
    let template = repo_path("examples/templates/minecraft-gift-command.tiktemplate.json");
    assert!(template.is_file(), "example template ships with the repo");

    let imported = run_json(
        &home,
        &[
            "template",
            "import",
            template.to_str().expect("utf8 path"),
            "--param",
            "giftName=Rose",
        ],
    );
    let entries = imported
        .get("imported")
        .and_then(Value::as_array)
        .expect("imported array");
    assert_eq!(entries.len(), 1);
    assert_eq!(imported.get("savedToGallery"), Some(&Value::Bool(true)));
    let event_id = entries[0]
        .get("event")
        .and_then(|event| event.get("id"))
        .and_then(Value::as_str)
        .expect("created event id");

    let list = run_json(&home, &["automation", "list", "--kind", "all"]);
    let events = list
        .get("events")
        .and_then(Value::as_array)
        .expect("events");
    let actions = list
        .get("actions")
        .and_then(Value::as_array)
        .expect("actions");
    assert!(events
        .iter()
        .any(|entry| entry.get("id").and_then(Value::as_str) == Some(event_id)));
    assert_eq!(actions.len(), 1);
    // Params resolved at creation; the stored URL host is literal while the
    // runtime span survives for per-gift rendering.
    let config = actions[0].get("config").expect("action config");
    assert_eq!(
        config.get("url").and_then(Value::as_str),
        Some("http://127.0.0.1:8080/api/chat")
    );
    let body = config.get("body").and_then(Value::as_str).expect("body");
    assert!(
        body.contains("/give @p minecraft:apple"),
        "body keeps command: {body}"
    );
    assert!(
        body.contains("{{ event.data.repeatCount }}"),
        "runtime span survives: {body}"
    );
    let stored = events
        .iter()
        .find(|entry| entry.get("id").and_then(Value::as_str) == Some(event_id))
        .expect("stored event");
    assert_eq!(
        stored
            .get("filters")
            .and_then(|filters| filters.get(0))
            .and_then(|filter| filter.get("value")),
        Some(&Value::String("Rose".to_owned()))
    );

    let gallery = run_json(&home, &["template", "list"]);
    let customs = gallery
        .get("templates")
        .and_then(Value::as_array)
        .expect("gallery");
    assert!(customs
        .iter()
        .any(|entry| entry.get("id").and_then(Value::as_str) == Some("minecraft-gift-command")));

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn profile_dry_run_plans_every_rule_offline() {
    let home = isolated_home("profile-dryrun");
    let profile = repo_path("examples/profiles/minecraft-gifts.tikprofile.json");
    assert!(profile.is_file(), "example profile ships with the repo");

    // No host needed: dry-run never connects.
    let planned = run_json(
        &home,
        &[
            "template",
            "profile-import",
            profile.to_str().expect("utf8 path"),
            "--dry-run",
            "--param",
            "commandPort=9090",
        ],
    );
    assert_eq!(planned.get("dryRun"), Some(&Value::Bool(true)));
    let plans = planned
        .get("plans")
        .and_then(Value::as_array)
        .expect("plans");
    assert_eq!(plans.len(), 6);
    for plan in plans {
        let url = plan
            .get("actions")
            .and_then(|actions| actions.get(0))
            .and_then(|action| action.get("config"))
            .and_then(|config| config.get("url"))
            .and_then(Value::as_str)
            .expect("plan action url");
        assert_eq!(
            url, "http://127.0.0.1:9090/api/chat",
            "CLI param wins: {url}"
        );
        let trigger = plan
            .get("requirements")
            .and_then(|requirements| requirements.get("trigger"))
            .and_then(Value::as_str)
            .expect("requirement trigger");
        assert!(trigger.starts_with("tiktok."));
    }

    // Dry-run creates nothing.
    let list = run_json(&home, &["automation", "list", "--kind", "all"]);
    assert!(list
        .get("events")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty));
    assert!(list
        .get("actions")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty));

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn profile_import_creates_all_rules_then_exports() {
    let home = isolated_home("profile-roundtrip");
    let profile = repo_path("examples/profiles/minecraft-gifts.tikprofile.json");

    let imported = run_json(
        &home,
        &[
            "template",
            "profile-import",
            profile.to_str().expect("utf8 path"),
        ],
    );
    assert_eq!(
        imported.get("profile"),
        Some(&Value::String("minecraft-gifts".to_owned()))
    );
    assert_eq!(
        imported
            .get("imported")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(6)
    );

    let list = run_json(&home, &["automation", "list", "--kind", "all"]);
    assert_eq!(
        list.get("events").and_then(Value::as_array).map(Vec::len),
        Some(6)
    );
    assert_eq!(
        list.get("actions").and_then(Value::as_array).map(Vec::len),
        Some(6)
    );

    let out = home.join("roundtrip.tikprofile.json");
    let exported = run_json(
        &home,
        &[
            "template",
            "profile-export",
            "--out",
            out.to_str().expect("utf8 path"),
        ],
    );
    assert_eq!(exported.get("entries").and_then(Value::as_u64), Some(6));
    let text = std::fs::read_to_string(&out).expect("profile file written");
    let doc: Value = serde_json::from_str(&text).expect("profile file is JSON");
    assert_eq!(doc.get("profileVersion"), Some(&Value::Number(1.into())));

    let _ = std::fs::remove_dir_all(&home);
}
