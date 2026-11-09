#![cfg(feature = "persistence")]
//! Real host authorization and business storage, with no TikTok connection or
//! external plugin execution. JSON claims cannot become execution authority.
use serde_json::{json, Value};
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteSession, RequestContext, RpcResponse};
use tiktools_core::{
    collaboration::{
        hash_secret, Capability, ClientKind, Collect, Grant, GrantSet, InviteCreate, Redeem,
        ResourceKind,
    },
    control::AutomationKind,
    paths::AppPaths,
    AppCore, HostEmitter, NoopMediaHost,
};
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: tiktools_core::ipc::messages::HostMessage) {}
}
struct Fixture {
    core: Arc<AppCore>,
    api: ControlApi,
    root: std::path::PathBuf,
}
struct Controller {
    context: RequestContext,
    principal: String,
    session: String,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-api-provenance-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(NoopMediaHost),
            paths,
        ));
        Self {
            api: ControlApi::new(core.clone()),
            core,
            root,
        }
    }
    fn instance(&self) -> String {
        self.core.collaboration().unwrap().instance_id().into()
    }
    fn pair(&self, grant: Grant) -> Controller {
        let service = self.core.collaboration().unwrap();
        let mut configuration = service.configuration().unwrap();
        configuration.enabled = true;
        configuration.advertised_origin = Some("http://127.0.0.1:8788".into());
        service.configure(configuration).unwrap();
        let invite = service.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "a".repeat(64);
        let request = service
            .redeem(
                Redeem {
                    invite_id: invite.invite_id,
                    secret: invite.secret,
                    client_label: "Draft author".into(),
                    client_kind: ClientKind::Agent,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "fixture",
            )
            .unwrap();
        service.decide_request(&request.request_id, true).unwrap();
        let credentials = service
            .collect(
                Collect {
                    request_id: request.request_id,
                    exchange_secret: exchange,
                },
                "fixture",
            )
            .unwrap();
        let auth = service
            .authenticate_live(&credentials.access_token, false, false)
            .unwrap();
        let identity = auth.snapshot().unwrap();
        Controller {
            context: RequestContext::Remote(RemoteSession::from_authorization(auth).unwrap()),
            principal: identity.principal_id,
            session: identity.session_id,
        }
    }
    async fn call(
        &self,
        context: &RequestContext,
        method: &str,
        params: Value,
        meta: Value,
    ) -> RpcResponse {
        let mut request = json!({"id":1,"method":method,"params":params});
        if !meta.is_null() {
            request["meta"] = meta;
        }
        self.api.execute_value_with_context(context, &request).await
    }
    async fn owner(&self, method: &str, params: Value, meta: Value) -> RpcResponse {
        self.call(&RequestContext::LocalOwner, method, params, meta)
            .await
    }
    async fn remote(
        &self,
        c: &Controller,
        method: &str,
        params: Value,
        meta: Value,
    ) -> RpcResponse {
        self.call(&c.context, method, params, meta).await
    }
    async fn authority(&self, kind: &str, id: &str) -> Value {
        let response = self
            .owner(
                "automation.authority.get",
                json!({"kind":kind,"id":id}),
                Value::Null,
            )
            .await;
        assert!(response.is_ok(), "{response:?}");
        let meta = response.meta.as_ref().unwrap();
        assert_eq!(meta.resource_versions.len(), 1);
        assert_eq!(meta.resource_versions[0].scope, kind);
        response.result.unwrap()
    }
    async fn close(self) {
        self.core.shutdown().await;
        let _ = std::fs::remove_dir_all(self.root);
    }
}
fn grant(ids: &[&str]) -> Grant {
    Grant {
        capabilities: [
            Capability::AutomationRead,
            Capability::AutomationEdit,
            Capability::AutomationToggle,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            ResourceKind::Automation,
            ids.iter().map(|id| (*id).into()).collect(),
        )]
        .into(),
        ..Grant::default()
    }
}
fn metadata(instance: &str, id: &str, revision: &str) -> Value {
    json!({"commandId":id,"hostInstanceId":instance,"expectedRevision":revision})
}
fn draft(kind: &str, id: &str) -> Value {
    json!({"id":id,"name":format!("{kind} proposal"),"enabled":false,"trigger":"tiktok.chat","actionIds":["draft"],"typeId":"core.log","config":{"message":"author-only-source-marker"},"source":"private-script-marker","authority":{"kind":"owner"},"provenance":{"authority":{"kind":"owner"},"generation":"999"}})
}
fn assert_delegated(value: &Value, c: &Controller, kind: &str, id: &str) {
    assert_eq!(value["id"], id);
    assert_eq!(value["kind"], kind);
    assert_eq!(value["provenance"]["authority"]["kind"], "delegated");
    assert_eq!(value["provenance"]["authority"]["principalId"], c.principal);
    assert_eq!(value["provenance"]["authority"]["sessionId"], c.session);
    let generation = value["provenance"]["generation"]
        .as_u64()
        .or_else(|| value["provenance"]["generation"].as_str()?.parse().ok());
    assert!(generation.is_some_and(|generation| generation > 0 && generation != 999));
}
fn assert_pending(response: &RpcResponse, kind: &str, id: &str, revision: &str) {
    assert!(response.is_ok(), "{response:?}");
    assert_eq!(
        response.result.as_ref().unwrap(),
        &json!({"kind":kind,"id":id,"enabled":false,"pendingReview":true})
    );
    let meta = response.meta.as_ref().unwrap();
    assert_eq!(meta.resource_versions.len(), 1);
    assert_eq!(meta.resource_versions[0].scope, kind);
    assert_eq!(meta.resource_versions[0].id, id);
    assert_eq!(meta.resource_versions[0].revision, revision);
    let resource = meta.command.as_ref().unwrap().resource.as_ref().unwrap();
    assert_eq!(resource.kind, ResourceKind::Automation);
    assert_eq!(resource.scope, kind);
    assert_eq!(resource.id, id);
}

