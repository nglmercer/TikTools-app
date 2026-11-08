//! Template/profile import over the real binary: each test spawns
//! `tiktools --standalone` under an isolated `TIKTOOLS_HOME`, so the full
//! path (argv parsing, template engine, record creation, gallery) is
//! exercised without touching real user data or a running host.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{io::Read, thread};

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
    std::fs::create_dir_all(path.join("data")).unwrap();
    for file in [
        "tiktok-points.db",
        "tiktok-automation.db",
        "tiktok-analytics.db",
    ] {
        std::fs::File::create(path.join("data").join(file)).unwrap();
    }
    path
}

fn command(home: &std::path::Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tiktools"));
    command
        .args(args)
        .env("TIKTOOLS_HOME", home)
        .env("TIKTOOLS_CLOUD_ORIGIN", "")
        .env_remove("TIKTOOLS_SESSION_COOKIE");
    for (key, folder) in [
        ("TIKTOOLS_DATA_DIR", "data"),
        ("TIKTOOLS_PLUGINS_DIR", "plugins"),
        ("TIKTOOLS_PLUGIN_DATA_DIR", "plugin-data"),
        ("TIKTOOLS_BUILTIN_PLUGINS_DIR", "builtins"),
        ("TIKTOOLS_DEV_PLUGINS_DIR", "dev-plugins"),
        ("TIKTOOLS_LOG_DIR", "logs"),
        ("TIKTOOLS_TEMP_DIR", "temp"),
    ] {
        command.env(key, home.join(folder));
    }
    command
}

