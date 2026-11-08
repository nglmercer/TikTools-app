use super::*;
use serde_json::json;

fn binding() -> Binding {
    Binding {
        profile_id: "profile".into(),
        origin: "https://host.example".into(),
        host_id: "host".into(),
        principal_id: "author".into(),
        session_id: "device".into(),
    }
}
fn path(home: &tempfile::TempDir) -> PathBuf {
    let dir = home.path().join("private");
    directory::check_directory(&dir, true).unwrap();
    dir.join("profile.proposals")
}
fn draft(kind: AutomationFamily, text: &str) -> ProposalDraft {
    ProposalDraft {
        kind,
        id: "same".into(),
        mode: ProposalMode::Create,
        base_revision: "0".into(),
        text: text.into(),
        updated_at: 1,
    }
}

#[test]
fn explicit_storage_separates_families_and_fences_delete_recreate_and_profile_changes() {
    let home = tempfile::tempdir().unwrap();
    let p = path(&home);
    let b = binding();
    assert_eq!(read(&p, &b).unwrap().revision, "0");
    assert!(!p.exists());
    let text = "unfinished JSON 🧡\nfixture-secret";
    change(
        &p,
        &b,
        "0",
        AutomationFamily::Action,
        "same",
        Some(draft(AutomationFamily::Action, text)),
    )
    .unwrap();
    change(
        &p,
        &b,
        "1",
        AutomationFamily::Event,
        "same",
        Some(draft(AutomationFamily::Event, "event copy")),
    )
    .unwrap();
    let d = read(&p, &b).unwrap();
    assert_eq!(d.records.len(), 2);
    assert_eq!(d.records["action:same"].text, text);
    let mut other = b.clone();
    other.session_id = "other".into();
    assert_eq!(read(&p, &other).err().unwrap().code, "proposal_invalid");
    assert_eq!(
        change(&p, &b, "1", AutomationFamily::Action, "same", None)
            .unwrap_err()
            .code,
        "proposal_conflict"
    );
    change(&p, &b, "2", AutomationFamily::Action, "same", None).unwrap();
    assert_eq!(
        change(
            &p,
            &b,
            "2",
            AutomationFamily::Action,
            "same",
            Some(draft(AutomationFamily::Action, "stale"))
        )
        .unwrap_err()
        .code,
        "proposal_conflict"
    );
    let d = read(&p, &b).unwrap();
    assert_eq!(d.revision, "3");
    assert_eq!(d.records.len(), 1);
    assert!(d.records.contains_key("event:same"));
    remove(&p).unwrap();
    assert!(!p.exists());
}

#[test]
fn malformed_misbound_oversized_and_full_storage_preserve_the_last_copy() {
    let home = tempfile::tempdir().unwrap();
    let p = path(&home);
    let b = binding();
    change(
        &p,
        &b,
        "0",
        AutomationFamily::Action,
        "same",
        Some(draft(AutomationFamily::Action, "retained")),
    )
    .unwrap();
    let bytes = fs::read(&p).unwrap();
    for mut candidate in [
        draft(AutomationFamily::Event, "x"),
        draft(AutomationFamily::Action, "x"),
    ] {
        candidate.mode = ProposalMode::Update;
        assert_eq!(
            change(
                &p,
                &b,
                "1",
                AutomationFamily::Action,
                "same",
                Some(candidate)
            )
            .unwrap_err()
            .code,
            "proposal_invalid"
        );
        assert_eq!(fs::read(&p).unwrap(), bytes);
    }
    let large = draft(AutomationFamily::Action, &"🧡".repeat(TEXT_BYTES / 4 + 1));
    assert_eq!(
        change(&p, &b, "1", AutomationFamily::Action, "same", Some(large))
            .unwrap_err()
            .code,
        "proposal_limit"
    );
    let mut data = read(&p, &b).unwrap();
    for n in 1..RECORDS {
        let mut d = draft(AutomationFamily::Action, "x");
        d.id = format!("a-{n}");
        data.records.insert(key(d.kind, &d.id), d);
    }
    write(&p, &data).unwrap();
    let before = fs::read(&p).unwrap();
    let mut extra = draft(AutomationFamily::Action, "x");
    extra.id = "extra".into();
    assert_eq!(
        change(&p, &b, "1", extra.kind, "extra", Some(extra))
            .unwrap_err()
            .code,
        "proposal_limit"
    );
    assert_eq!(fs::read(&p).unwrap(), before);
    for d in data.records.values_mut() {
        d.text = "x".repeat(TEXT_BYTES);
    }
    assert_eq!(write(&p, &data).unwrap_err().code, "proposal_limit");
    assert_eq!(fs::read(&p).unwrap(), before);
    for field in ["version", "binding", "revision", "records"] {
        let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        v[field] = match field {
            "version" => json!(99),
            "revision" => json!("9223372036854775808"),
            _ => json!({}),
        };
        if field == "binding" {
            v[field] = json!({"profileId":"profile","origin":"https://host.example","hostId":"host","principalId":"other","sessionId":"device"});
        }
        if field == "records" {
            v[field] = json!({"event:same":draft(AutomationFamily::Action,"x")});
        }
        let raw = serde_json::to_vec(&v).unwrap();
        fs::write(&p, &raw).unwrap();
        assert_eq!(read(&p, &b).err().unwrap().code, "proposal_invalid");
        assert_eq!(fs::read(&p).unwrap(), raw);
    }
}