#[tokio::test]
async fn stored_reusable_policy_enables_saved_remote_definition() {
    use tiktools_core::collaboration::{AutomationDelegation, DelegatedAutomationEffect};
    let f = Fixture::new();
    let mut author_grant = grant(&["action:draft"]);
    author_grant.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Log].into(),
        allow_dynamic_targets: false,
    });
    let author = f.pair(author_grant);
    let instance = f.instance();
    let clean = json!({"id":"draft","name":"Log","enabled":false,"typeId":"core.log","config":{"message":"hello"}});
    let mut enabled = clean.clone();
    enabled["enabled"] = true.into();
    let rejected = f
        .remote(
            &author,
            "automation.create",
            json!({"kind":"action","record":enabled}),
            metadata(&instance, "enabled-create", "0"),
        )
        .await;
    assert_eq!(rejected.error.unwrap().code, "forbidden");
    let created = f
        .remote(
            &author,
            "automation.create",
            json!({"kind":"action","record":clean}),
            metadata(&instance, "disabled-create", "0"),
        )
        .await;
    assert_pending(&created, "action", "draft", "1");
    let preview = f
        .owner(
            "automation.delegation.preflight",
            json!({"kind":"action","id":"draft"}),
            Value::Null,
        )
        .await;
    assert_eq!(
        preview.result.as_ref().unwrap()["eligible"],
        true,
        "{preview:?}"
    );
    let enable = f
        .remote(
            &author,
            "automation.enable",
            json!({"kind":"action","id":"draft"}),
            metadata(&instance, "enable-draft", "1"),
        )
        .await;
    assert!(enable.is_ok(), "{enable:?}");
    assert_eq!(
        enable.result.unwrap(),
        json!({"kind":"action","id":"draft","enabled":true})
    );
    assert_delegated(
        &f.authority("action", "draft").await,
        &author,
        "action",
        "draft",
    );
    let owner = f
        .owner(
            "automation.delegation.preflight",
            json!({"kind":"action","id":"draft"}),
            Value::Null,
        )
        .await;
    assert!(owner.is_ok(), "{owner:?}");
    assert_eq!(owner.result.unwrap()["executionAuthorized"], false);
    let remote = f
        .remote(
            &author,
            "automation.delegation.preflight",
            json!({"kind":"action","id":"draft"}),
            Value::Null,
        )
        .await;
    assert_eq!(remote.error.unwrap().code, "forbidden");
    f.close().await;
}

