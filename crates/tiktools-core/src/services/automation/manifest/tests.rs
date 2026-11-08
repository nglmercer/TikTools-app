use super::*;
use serde_json::json;

fn provenance(principal: Option<(&str, &str)>, generation: &str) -> Value {
    let authority = match principal {
        Some((principal, session)) => {
            json!({"kind":"delegated","principalId":principal,"sessionId":session})
        }
        None => json!({"kind":"owner"}),
    };
    json!({"authority":authority,"generation":generation})
}
fn version(
    kind: AutomationFamily,
    id: &str,
    revision: &str,
) -> crate::collaboration::ResourceVersion {
    crate::collaboration::ResourceVersion {
        kind: ResourceKind::Automation,
        scope: kind.scope().into(),
        id: id.into(),
        revision: revision.into(),
    }
}
fn action(id: &str, type_id: &str, config: Value) -> Value {
    json!({"id":id,"typeId":type_id,"enabled":false,"config":config})
}
fn action_review(record: Value, author: Option<(&str, &str)>) -> Versioned<Value> {
    let id = record["id"].as_str().unwrap().to_owned();
    Versioned::new(
        json!({"kind":"action","id":id,"record":record,"provenance":provenance(author,"1"),"dependencies":[]}),
        vec![version(AutomationFamily::Action, &id, "1")],
    )
}
fn event_review(actions: &[Value]) -> Versioned<Value> {
    let ids: Vec<_> = actions.iter().map(|a| a["id"].clone()).collect();
    let dependencies: Vec<_> = actions
        .iter()
        .map(|a| json!({"id":a["id"],"record":a,"provenance":provenance(None,"1")}))
        .collect();
    let mut versions = vec![version(AutomationFamily::Event, "event", "1")];
    versions.extend(
        actions
            .iter()
            .map(|a| version(AutomationFamily::Action, a["id"].as_str().unwrap(), "1")),
    );
    Versioned::new(
        json!({"kind":"event","id":"event","record":{"id":"event","enabled":false,"trigger":"tiktok.chat","actionIds":ids,"runMode":"random"},"provenance":provenance(None,"1"),"dependencies":dependencies}),
        versions,
    )
}
fn owner(review: &Versioned<Value>) -> ExecutionManifest {
    build(
        review,
        ManifestSubjects {
            host_id: "host",
            subjects: &[],
        },
    )
    .unwrap()
}
fn grant() -> Grant {
    Grant {
        capabilities: [
            Capability::AutomationRead,
            Capability::AutomationToggle,
            Capability::PointsRead,
            Capability::PointsAdjust,
        ]
        .into(),
        ..Grant::default()
    }
}

fn effect_grant() -> Grant {
    let mut grant = grant();
    grant.capabilities.extend([
        Capability::LiveRead,
        Capability::HttpRequest,
        Capability::AudioPlay,
        Capability::EventsEmit,
        Capability::ScriptsExecute,
    ]);
    grant.effects = serde_json::from_value(json!({
        "http":[{"origin":"https://hooks.example","methods":["POST"],"paths":["/live"],"queryFields":[],"headerNames":[],"renderFields":["event.data.comment"],"bodyFields":["message"],"allowTextBody":false,"maxBodyBytes":1024,"maxResponseBytes":1024,"allowPrivateNetwork":false}],
        "audio":[{"fileRefs":["/media/alert.wav"],"output":"host-default","maxVolume":0.5,"overlaps":["drop"]}],
        "emitTopics":["moderator.alert"]
    })).unwrap();
    grant
}

fn delegated_effect(record: Value, grant: &Grant) -> ExecutionManifest {
    build(
        &action_review(record, Some(("a", "s-a"))),
        ManifestSubjects {
            host_id: "host",
            subjects: &[ManifestSubject {
                principal_id: "a",
                session_id: "s-a",
                grant_revision: "1",
                configuration_revision: "1",
                grant,
            }],
        },
    )
    .unwrap()
}

