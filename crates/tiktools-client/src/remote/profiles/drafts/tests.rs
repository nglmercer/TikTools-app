use super::*;
fn draft_path(home: &tempfile::TempDir) -> PathBuf {
    let root = home.path().join("private");
    directory::check_directory(&root, true).unwrap();
    root.join("drafts")
}
fn binding() -> Binding {
    Binding {
        profile_id: "profile".into(),
        origin: "https://host.example".into(),
        host_id: "host".into(),
        principal_id: "moderator".into(),
        session_id: "device".into(),
    }
}
fn draft(text: &str) -> Draft {
    Draft {
        id: "welcome".into(),
        base_revision: "9007199254740993".into(),
        text: text.into(),
        updated_at: 1,
    }
}

#[test]
fn explicit_drafts_preserve_arbitrary_text_base_revision_and_delete_recreate_fences() {
    let home = tempfile::tempdir().unwrap();
    let path = draft_path(&home);
    let b = binding();
    assert!(!path.exists());
    assert_eq!(read(&path, &b).unwrap().revision, "0");
    assert!(!path.exists());
    let text = "an unfinished edit\nwith unicode 🧡 and a fixture token";
    assert_eq!(
        change(&path, &b, "0", "welcome", Some(draft(text))).unwrap(),
        "1"
    );
    let data = read(&path, &b).unwrap();
    assert_eq!(data.records["welcome"].text, text);
    assert_eq!(data.records["welcome"].base_revision, "9007199254740993");
    assert_eq!(
        change(&path, &b, "0", "welcome", Some(draft("stale")))
            .unwrap_err()
            .code,
        "draft_conflict"
    );
    assert_eq!(change(&path, &b, "1", "welcome", None).unwrap(), "2");
    assert!(read(&path, &b).unwrap().records.is_empty());
    assert_eq!(
        change(&path, &b, "0", "welcome", Some(draft("stale initial page")))
            .unwrap_err()
            .code,
        "draft_conflict"
    );
    assert_eq!(
        change(&path, &b, "2", "welcome", Some(draft("new"))).unwrap(),
        "3"
    );
    let mut other = b.clone();
    other.session_id = "other".into();
    assert_eq!(read(&path, &other).err().unwrap().code, "draft_invalid");
    let bytes = fs::read(&path).unwrap();
    let mut large = draft(&"🧡".repeat(TEXT_BYTES / 4 + 1));
    large.id = "welcome".into();
    assert!(change(&path, &b, "3", "welcome", Some(large)).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(
        !format!("{:?}", change(&path, &other, "3", "welcome", None).err())
            .contains("fixture token")
    );
}

#[test]
fn malformed_oversized_or_exhausted_storage_never_evicts_a_saved_draft() {
    let home = tempfile::tempdir().unwrap();
    let path = draft_path(&home);
    let b = binding();
    change(&path, &b, "0", "welcome", Some(draft("retained"))).unwrap();
    let mut data = read(&path, &b).unwrap();
    for n in 1..RECORDS {
        let mut d = draft("x");
        d.id = format!("d-{n}");
        data.records.insert(d.id.clone(), d);
    }
    write(&path, &data).unwrap();
    let before = fs::read(&path).unwrap();
    let mut extra = draft("new");
    extra.id = "extra".into();
    assert_eq!(
        change(&path, &b, "1", "extra", Some(extra))
            .unwrap_err()
            .code,
        "draft_limit"
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    for d in data.records.values_mut() {
        d.text = "x".repeat(TEXT_BYTES);
    }
    assert_eq!(write(&path, &data).unwrap_err().code, "draft_limit");
    assert_eq!(fs::read(&path).unwrap(), before);
    for raw in [b"invalid".as_slice(), br#"{"version":99}"#] {
        fs::write(&path, raw).unwrap();
        assert_eq!(read(&path, &b).err().unwrap().code, "draft_invalid");
        assert_eq!(fs::read(&path).unwrap(), raw);
    }
    let f = options().write(true).open(&path).unwrap();
    f.set_len(FILE_BYTES + 1).unwrap();
    assert!(read(&path, &b).is_err());
}

#[cfg(unix)]
#[test]
fn draft_paths_reject_public_files_symlinks_and_hardlinks_without_touching_targets() {
    use std::os::unix::{
        fs::{symlink, PermissionsExt},
        prelude::MetadataExt,
    };
    let home = tempfile::tempdir().unwrap();
    let path = draft_path(&home);
    let b = binding();
    change(&path, &b, "0", "welcome", Some(draft("private"))).unwrap();
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(read(&path, &b).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let link = home.path().join("link");
    symlink(&path, &link).unwrap();
    assert!(read(&link, &b).is_err());
    assert!(remove(&link).is_err());
    fs::remove_file(&link).unwrap();
    fs::hard_link(&path, &link).unwrap();
    assert!(read(&link, &b).is_err());
    assert!(remove(&link).is_err());
    fs::remove_file(&link).unwrap();
    assert_eq!(read(&path, &b).unwrap().records["welcome"].text, "private");
    remove(&path).unwrap();
    assert!(!path.exists());
}

#[test]
#[ignore = "subprocess-only draft crash fixture"]
fn draft_crash_child() {
    let path = PathBuf::from(std::env::var_os("TIKTOOLS_DRAFT_PATH").unwrap());
    change(&path, &binding(), "1", "welcome", Some(draft("new text"))).unwrap();
    panic!("child did not terminate at the requested persistence boundary");
}
#[test]
fn process_death_before_and_after_atomic_replacement_preserves_one_complete_saved_copy() {
    for stage in ["before-rename", "after-rename", "synced"] {
        let home = tempfile::tempdir().unwrap();
        let path = draft_path(&home);
        let b = binding();
        change(&path, &b, "0", "welcome", Some(draft("old text"))).unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "remote::profiles::drafts::tests::draft_crash_child",
                "--ignored",
            ])
            .env("TIKTOOLS_DRAFT_PATH", &path)
            .env("TIKTOOLS_DRAFT_CRASH", stage)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(88));
        let data = read(&path, &b).unwrap();
        assert_eq!(
            data.records["welcome"].text,
            if stage == "before-rename" {
                "old text"
            } else {
                "new text"
            }
        );
        assert_eq!(
            data.revision,
            if stage == "before-rename" { "1" } else { "2" }
        );
        // Repeated crashes reuse one private staging file; explicit removal
        // cleans both it and the saved copy without promoting staged text.
        if stage == "before-rename" {
            assert!(path.with_extension("drafts.tmp").exists());
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "remote::profiles::drafts::tests::draft_crash_child",
                    "--ignored",
                ])
                .env("TIKTOOLS_DRAFT_PATH", &path)
                .env("TIKTOOLS_DRAFT_CRASH", stage)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(88));
            assert_eq!(
                std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
                2
            );
        }
        remove(&path).unwrap();
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            0
        );
    }
}
