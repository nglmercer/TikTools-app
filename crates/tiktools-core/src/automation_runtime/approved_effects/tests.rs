use super::*;

#[test]
fn audio_present_options_never_silently_fall_back_to_defaults() {
    let empty = serde_json::Map::new();
    let (volume, overlap, kind) = approved_audio_options(&empty).unwrap();
    assert_eq!(volume, 1.0);
    assert_eq!(overlap, "allow");
    assert_eq!(kind, AudioOverlap::Allow);
    for volume in [
        json!(null),
        json!(false),
        json!("malformed"),
        json!("NaN"),
        json!("inf"),
        json!(-0.1),
        json!(1.01),
        json!({}),
    ] {
        assert!(approved_audio_options(json!({"volume":volume}).as_object().unwrap()).is_err());
    }
    for overlap in [
        json!(null),
        json!(false),
        json!(1),
        json!("unknown"),
        json!({}),
    ] {
        assert!(approved_audio_options(json!({"overlap":overlap}).as_object().unwrap()).is_err());
    }
    let options = json!({"volume":"0.5","overlap":"restart"});
    let (volume, overlap, kind) = approved_audio_options(options.as_object().unwrap()).unwrap();
    assert_eq!(
        (volume, overlap, kind),
        (0.5, "restart", AudioOverlap::Restart)
    );
}

#[test]
fn audio_canonical_target_must_match_every_subject_file_and_option_scope() {
    use crate::collaboration::{AudioEffectGrant, Capability, Grant};
    let root =
        std::env::temp_dir().join(format!("tiktools-audio-pathscope-{}", fastrand::u128(..)));
    std::fs::create_dir_all(&root).unwrap();
    for name in ["wanted.wav", "other.wav"] {
        std::fs::write(root.join(name), b"audio fixture").unwrap();
    }
    let wanted = crate::services::audio_file_ref_from_config("wanted.wav", &root).unwrap();
    let other = crate::services::audio_file_ref_from_config("other.wav", &root).unwrap();
    let mut allowed = Grant::default();
    allowed.capabilities.insert(Capability::AudioPlay);
    allowed.effects.audio.push(AudioEffectGrant {
        file_refs: [wanted.path.clone()].into(),
        output: "host-default".into(),
        max_volume: 0.5,
        overlaps: ["drop".into()].into(),
    });
    let mut only_other = allowed.clone();
    only_other.effects.audio[0].file_refs = [other.path.clone()].into();
    assert!(
        check_approved_audio_scope(&wanted.path, &wanted.path, &[&allowed], 0.5, "drop").is_ok()
    );
    assert!(
        check_approved_audio_scope(&wanted.path, &wanted.path, &[&only_other], 0.5, "drop")
            .is_err()
    );
    assert!(check_approved_audio_scope(
        &wanted.path,
        &wanted.path,
        &[&allowed, &only_other],
        0.5,
        "drop"
    )
    .is_err());
    assert!(
        check_approved_audio_scope(&wanted.path, &other.path, &[&only_other], 0.5, "drop").is_err()
    );
    assert!(
        check_approved_audio_scope(&wanted.path, &wanted.path, &[&allowed], 0.6, "drop").is_err()
    );
    assert!(
        check_approved_audio_scope(&wanted.path, &wanted.path, &[&allowed], 0.5, "restart")
            .is_err()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "http")]
#[test]
fn dns_destination_checks_reject_any_private_mapped_or_multicast_address() {
    let public: std::net::SocketAddr = "8.8.8.8:443".parse().unwrap();
    let public_v6: std::net::SocketAddr = "[2606:4700:4700::1111]:443".parse().unwrap();
    assert!(validate_effect_addresses(&[public, public_v6], false).is_ok());
    assert!(validate_effect_addresses(&[], false).is_err());
    assert!(validate_effect_addresses(&vec![public; 65], false).is_err());
    for blocked in [
        "127.0.0.1:443",
        "10.0.0.1:443",
        "0.0.0.0:443",
        "100.64.0.1:443",
        "169.254.169.254:443",
        "172.16.0.1:443",
        "192.168.0.1:443",
        "224.0.0.1:443",
        "255.255.255.255:443",
        "[::1]:443",
        "[::]:443",
        "[fc00::1]:443",
        "[fe80::1]:443",
        "[ff02::1]:443",
        "[::ffff:127.0.0.1]:443",
        "[::ffff:10.0.0.1]:443",
        "[::ffff:169.254.169.254]:443",
    ] {
        let blocked: std::net::SocketAddr = blocked.parse().unwrap();
        assert!(
            validate_effect_addresses(&[public, blocked], false).is_err(),
            "{blocked}"
        );
        assert!(
            validate_effect_addresses(&[blocked], true).is_ok(),
            "explicit private network scope: {blocked}"
        );
    }
}