#[test]
fn scoped_custom_trigger_requires_every_authors_exact_topic_and_capability() {
    let mut review = event_review(&[action("child", "core.log", json!({"message":"scoped"}))]);
    review.value["record"]["trigger"] = json!("moderator.alert");
    review.value["provenance"] = provenance(Some(("a", "s-a")), "1");
    review.value["dependencies"][0]["provenance"] = provenance(Some(("b", "s-b")), "1");
    let first = effect_grant();
    let mut second = effect_grant();
    let capture = |second: &Grant| {
        build(
            &review,
            ManifestSubjects {
                host_id: "host",
                subjects: &[
                    ManifestSubject {
                        principal_id: "a",
                        session_id: "s-a",
                        grant_revision: "1",
                        configuration_revision: "1",
                        grant: &first,
                    },
                    ManifestSubject {
                        principal_id: "b",
                        session_id: "s-b",
                        grant_revision: "1",
                        configuration_revision: "1",
                        grant: second,
                    },
                ],
            },
        )
        .unwrap()
    };
    let allowed = capture(&second);
    assert_eq!(allowed.reasons, [ManifestReason::ApprovalRequired]);
    assert!(!allowed.execution_authorized);
    second.effects.emit_topics.clear();
    assert!(capture(&second)
        .reasons
        .contains(&ManifestReason::ResourceForbidden));
    second = effect_grant();
    second.capabilities.remove(&Capability::EventsEmit);
    assert!(capture(&second)
        .reasons
        .contains(&ManifestReason::MissingCapability));
    review.value["record"]["trigger"] = json!("MODERATOR.alert");
    let malformed = build(
        &review,
        ManifestSubjects {
            host_id: "host",
            subjects: &[
                ManifestSubject {
                    principal_id: "a",
                    session_id: "s-a",
                    grant_revision: "1",
                    configuration_revision: "1",
                    grant: &first,
                },
                ManifestSubject {
                    principal_id: "b",
                    session_id: "s-b",
                    grant_revision: "1",
                    configuration_revision: "1",
                    grant: &first,
                },
            ],
        },
    )
    .unwrap();
    assert!(malformed
        .reasons
        .contains(&ManifestReason::InvalidConfiguration));
}

#[test]
fn scoped_script_source_and_emit_payload_have_explicit_admission_bounds() {
    let grant = effect_grant();
    let oversized = delegated_effect(
        action(
            "script",
            "core.code",
            json!({"source":"x".repeat(crate::services::SCOPED_SCRIPT_LIMITS.source_bytes + 1)}),
        ),
        &grant,
    );
    assert!(oversized.effects[0]
        .reasons
        .contains(&ManifestReason::InvalidConfiguration));
    let emit = delegated_effect(
        action(
            "emit",
            "core.emit",
            json!({"type":"MODERATOR.ALERT","data":{"message":"{{event.data.comment}}"}}),
        ),
        &grant,
    );
    assert_eq!(emit.effects[0].decision, EffectDecision::Classified);
    assert!(emit.effects[0]
        .required_capabilities
        .contains(&Capability::LiveRead));
    assert!(emit.effects[0]
        .required_capabilities
        .contains(&Capability::EventsEmit));
    let oversized = delegated_effect(
        action(
            "emit",
            "core.emit",
            json!({"type":"moderator.alert","data":{"message":"x".repeat(64 * 1024)}}),
        ),
        &grant,
    );
    assert!(oversized.effects[0]
        .reasons
        .contains(&ManifestReason::InvalidConfiguration));
    let nested = delegated_effect(
        action(
            "emit",
            "core.emit",
            json!({"type":"moderator.alert","data":{"nested":[{"message":"{{event.data.comment}}"}]}}),
        ),
        &grant,
    );
    assert_eq!(nested.effects[0].render_fields, ["event.data.comment"]);
    assert!(nested.effects[0]
        .required_capabilities
        .contains(&Capability::LiveRead));
    let nested_global = delegated_effect(
        action(
            "emit",
            "core.emit",
            json!({"type":"moderator.alert","data":{"nested":[{"message":"{{globals.secret}}"}]}}),
        ),
        &grant,
    );
    assert!(nested_global.effects[0]
        .reasons
        .contains(&ManifestReason::GlobalAccessRequiresPolicy));
}

