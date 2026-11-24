use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tiktools_plugin_store::*;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
struct Rule {
    enabled: bool,
    label: String,
}

fn scratch(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!(
        "tiktools-store-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn rule(enabled: bool) -> Rule {
    Rule {
        enabled,
        label: "welcome".into(),
    }
}

#[test]
fn put_get_list_round_trip_with_increasing_revisions() {
    let dir = scratch("roundtrip");
    let store = Store::open(&dir).unwrap();
    let rules = store.collection::<Rule>("rules").unwrap();
    let first = rules.put("b", &rule(true), Expect::Absent).unwrap();
    let second = rules.put("a", &rule(false), Expect::Any).unwrap();
    assert!(second.revision > first.revision);
    assert_eq!(rules.get("b").unwrap().unwrap().value, rule(true));
    let ids: Vec<_> = rules.list().unwrap().into_iter().map(|r| r.id).collect();
    assert_eq!(ids, ["a", "b"]);
    let enabled = rules.query(|rule| rule.enabled).unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].id, "b");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn stale_and_duplicate_writers_get_conflicts() {
    let dir = scratch("conflicts");
    let store = Store::open(&dir).unwrap();
    let rules = store.collection::<Rule>("rules").unwrap();
    let saved = rules.put("r", &rule(true), Expect::Absent).unwrap();
    assert!(matches!(
        rules.put("r", &rule(false), Expect::Absent),
        Err(StoreError::Conflict { .. })
    ));
    assert!(matches!(
        rules.put("r", &rule(false), Expect::Revision(saved.revision + 1)),
        Err(StoreError::Conflict { .. })
    ));
    let updated = rules
        .put("r", &rule(false), Expect::Revision(saved.revision))
        .unwrap();
    assert_eq!(rules.get("r").unwrap().unwrap().revision, updated.revision);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn deleted_then_recreated_records_never_reuse_a_revision() {
    let dir = scratch("aba");
    let store = Store::open(&dir).unwrap();
    let rules = store.collection::<Rule>("rules").unwrap();
    let old = rules.put("r", &rule(true), Expect::Absent).unwrap();
    assert!(rules.delete("r", Expect::Revision(old.revision)).unwrap());
    let recreated = rules.put("r", &rule(true), Expect::Absent).unwrap();
    assert!(recreated.revision > old.revision);
    assert!(matches!(
        rules.delete("r", Expect::Revision(old.revision)),
        Err(StoreError::Conflict { .. })
    ));
    assert!(!rules.delete("missing", Expect::Any).unwrap());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn data_survives_reopening_the_store() {
    let dir = scratch("reopen");
    {
        let store = Store::open(&dir).unwrap();
        store
            .collection::<Rule>("rules")
            .unwrap()
            .put("kept", &rule(true), Expect::Absent)
            .unwrap();
    }
    let reopened = Store::open(&dir).unwrap();
    let kept = reopened
        .collection::<Rule>("rules")
        .unwrap()
        .get("kept")
        .unwrap();
    assert_eq!(kept.unwrap().value, rule(true));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn settings_default_then_update_with_optimistic_check() {
    let dir = scratch("settings");
    let store = Store::open(&dir).unwrap();
    let settings = store.settings::<Rule>("overlay").unwrap();
    assert_eq!(settings.get().unwrap(), Rule::default());
    settings.update(|value| value.enabled = true).unwrap();
    let first = settings.get().unwrap();
    assert!(first.enabled);
    settings.save(&rule(false), Expect::Any).unwrap();
    assert!(!settings.get().unwrap().enabled);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rejects_bad_collection_names_and_record_ids() {
    let dir = scratch("names");
    let store = Store::open(&dir).unwrap();
    for name in ["", "Rules", "../x", "1abc", "has space", &"a".repeat(65)] {
        assert!(
            matches!(
                store.collection::<Rule>(name),
                Err(StoreError::InvalidName(_))
            ),
            "accepted name {name:?}"
        );
    }
    let rules = store.collection::<Rule>("rules").unwrap();
    assert!(matches!(
        rules.put("", &rule(true), Expect::Any),
        Err(StoreError::InvalidId(_))
    ));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn oversized_records_are_refused_before_writing() {
    let dir = scratch("limits");
    let store = Store::open(&dir).unwrap();
    let rules = store.collection::<Rule>("rules").unwrap();
    let huge = Rule {
        enabled: true,
        label: "x".repeat(MAX_RECORD_BYTES),
    };
    assert!(matches!(
        rules.put("big", &huge, Expect::Any),
        Err(StoreError::TooLarge { what: "record", .. })
    ));
    assert!(rules.list().unwrap().is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn damaged_or_future_files_are_errors_not_silent_resets() {
    let dir = scratch("damaged");
    let store = Store::open(&dir).unwrap();
    let rules = store.collection::<Rule>("rules").unwrap();
    fs::write(dir.join("rules.json"), b"{not json").unwrap();
    assert!(matches!(rules.list(), Err(StoreError::Corrupt(_))));
    fs::write(
        dir.join("rules.json"),
        br#"{"formatVersion":2,"sequence":0,"records":{}}"#,
    )
    .unwrap();
    assert!(matches!(
        rules.list(),
        Err(StoreError::UnsupportedFormat(2))
    ));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn file_area_round_trips_and_rejects_escaping_paths() {
    let dir = scratch("files");
    let store = Store::open(&dir).unwrap();
    let files = store.files();
    files.write("exports/one.bin", &[1, 2, 3]).unwrap();
    assert_eq!(files.read("exports/one.bin").unwrap(), Some(vec![1, 2, 3]));
    assert_eq!(files.list().unwrap(), ["exports/one.bin"]);
    assert!(files.remove("exports/one.bin").unwrap());
    assert_eq!(files.read("exports/one.bin").unwrap(), None);
    for bad in [
        "",
        "/etc/passwd",
        "../outside",
        "a/../b",
        "./x",
        "a\\b",
        "a/\0b",
    ] {
        assert!(
            matches!(files.write(bad, b"x"), Err(StoreError::InvalidPath(_))),
            "accepted path {bad:?}"
        );
        assert!(files.read(bad).is_err(), "read accepted {bad:?}");
    }
    let _ = fs::remove_dir_all(dir);
}
