use super::*;
use crate::{RequestMetadata, TikToolsClient};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tiktools_collaboration::{Capability, Configuration, Grant, InviteCreate};
use tiktools_core::{
    ipc::messages::HostMessage, paths::AppPaths, AppCore, HostEmitter, NoopMediaHost,
};
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
pub(super) struct Fixture {
    pub home: tempfile::TempDir,
    pub core: Arc<AppCore>,
    pub service: Arc<tiktools_collaboration::CollaborationService>,
    pub endpoint: RemoteEndpoint,
    pub pin: String,
    pub origin: String,
    gateway: tiktools_remote::RemoteGateway,
}
impl Fixture {
    pub async fn new() -> Self {
        Self::configured(|_| {}, |_| {}).await
    }
    pub async fn configured(
        cert: impl FnOnce(&mut rcgen::CertificateParams),
        config: impl FnOnce(&mut Configuration),
    ) -> Self {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("host");
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root,
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
        let api = Arc::new(crate::ControlApi::new(core.clone()));
        let service = core.collaboration().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("https://{address}");
        let mut params = rcgen::CertificateParams::new(vec!["127.0.0.1".into()]).unwrap();
        cert(&mut params);
        let key = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        let path = home.path().join("cert.pem");
        let key_path = home.path().join("key.pem");
        std::fs::write(&path, cert.pem()).unwrap();
        std::fs::write(&key_path, key.serialize_pem()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let mut configuration = Configuration {
            enabled: true,
            bind_address: address.to_string(),
            advertised_origin: Some(origin.clone()),
            tls_certificate: Some(path.to_string_lossy().into()),
            tls_private_key: Some(key_path.to_string_lossy().into()),
            ..Configuration::default()
        };
        config(&mut configuration);
        service.configure(configuration).unwrap();
        let pin = super::trust::fingerprint(cert.der());
        let endpoint = RemoteEndpoint::new(
            &origin,
            TlsTrust::CertificateSha256 {
                sha256: pin.clone(),
            },
        )
        .await
        .unwrap();
        let gateway = tiktools_remote::start_on(api, listener).await.unwrap();
        Self {
            home,
            core,
            service,
            endpoint,
            pin,
            origin,
            gateway,
        }
    }
    pub async fn pair(&self, grant: Grant) -> NativeCredentials {
        let invite = self
            .service
            .create_invitation(InviteCreate { grant })
            .unwrap();
        let mut pending = self
            .endpoint
            .begin_pairing(
                &invite.invite_id,
                &SecretString::from(invite.secret),
                "SDK controller",
                ClientKind::Agent,
            )
            .await
            .unwrap();
        assert!(matches!(
            pending.poll().await.unwrap(),
            PairingPoll::Pending
        ));
        self.service
            .decide_request(&pending.request_id, true)
            .unwrap();
        match pending.poll().await.unwrap() {
            PairingPoll::Approved(credentials) => credentials,
            _ => panic!("approved pairing stayed pending"),
        }
    }
    pub async fn finish(self) {
        self.gateway.shutdown().await.unwrap();
        self.core.shutdown().await;
        drop(self.endpoint);
        drop(self.service);
        drop(self.core);
        drop(self.home);
    }
}
pub(super) fn workflow_grant() -> Grant {
    Grant {
        capabilities: [Capability::WorkflowsRead, Capability::LiveRead].into(),
        resources: [(
            tiktools_collaboration::ResourceKind::Workflow,
            ["welcome".into()].into(),
        )]
        .into(),
        ..Grant::default()
    }
}
#[tokio::test]
async fn native_pairing_negotiation_typed_calls_and_metadata_share_one_host_without_client_runtime()
{
    let f = Fixture::new().await;
    let credentials = f.pair(workflow_grant()).await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Allowed"}))
        .unwrap();
    f.core
        .workflow_save(json!({"id":"hidden","name":"Hidden"}))
        .unwrap();
    let remote = RemoteClient::connect(&f.endpoint, &credentials, ClientKind::Agent)
        .await
        .unwrap();
    assert_eq!(remote.hello().session_id, credentials.session_id());
    let sdk = TikToolsClient::Remote(remote.clone());
    let (a, b) = tokio::join!(
        sdk.call_value("live.status", json!({})),
        sdk.call_with_meta::<_, serde_json::Value>("workflows.get", json!({"id":"welcome"}), None)
    );
    assert_eq!(a.unwrap()["connected"], false);
    let b = b.unwrap();
    assert_eq!(b.result["id"], "welcome");
    assert_eq!(b.meta.unwrap().resource_versions[0].revision, "1");
    assert_eq!(
        sdk.call_value("workflows.get", json!({"id":"hidden"}))
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert_eq!(
        sdk.call_value("system.shutdown", json!({}))
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert!(!f.core.is_shutdown());
    let schema = sdk.call_value("rpc.discover", json!({})).await.unwrap();
    assert!(!schema["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["name"] == "system.shutdown"));
    assert_eq!(f.service.gateway_status().unwrap().connections, 1);
    remote.close();
    drop(sdk);
    drop(remote);
    f.finish().await;
}
#[tokio::test]
async fn certificate_probe_does_not_approve_trust_and_pins_names_and_dates_are_enforced() {
    let f = Fixture::new().await;
    assert_eq!(probe_fingerprint(&f.origin).await.unwrap(), f.pin);
    let system = RemoteEndpoint::new(&f.origin, TlsTrust::System)
        .await
        .unwrap();
    assert!(system.info().await.is_err());
    let wrong = RemoteEndpoint::new(
        &f.origin,
        TlsTrust::CertificateSha256 {
            sha256: "f".repeat(64),
        },
    )
    .await
    .unwrap();
    assert!(wrong.info().await.is_err());
    assert_eq!(
        f.endpoint.info().await.unwrap().host_id,
        f.service.host_id()
    );
    f.finish().await;
    let expired = Fixture::configured(
        |p| {
            p.not_before = rcgen::date_time_ymd(2000, 1, 1);
            p.not_after = rcgen::date_time_ymd(2001, 1, 1);
        },
        |_| {},
    )
    .await;
    assert!(expired.endpoint.info().await.is_err());
    expired.finish().await;
    let wrong_name = Fixture::configured(
        |p| {
            p.subject_alt_names = vec![rcgen::SanType::DnsName("other.example".try_into().unwrap())]
        },
        |_| {},
    )
    .await;
    assert!(wrong_name.endpoint.info().await.is_err());
    wrong_name.finish().await;
    let wrong_purpose = Fixture::configured(
        |p| p.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ClientAuth],
        |_| {},
    )
    .await;
    assert!(wrong_purpose.endpoint.info().await.is_err());
    wrong_purpose.finish().await;
}
#[tokio::test]
async fn negotiated_budgets_bound_admission_and_reject_large_params_without_killing_the_connection()
{
    let f = Fixture::configured(
        |_| {},
        |c| {
            c.limits.pending_calls = 1;
            c.limits.frame_bytes = 4096;
            c.limits.params_bytes = 1024;
        },
    )
    .await;
    let creds = f.pair(workflow_grant()).await;
    let remote = RemoteClient::connect(&f.endpoint, &creds, ClientKind::Agent)
        .await
        .unwrap();
    assert_eq!(
        remote
            .call_value_with_meta("live.status", json!({"large":"x".repeat(2048)}), None)
            .await
            .unwrap_err()
            .code,
        "too_large"
    );
    let (first, second) = tokio::join!(
        remote.call_value_with_meta("live.status", json!({}), None),
        remote.call_value_with_meta("live.status", json!({}), None)
    );
    assert!(first.is_ok());
    assert_eq!(second.unwrap_err().code, "overloaded");
    assert!(remote
        .call_value_with_meta("live.status", json!({}), None)
        .await
        .is_ok());
    remote.close();
    f.finish().await;
}
#[tokio::test]
async fn recovery_preserves_sequence_until_ack_and_hides_disconnected_state() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    f.core
        .workflow_save(json!({"id":"welcome","name":"Original"}))
        .unwrap();
    let remote = RemoteClient::connect(&f.endpoint, &creds, ClientKind::Agent)
        .await
        .unwrap();
    let mut recovery = RecoveryStream::start(remote.clone(), [StreamDomain::Workflows].into())
        .await
        .unwrap();
    assert!(recovery.is_ready());
    assert_eq!(recovery.baseline().unwrap().workflows.len(), 1);
    f.core.workflow_set_enabled("welcome", true).unwrap();
    loop {
        let before = recovery.cursor().unwrap().last_applied_sequence.clone();
        let event = tokio::time::timeout(Duration::from_secs(4), recovery.next_event())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovery.cursor().unwrap().last_applied_sequence, before);
        assert_eq!(
            recovery.next_event().await.unwrap_err().code,
            "ack_required"
        );
        recovery.acknowledge(&event).unwrap();
        if event.resource.revision == "2" {
            break;
        }
    }
    remote.close();
    assert!(recovery.baseline().is_none());
    assert!(!recovery.is_ready());
    drop(recovery);
    f.finish().await;
}

#[tokio::test]
async fn mixed_recovery_pages_match_versions_by_kind_and_points_commands_stay_private() {
    use tiktools_collaboration::{Grant as G, ResourceKind};
    let f = Fixture::configured(
        |_| {},
        |c| {
            c.limits.frame_bytes = 4096;
            c.limits.params_bytes = 1024;
        },
    )
    .await;
    let grant = G {
        capabilities: [
            Capability::WorkflowsRead,
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(ResourceKind::Viewer, ["same".into()].into())].into(),
        ..G::default()
    };
    let creds = f.pair(grant.clone()).await;
    let other_creds = f.pair(grant).await;
    for n in 0..12 {
        let id = if n == 0 {
            "same".into()
        } else {
            format!("workflow-{n}")
        };
        f.core
            .workflow_save(json!({"id":id,"name":"Workflow","body":"x".repeat(600)}))
            .unwrap();
    }
    f.core
        .points_command(
            tiktools_core::control::CommandCaller::LocalOwner,
            tiktools_collaboration::AdditiveCommandRequest {
                command_id: "seed".into(),
                host_instance_id: f.service.instance_id().into(),
            },
            tiktools_core::control::PointsAdjustment {
                expected_currency: None,
                unique_id: "same".into(),
                delta: 3.0,
            },
        )
        .unwrap();
    let remote = RemoteClient::connect(&f.endpoint, &creds, ClientKind::Agent)
        .await
        .unwrap();
    let other = RemoteClient::connect(&f.endpoint, &other_creds, ClientKind::Agent)
        .await
        .unwrap();
    let mut stream = RecoveryStream::start(
        remote.clone(),
        [
            StreamDomain::Workflows,
            StreamDomain::Points,
            StreamDomain::Commands,
        ]
        .into(),
    )
    .await
    .unwrap();
    let baseline = stream.baseline().unwrap();
    assert_eq!(baseline.workflows.len(), 12);
    assert_eq!(baseline.points.len(), 1);
    assert_eq!(baseline.resource_versions.len(), 13);
    assert_eq!(baseline.points[0]["uniqueId"], "same");
    let mut other_stream = RecoveryStream::start(
        other.clone(),
        [StreamDomain::Points, StreamDomain::Commands].into(),
    )
    .await
    .unwrap();
    // The second controller needs no workflow subscription for points recovery.
    let sdk = TikToolsClient::Remote(remote.clone());
    let result = sdk
        .call_with_meta::<_, serde_json::Value>(
            "points.adjust",
            json!({"uniqueId":"same","delta":2}),
            Some(RequestMetadata {
                command_id: Some("adjust-once".into()),
                host_instance_id: Some(f.service.instance_id().into()),
                expected_revision: None,
            }),
        )
        .await
        .unwrap();
    assert_eq!(result.result["totalPoints"], 5.0);
    let mut topics = std::collections::BTreeSet::new();
    while topics.len() < 2 {
        let event = tokio::time::timeout(Duration::from_secs(4), stream.next_event())
            .await
            .unwrap()
            .unwrap();
        if event.command_id.as_deref() == Some("adjust-once") {
            topics.insert(event.topic.clone());
        }
        stream.acknowledge(&event).unwrap();
    }
    assert_eq!(
        topics,
        ["command.changed".to_owned(), "points.changed".to_owned()].into()
    );
    loop {
        let event = tokio::time::timeout(Duration::from_secs(4), other_stream.next_event())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.topic, "points.changed");
        assert!(event.command_id.is_none());
        let current = event.resource.revision == "2";
        other_stream.acknowledge(&event).unwrap();
        if current {
            break;
        }
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(200), other_stream.next_event())
            .await
            .is_err()
    );
    remote.close();
    other.close();
    f.finish().await;
}