#[test]
fn external_effects_require_their_current_capabilities_and_exact_typed_scopes() {
    let grant = effect_grant();
    let http = action(
        "http",
        "core.fetch",
        json!({"url":"https://hooks.example/live","body":{"message":"hello"},"maxResponseBytes":1024}),
    );
    let manifest = delegated_effect(http.clone(), &grant);
    assert_eq!(manifest.effects[0].decision, EffectDecision::Classified);
    assert_eq!(
        manifest.effects[0].required_capabilities,
        [Capability::HttpRequest]
    );
    assert!(!manifest.execution_authorized);
    let mut narrow = grant.clone();
    narrow.effects.http.clear();
    assert!(delegated_effect(http.clone(), &narrow).effects[0]
        .reasons
        .contains(&ManifestReason::ResourceForbidden));
    narrow = grant.clone();
    narrow.capabilities.remove(&Capability::HttpRequest);
    assert!(delegated_effect(http, &narrow).effects[0]
        .reasons
        .contains(&ManifestReason::MissingCapability));
    let wrong = action(
        "http",
        "core.fetch",
        json!({"url":"https://other.example/live"}),
    );
    assert!(delegated_effect(wrong, &grant).effects[0]
        .reasons
        .contains(&ManifestReason::ResourceForbidden));
    let response_emit = action(
        "http",
        "core.fetch",
        json!({"url":"https://hooks.example/live","emitResponseAs":"moderator.alert"}),
    );
    assert!(delegated_effect(response_emit, &grant).effects[0]
        .reasons
        .contains(&ManifestReason::EffectPolicyUnavailable));
    let audio = delegated_effect(
        action(
            "audio",
            "audio.play",
            json!({"fileRef":{"path":"/media/alert.wav"},"volume":0.5,"overlap":"drop"}),
        ),
        &grant,
    );
    assert_eq!(
        audio.effects[0].required_capabilities,
        [Capability::AudioPlay]
    );
    let emit = delegated_effect(
        action("emit", "core.emit", json!({"type":"moderator.other"})),
        &grant,
    );
    assert!(emit.effects[0]
        .reasons
        .contains(&ManifestReason::ResourceForbidden));
    let script = delegated_effect(
        action(
            "script",
            "core.code",
            json!({"source":"return { logs: ['reviewed'] };"}),
        ),
        &grant,
    );
    assert!(script.effects[0]
        .required_capabilities
        .contains(&Capability::ScriptsExecute));
    assert!(script.effects[0]
        .required_capabilities
        .contains(&Capability::LiveRead));
    assert!(script.effects[0]
        .reasons
        .contains(&ManifestReason::OpaqueScript));
}

#[test]
fn templated_http_arguments_keep_final_sink_checks_without_literal_template_denials() {
    let grant = effect_grant();
    for config in [
        json!({"url":"https://hooks.example/live","body":"{\"message\":\"{{event.data.comment}}\"}"}),
        json!({"url":"https://hooks.example/{{event.data.comment}}"}),
    ] {
        let manifest = delegated_effect(action("http", "core.fetch", config), &grant);
        assert!(!manifest.effects[0]
            .reasons
            .contains(&ManifestReason::ResourceForbidden));
        assert!(manifest.effects[0]
            .required_capabilities
            .contains(&Capability::LiveRead));
        assert!(!manifest.execution_authorized);
    }
    assert_eq!(effect_render_fields(&json!({"url":"https://hooks.example/live","body":"{{ user.uniqueId }} {{event.data.comment}}"})).unwrap(), ["event.data.comment", "event.user.uniqueId"]);
    assert!(effect_render_fields(&json!({"body":"{{globals.token}}"})).is_err());
    assert!(effect_render_fields(&json!({"body":"{{event}}"})).is_err());
    assert!(effect_render_fields(&json!({"body":"{{unclosed"})).is_err());
}

