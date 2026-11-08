use super::*;

fn http() -> HttpEffectGrant {
    HttpEffectGrant {
        origin: "https://hooks.example".into(),
        methods: ["POST".into()].into(),
        paths: ["/live".into()].into(),
        query_fields: ["source".into()].into(),
        header_names: ["content-type".into()].into(),
        render_fields: ["event.data.comment".into()].into(),
        body_fields: ["message".into()].into(),
        allow_text_body: false,
        max_body_bytes: 256,
        max_response_bytes: 1024,
        allow_private_network: false,
    }
}

fn full_grant() -> Grant {
    Grant {
        capabilities: [
            Capability::HttpRequest,
            Capability::AudioPlay,
            Capability::EventsEmit,
            Capability::LiveRead,
        ]
        .into(),
        effects: EffectGrants {
            http: vec![http()],
            audio: vec![AudioEffectGrant {
                file_refs: ["/media/alert.wav".into()].into(),
                output: "host-default".into(),
                max_volume: 0.5,
                overlaps: ["drop".into()].into(),
            }],
            emit_topics: ["moderator.alert".into()].into(),
        },
        ..Default::default()
    }
}

fn request() -> HttpEffectRequest<'static> {
    HttpEffectRequest {
        method: "POST",
        url: "https://hooks.example/live",
        header_names: &[],
        render_fields: &[],
        body: Some(r#"{"message":"hi"}"#),
        response_limit: 512,
        allow_private_network: false,
    }
}

#[test]
fn old_five_field_grants_keep_their_wire_shape_and_never_gain_effects() {
    let value = serde_json::to_value(Grant::viewer()).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 5);
    assert!(value.get("effects").is_none());
    let mut old: Grant = serde_json::from_value(value.clone()).unwrap();
    old.capabilities.insert(Capability::HttpRequest);
    old.capabilities.insert(Capability::AudioPlay);
    old.capabilities.insert(Capability::EventsEmit);
    assert!(!old.allows_http_effect(&request()));
    assert!(!old.allows_audio_effect("/media/alert.wav", "host-default", 0.1, "drop"));
    assert!(!old.allows_emit_topic("moderator.alert"));
    let extended = serde_json::to_value(full_grant()).unwrap();
    assert_eq!(extended.as_object().unwrap().len(), 6);
    assert_eq!(
        serde_json::from_value::<Grant>(extended).unwrap(),
        full_grant()
    );
    let mut unknown = value;
    unknown["effects"] = serde_json::json!({"http":[],"audio":[],"emitTopics":[],"all":true});
    assert!(serde_json::from_value::<Grant>(unknown).is_err());
}

#[test]
fn request_checks_every_final_argument_with_one_whole_rule() {
    let grant = full_grant();
    grant.validate().unwrap();
    assert!(grant.allows_http_effect(&request()));
    for url in [
        "https://other.example/live",
        "https://hooks.example/other",
        "https://hooks.example/live?secret=yes",
        "https://hooks.example/live?source=a&source=b",
        "https://hooks.example/live#fragment",
        "https://user:pass@hooks.example/live",
        "https://hooks.example/a/../live",
        "https://hooks.example/%6cive",
    ] {
        assert!(
            !grant.allows_http_effect(&HttpEffectRequest { url, ..request() }),
            "{url}"
        );
    }
    assert!(grant.allows_http_effect(&HttpEffectRequest {
        url: "https://hooks.example/live?source=chat",
        ..request()
    }));
    for method in ["GET", "post", "CONNECT", "TRACE"] {
        assert!(!grant.allows_http_effect(&HttpEffectRequest {
            method,
            ..request()
        }));
    }
    assert!(!grant.allows_http_effect(&HttpEffectRequest {
        response_limit: 1025,
        ..request()
    }));
    assert!(!grant.allows_http_effect(&HttpEffectRequest {
        response_limit: 0,
        ..request()
    }));
    assert!(!grant.allows_http_effect(&HttpEffectRequest {
        allow_private_network: true,
        ..request()
    }));
    let mut mixed = grant.clone();
    mixed.effects.http[0].paths = ["/other".into()].into();
    let mut second = http();
    second.methods = ["GET".into()].into();
    mixed.effects.http.push(second);
    assert!(!mixed.allows_http_effect(&request()));
}

