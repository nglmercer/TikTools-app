use super::*;
use ring::signature::{UnparsedPublicKey, ED25519};

#[test]
fn canonical_json_sorts_keys_recursively() {
    let value = json!({"b": 1, "a": {"d": [true, null], "c": "x"}});
    assert_eq!(
        canonical_json(&value),
        r#"{"a":{"c":"x","d":[true,null]},"b":1}"#
    );
}

#[test]
fn proofs_verify_only_for_the_exact_challenge() {
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let key = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let nonce = [7u8; 32];
    let message = proof_message("enroll", &nonce, "abc");
    let signature = key.sign(&message);
    let public = UnparsedPublicKey::new(&ED25519, key.public_key().as_ref());
    assert!(public.verify(&message, signature.as_ref()).is_ok());
    let other = proof_message("ticket", &nonce, "abc");
    assert!(public.verify(&other, signature.as_ref()).is_err());
}

#[test]
fn identifiers_roles_and_labels_are_bounded() {
    assert!(valid_uuid("3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c").is_ok());
    assert!(valid_uuid("../hosts").is_err());
    assert!(one_of("admin", PRESET_ROLES, "role").is_err());
    assert!(bounded_text("  Team  ", MAX_NAME_CHARS, "name").is_ok());
    assert!(bounded_text("bad\u{0007}", MAX_NAME_CHARS, "name").is_err());
}

fn scratch_teams(name: &str) -> (CloudTeams, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "teams-{name}-{}-{}",
        std::process::id(),
        fastrand::u64(..)
    ));
    let paths = AppPaths {
        root: dir.clone(),
        data: dir.join("data"),
        plugins: dir.join("plugins"),
        plugin_data: dir.join("plugin-data"),
        builtin_plugins: dir.join("builtin"),
        development_plugins: None,
        logs: dir.join("logs"),
        temp: dir.join("temp"),
    };
    (CloudTeams::new(&paths), dir)
}

const HOST: &str = "3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c";
const DEVICE_A: &str = "11111111-2222-4333-8444-555555555555";
const DEVICE_B: &str = "66666666-7777-4888-9999-aaaaaaaaaaaa";