#[test]
fn complete_manifest_binds_disabled_random_unselected_missing_and_retained_dependencies_without_source(
) {
    let actions = [
        action("safe", "core.log", json!({"message":"secret literal"})),
        action(
            "hidden-script",
            "core.code",
            json!({"source":"throw new Error('fixture private script')"}),
        ),
    ];
    let mut review = event_review(&actions);
    review.value["record"]["actionIds"] =
        json!(["safe", "safe", "hidden-script", "missing", "deleted"]);
    review.value["dependencies"].as_array_mut().unwrap().extend([
        json!({"id":"missing","record":null,"provenance":null}),
        json!({"id":"deleted","record":null,"provenance":provenance(Some(("principal-b","session-b")),"7")}),
    ]);
    review.resource_versions.extend([
        version(AutomationFamily::Action, "missing", "0"),
        version(AutomationFamily::Action, "deleted", "8"),
    ]);
    review.value["dependencies"][1]["provenance"] =
        provenance(Some(("principal-a", "session-a")), "3");
    review.value["record"]["authority"] = json!({"kind":"owner","principalId":"forged"});
    let g = grant();
    let subjects = [
        ManifestSubject {
            principal_id: "principal-b",
            session_id: "session-b",
            grant_revision: "12",
            configuration_revision: "1",
            grant: &g,
        },
        ManifestSubject {
            principal_id: "principal-a",
            session_id: "session-a",
            grant_revision: "2",
            configuration_revision: "1",
            grant: &g,
        },
    ];
    assert_eq!(
        required_subjects(&review).unwrap(),
        vec![
            ("principal-a".into(), "session-a".into()),
            ("principal-b".into(), "session-b".into())
        ]
    );
    let manifest = build(
        &review,
        ManifestSubjects {
            host_id: "host",
            subjects: &subjects,
        },
    )
    .unwrap();
    assert_eq!(
        manifest
            .dependencies
            .iter()
            .map(|d| d.id.as_str())
            .collect::<Vec<_>>(),
        ["safe", "hidden-script", "missing", "deleted"]
    );
    assert_eq!(manifest.effects.len(), 4);
    assert_eq!(manifest.effects[1].kind, EffectKind::Script);
    assert_eq!(manifest.effects[1].enabled, Some(false));
    assert_eq!(manifest.effects[2].kind, EffectKind::MissingDefinition);
    assert_eq!(manifest.dependencies[2].revision, "0");
    assert!(manifest.dependencies[2].provenance.is_none());
    assert_eq!(manifest.dependencies[3].revision, "8");
    assert!(manifest.dependencies[3].provenance.is_some());
    assert!(!manifest.execution_authorized);
    assert!(manifest
        .reasons
        .contains(&ManifestReason::MissingDependency));
    let output = serde_json::to_string(&manifest).unwrap();
    for secret in [
        "secret literal",
        "fixture private script",
        "forged",
        "\"config\"",
        "\"source\"",
        "\"record\"",
    ] {
        assert!(!output.contains(secret), "{secret}");
    }
    assert_eq!(manifest.subjects[0].principal_id, "principal-a");
}

