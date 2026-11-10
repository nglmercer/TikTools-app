//! Real saved approvals, VM execution, child rules and external handoff.
use super::*;
use crate::automation_runtime::context::ExecutionContext;
use crate::control::{EffectKind, EffectState};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn script_grant() -> Grant {
    let mut grant = grant();
    grant
        .capabilities
        .extend([Capability::ScriptsExecute, Capability::EventsEmit]);
    grant.effects.emit_topics.insert("moderator.adjust".into());
    grant
}
fn approve_script(f: &Fixture, auth: &Authorization, source: &str) {
    f.draft(auth,AutomationKind::Action,json!({"id":"script","name":"Scoped script","enabled":false,"typeId":"core.code","config":{"source":source}}));
    f.approve(
        AutomationKind::Action,
        "script",
        vec![enable(AutomationKind::Action, "script")],
    );
}
fn owner_points_child(f: &Fixture, delta: f64) {
    let mut action = points("owner-child", "{{event.user.uniqueId}}", delta);
    action["enabled"] = true.into();
    f.core()
        .automation_create(AutomationKind::Action, action)
        .unwrap();
    f.core().automation_create(AutomationKind::Event,json!({"id":"owner-event","name":"Owner child","enabled":true,"trigger":"moderator.adjust","actionIds":["owner-child"]})).unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reusable_script_runs_public_input_once_and_stops_after_revocation() {
    let f = Fixture::new();
    let mut allowed = grant();
    allowed
        .capabilities
        .extend([Capability::AutomationToggle, Capability::ScriptsExecute]);
    allowed.automation_delegation = Some(AutomationDelegation {
        effects: [DelegatedAutomationEffect::Script].into(),
        allow_dynamic_targets: true,
        allow_owner_edits: false,
        allow_owner_source: false,
        allow_other_author_edits: false,
        allow_other_author_source: false,
    });
    let auth = f.pair(allowed.clone());
    let identity = auth.snapshot().unwrap();
    for (kind, record) in [
        (
            AutomationKind::Action,
            json!({"id":"script","name":"Code","enabled":true,"typeId":"core.code","schemaVersion":2,"config":{"source":"return {log:[event.user.uniqueId]};"}}),
        ),
        (
            AutomationKind::Event,
            json!({"id":"rule","name":"Chat","enabled":true,"trigger":"tiktok.chat","actionIds":["script"],"filters":[],"cooldownMs":0,"cooldownScope":"global","runMode":"all","schemaVersion":1}),
        ),
    ] {
        f.core()
            .automation_command(
                CommandCaller::Remote(auth.clone()),
                f.request("0"),
                AutomationMutation::Create { kind, record },
            )
            .unwrap();
    }
    f.core()
        .run_automation_event(AutomationEvent::owner_without_delegation(input("alice")))
        .await;
    assert!(f
        .core()
        .automation_effect_history(None, 10)
        .unwrap()
        .items
        .is_empty());
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    let receipts = f.core().automation_effect_history(None, 10).unwrap();
    assert_eq!(receipts.items.len(), 1);
    assert_eq!(receipts.items[0].kind, EffectKind::Script);
    assert_eq!(receipts.items[0].state, EffectState::Succeeded);
    let runs = f.core().automation_runs();
    assert!(
        runs.iter()
            .any(|run| run["actionId"] == "script" && run["status"] == "ok"),
        "{runs:?}"
    );
    assert!(!serde_json::to_string(&runs).unwrap().contains("private-"));
    allowed.automation_delegation = None;
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: allowed,
        })
        .unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(
        f.core()
            .automation_effect_history(None, 10)
            .unwrap()
            .items
            .len(),
        1
    );
    f.core().shutdown().await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn approved_script_emits_to_owner_points_without_losing_limits_or_replaying() {
    let f = Fixture::new();
    owner_points_child(&f, 2.0);
    let mut grant = script_grant();
    grant.max_points_delta = Some(2.0);
    let auth = f.pair(grant);
    approve_script(&f,&auth,"console.log(event.user.uniqueId); return {emit:[{type:'moderator.adjust',data:{count:2}}]};");
    let body = f.body(AutomationKind::Action, "script");
    let event = input("alice");
    let root = ExecutionContext::owner();
    let run = f
        .core()
        .execute_action_with_context(&body, &event, None, &root, false)
        .await;
    assert_eq!(run["status"], "ok", "{run}");
    assert_eq!(f.points("alice"), 2.0);
    assert!(!run.to_string().contains("private-"));
    assert!(f.core().last_event_for("moderator.adjust").is_none());
    let receipts = f.core().automation_effect_history(None, 10).unwrap();
    assert_eq!(receipts.items.len(), 2);
    assert!(receipts
        .items
        .iter()
        .all(|row| row.state == EffectState::Succeeded));
    assert!(receipts
        .items
        .iter()
        .any(|row| row.kind == EffectKind::Script));
    let replay = f
        .core()
        .execute_action_with_context(&body, &event, None, &root, false)
        .await;
    assert_eq!(replay["status"], "error");
    assert_eq!(f.points("alice"), 2.0);
    f.assert_delegated(AutomationKind::Action, "script");
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_child_cannot_widen_a_script_author_points_limit() {
    let f = Fixture::new();
    owner_points_child(&f, 3.0);
    let mut grant = script_grant();
    grant.max_points_delta = Some(2.0);
    let auth = f.pair(grant);
    approve_script(
        &f,
        &auth,
        "return {emit:{type:'moderator.adjust',data:{count:2}}};",
    );
    let _ = execute(&f, "script", &input("alice")).await;
    assert_eq!(f.points("alice"), 0.0);
    assert!(f
        .core()
        .automation_runs()
        .iter()
        .any(|run| run["actionId"] == "owner-child" && run["status"] == "error"));
}
#[cfg(feature = "http")]
fn http_rule(origin: String, all_fields: bool) -> HttpEffectGrant {
    HttpEffectGrant {
        origin,
        methods: ["POST".into()].into(),
        paths: ["/hook".into()].into(),
        query_fields: Default::default(),
        header_names: ["content-type".into()].into(),
        render_fields: if all_fields {
            PUBLIC_EFFECT_RENDER_FIELDS
                .iter()
                .map(|field| (*field).into())
                .collect()
        } else {
            ["event.data.comment".into()].into()
        },
        body_fields: ["comment".into()].into(),
        allow_text_body: false,
        max_body_bytes: 1024,
        max_response_bytes: 64,
        allow_private_network: true,
    }
}
#[cfg(feature = "http")]
fn fetch_source(origin: &str) -> String {
    format!("return {{log:[event.user.uniqueId],fetch:{{url:{},method:'POST',headers:{{'content-type':'application/json'}},body:{{comment:data.comment}},allowPrivateNetwork:true,maxResponseBytes:64}}}};",serde_json::to_string(&format!("{origin}/hook")).unwrap())
}
#[cfg(feature = "http")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opaque_script_http_literals_require_all_input_fields_and_one_pinned_request() {
    let f = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut grant = script_grant();
    grant.capabilities.insert(Capability::HttpRequest);
    grant.effects.http.push(http_rule(origin.clone(), true));
    let auth = f.pair(grant);
    approve_script(&f, &auth, &fetch_source(&origin));
    let server = tokio::spawn(async move {
        let (mut socket, _) =
            tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0u8; 4096];
            let read =
                tokio::time::timeout(std::time::Duration::from_secs(5), socket.read(&mut chunk))
                    .await
                    .unwrap()
                    .unwrap();
            assert!(read > 0, "incomplete request");
            bytes.extend_from_slice(&chunk[..read]);
            assert!(bytes.len() <= 8192);
            let text = String::from_utf8_lossy(&bytes);
            if let Some(end) = text.find("\r\n\r\n") {
                let body_len = text[..end]
                    .lines()
                    .find_map(|line| {
                        line.split_once(':')
                            .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + body_len {
                    break;
                }
            }
        }
        let request = String::from_utf8_lossy(&bytes);
        assert!(request.contains("hello"));
        assert!(!request.contains("private-"));
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
    });
    let root = ExecutionContext::owner();
    let body = f.body(AutomationKind::Action, "script");
    let event = input("alice");
    let run = f
        .core()
        .execute_action_with_context(&body, &event, None, &root, false)
        .await;
    assert_eq!(run["status"], "ok", "{run}");
    server.await.unwrap();
    let replay = f
        .core()
        .execute_action_with_context(&body, &event, None, &root, false)
        .await;
    assert_eq!(replay["status"], "error");
    let receipts = f.core().automation_effect_history(None, 10).unwrap();
    assert_eq!(receipts.items.len(), 2);
    assert!(receipts
        .items
        .iter()
        .all(|row| row.state == EffectState::Succeeded));
}
#[cfg(feature = "http")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn script_cannot_convert_disallowed_input_flow_to_an_http_literal() {
    let f = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut grant = script_grant();
    grant.capabilities.insert(Capability::HttpRequest);
    grant.effects.http.push(http_rule(origin.clone(), false));
    let auth = f.pair(grant);
    approve_script(&f, &auth, &fetch_source(&origin));
    let run = execute(&f, "script", &input("alice")).await;
    assert_eq!(run["status"], "error", "{run}");
    let receipts = f.core().automation_effect_history(None, 10).unwrap();
    assert_eq!(receipts.items.len(), 1);
    assert_eq!(receipts.items[0].kind, EffectKind::Script);
    assert_eq!(receipts.items[0].state, EffectState::Unknown);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept())
            .await
            .is_err()
    );
}