#[test]
fn headers_render_fields_and_body_fields_are_explicit_and_bounded() {
    let mut grant = full_grant();
    let names = vec!["Content-Type".into()];
    assert!(grant.allows_http_effect(&HttpEffectRequest {
        header_names: &names,
        ..request()
    }));
    for name in [
        "authorization",
        "cookie",
        "x-other",
        "content-type\r\nx-evil",
    ] {
        let names = vec![name.into()];
        assert!(!grant.allows_http_effect(&HttpEffectRequest {
            header_names: &names,
            ..request()
        }));
    }
    grant.effects.http[0]
        .header_names
        .insert("authorization".into());
    assert!(grant.allows_http_effect(&HttpEffectRequest {
        header_names: &["authorization".into()],
        ..request()
    }));
    let fields = vec!["event.data.comment".into()];
    assert!(grant.allows_http_effect(&HttpEffectRequest {
        render_fields: &fields,
        ..request()
    }));
    grant.capabilities.remove(&Capability::LiveRead);
    assert!(!grant.allows_http_effect(&HttpEffectRequest {
        render_fields: &fields,
        ..request()
    }));
    grant.capabilities.insert(Capability::LiveRead);
    assert!(!grant.allows_http_effect(&HttpEffectRequest {
        render_fields: &["event.user".into()],
        ..request()
    }));
    for body in [r#"{"secret":"no"}"#, "text", "[]", "42"] {
        assert!(!grant.allows_http_effect(&HttpEffectRequest {
            body: Some(body),
            ..request()
        }));
    }
    grant.effects.http[0].allow_text_body = true;
    assert!(grant.allows_http_effect(&HttpEffectRequest {
        body: Some("text"),
        ..request()
    }));
    assert!(!grant.allows_http_effect(&HttpEffectRequest {
        body: Some(r#"{"secret":"no"}"#),
        ..request()
    }));
    let large = "a".repeat(257);
    assert!(!grant.allows_http_effect(&HttpEffectRequest {
        body: Some(&large),
        ..request()
    }));
    assert!(grant.allows_http_effect(&HttpEffectRequest {
        body: None,
        ..request()
    }));
}

#[test]
fn invalid_http_restrictions_are_rejected_instead_of_normalized() {
    for origin in [
        "https://hooks.example/",
        "https://HOOKS.example",
        "https://hooks.example:443",
        "https://user@hooks.example",
        "http://hooks.example",
        "https://hooks.example/live",
        "https://hooks.example?x=1",
    ] {
        let mut rule = http();
        rule.origin = origin.into();
        assert!(rule.validate().is_err(), "{origin}");
    }
    for path in [
        "/a/../live",
        "/a/./live",
        "/%2f",
        "/%2e%2e/live",
        "/live?x",
        "/live#x",
        "/a\\b",
        "/*",
        "/{{event.id}}",
        "//other.example/live",
    ] {
        let mut rule = http();
        rule.paths = [path.into()].into();
        assert!(rule.validate().is_err(), "{path}");
    }
    for method in ["post", "CONNECT", "*", "TRACE"] {
        let mut rule = http();
        rule.methods = [method.into()].into();
        assert!(rule.validate().is_err());
    }
    let mut rule = http();
    rule.header_names = ["Content-Type".into()].into();
    assert!(rule.validate().is_err());
    rule = http();
    rule.render_fields = ["globals.token".into()].into();
    assert!(rule.validate().is_err());
    rule = http();
    rule.max_body_bytes = MAX_EFFECT_BODY_BYTES + 1;
    assert!(rule.validate().is_err());
    rule = http();
    rule.max_response_bytes = 0;
    assert!(rule.validate().is_err());
    rule = http();
    rule.max_response_bytes = MAX_EFFECT_RESPONSE_BYTES + 1;
    assert!(rule.validate().is_err());
    rule = http();
    rule.allow_private_network = true;
    rule.origin = "http://127.0.0.1:8080".into();
    rule.validate().unwrap();
    rule.allow_private_network = false;
    assert!(rule.validate().is_err());
}

#[test]
fn audio_and_topics_have_no_missing_scope_or_wildcard_shortcuts() {
    let mut grant = full_grant();
    assert!(grant.allows_audio_effect("/media/alert.wav", "host-default", 0.5, "drop"));
    for (file, output, volume, overlap) in [
        ("/media/other.wav", "host-default", 0.5, "drop"),
        ("/media/alert.wav", "remote", 0.5, "drop"),
        ("/media/alert.wav", "host-default", 0.6, "drop"),
        ("/media/alert.wav", "host-default", f64::NAN, "drop"),
        ("/media/alert.wav", "host-default", 0.5, "allow"),
    ] {
        assert!(!grant.allows_audio_effect(file, output, volume, overlap));
    }
    assert!(grant.allows_emit_topic("moderator.alert"));
    assert!(!grant.allows_emit_topic("moderator.other"));
    for bad in [
        "*",
        "moderator.*",
        "MODERATOR.alert",
        "moderator.alert ",
        "moderator.\nalert",
    ] {
        grant.effects.emit_topics = [bad.into()].into();
        assert!(grant.validate().is_err());
        assert!(!grant.allows_emit_topic(bad));
    }
    for bad in [
        "relative.wav",
        "/media/../alert.wav",
        "/media/*.wav",
        "/media/{{event.id}}",
        "/media/alert\n.wav",
    ] {
        let mut rule = full_grant().effects.audio.remove(0);
        rule.file_refs = [bad.into()].into();
        assert!(rule.validate().is_err());
    }
}