#[test]
fn manifest_digest_is_order_independent_for_objects_and_changes_with_every_authority_binding() {
    let review = action_review(
        action(
            "same",
            "core.log",
            json!({"message":"{{event.user.uniqueId}}","extra":{"z":0,"a":1}}),
        ),
        Some(("author", "session")),
    );
    let g = grant();
    let subjects = [ManifestSubject {
        principal_id: "author",
        session_id: "session",
        grant_revision: "1",
        configuration_revision: "1",
        grant: &g,
    }];
    let first = build(
        &review,
        ManifestSubjects {
            host_id: "host",
            subjects: &subjects,
        },
    )
    .unwrap();
    let ordered: Value = serde_json::from_str(r#"{"z":0,"a":1}"#).unwrap();
    let reversed: Value = serde_json::from_str(r#"{"a":1,"z":0}"#).unwrap();
    assert_eq!(
        digest("definition", &ordered, 1024).unwrap(),
        digest("definition", &reversed, 1024).unwrap()
    );
    assert_eq!(first.manifest_digest.len(), 64);
    assert!(first
        .manifest_digest
        .bytes()
        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    for change in ["definition", "revision", "generation", "host", "grant"] {
        let mut changed = review.clone();
        let mut host = "host";
        let mut revision = "1";
        match change {
            "definition" => changed.value["record"]["config"]["message"] = json!("changed"),
            "revision" => changed.resource_versions[0].revision = "2".into(),
            "generation" => changed.value["provenance"]["generation"] = json!("2"),
            "host" => host = "different-host",
            "grant" => revision = "2",
            _ => unreachable!(),
        }
        let subjects = [ManifestSubject {
            principal_id: "author",
            session_id: "session",
            grant_revision: revision,
            configuration_revision: "1",
            grant: &g,
        }];
        let next = build(
            &changed,
            ManifestSubjects {
                host_id: host,
                subjects: &subjects,
            },
        )
        .unwrap();
        assert_ne!(next.manifest_digest, first.manifest_digest, "{change}");
    }
}

#[test]
fn missing_extra_duplicate_or_invalid_subjects_cannot_silently_create_effective_owner_authority() {
    let review = action_review(
        action("same", "core.delay", json!({"ms":10})),
        Some(("author", "session")),
    );
    let g = grant();
    assert_eq!(
        build(
            &review,
            ManifestSubjects {
                host_id: "host",
                subjects: &[]
            }
        )
        .unwrap_err(),
        ManifestError::InvalidSubjects
    );
    for pair in [[("author", "other")], [("other", "session")]] {
        let subjects = [ManifestSubject {
            principal_id: pair[0].0,
            session_id: pair[0].1,
            grant_revision: "1",
            configuration_revision: "1",
            grant: &g,
        }];
        assert_eq!(
            build(
                &review,
                ManifestSubjects {
                    host_id: "host",
                    subjects: &subjects
                }
            )
            .unwrap_err(),
            ManifestError::InvalidSubjects
        );
    }
    let duplicate = [
        ManifestSubject {
            principal_id: "author",
            session_id: "session",
            grant_revision: "1",
            configuration_revision: "1",
            grant: &g,
        },
        ManifestSubject {
            principal_id: "author",
            session_id: "session",
            grant_revision: "1",
            configuration_revision: "1",
            grant: &g,
        },
    ];
    assert_eq!(
        build(
            &review,
            ManifestSubjects {
                host_id: "host",
                subjects: &duplicate
            }
        )
        .unwrap_err(),
        ManifestError::InvalidSubjects
    );
    let mut owner_review = review;
    owner_review.value["provenance"] = provenance(None, "1");
    assert!(owner(&owner_review).subjects.is_empty());
    assert_eq!(
        build(
            &owner_review,
            ManifestSubjects {
                host_id: "host",
                subjects: &duplicate[..1]
            }
        )
        .unwrap_err(),
        ManifestError::InvalidSubjects
    );
}

#[test]
fn every_subject_constrains_the_whole_effect_closure_and_points_limits() {
    let mut review = event_review(&[action(
        "points",
        "core.points",
        json!({"uniqueId":"alice","delta":-10}),
    )]);
    review.value["provenance"] = provenance(Some(("a", "s-a")), "1");
    review.value["dependencies"][0]["provenance"] = provenance(Some(("b", "s-b")), "1");
    let g = grant();
    let mut narrow = grant();
    narrow.max_points_delta = Some(5.0);
    let subjects = [
        ManifestSubject {
            principal_id: "a",
            session_id: "s-a",
            grant_revision: "1",
            configuration_revision: "1",
            grant: &g,
        },
        ManifestSubject {
            principal_id: "b",
            session_id: "s-b",
            grant_revision: "1",
            configuration_revision: "1",
            grant: &narrow,
        },
    ];
    let manifest = build(
        &review,
        ManifestSubjects {
            host_id: "host",
            subjects: &subjects,
        },
    )
    .unwrap();
    assert_eq!(manifest.effects[0].decision, EffectDecision::Unsupported);
    assert!(manifest.effects[0]
        .reasons
        .contains(&ManifestReason::ResourceForbidden));
    narrow.max_points_delta = None;
    narrow.capabilities.remove(&Capability::PointsAdjust);
    narrow
        .resources
        .insert(ResourceKind::Automation, ["event:event".into()].into());
    narrow
        .resources
        .insert(ResourceKind::Live, ["creator".into()].into());
    let subjects = [
        ManifestSubject {
            principal_id: "a",
            session_id: "s-a",
            grant_revision: "1",
            configuration_revision: "1",
            grant: &g,
        },
        ManifestSubject {
            principal_id: "b",
            session_id: "s-b",
            grant_revision: "2",
            configuration_revision: "1",
            grant: &narrow,
        },
    ];
    let manifest = build(
        &review,
        ManifestSubjects {
            host_id: "host",
            subjects: &subjects,
        },
    )
    .unwrap();
    for reason in [
        ManifestReason::MissingCapability,
        ManifestReason::ResourceForbidden,
        ManifestReason::LiveRestriction,
    ] {
        assert!(manifest.reasons.contains(&reason));
    }
}

#[test]
fn known_effects_are_classified_without_claiming_unsupported_policies_or_plugin_bindings() {
    let cases = [
        (
            "core.log",
            json!({"message":"hi {{event.user.uniqueId}}"}),
            EffectKind::Log,
            EffectDecision::Classified,
        ),
        (
            "core.delay",
            json!({"ms":10}),
            EffectKind::Delay,
            EffectDecision::Classified,
        ),
        (
            "core.points",
            json!({"uniqueId":"alice","delta":-10}),
            EffectKind::Points,
            EffectDecision::Classified,
        ),
        (
            "core.points.subtract",
            json!({"uniqueId":"{{ event.user.uniqueId }}","amount":5}),
            EffectKind::Points,
            EffectDecision::ReviewRequired,
        ),
        (
            "core.code",
            json!({"source":"return {emit: [{type: 'nested'}]}"}),
            EffectKind::Script,
            EffectDecision::ReviewRequired,
        ),
        (
            "core.fetch",
            json!({"url":"https://example.com/?key=fixture-private-token"}),
            EffectKind::Http,
            EffectDecision::Unsupported,
        ),
        (
            "core.audio.play",
            json!({"fileRef":{"path":"{{ event.path }}"}}),
            EffectKind::Audio,
            EffectDecision::Unsupported,
        ),
        (
            "core.emit",
            json!({"type":"overlay.alert","data":{"user":"{{event.user.uniqueId}}"}}),
            EffectKind::Emit,
            EffectDecision::Unsupported,
        ),
        (
            "minecraft.server.command",
            json!({"command":"secret fixture command"}),
            EffectKind::Plugin,
            EffectDecision::Unsupported,
        ),
        (
            "core.future",
            json!({}),
            EffectKind::Unknown,
            EffectDecision::Unsupported,
        ),
    ];
    for (type_id, config, kind, decision) in cases {
        let manifest = owner(&action_review(action("same", type_id, config), None));
        let effect = &manifest.effects[0];
        assert_eq!(effect.kind, kind, "{type_id}");
        assert_eq!(effect.decision, decision, "{type_id}");
        assert!(!manifest.execution_authorized);
        let output = serde_json::to_string(&manifest).unwrap();
        assert!(!output.contains("fixture-private-token"));
        assert!(!output.contains("secret fixture command"));
    }
}

#[test]
fn render_inventory_does_not_turn_global_or_malformed_expressions_into_an_allowlist() {
    for (text, reason) in [
        (
            "{{globals.apiKey}} {{event.user.uniqueId}}",
            ManifestReason::GlobalAccessRequiresPolicy,
        ),
        ("{{ event }}", ManifestReason::UnclassifiableRenderField),
        (
            "{{event.user[0]}}",
            ManifestReason::UnclassifiableRenderField,
        ),
        ("{{ unfinished", ManifestReason::UnclassifiableRenderField),
    ] {
        let manifest = owner(&action_review(
            action("same", "core.log", json!({"message":text})),
            None,
        ));
        assert!(manifest.effects[0].reasons.contains(&reason));
        assert_eq!(
            manifest.effects[0].decision,
            if reason == ManifestReason::GlobalAccessRequiresPolicy {
                EffectDecision::Unsupported
            } else {
                EffectDecision::ReviewRequired
            }
        );
    }
    let manifest = owner(&action_review(
        action(
            "same",
            "core.log",
            json!({"message":"{{user.uniqueId}} {{ event.user.uniqueId }}"}),
        ),
        None,
    ));
    assert_eq!(manifest.effects[0].render_fields, ["event.user.uniqueId"]);
    let text = (0..300)
        .map(|n| format!("{{{{event.field{n}}}}}"))
        .collect::<String>();
    let manifest = owner(&action_review(
        action("same", "core.log", json!({"message":text})),
        None,
    ));
    assert_eq!(manifest.effects[0].render_fields.len(), 256);
    assert!(manifest.effects[0]
        .reasons
        .contains(&ManifestReason::UnclassifiableRenderField));
}

#[test]
fn incomplete_reordered_or_corrupt_review_envelopes_fail_instead_of_dropping_dependencies() {
    let original = event_review(&[
        action("a", "core.log", json!({})),
        action("b", "core.delay", json!({})),
    ]);
    for change in [
        "missing",
        "order",
        "scope",
        "revision",
        "authority",
        "record",
        "extra",
    ] {
        let mut review = original.clone();
        match change {
            "missing" => {
                review.value["dependencies"].as_array_mut().unwrap().pop();
            }
            "order" => review.value["dependencies"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            "scope" => review.resource_versions[1].scope = "event".into(),
            "revision" => review.resource_versions[1].revision = "01".into(),
            "authority" => {
                review.value["dependencies"][0]["provenance"]["authority"]["kind"] =
                    json!("caller-owner")
            }
            "record" => review.value["dependencies"][0]["record"]["id"] = json!("other"),
            "extra" => review.value["rawSource"] = json!("must not enter manifest"),
            _ => unreachable!(),
        }
        assert_eq!(
            build(
                &review,
                ManifestSubjects {
                    host_id: "host",
                    subjects: &[]
                }
            )
            .unwrap_err(),
            ManifestError::InvalidReview,
            "{change}"
        );
    }
    let missing = Versioned::new(
        json!({"kind":"action","id":"missing","record":null,"provenance":null,"dependencies":[]}),
        vec![version(AutomationFamily::Action, "missing", "0")],
    );
    assert_eq!(
        build(
            &missing,
            ManifestSubjects {
                host_id: "host",
                subjects: &[]
            }
        )
        .unwrap_err(),
        ManifestError::MissingPrimary
    );
}

#[test]
fn source_and_depth_budgets_fail_without_echoing_private_content() {
    let large = action_review(
        action(
            "large",
            "core.log",
            json!({"message":"private marker".repeat(DEFINITION_BYTES)}),
        ),
        None,
    );
    let error = build(
        &large,
        ManifestSubjects {
            host_id: "host",
            subjects: &[],
        },
    )
    .unwrap_err();
    assert_eq!(error, ManifestError::TooLarge);
    assert!(!error.to_string().contains("private marker"));
    let mut nested = json!(0);
    for _ in 0..70 {
        nested = json!({"next":nested});
    }
    let deep = action_review(
        action("deep", "core.log", json!({"message":"x","extra":nested})),
        None,
    );
    assert_eq!(
        build(
            &deep,
            ManifestSubjects {
                host_id: "host",
                subjects: &[]
            }
        )
        .unwrap_err(),
        ManifestError::TooLarge
    );
}

#[test]
fn render_inventory_matches_legacy_event_lookup_instead_of_claiming_a_different_field() {
    let event = json!({"ual":"legacy","User":"second strip","user":{"uniqueId":"alice"}});
    for (raw, normalized) in [
        ("eventual", "event.ual"),
        ("event.eventUser", "event.User"),
        ("event..user..uniqueId", "event.user.uniqueId"),
    ] {
        let manifest = owner(&action_review(
            action(
                "same",
                "core.log",
                json!({"message":format!("{{{{{raw}}}}}")}),
            ),
            None,
        ));
        assert_eq!(manifest.effects[0].render_fields, [normalized]);
        assert_eq!(
            crate::helpers::read_event_path(&event, raw),
            crate::helpers::read_event_path(&event, normalized)
        );
    }
    let ambiguous = owner(&action_review(
        action("same", "core.log", json!({"message":"{{ .eventUser }}"})),
        None,
    ));
    assert!(ambiguous.effects[0].render_fields.is_empty());
    assert!(ambiguous.effects[0]
        .reasons
        .contains(&ManifestReason::UnclassifiableRenderField));
}

#[test]
fn plugin_or_arbitrary_trigger_cannot_claim_a_complete_catalog_from_core_only_actions() {
    let mut review = event_review(&[action("log", "core.log", json!({"message":"hello"}))]);
    review.value["record"]["trigger"] = json!("plugin.custom.event");
    let manifest = owner(&review);
    assert!(manifest
        .reasons
        .contains(&ManifestReason::PluginBindingUnavailable));
}