fn run(home: &std::path::Path, args: &[&str]) -> (bool, String, String) {
    let output = command(home, args).output().expect("spawn tiktools");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn rejects_expansion_before_effect(home: &std::path::Path, args: &[&str]) {
    let mut full = vec!["--standalone", "--json"];
    full.extend(args.iter());
    let started = Instant::now();
    let mut child = command(home, &full)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bounded import");
    // Drain both pipes while polling, just as Command::output does. A regression
    // that prints expanded source must not deadlock this deadline assertion.
    fn drain(mut reader: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
        thread::spawn(move || {
            let mut captured = Vec::new();
            reader
                .by_ref()
                .take(16 * 1024)
                .read_to_end(&mut captured)
                .unwrap();
            std::io::copy(&mut reader, &mut std::io::sink()).unwrap();
            captured
        })
    }
    let stdout = drain(child.stdout.take().unwrap());
    let stderr = drain(child.stderr.take().unwrap());
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            timed_out = true;
            break child.wait().unwrap();
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout.join().unwrap();
    let stderr = stderr.join().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    assert!(
        !timed_out,
        "bounded import exceeded its 10-second deadline: {args:?}; captured output: {text}"
    );
    assert!(!status.success(), "oversized expansion was accepted");
    assert!(
        text.contains("too_large") && text.contains("Template expansion exceeds"),
        "{text}"
    );
    assert!(
        !text.contains("private-expansion-source"),
        "error exposed source"
    );
}

fn no_import_effects(home: &std::path::Path) {
    let library = run_json(home, &["api", "call", "automation.library.snapshot", "{}"]);
    assert_eq!(library["definitions"]["actions"], serde_json::json!([]));
    assert_eq!(library["definitions"]["events"], serde_json::json!([]));
    assert_eq!(library["catalog"]["packs"], serde_json::json!([]));
    assert_eq!(library["catalog"]["templates"], serde_json::json!([]));
    let state = run_json(home, &["api", "call", "automation.library.get", "{}"]);
    assert!(state["pending"].is_null());
    assert_eq!(state["recent"], serde_json::json!([]));
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
    let template = repo_path("tests/fixtures/templates/minecraft-gift-command.tiktemplate.json");
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
    let native = run_json(&home, &["api", "call", "automation.library.get", "{}"]);
    assert!(native["pending"].is_null());
    assert_eq!(native["recent"].as_array().unwrap().len(), 1);
    assert_eq!(native["recent"][0]["intent"]["operation"], "apply-template");
    assert_eq!(
        imported["acknowledgement"]["command"]["commandId"],
        native["recent"][0]["intent"]["commandId"]
    );

    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn profile_dry_run_plans_every_rule_offline() {
    let home = isolated_home("profile-dryrun");
    let profile = repo_path("tests/fixtures/profiles/minecraft-gifts.tikprofile.json");
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
    let profile = repo_path("tests/fixtures/profiles/minecraft-gifts.tikprofile.json");

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
    let library = run_json(&home, &["api", "call", "automation.library.snapshot", "{}"]);
    assert_eq!(library["catalog"]["activeId"], "minecraft-gifts");
    assert_eq!(
        library["catalog"]["packs"][0]["eventIds"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    assert_eq!(library["catalog"]["templates"].as_array().unwrap().len(), 6);
    let native = run_json(&home, &["api", "call", "automation.library.get", "{}"]);
    assert_eq!(native["recent"].as_array().unwrap().len(), 1);
    assert_eq!(native["recent"][0]["intent"]["operation"], "import-profile");

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

#[test]
fn multi_template_import_is_one_native_transaction_and_no_save_keeps_gallery_empty() {
    let home = isolated_home("template-native-batch");
    let original: Value = serde_json::from_str(
        &std::fs::read_to_string(repo_path(
            "tests/fixtures/templates/minecraft-gift-command.tiktemplate.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let mut second = original.clone();
    second["id"] = Value::String("second-template".into());
    let document = home.join("templates.json");
    std::fs::write(
        &document,
        serde_json::to_vec(&vec![original, second]).unwrap(),
    )
    .unwrap();
    let imported = run_json(
        &home,
        &[
            "template",
            "import",
            document.to_str().unwrap(),
            "--no-save",
            "--disabled",
        ],
    );
    assert_eq!(imported["imported"].as_array().unwrap().len(), 2);
    assert_eq!(imported["savedToGallery"], false);
    let library = run_json(&home, &["api", "call", "automation.library.snapshot", "{}"]);
    assert_eq!(
        library["definitions"]["actions"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        library["definitions"]["events"].as_array().unwrap().len(),
        2
    );
    assert_eq!(library["catalog"]["templates"], serde_json::json!([]));
    for key in ["actions", "events"] {
        assert!(library["definitions"][key]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["enabled"] == false));
    }
    let native = run_json(&home, &["api", "call", "automation.library.get", "{}"]);
    assert_eq!(native["recent"].as_array().unwrap().len(), 1);
    assert_eq!(native["recent"][0]["receipt"]["affectedResources"], 5);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn reusable_profile_template_creates_each_graph_but_upserts_one_gallery_copy() {
    let home = isolated_home("profile-reused-template");
    let mut profile: Value = serde_json::from_str(
        &std::fs::read_to_string(repo_path(
            "tests/fixtures/profiles/minecraft-gifts.tikprofile.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let first = profile["templates"][0].clone();
    profile["id"] = Value::String("reused-profile".into());
    profile["name"] = Value::String("Reusable profile".into());
    profile["templates"] = serde_json::json!([first, first]);
    let file = home.join("reused.tikprofile.json");
    std::fs::write(&file, serde_json::to_vec(&profile).unwrap()).unwrap();
    let result = run_json(
        &home,
        &["template", "profile-import", file.to_str().unwrap()],
    );
    assert_eq!(result["imported"].as_array().unwrap().len(), 2);
    let library = run_json(&home, &["api", "call", "automation.library.snapshot", "{}"]);
    assert_eq!(
        library["definitions"]["actions"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        library["definitions"]["events"].as_array().unwrap().len(),
        2
    );
    assert_eq!(library["catalog"]["templates"].as_array().unwrap().len(), 1);
    assert_eq!(library["catalog"]["activeId"], "reused-profile");
    let history = run_json(&home, &["api", "call", "automation.library.get", "{}"]);
    assert_eq!(history["recent"].as_array().unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn oversized_string_expansion_fails_before_materializing_graph_gallery_or_native_intent() {
    let home = isolated_home("template-expansion-bound");
    let template = serde_json::json!({"templateVersion":1,"id":"expansion-template","title":"Expansion fixture","description":"","icon":"plugin",
        "params":{"type":"object","properties":{"p":{"type":"string","default":"private-expansion-source".repeat(500)}}},
        "actions":[{"name":"Expanded action","typeId":"core.log","enabled":false,"config":{"message":"{{params.p}}".repeat(2000)}}],
        "event":{"name":"Expanded event","enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all"}});
    let template_file = home.join("expansion.tiktemplate.json");
    std::fs::write(&template_file, serde_json::to_vec(&template).unwrap()).unwrap();
    let profile = serde_json::json!({"profileVersion":1,"id":"expansion-profile","name":"Expansion profile","description":"","params":{},"templates":[{"template":template,"params":{}}]});
    let profile_file = home.join("expansion.tikprofile.json");
    std::fs::write(&profile_file, serde_json::to_vec(&profile).unwrap()).unwrap();
    for (verb, path) in [
        ("import", &template_file),
        ("profile-import", &profile_file),
    ] {
        assert!(std::fs::metadata(path).unwrap().len() < 65_536);
        for dry in [false, true] {
            let mut args = vec!["template", verb, path.to_str().unwrap()];
            if dry {
                args.push("--dry-run");
            }
            rejects_expansion_before_effect(&home, &args);
        }
    }
    no_import_effects(&home);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn profile_expansion_shares_one_aggregate_budget_across_actual_and_dry_run_entries() {
    let home = isolated_home("profile-aggregate-expansion");
    let actions:Vec<_>=(0..8).map(|index|serde_json::json!({"name":format!("Action {index}"),"typeId":"core.log","enabled":false,"config":{"message":"{{params.p}}".repeat(7)}})).collect();
    let template = serde_json::json!({"templateVersion":1,"id":"aggregate-template","title":"Aggregate fixture","description":"","icon":"plugin","actions":actions,
        "event":{"name":"Aggregate event","enabled":false,"trigger":"tiktok.chat","filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all"}});
    let profile = serde_json::json!({"profileVersion":1,"id":"aggregate-profile","name":"Aggregate profile","description":"","params":{"p":"s".repeat(32_000)},
        "templates":vec![serde_json::json!({"template":template,"params":{}});5]});
    let file = home.join("aggregate.tikprofile.json");
    std::fs::write(&file, serde_json::to_vec(&profile).unwrap()).unwrap();
    assert!(std::fs::metadata(&file).unwrap().len() < 65_536);
    for dry in [false, true] {
        let mut args = vec!["template", "profile-import", file.to_str().unwrap()];
        if dry {
            args.push("--dry-run");
        }
        rejects_expansion_before_effect(&home, &args);
    }
    no_import_effects(&home);
    let _ = std::fs::remove_dir_all(home);
}