#[cfg(feature = "http")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn script_flow_survives_an_emitted_event_and_owner_http_rule() {
    let f = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    f.core().automation_create(AutomationKind::Action,json!({"id":"owner-http","name":"Owner HTTP","enabled":true,"typeId":"core.fetch","config":{
        "url":format!("{origin}/hook"),"method":"POST","headers":{"content-type":"application/json"},"body":"{\"comment\":\"{{event.data.comment}}\"}","allowPrivateNetwork":true,"maxResponseBytes":64
    }})).unwrap();
    f.core().automation_create(AutomationKind::Event,json!({"id":"owner-http-event","name":"Owner child HTTP","enabled":true,"trigger":"moderator.adjust","actionIds":["owner-http"]})).unwrap();
    let mut grant = script_grant();
    grant.capabilities.insert(Capability::HttpRequest);
    grant.effects.http.push(http_rule(origin.clone(), false));
    let auth = f.pair(grant);
    approve_script(
        &f,
        &auth,
        "return {emit:{type:'moderator.adjust',data:{comment:data.comment}}};",
    );
    let run = execute(&f, "script", &input("alice")).await;
    assert_eq!(run["status"], "ok", "{run}");
    assert!(f
        .core()
        .automation_runs()
        .iter()
        .any(|run| run["actionId"] == "owner-http" && run["status"] == "error"));
    let receipts = f.core().automation_effect_history(None, 10).unwrap();
    assert_eq!(receipts.items.len(), 2);
    assert!(receipts
        .items
        .iter()
        .all(|row| row.kind != EffectKind::Http));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saturated_script_capacity_rejects_before_vm_or_effect_acceptance() {
    let f = Fixture::new();
    let auth = f.pair(script_grant());
    approve_script(&f, &auth, "return {log:['bounded']};");
    let held = f
        .core()
        .automation_state
        .scoped_script_slots
        .clone()
        .acquire_many_owned(crate::services::SCOPED_SCRIPT_LIMITS.max_concurrent_evaluations as u32)
        .await
        .unwrap();
    let root = ExecutionContext::owner();
    let body = f.body(AutomationKind::Action, "script");
    let event = input("alice");
    let busy = f
        .core()
        .execute_action_with_context(&body, &event, None, &root, false)
        .await;
    assert_eq!(busy["status"], "error");
    assert!(busy["summary"].as_str().unwrap().contains("busy"));
    assert!(f
        .core()
        .automation_effect_history(None, 10)
        .unwrap()
        .items
        .is_empty());
    drop(held);
    let admitted = f
        .core()
        .execute_action_with_context(&body, &event, None, &root, false)
        .await;
    assert_eq!(admitted["status"], "ok", "{admitted}");
    assert_eq!(
        f.core()
            .automation_effect_history(None, 10)
            .unwrap()
            .items
            .len(),
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_root_nested_script_emits_reach_depth_two_and_cannot_reset_depth_at_three() {
    let f = Fixture::new();
    let mut allowed = script_grant();
    allowed
        .effects
        .emit_topics
        .insert("moderator.second".into());
    allowed.max_points_delta = Some(2.0);
    let auth = f.pair(allowed);
    f.draft(&auth,AutomationKind::Action,json!({"id":"script","enabled":false,"typeId":"core.code","config":{"source":"return {emit:{type:'moderator.adjust',data:{count:2}}};"}}));
    f.draft(
        &auth,
        AutomationKind::Event,
        event("delegated-root", &["script"]),
    );
    f.approve(
        AutomationKind::Event,
        "delegated-root",
        vec![
            enable(AutomationKind::Event, "delegated-root"),
            enable(AutomationKind::Action, "script"),
        ],
    );
    f.core().automation_create(AutomationKind::Action,json!({"id":"owner-second-script","enabled":true,"typeId":"core.code","config":{"source":"return {emit:{type:'moderator.second',data:{count:2,depth:0}}};"}})).unwrap();
    f.core().automation_create(AutomationKind::Event,json!({"id":"owner-first-event","enabled":true,"trigger":"moderator.adjust","actionIds":["owner-second-script"]})).unwrap();
    let mut points = points("owner-final-points", "alice", 2.0);
    points["enabled"] = true.into();
    f.core()
        .automation_create(AutomationKind::Action, points)
        .unwrap();
    f.core().automation_create(AutomationKind::Action,json!({"id":"owner-loop-script","enabled":true,"typeId":"core.code","config":{"source":"return {emit:{type:'moderator.adjust',data:{depth:0,count:2}}};"}})).unwrap();
    f.core().automation_create(AutomationKind::Event,json!({"id":"owner-second-event","enabled":true,"trigger":"moderator.second","actionIds":["owner-final-points","owner-loop-script"]})).unwrap();
    f.core()
        .run_automation_event(AutomationEvent::owner(input("alice")))
        .await;
    assert_eq!(f.points("alice"), 2.0);
    let runs = f.core().automation_runs();
    assert_eq!(
        runs.iter()
            .filter(|run| run["actionId"] == "owner-final-points")
            .count(),
        1
    );
    assert!(runs
        .iter()
        .any(|run| run["actionId"] == "owner-loop-script" && run["status"] == "error"));
    assert!(runs.iter().any(|run| run["actionId"] == "owner-loop-script"
        && run["error"]
            .as_str()
            .is_some_and(|message| message.contains("depth"))));
    let receipts = f.core().automation_effect_history(None, 20).unwrap();
    assert_eq!(
        receipts
            .items
            .iter()
            .filter(|row| row.kind == EffectKind::Emit)
            .count(),
        2
    );
    assert_eq!(
        receipts
            .items
            .iter()
            .filter(|row| row.kind == EffectKind::Script)
            .count(),
        3
    );
    assert!(f.core().last_event_for("moderator.second").is_none());
}