#[test]
fn device_key_is_created_once_and_reused() {
    let (teams, dir) = scratch_teams("key");
    let first = teams.device_key().unwrap();
    let second = teams.device_key().unwrap();
    assert_eq!(first.public_key().as_ref(), second.public_key().as_ref());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn signed_peer_operations_do_not_replace_a_missing_or_corrupt_enrolled_key() {
    let (teams, dir) = scratch_teams("enrolled-key");
    assert!(teams.enrolled_device_key().is_err());
    assert!(!teams.key_path.exists());
    let public = teams.device_key().unwrap().public_key().as_ref().to_vec();
    assert_eq!(
        teams.enrolled_device_key().unwrap().public_key().as_ref(),
        public
    );
    fs::remove_file(&teams.key_path).unwrap();
    assert!(teams.enrolled_device_key().is_err());
    assert!(!teams.key_path.exists());
    fs::write(&teams.key_path, b"corrupt enrolled identity").unwrap();
    assert!(teams.enrolled_device_key().is_err());
    assert_eq!(
        fs::read(&teams.key_path).unwrap(),
        b"corrupt enrolled identity"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_losing_concurrent_creation_loads_the_winning_key() {
    let (teams, dir) = scratch_teams("race");
    let winner = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
    // The loser decided to create, then found the winner's file in place.
    private_file::create_exclusive(&teams.key_path, winner.as_ref()).unwrap();
    let loser = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
    let error = private_file::create_exclusive(&teams.key_path, loser.as_ref()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    let loaded = teams.winning_key().unwrap();
    let expected = Ed25519KeyPair::from_pkcs8(winner.as_ref()).unwrap();
    assert_eq!(loaded.public_key().as_ref(), expected.public_key().as_ref());
    assert_eq!(fs::read(&teams.key_path).unwrap(), winner.as_ref());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_invalid_key_is_an_explicit_error_and_is_never_replaced() {
    let (teams, dir) = scratch_teams("invalid");
    fs::create_dir_all(teams.key_path.parent().unwrap()).unwrap();
    fs::write(&teams.key_path, b"not a pkcs8 key").unwrap();
    let error = teams.device_key().err().unwrap();
    assert_eq!(error.message, "device key is invalid");
    assert_eq!(fs::read(&teams.key_path).unwrap(), b"not a pkcs8 key");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn saving_records_twice_works_and_keeps_the_device_identity() {
    let (teams, dir) = scratch_teams("records");
    let identity = teams.device_key().unwrap().public_key().as_ref().to_vec();
    let key_bytes = fs::read(&teams.key_path).unwrap();

    teams.save_host_binding(HOST, DEVICE_A).unwrap();
    teams.save_host_binding(HOST, DEVICE_B).unwrap();
    assert_eq!(
        teams.host_binding().unwrap(),
        Some((HOST.to_owned(), DEVICE_B.to_owned()))
    );
    teams.save_controller_device(DEVICE_A).unwrap();
    teams.save_controller_device(DEVICE_B).unwrap();
    assert_eq!(teams.controller_device().unwrap(), DEVICE_B);

    assert_eq!(fs::read(&teams.key_path).unwrap(), key_bytes);
    assert_eq!(
        teams.device_key().unwrap().public_key().as_ref(),
        identity.as_slice()
    );
    let leftovers = fs::read_dir(teams.key_path.parent().unwrap())
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(leftovers, 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_invalid_record_is_rejected_before_the_previous_one_is_touched() {
    let (teams, dir) = scratch_teams("validate");
    teams.save_host_binding(HOST, DEVICE_A).unwrap();
    assert!(teams.save_host_binding(HOST, "../device").is_err());
    assert!(teams.save_controller_device("nope").is_err());
    assert_eq!(
        teams.host_binding().unwrap(),
        Some((HOST.to_owned(), DEVICE_A.to_owned()))
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_replacement_keeps_the_previous_record() {
    let (teams, dir) = scratch_teams("blocked");
    teams.save_host_binding(HOST, DEVICE_A).unwrap();
    // Replace the binding file by a directory-shaped obstacle only after
    // saving once: use a record path whose parent is a regular file.
    let blocked = CloudTeams {
        key_path: teams.key_path.clone(),
        binding_path: teams.binding_path.join("child.json"),
        controller_path: teams.controller_path.clone(),
    };
    assert!(blocked.save_host_binding(HOST, DEVICE_B).is_err());
    assert_eq!(
        teams.host_binding().unwrap(),
        Some((HOST.to_owned(), DEVICE_A.to_owned()))
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn teams_failures_use_teams_wording_and_keep_only_allowed_diagnostics() {
    let error = map_failure(
        StatusCode::NOT_FOUND,
        br#"{"code":"notFound","requestId":"r-1","internal":"db password"}"#,
    );
    assert_eq!(error.code, "not_found");
    assert_eq!(error.message, "team item not found");
    assert_eq!(
        error.data,
        Some(json!({"cloud": {"code": "notFound", "requestId": "r-1"}}))
    );
    assert_eq!(map_failure(StatusCode::CONFLICT, b"").code, "conflict");
}

#[test]
fn local_host_identity_requires_both_binding_ids_and_ignores_server_flags() {
    let device = json!({"id": "local-device", "hostId": "local-host", "kind": "host", "status": "active", "label": "PC", "isThisDevice": true});
    let mut list: TeamDeviceList = serde_json::from_value(json!({"items": [device]})).unwrap();
    mark_local_host_devices(&mut list, None);
    assert!(!list.items[0].is_this_device);
    mark_local_host_devices(
        &mut list,
        Some(&("other-host".into(), "local-device".into())),
    );
    assert!(!list.items[0].is_this_device);
    mark_local_host_devices(
        &mut list,
        Some(&("local-host".into(), "local-device".into())),
    );
    assert!(list.items[0].is_this_device);
    list.items[0].kind = "controller".into();
    mark_local_host_devices(
        &mut list,
        Some(&("local-host".into(), "local-device".into())),
    );
    assert!(!list.items[0].is_this_device);
}