#[cfg(unix)]
#[test]
fn public_and_linked_saved_or_staging_files_cannot_be_read_replaced_or_removed() {
    use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
    let home = tempfile::tempdir().unwrap();
    let p = path(&home);
    let b = binding();
    change(
        &p,
        &b,
        "0",
        AutomationFamily::Action,
        "same",
        Some(draft(AutomationFamily::Action, "private")),
    )
    .unwrap();
    assert_eq!(fs::metadata(&p).unwrap().mode() & 0o777, 0o600);
    fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(read(&p, &b).is_err());
    fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
    let staging = p.with_extension("proposals.tmp");
    symlink(&p, &staging).unwrap();
    let bytes = fs::read(&p).unwrap();
    assert!(change(&p, &b, "1", AutomationFamily::Action, "same", None).is_err());
    assert!(remove(&p).is_err());
    assert_eq!(fs::read(&p).unwrap(), bytes);
    fs::remove_file(&staging).unwrap();
    fs::hard_link(&p, &staging).unwrap();
    assert!(read(&p, &b).is_err());
    assert!(remove(&p).is_err());
    assert_eq!(fs::read(&p).unwrap(), bytes);
    fs::remove_file(&staging).unwrap();
    remove(&p).unwrap();
}

#[test]
#[ignore = "private subprocess crash fixture"]
fn proposal_crash_child() {
    let p = PathBuf::from(std::env::var_os("TIKTOOLS_PROPOSAL_PATH").unwrap());
    change(
        &p,
        &binding(),
        "1",
        AutomationFamily::Action,
        "same",
        Some(draft(AutomationFamily::Action, "new text")),
    )
    .unwrap();
    panic!("crash boundary was not reached");
}
#[test]
fn death_around_replacement_keeps_one_complete_copy_and_never_promotes_staging() {
    for stage in ["before-rename", "after-rename", "synced"] {
        let home = tempfile::tempdir().unwrap();
        let p = path(&home);
        let b = binding();
        change(
            &p,
            &b,
            "0",
            AutomationFamily::Action,
            "same",
            Some(draft(AutomationFamily::Action, "old text")),
        )
        .unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "remote::profiles::proposals::tests::proposal_crash_child",
            ])
            .env("TIKTOOLS_PROPOSAL_PATH", &p)
            .env("TIKTOOLS_PROPOSAL_CRASH", stage)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(89));
        let data = read(&p, &b).unwrap();
        assert_eq!(
            data.records["action:same"].text,
            if stage == "before-rename" {
                "old text"
            } else {
                "new text"
            }
        );
        let staging = p.with_extension("proposals.tmp");
        assert_eq!(staging.exists(), stage == "before-rename");
        remove(&p).unwrap();
        assert!(!staging.exists());
    }
}
