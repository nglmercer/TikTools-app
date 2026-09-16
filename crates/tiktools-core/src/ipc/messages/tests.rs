use super::*;

#[test]
fn preserves_existing_camel_case_contract() {
    let message = PageMessage::parse(
        r#"{"type":"connect","uniqueId":"creator","sessionCookie":"sid","roomId":"room"}"#,
    )
    .unwrap();
    assert_eq!(message.type_name(), "connect");
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("uniqueId"));
    assert!(json.contains("sessionCookie"));
}

#[test]
fn rejects_unknown_discriminator_and_bad_settings() {
    assert!(PageMessage::parse(r#"{"type":"not-a-message"}"#).is_err());
    assert!(PageMessage::parse(
        r#"{"type":"save-plugin-settings","id":"demo","values":{"nested":{}}}"#,
    )
    .is_err());
    assert!(PageMessage::parse(&format!(
        r#"{{"type":"save-plugin-settings","id":"demo","values":{{"value":"{}"}}}}"#,
        "x".repeat(4_097)
    ))
    .is_err());
}

#[test]
fn analytics_summary_has_a_dedicated_wire_contract() {
    let message = PageMessage::parse(
        r#"{"type":"get-analytics-summary","creatorUniqueId":"creator","startDay":20275,"endDay":20281,"limit":10}"#,
    )
    .unwrap();
    assert_eq!(message.type_name(), "get-analytics-summary");
    assert!(matches!(
        message,
        PageMessage::GetAnalyticsSummary {
            creator_unique_id: Some(_),
            start_day: Some(20275),
            end_day: Some(20281),
            limit: Some(10),
        }
    ));
    assert!(
        PageMessage::parse(r#"{"type":"get-analytics-summary","creatorUniqueId":""}"#).is_err()
    );

    let json = HostMessage::AnalyticsSummary {
        summary: serde_json::json!({"creatorUniqueId": "creator"}),
    }
    .to_json()
    .unwrap();
    assert!(json.starts_with(r#"{"type":"analytics-summary""#));
    assert!(json.contains("creatorUniqueId"));
}

#[test]
fn host_message_has_wire_type() {
    let json = HostMessage::PointsConfig {
        config: PointsConfig::default(),
    }
    .to_json()
    .unwrap();
    assert!(json.starts_with(r#"{"type":"points-config""#));
    assert!(json.contains("pointsPerCoin"));
}

#[test]
fn media_picker_preserves_the_public_json_contract() {
    let message = PageMessage::parse(
        r#"{"type":"open-media-picker","requestId":"media-1","mode":"file","kind":"audio","extensions":["wav","mp3"]}"#,
    )
    .unwrap();
    assert_eq!(message.type_name(), "open-media-picker");
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("requestId"));
    assert!(!json.contains("initialDirectory"));

    let response = HostMessage::MediaSelected {
        request_id: "media-1".to_owned(),
        selection: None,
        error: None,
    }
    .to_json()
    .unwrap();
    assert_eq!(
        response,
        r#"{"type":"media-selected","requestId":"media-1"}"#
    );
}

#[test]
fn install_plugin_package_round_trips_and_validates_path() {
    let message = PageMessage::parse(
        r#"{"type":"install-plugin-package","path":"C:\\Temp\\demo.plugin","replaceExisting":false}"#,
    )
    .unwrap();
    assert_eq!(message.type_name(), "install-plugin-package");
    match message {
        PageMessage::InstallPluginPackage {
            path,
            replace_existing,
        } => {
            assert_eq!(path, r"C:\Temp\demo.plugin");
            assert!(!replace_existing);
        }
        other => panic!("unexpected message: {other:?}"),
    }
    let with_replace = PageMessage::parse(
        r#"{"type":"install-plugin-package","path":"/tmp/demo.plugin","replaceExisting":true}"#,
    )
    .unwrap();
    match with_replace {
        PageMessage::InstallPluginPackage {
            replace_existing, ..
        } => assert!(replace_existing),
        other => panic!("unexpected message: {other:?}"),
    }
    // replaceExisting defaults to false so the first install never
    // silently replaces an existing plugin.
    let defaulted =
        PageMessage::parse(r#"{"type":"install-plugin-package","path":"/tmp/demo.plugin"}"#)
            .unwrap();
    match defaulted {
        PageMessage::InstallPluginPackage {
            replace_existing, ..
        } => assert!(!replace_existing),
        other => panic!("unexpected message: {other:?}"),
    }

    assert!(PageMessage::parse(r#"{"type":"install-plugin-package","path":""}"#).is_err());
    assert!(PageMessage::parse(r#"{"type":"install-plugin-package","path":"a\0b"}"#,).is_err());
    let oversized = "x".repeat(MAX_PLUGIN_PACKAGE_PATH_LEN + 1);
    assert!(PageMessage::parse(&format!(
        r#"{{"type":"install-plugin-package","path":"{oversized}"}}"#
    ))
    .is_err());
}

#[test]
fn plugin_install_result_uses_structured_codes() {
    let success = HostMessage::plugin_install_success("demo".to_owned(), "1.0.0".to_owned(), false)
        .to_json()
        .unwrap();
    assert!(success.contains(r#""type":"plugin-install-result""#));
    assert!(success.contains(r#""success":true"#));
    assert!(success.contains(r#""id":"demo""#));

    let failure = HostMessage::plugin_install_failure(
        PluginInstallErrorCode::AlreadyInstalled,
        "plugin is already installed: demo".to_owned(),
    )
    .to_json()
    .unwrap();
    assert!(failure.contains(r#""success":false"#));
    assert!(failure.contains("already-installed"));

    assert_eq!(
        classify_plugin_install_error("plugin is already installed: demo"),
        PluginInstallErrorCode::AlreadyInstalled
    );
    assert_eq!(
        classify_plugin_install_error("plugin manifest field `id` is invalid"),
        PluginInstallErrorCode::InvalidPackage
    );
    assert_eq!(
        classify_plugin_install_error("plugin manifest field `protocolVersion` is invalid"),
        PluginInstallErrorCode::Incompatible
    );
    assert_eq!(
        classify_plugin_install_error("checksum mismatch in demo: index.js"),
        PluginInstallErrorCode::InvalidPackage
    );
}

#[test]
fn plugin_progress_message_preserves_optional_progress() {
    let message = HostMessage::PluginProgress {
        plugin_id: "sonicboom.tts".to_owned(),
        state: PluginProgressState::Downloading,
        progress: Some(0.5),
        message: "Downloading model: 50%.".to_owned(),
    }
    .to_json()
    .unwrap();
    assert_eq!(
        message,
        r#"{"type":"plugin-progress","pluginId":"sonicboom.tts","state":"downloading","progress":0.5,"message":"Downloading model: 50%."}"#
    );
}

#[test]
fn uninstall_plugin_package_has_a_dedicated_wire_contract() {
    let message = PageMessage::parse(r#"{"type":"uninstall-plugin-package","id":"demo"}"#).unwrap();
    assert_eq!(message.type_name(), "uninstall-plugin-package");
    assert!(matches!(message, PageMessage::UninstallPluginPackage { id } if id == "demo"));

    let success = HostMessage::plugin_uninstall_success("demo".to_owned())
        .to_json()
        .unwrap();
    assert_eq!(
        success,
        r#"{"type":"plugin-uninstall-result","success":true,"id":"demo"}"#
    );
}