#[tokio::test]
async fn disabled_remote_drafts_are_author_bound_family_scoped_and_never_reveal_definition_receipts(
) {
    let f = Fixture::new();
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        f.core
            .automation_create(
                kind,
                json!({"id":"owned","name":"Owner record","enabled":true}),
            )
            .unwrap();
    }
    let ids = [
        "action:draft",
        "event:draft",
        "action:owned",
        "event:owned",
        "action:retained",
    ];
    let author = f.pair(grant(&ids));
    let other = f.pair(grant(&ids));
    let instance = f.instance();
    for kind in ["action", "event"] {
        let record = draft(kind, "draft");
        let command = format!("create-{kind}");
        let created = f
            .remote(
                &author,
                "automation.create",
                json!({"kind":kind,"record":record}),
                metadata(&instance, &command, "0"),
            )
            .await;
        assert_pending(&created, kind, "draft", "1");
        assert_delegated(&f.authority(kind, "draft").await, &author, kind, "draft");
        let read = f
            .remote(
                &author,
                "automation.get",
                json!({"kind":kind,"id":"draft"}),
                Value::Null,
            )
            .await;
        assert!(read.is_ok(), "{read:?}");
        let body = read.result.unwrap();
        assert_eq!(body.as_object().unwrap().len(), 5);
        assert_eq!(body["definitionRedacted"], true);
        assert!(!body.to_string().contains("author-only-source-marker"));
        assert!(!body.to_string().contains("private-script-marker"));
        let outcome = f
            .remote(
                &author,
                "collaboration.commands.get",
                json!({"commandId":command}),
                Value::Null,
            )
            .await;
        assert!(outcome.is_ok(), "{outcome:?}");
        assert_eq!(outcome.result.unwrap()["result"], created.result.unwrap());
        let mut changed = record.clone();
        changed["name"] = "Author revision".into();
        let update = f
            .remote(
                &author,
                "automation.update",
                json!({"kind":kind,"id":"draft","record":changed}),
                metadata(&instance, &format!("update-{kind}"), "1"),
            )
            .await;
        assert_pending(&update, kind, "draft", "2");
        let refused = f
            .remote(
                &other,
                "automation.update",
                json!({"kind":kind,"id":"draft","record":record}),
                metadata(&instance, &format!("steal-{kind}"), "2"),
            )
            .await;
        assert_eq!(refused.error.unwrap().code, "forbidden");
        assert_delegated(&f.authority(kind, "draft").await, &author, kind, "draft");
        for (method, params) in [
            (
                "automation.create",
                json!({"kind":kind,"record":draft(kind,"owned")}),
            ),
            (
                "automation.update",
                json!({"kind":kind,"id":"owned","record":draft(kind,"owned")}),
            ),
        ] {
            assert_eq!(
                f.remote(
                    &author,
                    method,
                    params,
                    metadata(
                        &instance,
                        &format!("owner-{kind}-{}", method.replace('.', "-")),
                        "1"
                    )
                )
                .await
                .error
                .unwrap()
                .code,
                "forbidden"
            );
        }
        for enabled in [Value::Bool(true), Value::Null, json!(0)] {
            let mut invalid = draft(kind, "draft");
            invalid["enabled"] = enabled;
            let error = f
                .remote(
                    &author,
                    "automation.update",
                    json!({"kind":kind,"id":"draft","record":invalid}),
                    metadata(&instance, &format!("activation-{kind}"), "2"),
                )
                .await
                .error
                .unwrap();
            assert!(matches!(
                error.code.as_str(),
                "forbidden" | "invalid_params"
            ));
        }
        assert_eq!(
            f.remote(
                &author,
                "automation.create",
                json!({"kind":kind,"record":record}),
                Value::Null
            )
            .await
            .error
            .unwrap()
            .code,
            "forbidden"
        );
        assert_eq!(
            f.remote(
                &author,
                "automation.authority.get",
                json!({"kind":kind,"id":"draft"}),
                Value::Null
            )
            .await
            .error
            .unwrap()
            .code,
            "forbidden"
        );
    }
    // Owner tombstones retain their authority; absence does not make them claimable.
    f.core
        .automation_delete(AutomationKind::Event, "owned")
        .unwrap();
    assert_eq!(
        f.remote(
            &author,
            "automation.create",
            json!({"kind":"event","record":draft("event","owned")}),
            metadata(&instance, "owner-tombstone", "2")
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    // The same rule prevents another delegated principal claiming an author's ID.
    let created = f
        .remote(
            &author,
            "automation.create",
            json!({"kind":"action","record":draft("action","retained")}),
            metadata(&instance, "retain", "0"),
        )
        .await;
    assert_pending(&created, "action", "retained", "1");
    f.core
        .automation_delete(AutomationKind::Action, "retained")
        .unwrap();
    assert_eq!(
        f.remote(
            &other,
            "automation.create",
            json!({"kind":"action","record":draft("action","retained")}),
            metadata(&instance, "retained-steal", "2")
        )
        .await
        .error
        .unwrap()
        .code,
        "forbidden"
    );
    let restored = f
        .remote(
            &author,
            "automation.create",
            json!({"kind":"action","record":draft("action","retained")}),
            metadata(&instance, "retained-own", "2"),
        )
        .await;
    assert_pending(&restored, "action", "retained", "3");
    assert_delegated(
        &f.authority("action", "retained").await,
        &author,
        "action",
        "retained",
    );
    f.close().await;
}

#[tokio::test]
async fn owner_adoption_requires_ack_and_cas_disables_records_and_keeps_old_receipts_immutable() {
    let f = Fixture::new();
    let author = f.pair(grant(&["action:draft", "event:draft"]));
    let instance = f.instance();
    f.core.points_adjust("provenance-viewer", 10.0).unwrap();
    let mut action = draft("action", "draft");
    action["typeId"] = "core.points.subtract".into();
    action["config"] = json!({"uniqueId":"provenance-viewer","amount":2});
    let event = draft("event", "draft");
    for (kind, record) in [("action", action.clone()), ("event", event.clone())] {
        assert_pending(
            &f.remote(
                &author,
                "automation.create",
                json!({"kind":kind,"record":record}),
                metadata(&instance, &format!("original-{kind}"), "0"),
            )
            .await,
            kind,
            "draft",
            "1",
        );
    }
    // Legacy owner writes and bulk toggles cannot silently convert an author's
    // record into owner authority, nor can explicit owner draft fire bypass it.
    f.core
        .automation_set_enabled(AutomationKind::Action, "draft", true)
        .unwrap();
    f.core
        .automation_set_enabled(AutomationKind::Event, "draft", true)
        .unwrap();
    let mut ordinary_owner_save = action.clone();
    ordinary_owner_save["enabled"] = true.into();
    f.core
        .automation_update(AutomationKind::Action, "draft", ordinary_owner_save)
        .unwrap();
    let bulk=f.owner("automation.set_enablement",json!({"updates":[{"kind":"action","id":"draft","enabled":true},{"kind":"event","id":"draft","enabled":true}]}),Value::Null).await;
    assert!(bulk.is_ok(), "{bulk:?}");
    for (kind, id) in [("action", "draft"), ("event", "draft")] {
        assert_delegated(&f.authority(kind, id).await, &author, kind, id);
    }
    for record in [Value::Null, event.clone()] {
        let mut params = json!({"trigger":"tiktok.chat","event":{"type":"tiktok.chat","user":{"uniqueId":"provenance-viewer"},"data":{}}});
        if !record.is_null() {
            params["record"] = record;
        }
        let fired = f.owner("automation.fire", params, Value::Null).await;
        assert!(fired.is_ok(), "{fired:?}");
    }
    assert_eq!(
        f.core.points_viewer("provenance-viewer").unwrap()["points"],
        10.0
    );
    for kind in ["action", "event"] {
        let refused = f
            .remote(
                &author,
                "automation.adopt",
                json!({"kind":kind,"id":"draft","acknowledgeAuthority":true}),
                metadata(&instance, "remote-adopt", "0"),
            )
            .await;
        assert_eq!(refused.error.unwrap().code, "forbidden");
        let no_ack = f
            .owner(
                "automation.adopt",
                json!({"kind":kind,"id":"draft","acknowledgeAuthority":false}),
                metadata(&instance, "missing-ack", "0"),
            )
            .await
            .error
            .unwrap();
        assert!(matches!(
            no_ack.code.as_str(),
            "confirmation_required" | "invalid_params"
        ));
        let missing_meta = f
            .owner(
                "automation.adopt",
                json!({"kind":kind,"id":"draft","acknowledgeAuthority":true}),
                Value::Null,
            )
            .await
            .error
            .unwrap();
        assert!(matches!(
            missing_meta.code.as_str(),
            "invalid_params" | "unsupported_remote"
        ));
        let stale = f
            .owner(
                "automation.adopt",
                json!({"kind":kind,"id":"draft","acknowledgeAuthority":true}),
                metadata(&instance, &format!("stale-adopt-{kind}"), "0"),
            )
            .await;
        assert_eq!(stale.error.unwrap().code, "revision_conflict");
        assert_delegated(&f.authority(kind, "draft").await, &author, kind, "draft");
        let parsed = if kind == "event" {
            AutomationKind::Event
        } else {
            AutomationKind::Action
        };
        let before = f
            .core
            .automation_get_versioned(parsed, "draft")
            .unwrap()
            .resource_versions[0]
            .revision
            .clone();
        let adopted = f
            .owner(
                "automation.adopt",
                json!({"kind":kind,"id":"draft","acknowledgeAuthority":true}),
                metadata(&instance, &format!("adopt-{kind}"), &before),
            )
            .await;
        assert!(adopted.is_ok(), "{adopted:?}");
        assert_eq!(
            adopted.result.as_ref().unwrap(),
            &json!({"id":"draft","adopted":true})
        );
        assert_eq!(
            f.core.automation_get(parsed, "draft").unwrap()["enabled"],
            false
        );
        assert_eq!(
            adopted.meta.as_ref().unwrap().resource_versions[0].scope,
            kind
        );
        assert_eq!(
            adopted.meta.as_ref().unwrap().resource_versions[0]
                .revision
                .parse::<u64>()
                .unwrap(),
            before.parse::<u64>().unwrap() + 1
        );
        let authority = f.authority(kind, "draft").await;
        assert_eq!(authority["provenance"]["authority"]["kind"], "owner");
        assert_eq!(
            f.core.automation_get(parsed, "draft").unwrap()["enabled"],
            false
        );
        f.core
            .automation_set_enabled(parsed, "draft", true)
            .unwrap();
        let current = f
            .core
            .automation_get_versioned(parsed, "draft")
            .unwrap()
            .resource_versions[0]
            .revision
            .clone();
        let takeover = f
            .remote(
                &author,
                "automation.update",
                json!({"kind":kind,"id":"draft","record":draft(kind,"draft")}),
                metadata(&instance, &format!("post-adoption-{kind}"), &current),
            )
            .await;
        assert_eq!(takeover.error.unwrap().code, "forbidden");
    }
    // Adoption does not execute a trigger. A subsequent explicit owner fire is
    // the first operation allowed to run the newly owner-owned definitions.
    assert_eq!(
        f.core.points_viewer("provenance-viewer").unwrap()["points"],
        10.0
    );
    let fired=f.owner("automation.fire",json!({"trigger":"tiktok.chat","event":{"type":"tiktok.chat","user":{"uniqueId":"provenance-viewer"},"data":{}}}),Value::Null).await;
    assert!(fired.is_ok(), "{fired:?}");
    assert_eq!(
        f.core.points_viewer("provenance-viewer").unwrap()["points"],
        8.0
    );
    for (kind, record) in [("action", action), ("event", event)] {
        let before = f
            .core
            .automation_get_versioned(
                if kind == "event" {
                    AutomationKind::Event
                } else {
                    AutomationKind::Action
                },
                "draft",
            )
            .unwrap();
        let replay = f
            .remote(
                &author,
                "automation.create",
                json!({"kind":kind,"record":record}),
                metadata(&instance, &format!("original-{kind}"), "0"),
            )
            .await;
        assert_pending(&replay, kind, "draft", "1");
        let outcome = f
            .remote(
                &author,
                "collaboration.commands.get",
                json!({"commandId":format!("original-{kind}")}),
                Value::Null,
            )
            .await;
        assert!(outcome.is_ok(), "{outcome:?}");
        assert_eq!(
            outcome.result.as_ref().unwrap()["result"],
            replay.result.unwrap()
        );
        let after = f
            .core
            .automation_get_versioned(
                if kind == "event" {
                    AutomationKind::Event
                } else {
                    AutomationKind::Action
                },
                "draft",
            )
            .unwrap();
        assert_eq!(after.value, before.value);
        assert_eq!(after.resource_versions, before.resource_versions);
    }
    f.close().await;
}

#[tokio::test]
async fn draft_receipts_are_reauthorized_for_exact_family_after_grants_change() {
    let f = Fixture::new();
    let initial = grant(&["action:draft", "event:draft"]);
    let author = f.pair(initial.clone());
    let instance = f.instance();
    let mut receipts = Vec::new();
    for kind in ["action", "event"] {
        let response = f
            .remote(
                &author,
                "automation.create",
                json!({"kind":kind,"record":draft(kind,"draft")}),
                metadata(&instance, &format!("queued-{kind}"), "0"),
            )
            .await;
        assert_pending(&response, kind, "draft", "1");
        let outcome = f
            .remote(
                &author,
                "collaboration.commands.get",
                json!({"commandId":format!("queued-{kind}")}),
                Value::Null,
            )
            .await;
        assert!(outcome.is_ok(), "{outcome:?}");
        receipts.push((kind, response.result.unwrap(), outcome.result.unwrap()));
    }
    let service = f.core.collaboration().unwrap();
    let current = service.principal(&author.principal).unwrap();
    service
        .set_grant(GrantSet {
            principal_id: author.principal.clone(),
            expected_revision: current.grant_revision,
            grant: Grant {
                resources: [(ResourceKind::Automation, ["action:draft".into()].into())].into(),
                ..initial
            },
        })
        .unwrap();
    let RequestContext::Remote(session) = &author.context else {
        panic!()
    };
    for (kind, receipt, outcome) in receipts {
        if kind == "action" {
            assert!(f
                .api
                .authorize_remote_response(session, "automation.create", &receipt)
                .is_ok());
            assert!(f
                .api
                .authorize_remote_response(session, "collaboration.commands.get", &outcome)
                .is_ok());
        } else {
            assert!(f
                .api
                .authorize_remote_response(session, "automation.create", &receipt)
                .is_err());
            assert!(f
                .api
                .authorize_remote_response(session, "collaboration.commands.get", &outcome)
                .is_err());
            assert_eq!(
                f.remote(
                    &author,
                    "collaboration.commands.get",
                    json!({"commandId":"queued-event"}),
                    Value::Null
                )
                .await
                .error
                .unwrap()
                .code,
                "forbidden"
            );
            assert!(f.api.authorize_remote_failure(session,"automation.create",Some(&json!({"resourceVersions":[{"kind":"automation","scope":"event","id":"draft","revision":"1"}]}))).is_err());
        }
    }
    f.close().await;
}

#[tokio::test]
async fn owner_review_returns_the_complete_typed_graph_and_never_exposes_it_remotely() {
    let f = Fixture::new();
    let author = f.pair(grant(&["action:draft", "event:draft"]));
    let instance = f.instance();
    let created = f
        .remote(
            &author,
            "automation.create",
            json!({"kind":"action","record":draft("action","draft")}),
            metadata(&instance, "review-action", "0"),
        )
        .await;
    assert_pending(&created, "action", "draft", "1");
    f.core
        .automation_create(
            AutomationKind::Action,
            json!({"id":"removed","name":"retained","enabled":false}),
        )
        .unwrap();
    f.core
        .automation_delete(AutomationKind::Action, "removed")
        .unwrap();
    f.core.automation_create(AutomationKind::Event,json!({"id":"draft","name":"owner event","enabled":false,"random":true,"actionIds":["draft","missing","draft","removed"]})).unwrap();
    let reviewed = f
        .owner(
            "automation.review",
            json!({"kind":"event","id":"draft"}),
            Value::Null,
        )
        .await;
    assert!(reviewed.is_ok(), "{reviewed:?}");
    let body = reviewed.result.as_ref().unwrap();
    let typed: tiktools_control_api::modules::automation::AutomationReviewResult =
        serde_json::from_value(body.clone()).unwrap();
    assert_eq!(typed.kind, AutomationKind::Event);
    assert_eq!(body["record"]["enabled"], false);
    assert_eq!(body["provenance"]["authority"]["kind"], "owner");
    assert_eq!(
        body["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["draft", "missing", "removed"]
    );
    assert_eq!(
        body["dependencies"][0]["record"]["provenance"]["authority"]["kind"],
        "owner"
    );
    assert_eq!(
        body["dependencies"][0]["provenance"]["authority"]["kind"],
        "delegated"
    );
    assert_eq!(
        body["dependencies"][0]["provenance"]["authority"]["principalId"],
        author.principal
    );
    assert_eq!(body["dependencies"][0]["provenance"]["generation"], "1");
    assert_eq!(
        body["dependencies"][1],
        json!({"id":"missing","record":null,"provenance":null})
    );
    assert!(body["dependencies"][2]["record"].is_null());
    assert_eq!(body["dependencies"][2]["provenance"]["generation"], "2");
    let versions = &reviewed.meta.as_ref().unwrap().resource_versions;
    assert_eq!(
        versions
            .iter()
            .map(|v| (v.scope.as_str(), v.id.as_str(), v.revision.as_str()))
            .collect::<Vec<_>>(),
        [
            ("event", "draft", "1"),
            ("action", "draft", "1"),
            ("action", "missing", "0"),
            ("action", "removed", "2")
        ]
    );
    assert!(reviewed.meta.as_ref().unwrap().command.is_none());
    let action = f
        .owner(
            "automation.review",
            json!({"kind":"action","id":"draft"}),
            Value::Null,
        )
        .await;
    assert!(action.is_ok());
    assert_eq!(action.result.unwrap()["dependencies"], json!([]));
    assert_eq!(action.meta.unwrap().resource_versions.len(), 1);
    let absent = f
        .owner(
            "automation.review",
            json!({"id":"never-created"}),
            Value::Null,
        )
        .await;
    assert!(absent.is_ok());
    assert!(absent.result.unwrap()["record"].is_null());
    assert_eq!(absent.meta.unwrap().resource_versions[0].revision, "0");
    for meta in [
        json!({"expectedRevision":"1"}),
        json!({"hostInstanceId":instance}),
        metadata(&instance, "illegal-review-command", "1"),
    ] {
        let expected_code =
            if meta.get("commandId").is_some() || meta.get("hostInstanceId").is_some() {
                "unsupported_remote"
            } else {
                "invalid_params"
            };
        let invalid = f
            .owner(
                "automation.review",
                json!({"kind":"event","id":"draft"}),
                meta,
            )
            .await;
        assert_eq!(invalid.error.unwrap().code, expected_code);
        assert!(invalid.meta.is_none());
    }
    for meta in [Value::Null, metadata(&instance, "remote-review", "1")] {
        let denied = f
            .remote(
                &author,
                "automation.review",
                json!({"kind":"event","id":"draft"}),
                meta,
            )
            .await;
        assert!(denied.result.is_none());
        assert!(denied.meta.is_none());
        let error = denied.error.unwrap();
        assert_eq!(error.code, "forbidden");
        assert!(error.data.is_none());
    }
    let RequestContext::Remote(session) = &author.context else {
        panic!()
    };
    assert!(f
        .api
        .authorize_remote_response(session, "automation.review", body)
        .is_err());
    assert!(f
        .api
        .authorize_remote_failure(
            session,
            "automation.review",
            Some(&json!({"resourceVersions":versions}))
        )
        .is_err());
    f.close().await;
}

#[tokio::test]
async fn owner_review_rejects_malformed_and_over_budget_graphs_without_partial_metadata() {
    let f = Fixture::new();
    for dependencies in [
        json!("private-action-id"),
        json!([" noncanonical"]),
        json!([7]),
    ] {
        f.core
            .automation_save(
                AutomationKind::Event,
                &json!({"id":"bad","name":"review","enabled":false,"actionIds":dependencies}),
            )
            .unwrap();
        let failed = f
            .owner(
                "automation.review",
                json!({"kind":"event","id":"bad"}),
                Value::Null,
            )
            .await;
        assert_eq!(failed.error.unwrap().code, "invalid_params");
        assert!(failed.result.is_none());
        assert!(failed.meta.is_none());
    }
    for record in [
        json!({"id":"bad","name":"review","enabled":false,"source":"x".repeat(256*1024)}),
        json!({"id":"bad","name":"review","enabled":false,"actionIds":(0..513).map(|n|format!("action-{n}")).collect::<Vec<_>>()}),
    ] {
        f.core
            .automation_save(AutomationKind::Event, &record)
            .unwrap();
        let failed = f
            .owner(
                "automation.review",
                json!({"kind":"event","id":"bad"}),
                Value::Null,
            )
            .await;
        assert_eq!(failed.error.unwrap().code, "too_large");
        assert!(failed.result.is_none());
        assert!(failed.meta.is_none());
    }
    f.close().await;
}

#[tokio::test]
async fn reviewed_adoption_rejects_a_changed_dependency_without_changing_the_event_or_reserving_a_command(
) {
    let f = Fixture::new();
    let author = f.pair(grant(&["action:draft", "event:draft"]));
    let instance = f.instance();
    for kind in ["action", "event"] {
        assert_pending(
            &f.remote(
                &author,
                "automation.create",
                json!({"kind":kind,"record":draft(kind,"draft")}),
                metadata(&instance, &format!("fenced-{kind}"), "0"),
            )
            .await,
            kind,
            "draft",
            "1",
        );
    }
    let original = f
        .owner(
            "automation.review",
            json!({"kind":"event","id":"draft"}),
            Value::Null,
        )
        .await;
    assert!(original.is_ok());
    let reviewed =
        serde_json::to_value(&original.meta.as_ref().unwrap().resource_versions).unwrap();
    let mut changed = draft("action", "draft");
    changed["name"] = "changed after owner review".into();
    f.core
        .automation_update(AutomationKind::Action, "draft", changed)
        .unwrap();
    let blocked = f.owner("automation.adopt",json!({"kind":"event","id":"draft","acknowledgeAuthority":true,"reviewedVersions":reviewed}),metadata(&instance,"fenced-adoption","1")).await;
    let error = blocked.error.unwrap();
    assert_eq!(error.code, "revision_conflict");
    assert_eq!(
        error.data.unwrap()["resourceVersions"],
        json!([{"kind":"automation","scope":"action","id":"draft","revision":"2"}])
    );
    let after = f
        .owner(
            "automation.review",
            json!({"kind":"event","id":"draft"}),
            Value::Null,
        )
        .await;
    assert_eq!(
        after.result.as_ref().unwrap()["record"],
        original.result.as_ref().unwrap()["record"]
    );
    assert_eq!(
        after.result.as_ref().unwrap()["provenance"],
        original.result.as_ref().unwrap()["provenance"]
    );
    assert_eq!(
        after.meta.as_ref().unwrap().resource_versions[0].revision,
        "1"
    );
    let current = serde_json::to_value(&after.meta.as_ref().unwrap().resource_versions).unwrap();
    // Reusing this ID succeeds because the rejected attempt reserved nothing.
    let adopted = f.owner("automation.adopt",json!({"kind":"event","id":"draft","acknowledgeAuthority":true,"reviewedVersions":current}),metadata(&instance,"fenced-adoption","1")).await;
    assert!(adopted.is_ok(), "{adopted:?}");
    assert_eq!(
        adopted.result.unwrap(),
        json!({"id":"draft","adopted":true})
    );
    assert_eq!(adopted.meta.unwrap().resource_versions[0].revision, "2");
    let final_review = f
        .owner(
            "automation.review",
            json!({"kind":"event","id":"draft"}),
            Value::Null,
        )
        .await;
    assert_eq!(
        final_review.result.as_ref().unwrap()["record"]["enabled"],
        false
    );
    assert_eq!(
        final_review.result.as_ref().unwrap()["provenance"]["authority"]["kind"],
        "owner"
    );
    f.close().await;
}

#[tokio::test]
async fn remote_draft_defaults_keep_owner_projection_well_formed_and_invalid_writes_cannot_poison_it(
) {
    let f = Fixture::new();
    let author = f.pair(grant(&[
        "action:valid",
        "event:valid",
        "action:bad",
        "event:bad",
    ]));
    let instance = f.instance();
    for kind in ["action", "event"] {
        let key = if kind == "action" {
            "typeId"
        } else {
            "trigger"
        };
        let mut minimal = json!({"id":"valid","enabled":false,"source":"private-submitted-source","custom":{"keep":true}});
        minimal[key] = if kind == "action" {
            "future.plugin.action"
        } else {
            "tiktok.chat"
        }
        .into();
        let created = f
            .remote(
                &author,
                "automation.create",
                json!({"kind":kind,"record":minimal}),
                metadata(&instance, &format!("shape-{kind}"), "0"),
            )
            .await;
        assert_pending(&created, kind, "valid", "1");
        let review = f
            .owner(
                "automation.review",
                json!({"kind":kind,"id":"valid"}),
                Value::Null,
            )
            .await;
        assert!(review.is_ok(), "{review:?}");
        let record = &review.result.as_ref().unwrap()["record"];
        assert!(record["name"].as_str().is_some_and(|name| !name.is_empty()));
        assert_eq!(record["source"], "private-submitted-source");
        assert_eq!(record["custom"], json!({"keep":true}));
        if kind == "action" {
            assert_eq!(record["config"], json!({}));
            assert_eq!(record["schemaVersion"], 2);
        } else {
            assert_eq!(record["schemaVersion"], 1);
            assert_eq!(record["actionIds"], json!([]));
            assert_eq!(record["filters"], json!([]));
            assert_eq!(record["cooldownMs"], 0);
            assert_eq!(record["cooldownScope"], "global");
            assert_eq!(record["runMode"], "all");
        }
        let parsed = if kind == "action" {
            AutomationKind::Action
        } else {
            AutomationKind::Event
        };
        let before = f.core.automation_get_versioned(parsed, "valid").unwrap();
        let authority = f.authority(kind, "valid").await;
        let mut malformed = Vec::new();
        let base = json!({"id":"bad","name":"Malformed","enabled":false,"typeId":"core.log","trigger":"tiktok.chat"});
        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove(key);
        malformed.push(missing);
        let mut invalid = base.clone();
        invalid[key] = json!([]);
        malformed.push(invalid);
        if kind == "action" {
            for config in [Value::Null, json!([]), json!("not-an-object")] {
                let mut invalid = base.clone();
                invalid["config"] = config;
                malformed.push(invalid);
            }
        } else {
            for (field, value) in [
                ("actionIds", json!([7])),
                (
                    "filters",
                    json!([{"path":"event.data.x","operator":"eq","value":{}}]),
                ),
                (
                    "filters",
                    json!([{"path":"event.data.x","operator":"unknown"}]),
                ),
                ("cooldownMs", json!(-1)),
                ("cooldownScope", json!("unknown")),
                ("runMode", json!("unknown")),
                ("schemaVersion", json!(2)),
            ] {
                let mut invalid = base.clone();
                invalid[field] = value;
                malformed.push(invalid);
            }
        }
        for (index, body) in malformed.into_iter().enumerate() {
            for update in [false, true] {
                let command = format!("malformed-{kind}-{index}-{update}");
                let method = if update {
                    "automation.update"
                } else {
                    "automation.create"
                };
                let mut params = json!({"kind":kind,"record":body});
                if update {
                    params["id"] = "valid".into();
                }
                let denied = f
                    .remote(
                        &author,
                        method,
                        params,
                        metadata(&instance, &command, if update { "1" } else { "0" }),
                    )
                    .await;
                assert_eq!(denied.error.unwrap().code, "invalid_params");
                assert!(denied.result.is_none());
                assert!(denied.meta.is_none());
                assert_eq!(
                    f.core.automation_get_versioned(parsed, "valid").unwrap(),
                    before
                );
                assert_eq!(f.authority(kind, "valid").await, authority);
                let absent = f.core.automation_get_versioned(parsed, "bad").unwrap();
                assert!(absent.value.is_none());
                assert_eq!(absent.resource_versions[0].revision, "0");
                assert_eq!(
                    f.core
                        .collaboration_command_get(
                            tiktools_core::control::CommandCaller::LocalOwner,
                            tiktools_core::collaboration::CommandGet {
                                command_id: command,
                                principal_id: Some(author.principal.clone())
                            }
                        )
                        .unwrap_err()
                        .code,
                    "not_found"
                );
                let snapshot = f.core.behavior_snapshot_versioned().unwrap();
                for (family, key) in [("actions", "typeId"), ("events", "trigger")] {
                    for row in snapshot.value[family].as_array().unwrap() {
                        assert!(row[key].is_string());
                        assert!(row["name"].is_string());
                        assert!(row["enabled"].is_boolean());
                    }
                }
            }
        }
    }
    f.close().await;
}

#[tokio::test]
async fn malformed_remote_drafts_do_not_reveal_whether_owner_or_foreign_payload_validation_would_pass(
) {
    let f = Fixture::new();
    let ids = [
        "action:owner",
        "event:owner",
        "action:foreign",
        "event:foreign",
    ];
    let author = f.pair(grant(&ids));
    let foreign = f.pair(grant(&ids));
    let instance = f.instance();
    for kind in ["action", "event"] {
        let parsed = if kind == "action" {
            AutomationKind::Action
        } else {
            AutomationKind::Event
        };
        f.core
            .automation_create(parsed, draft(kind, "owner"))
            .unwrap();
        assert_pending(
            &f.remote(
                &foreign,
                "automation.create",
                json!({"kind":kind,"record":draft(kind,"foreign")}),
                metadata(&instance, &format!("foreign-{kind}"), "0"),
            )
            .await,
            kind,
            "foreign",
            "1",
        );
        for id in ["owner", "foreign"] {
            let before = f.core.automation_get_versioned(parsed, id).unwrap();
            let denied=f.remote(&author,"automation.update",json!({"kind":kind,"id":id,"record":{"name":"Malformed","enabled":false,"typeId":null,"trigger":null,"filters":"private-malformed-payload"}}),metadata(&instance,&format!("cannot-edit-{kind}-{id}"),"1")).await;
            assert_eq!(denied.error.unwrap().code, "forbidden");
            assert!(denied.result.is_none());
            assert!(denied.meta.is_none());
            assert_eq!(f.core.automation_get_versioned(parsed, id).unwrap(), before);
        }
    }
    f.close().await;
}