#[tokio::test]
async fn projected_recovery_distinguishes_complete_top_n_and_notifications_only_baselines() {
    use tiktools_collaboration::StreamDomain;
    let f = Fixture::new().await;
    for (id, amount) in [("bottom", 1.0), ("middle", 2.0), ("top", 3.0)] {
        f.core.points_adjust(id, amount).unwrap();
    }
    let credentials = f
        .pair(Grant {
            capabilities: [Capability::PointsRead].into(),
            ..Default::default()
        })
        .await;
    let remote = RemoteClient::connect(&f.endpoint, &credentials, ClientKind::Agent)
        .await
        .unwrap();
    assert_eq!(remote.hello().features.get("pointsProjection"), Some(&true));
    let domains = [StreamDomain::Points].into();
    let complete = RecoveryStream::start(remote.clone(), domains)
        .await
        .unwrap();
    assert_eq!(complete.baseline().unwrap().points_limit, None);
    assert_eq!(complete.baseline().unwrap().points.len(), 3);
    let top =
        RecoveryStream::start_projected(remote.clone(), [StreamDomain::Points].into(), Some(2))
            .await
            .unwrap();
    assert_eq!(top.baseline().unwrap().points_limit, Some(2));
    assert_eq!(top.baseline().unwrap().points.len(), 2);
    assert_eq!(top.baseline().unwrap().points[0]["uniqueId"], "top");
    assert!(top.baseline().unwrap().points[0].get("avatarUrl").is_none());
    let mut notifications =
        RecoveryStream::start_projected(remote.clone(), [StreamDomain::Points].into(), Some(0))
            .await
            .unwrap();
    assert_eq!(notifications.baseline().unwrap().points_limit, Some(0));
    assert!(notifications.baseline().unwrap().points.is_empty());
    f.core.points_adjust("bottom", 1.0).unwrap();
    let event = tokio::time::timeout(Duration::from_secs(4), notifications.next_event())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.resource.id, "bottom");
    notifications.acknowledge(&event).unwrap();
    assert!(RecoveryStream::start_projected(
        remote.clone(),
        [StreamDomain::Workflows].into(),
        Some(0)
    )
    .await
    .is_err());
    remote.close();
    f.finish().await;
}
#[tokio::test]
async fn access_rotation_keeps_session_identity_and_does_not_reuse_old_credentials() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    let old = RemoteClient::connect(&f.endpoint, &creds, ClientKind::Agent)
        .await
        .unwrap();
    let fresh = f.endpoint.rotate_credentials(&creds).await.unwrap();
    assert_eq!(fresh.session_id(), creds.session_id());
    let new = RemoteClient::connect(&f.endpoint, &fresh, ClientKind::Agent)
        .await
        .unwrap();
    assert!(
        RemoteClient::connect(&f.endpoint, &creds, ClientKind::Agent)
            .await
            .is_err()
    );
    assert!(new
        .call_value_with_meta("live.status", json!({}), Some(RequestMetadata::default()))
        .await
        .is_ok());
    f.endpoint.logout(&fresh).await.unwrap();
    tokio::time::timeout(Duration::from_secs(4), async {
        while !new.is_closed() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    old.close();
    new.close();
    f.finish().await;
}
#[tokio::test]
async fn dropping_last_client_closes_the_socket_even_with_a_live_event_receiver() {
    let f = Fixture::new().await;
    let creds = f.pair(workflow_grant()).await;
    let remote = RemoteClient::connect(&f.endpoint, &creds, ClientKind::Agent)
        .await
        .unwrap();
    let _events = remote.subscribe();
    drop(remote);
    tokio::time::timeout(Duration::from_secs(4), async {
        while f.service.gateway_status().unwrap().connections != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    f.finish().await;
}
#[test]
fn origins_and_fingerprint_input_fail_without_echoing_secret_fields() {
    for url in [
        "http://localhost",
        "https://user:private-secret@example.com",
        "https://example.com/path",
        "https://example.com/?secret=private-secret",
        "https://example.com/#private-secret",
    ] {
        let error = super::http::origin(url).unwrap_err();
        assert!(!error.to_string().contains("private-secret"));
    }
    assert_eq!(
        normalize_fingerprint(&format!(
            "SHA256:{}",
            "AB:".repeat(32).trim_end_matches(':')
        ))
        .unwrap(),
        "ab".repeat(32)
    );
    assert!(normalize_fingerprint("invalid").is_err());
    let mut backoff = ReconnectBackoff::default();
    for _ in 0..100 {
        let delay = backoff.next_delay();
        assert!((Duration::from_millis(500)..=Duration::from_secs(30)).contains(&delay));
    }
    backoff.reset();
    assert!(backoff.next_delay() < Duration::from_secs(1));
}

#[tokio::test]
async fn tls_controllers_recover_family_scoped_automation_summaries_and_exact_event_receipts() {
    use tiktools_collaboration::{ResourceKind, StreamDomain};
    use tiktools_core::control::AutomationKind;
    let f = Fixture::new().await;
    for kind in [AutomationKind::Action, AutomationKind::Event] {
        f.core.automation_create(kind,json!({"id":"same","name":"Allowed","enabled":true,"actionIds":["hidden"],"config":{"secret":"owner-secret"},"source":"owner-source"})).unwrap();
    }
    f.core
        .automation_create(
            AutomationKind::Event,
            json!({"id":"hidden","name":"Hidden private label","enabled":true}),
        )
        .unwrap();
    let creds = f
        .pair(Grant {
            capabilities: [
                Capability::AutomationRead,
                Capability::AutomationToggle,
                Capability::AutomationEdit,
                Capability::CommandsRead,
            ]
            .into(),
            resources: [(
                ResourceKind::Automation,
                ["event:same".into(), "action:same".into()].into(),
            )]
            .into(),
            ..Grant::default()
        })
        .await;
    let other_creds = f
        .pair(Grant {
            capabilities: [Capability::AutomationRead, Capability::CommandsRead].into(),
            resources: [(ResourceKind::Automation, ["event:same".into()].into())].into(),
            ..Grant::default()
        })
        .await;
    let remote = RemoteClient::connect(&f.endpoint, &creds, ClientKind::Agent)
        .await
        .unwrap();
    let other = RemoteClient::connect(&f.endpoint, &other_creds, ClientKind::Agent)
        .await
        .unwrap();
    assert_eq!(
        remote.hello().features.get("automationSummary"),
        Some(&true)
    );
    let domains = [StreamDomain::Automation, StreamDomain::Commands].into();
    let mut stream = RecoveryStream::start(remote.clone(), domains)
        .await
        .unwrap();
    let mut other_stream = RecoveryStream::start(
        other.clone(),
        [StreamDomain::Automation, StreamDomain::Commands].into(),
    )
    .await
    .unwrap();
    let baseline = stream.baseline().unwrap();
    assert_eq!(baseline.automation.len(), 2);
    assert_eq!(
        baseline
            .resource_versions
            .iter()
            .filter(|v| v.id == "same")
            .count(),
        2
    );
    assert!(!serde_json::to_string(baseline)
        .unwrap()
        .contains("owner-secret"));
    assert!(!serde_json::to_string(baseline)
        .unwrap()
        .contains("Hidden private label"));
    assert_eq!(other_stream.baseline().unwrap().automation.len(), 1);
    let sdk = TikToolsClient::Remote(remote.clone());
    // Scoped moderators with `AutomationToggle` may disable granted actions
    // (e5ca27b1); delete and enable stay forbidden for actions.
    for method in ["automation.delete", "automation.enable"] {
        let denied = sdk
            .call_with_meta::<_, serde_json::Value>(
                method,
                json!({"id":"same","kind":"action"}),
                Some(RequestMetadata {
                    command_id: Some("action-denied".into()),
                    host_instance_id: Some(f.service.instance_id().into()),
                    expected_revision: Some("1".into()),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(denied.code, "forbidden", "{method}");
    }
    let meta = RequestMetadata {
        command_id: Some("stop-once".into()),
        host_instance_id: Some(f.service.instance_id().into()),
        expected_revision: Some("1".into()),
    };
    let response = sdk
        .call_with_meta::<_, serde_json::Value>(
            "automation.disable",
            json!({"id":"same","kind":"event"}),
            Some(meta.clone()),
        )
        .await
        .unwrap();
    assert_eq!(response.result, json!({"id":"same","enabled":false}));
    assert_eq!(response.meta.unwrap().resource_versions[0].revision, "2");
    let mut topics = std::collections::BTreeSet::new();
    while topics.len() < 2 {
        let event = tokio::time::timeout(Duration::from_secs(4), stream.next_event())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.resource.scope, "event");
        if event.command_id.as_deref() == Some("stop-once") {
            topics.insert(event.topic.clone());
        }
        stream.acknowledge(&event).unwrap();
    }
    assert_eq!(
        topics,
        ["automation.changed".into(), "command.changed".into()].into()
    );
    loop {
        let event = tokio::time::timeout(Duration::from_secs(4), other_stream.next_event())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.topic, "automation.changed");
        assert!(event.command_id.is_none());
        let current = event.resource.revision == "2";
        other_stream.acknowledge(&event).unwrap();
        if current {
            break;
        }
    }
    // Lose the local acknowledgement and resolve its original ID on a fresh
    // TLS connection after the owner has changed the event again.
    f.core
        .automation_set_enabled(AutomationKind::Event, "same", true)
        .unwrap();
    drop(sdk);
    drop(stream);
    remote.close();
    let reopened = RemoteClient::connect(&f.endpoint, &creds, ClientKind::Agent)
        .await
        .unwrap();
    let sdk = TikToolsClient::Remote(reopened.clone());
    let outcome = sdk
        .call_value(
            "collaboration.commands.get",
            json!({"commandId":"stop-once"}),
        )
        .await
        .unwrap();
    assert_eq!(outcome["result"], json!({"id":"same","enabled":false}));
    let replay = sdk
        .call_with_meta::<_, serde_json::Value>(
            "automation.disable",
            json!({"id":"same","kind":"event"}),
            Some(meta),
        )
        .await
        .unwrap();
    assert_eq!(replay.meta.unwrap().resource_versions[0].revision, "2");
    assert_eq!(
        f.core
            .automation_get(AutomationKind::Event, "same")
            .unwrap()["enabled"],
        true
    );
    assert_eq!(
        f.core
            .automation_get_versioned(AutomationKind::Action, "same")
            .unwrap()
            .resource_versions[0]
            .revision,
        "1"
    );
    let denied = TikToolsClient::Remote(other.clone())
        .call_value(
            "collaboration.commands.get",
            json!({"commandId":"stop-once","principalId":creds.principal_id()}),
        )
        .await
        .unwrap_err();
    assert_eq!(denied.code, "forbidden");
    f.service
        .revoke_session(creds.session_id(), "owner")
        .unwrap();
    assert!(sdk.call_value("automation.list", json!({})).await.is_err());
    reopened.close();
    other.close();
    drop(other_stream);
    f.finish().await;
}
