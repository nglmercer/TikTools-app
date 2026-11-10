//! Real HTTP/WSS acceptance against one isolated host. Certificates and
//! credentials are generated only in a temporary test directory.
use futures_util::{SinkExt, StreamExt};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tiktools_collaboration::{
    CollaborationService, Configuration, Credentials, Grant, GrantSet, InviteCreate,
};
use tiktools_control_api::ControlApi;
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    tungstenite::{client::IntoClientRequest, Message},
    Connector, MaybeTlsStream, WebSocketStream,
};

static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn stream_event(socket: &mut Socket) -> Value {
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(message) = socket.next().await {
            match message.unwrap() {
                Message::Text(raw) => {
                    let value: Value = serde_json::from_str(&raw).unwrap();
                    if value["method"] == "event" {
                        return value["params"].clone();
                    }
                    assert_ne!(value["method"], "event.gap", "{value}");
                }
                Message::Ping(bytes) => socket.send(Message::Pong(bytes)).await.unwrap(),
                Message::Close(_) => panic!("stream closed before event"),
                _ => {}
            }
        }
        panic!("stream ended before event");
    })
    .await
    .unwrap()
}
async fn audit_revision(fixture: &Fixture, id: &str, revision: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if fixture
                .service
                .audit(tiktools_collaboration::AuditQuery {
                    after: 0,
                    limit: Some(100),
                })
                .unwrap()
                .iter()
                .filter_map(|e| e.details.as_ref())
                .any(|e| e.resource.id == id && e.after_revision == revision)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn tls_streams_filter_state_and_sequences_then_resume_the_same_session_after_disconnect() {
    let _env = ENV.lock().await;
    let fixture = Fixture::new(true).await;
    let grant = Grant {
        capabilities: [
            tiktools_collaboration::Capability::WorkflowsRead,
            tiktools_collaboration::Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            tiktools_collaboration::ResourceKind::Workflow,
            ["welcome".into()].into(),
        )]
        .into(),
        ..Grant::default()
    };
    fixture
        .core
        .workflow_save(json!({"id":"welcome","name":"Original"}))
        .unwrap();
    fixture
        .core
        .workflow_save(json!({"id":"private","name":"Owner private graph"}))
        .unwrap();
    audit_revision(&fixture, "welcome", "1").await;
    let credentials = fixture.native(grant.clone()).await;
    let mut socket = fixture.socket(&credentials.access_token).await;
    let hello = fixture.hello(&mut socket).await;
    assert_eq!(hello["result"]["features"]["subscriptions"], true);
    assert_eq!(
        hello["result"]["subscriptionDomains"],
        json!([
            "workflows",
            "points",
            "automation",
            "competitions",
            "commands"
        ])
    );
    assert_eq!(
        rpc(
            &mut socket,
            2,
            "collaboration.subscribe",
            json!({"domains":["live"]})
        )
        .await["error"]["code"],
        "unsupported_remote"
    );
    assert_eq!(
        rpc(
            &mut socket,
            3,
            "collaboration.subscribe",
            json!({"domains":["workflows"],"feed":true})
        )
        .await["error"]["code"],
        "unsupported_remote"
    );
    let sub = rpc(
        &mut socket,
        4,
        "collaboration.subscribe",
        json!({"domains":["workflows","commands"]}),
    )
    .await;
    let id = sub["result"]["subscriptionId"].clone();
    assert!(id.is_string(), "{sub}");
    let snapshot = rpc(
        &mut socket,
        5,
        "collaboration.snapshot",
        json!({"subscriptionId":id}),
    )
    .await;
    assert_eq!(snapshot["result"]["workflows"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["result"]["workflows"][0]["id"], "welcome");
    assert_eq!(snapshot["result"]["resourceVersions"][0]["revision"], "1");
    let baseline = snapshot["result"]["throughSequence"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let saved=fixture.api.execute_value(&json!({"id":"owner-save","method":"workflows.save","params":{"graph":{"id":"welcome","name":"Changed"}},"meta":{"commandId":"private-owner-command","hostInstanceId":fixture.service.instance_id(),"expectedRevision":"1"}})).await;
    assert!(saved.is_ok());
    let event = stream_event(&mut socket).await;
    assert_eq!(event["resource"]["id"], "welcome");
    assert_eq!(event["resource"]["revision"], "2");
    assert_eq!(event["sequence"], (baseline + 1).to_string());
    assert!(event.get("commandId").is_none());
    assert!(!event.to_string().contains("private-owner-command"));
    let listed = rpc(&mut socket, 6, "workflows.list", json!({})).await;
    assert_eq!(listed["result"]["workflows"].as_array().unwrap().len(), 1);
    assert_eq!(
        listed["meta"]["resourceVersions"].as_array().unwrap().len(),
        1
    );
    for resource in ["private", " welcome "] {
        assert_eq!(
            rpc(&mut socket, 7, "workflows.get", json!({"id":resource})).await["error"]["code"],
            "forbidden"
        );
    }
    socket.close(None).await.unwrap();
    drop(socket);
    tokio::time::timeout(Duration::from_secs(3), async {
        while fixture.service.gateway_status().unwrap().connections != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    fixture.core.workflow_set_enabled("private", true).unwrap();
    fixture.core.workflow_set_enabled("welcome", true).unwrap();
    audit_revision(&fixture, "welcome", "3").await;
    let other = fixture.native(grant.clone()).await;
    let mut wrong = fixture.socket(&other.access_token).await;
    fixture.hello(&mut wrong).await;
    let params = json!({"subscriptionId":id,"hostInstanceId":fixture.service.instance_id(),"lastAppliedSequence":event["sequence"]});
    assert_eq!(
        rpc(&mut wrong, 8, "collaboration.resume", params.clone()).await["error"]["code"],
        "resync_required"
    );
    wrong.close(None).await.unwrap();
    drop(wrong);
    let refreshed: Credentials = fixture
        .client
        .post(format!("{}/remote/v1/session/refresh", fixture.origin))
        .header("x-tiktools-client", "native")
        .json(&json!({"refreshToken":credentials.refresh_token}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(refreshed.session_id, credentials.session_id);
    let mut resumed = fixture.socket(&refreshed.access_token).await;
    fixture.hello(&mut resumed).await;
    let response = rpc(&mut resumed, 9, "collaboration.resume", params.clone()).await;
    assert_eq!(response["result"]["replaying"], true, "{response}");
    let next = stream_event(&mut resumed).await;
    assert_eq!(next["resource"]["revision"], "3");
    assert_eq!(next["resource"]["id"], "welcome");
    assert_eq!(next["sequence"], (baseline + 2).to_string());
    fixture
        .service
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "1".into(),
            grant: Grant {
                resources: [(
                    tiktools_collaboration::ResourceKind::Workflow,
                    ["private".into()].into(),
                )]
                .into(),
                ..grant
            },
        })
        .unwrap();
    closed(&mut resumed).await;
    drop(resumed);
    let mut changed = fixture.socket(&refreshed.access_token).await;
    fixture.hello(&mut changed).await;
    assert_eq!(
        rpc(&mut changed, 10, "collaboration.resume", params).await["error"]["code"],
        "resync_required"
    );
    changed.close(None).await.unwrap();
    drop(changed);
    fixture.finish().await;
}

#[tokio::test]
async fn independent_controllers_receive_scoped_changes_and_revocation_over_tls() {
    use tiktools_collaboration::{Capability, ResourceKind};
    let _env = ENV.lock().await;
    let fixture = Fixture::new(true).await;
    fixture
        .core
        .workflow_save(json!({"id":"shared","name":"Shared","enabled":true}))
        .unwrap();
    fixture
        .core
        .workflow_save(json!({"id":"private","name":"Owner private","enabled":true}))
        .unwrap();
    audit_revision(&fixture, "shared", "1").await;
    let scoped: std::collections::BTreeMap<ResourceKind, std::collections::BTreeSet<String>> =
        [(ResourceKind::Workflow, ["shared".into()].into())].into();
    let moderator = fixture
        .native(Grant {
            capabilities: [Capability::WorkflowsRead, Capability::WorkflowsToggle].into(),
            resources: scoped.clone(),
            ..Grant::default()
        })
        .await;
    let observer = fixture
        .native(Grant {
            capabilities: [Capability::WorkflowsRead].into(),
            resources: scoped,
            ..Grant::default()
        })
        .await;
    assert_ne!(moderator.principal_id, observer.principal_id);
    let mut sender = fixture.socket(&moderator.access_token).await;
    let mut viewer = fixture.socket(&observer.access_token).await;
    fixture.hello(&mut sender).await;
    fixture.hello(&mut viewer).await;
    let sub = rpc(
        &mut viewer,
        2,
        "collaboration.subscribe",
        json!({"domains":["workflows"]}),
    )
    .await;
    let subscription = sub["result"]["subscriptionId"].clone();
    assert!(subscription.is_string(), "{sub}");
    let snapshot = rpc(
        &mut viewer,
        3,
        "collaboration.snapshot",
        json!({"subscriptionId":subscription}),
    )
    .await;
    assert_eq!(snapshot["result"]["workflows"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["result"]["workflows"][0]["id"], "shared");
    let baseline = snapshot["result"]["throughSequence"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let command = rpc_meta(&mut sender, 2, "workflows.disable", json!({"id":"shared"}),
        Some(json!({"commandId":"moderator-disable-1","hostInstanceId":fixture.service.instance_id(),"expectedRevision":"1"}))).await;
    assert_eq!(command["result"], json!({"id":"shared","enabled":false}));
    let event = stream_event(&mut viewer).await;
    assert_eq!(event["resource"]["id"], "shared");
    assert_eq!(event["resource"]["revision"], "2");
    assert_eq!(event["sequence"], (baseline + 1).to_string());
    assert!(!event.to_string().contains("moderator-disable-1"));
    assert!(!event.to_string().contains("Owner private"));
    assert_eq!(
        fixture.core.workflow_get("shared").unwrap()["enabled"],
        false
    );
    assert_eq!(
        fixture.core.workflow_get("private").unwrap()["enabled"],
        true
    );
    fixture
        .service
        .revoke_session(&moderator.session_id, "local-owner")
        .unwrap();
    closed(&mut sender).await;
    assert_eq!(
        rpc(&mut viewer, 4, "workflows.get", json!({"id":"private"})).await["error"]["code"],
        "forbidden"
    );
    assert!(rpc(&mut viewer, 5, "workflows.list", json!({})).await["result"].is_object());
    viewer.close(None).await.unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn unread_controller_does_not_stop_owner_mutations_or_active_tls_controller() {
    use tiktools_collaboration::{Capability, ResourceKind};
    let _env = ENV.lock().await;
    let fixture = Fixture::new(true).await;
    fixture
        .core
        .workflow_save(json!({"id":"shared","name":"Shared","enabled":true}))
        .unwrap();
    audit_revision(&fixture, "shared", "1").await;
    let grant = Grant {
        capabilities: [Capability::WorkflowsRead].into(),
        resources: [(ResourceKind::Workflow, ["shared".into()].into())].into(),
        ..Grant::default()
    };
    let paused = fixture.native(grant.clone()).await;
    let active = fixture.native(grant).await;
    assert_ne!(paused.principal_id, active.principal_id);
    let mut stalled_socket = fixture.socket(&paused.access_token).await;
    let mut active_socket = fixture.socket(&active.access_token).await;
    fixture.hello(&mut stalled_socket).await;
    fixture.hello(&mut active_socket).await;
    for socket in [&mut stalled_socket, &mut active_socket] {
        let subscription = rpc(
            socket,
            2,
            "collaboration.subscribe",
            json!({"domains":["workflows"]}),
        )
        .await;
        let id = subscription["result"]["subscriptionId"].clone();
        assert!(id.is_string(), "{subscription}");
        let snapshot = rpc(
            socket,
            3,
            "collaboration.snapshot",
            json!({"subscriptionId":id}),
        )
        .await;
        assert_eq!(snapshot["result"]["workflows"][0]["id"], "shared");
        assert_eq!(snapshot["result"]["throughSequence"], "0");
    }

    // Deliberately stop reading the first TLS socket. The owner continues
    // committing changes while the other principal consumes every event.
    for n in 1..=24_u64 {
        fixture
            .core
            .workflow_set_enabled("shared", n % 2 == 0)
            .unwrap();
        let event = stream_event(&mut active_socket).await;
        assert_eq!(event["resource"]["id"], "shared");
        assert_eq!(event["resource"]["revision"], (n + 1).to_string());
        assert_eq!(event["sequence"], n.to_string());
        assert!(event.get("commandId").is_none());
    }
    assert_eq!(
        fixture.core.workflow_get("shared").unwrap()["enabled"],
        true
    );
    let read = rpc(
        &mut active_socket,
        4,
        "workflows.get",
        json!({"id":"shared"}),
    )
    .await;
    assert_eq!(read["result"]["id"], "shared", "{read}");
    drop(stalled_socket);
    active_socket.close(None).await.unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn small_frame_snapshot_pages_remain_immutable_and_incomplete_disconnect_requires_resync() {
    let _env = ENV.lock().await;
    let fixture = Fixture::configured(true, |c| {
        c.limits.frame_bytes = 4096;
        c.limits.params_bytes = 1024;
    })
    .await;
    for n in 0..12 {
        fixture
            .core
            .workflow_save(
                json!({"id":format!("wf-{n:02}"),"name":"Original","body":"x".repeat(800)}),
            )
            .unwrap();
    }
    let credentials = fixture
        .native(Grant {
            capabilities: [tiktools_collaboration::Capability::WorkflowsRead].into(),
            ..Grant::default()
        })
        .await;
    let mut socket = fixture.socket(&credentials.access_token).await;
    let hello = fixture.hello(&mut socket).await;
    assert_eq!(
        hello["result"]["features"]["browserApp"],
        cfg!(feature = "browser-app")
    );
    let sub = rpc(
        &mut socket,
        2,
        "collaboration.subscribe",
        json!({"domains":["workflows"]}),
    )
    .await;
    let id = sub["result"]["subscriptionId"].clone();
    let first = rpc(
        &mut socket,
        3,
        "collaboration.snapshot",
        json!({"subscriptionId":id}),
    )
    .await;
    assert!(first["result"]["nextPageToken"].is_string(), "{first}");
    fixture.core.workflow_set_enabled("wf-11", true).unwrap();
    let mut page = first["result"].clone();
    let mut count = 0;
    let mut wire_id = 4;
    loop {
        count += page["workflows"].as_array().unwrap().len();
        assert_eq!(page["snapshotId"], first["result"]["snapshotId"]);
        assert_eq!(page["throughSequence"], first["result"]["throughSequence"]);
        assert!(page["workflows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["enabled"] == false));
        let Some(token) = page["nextPageToken"].as_str() else {
            break;
        };
        page = rpc(
            &mut socket,
            wire_id,
            "collaboration.snapshot",
            json!({"subscriptionId":id,"pageToken":token}),
        )
        .await["result"]
            .clone();
        wire_id += 1;
    }
    assert_eq!(count, 12);
    loop {
        let event = stream_event(&mut socket).await;
        if event["resource"]["id"] == "wf-11" && event["resource"]["revision"] == "2" {
            break;
        }
    }
    let replacement = rpc(
        &mut socket,
        20,
        "collaboration.subscribe",
        json!({"domains":["workflows"]}),
    )
    .await;
    let id = replacement["result"]["subscriptionId"].clone();
    let page = rpc(
        &mut socket,
        21,
        "collaboration.snapshot",
        json!({"subscriptionId":id}),
    )
    .await;
    assert!(page["result"]["nextPageToken"].is_string());
    socket.close(None).await.unwrap();
    drop(socket);
    tokio::time::timeout(Duration::from_secs(3), async {
        while fixture.service.gateway_status().unwrap().connections != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let mut new = fixture.socket(&credentials.access_token).await;
    fixture.hello(&mut new).await;
    assert_eq!(rpc(&mut new,22,"collaboration.resume",json!({"subscriptionId":id,"hostInstanceId":fixture.service.instance_id(),"lastAppliedSequence":page["result"]["throughSequence"]})).await["error"]["code"],"resync_required");
    new.close(None).await.unwrap();
    drop(new);
    fixture.finish().await;
}
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

struct Fixture {
    home: tempfile::TempDir,
    core: Arc<AppCore>,
    api: Arc<ControlApi>,
    service: Arc<CollaborationService>,
    gateway: Option<tiktools_remote::RemoteGateway>,
    origin: String,
    client: Client,
    connector: Option<Connector>,
}

impl Fixture {
    async fn new(tls: bool) -> Self {
        Self::configured(tls, |_| {}).await
    }

    async fn configured(tls: bool, configure: impl FnOnce(&mut Configuration)) -> Self {
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("TIKTOOLS_HOME", home.path());
        std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.path().join("builtins"));
        std::env::set_var("TIKTOOLS_DEV_PLUGINS_DIR", home.path().join("dev-plugins"));
        std::env::set_var("TIKTOOLS_PLUGIN_DATA_DIR", home.path().join("plugin-data"));
        let root = home.path().to_path_buf();
        let paths = tiktools_core::paths::AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtins"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root,
        };
        paths.ensure_directories().unwrap();
        // A fresh home must not trigger import of the developer's ./data files.
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        let core = Arc::new(AppCore::with_paths(
            Arc::new(Emitter),
            Arc::new(tiktools_core::NoopMediaHost),
            paths,
        ));
        let api = Arc::new(ControlApi::new(Arc::clone(&core)));
        let service = core.collaboration().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("{}://{address}", if tls { "https" } else { "http" });
        let mut configuration = Configuration {
            enabled: true,
            bind_address: address.to_string(),
            advertised_origin: Some(origin.clone()),
            ..Configuration::default()
        };
        let mut builder = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5));
        let connector = if tls {
            let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
            let cert = home.path().join("test-cert.pem");
            let key = home.path().join("test-key.pem");
            std::fs::write(&cert, certified.cert.pem()).unwrap();
            std::fs::write(&key, certified.signing_key.serialize_pem()).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
            configuration.tls_certificate = Some(cert.to_string_lossy().into());
            configuration.tls_private_key = Some(key.to_string_lossy().into());
            builder = builder.add_root_certificate(
                reqwest::Certificate::from_pem(certified.cert.pem().as_bytes()).unwrap(),
            );
            let mut roots = rustls::RootCertStore::empty();
            roots.add(certified.cert.der().clone()).unwrap();
            let client = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
            Some(Connector::Rustls(Arc::new(client)))
        } else {
            None
        };
        configure(&mut configuration);
        service.configure(configuration).unwrap();
        let gateway = tiktools_remote::start_on(Arc::clone(&api), listener)
            .await
            .unwrap();
        assert_eq!(gateway.local_addr(), address);
        Self {
            home,
            core,
            api,
            service,
            gateway: Some(gateway),
            origin,
            client: builder.build().unwrap(),
            connector,
        }
    }

    async fn native(&self, grant: Grant) -> Credentials {
        let invitation = self
            .service
            .create_invitation(InviteCreate { grant })
            .unwrap();
        let exchange = "a".repeat(64);
        let pending:Value=self.client.post(format!("{}/remote/v1/pair/redeem",self.origin)).header("x-tiktools-client","native")
            .json(&json!({"inviteId":invitation.invite_id,"secret":invitation.secret,"clientLabel":"Moderator PC","clientKind":"desktop","exchangeSecretHash":tiktools_collaboration::hash_secret(&exchange)}))
            .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
        let collect = json!({"requestId":pending["requestId"],"exchangeSecret":exchange});
        let waiting = self
            .client
            .post(format!("{}/remote/v1/pair/status", self.origin))
            .header("x-tiktools-client", "native")
            .json(&collect)
            .send()
            .await
            .unwrap();
        assert_eq!(waiting.status(), StatusCode::ACCEPTED);
        let approved=self.api.execute_value(&json!({"id":1,"method":"collaboration.requests.approve","params":{"id":pending["requestId"]}})).await;
        assert!(approved.is_ok());
        self.client
            .post(format!("{}/remote/v1/pair/status", self.origin))
            .header("x-tiktools-client", "native")
            .json(&collect)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    fn ws_request(
        &self,
        headers: &[(&str, String)],
    ) -> tokio_tungstenite::tungstenite::http::Request<()> {
        let url = format!(
            "{}/remote/v1/ws",
            self.origin
                .replace("https://", "wss://")
                .replace("http://", "ws://")
        );
        let mut request = url.into_client_request().unwrap();
        for (key, value) in headers {
            request.headers_mut().insert(
                (*key)
                    .parse::<tokio_tungstenite::tungstenite::http::HeaderName>()
                    .unwrap(),
                value.parse().unwrap(),
            );
        }
        request
    }

    async fn socket(&self, token: &str) -> Socket {
        let request = self.ws_request(&[("authorization", format!("Bearer {token}"))]);
        tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            self.connector.clone(),
        )
        .await
        .unwrap()
        .0
    }

    async fn hello(&self, socket: &mut Socket) -> Value {
        rpc(
            socket,
            1,
            "collaboration.hello",
            json!({"protocolVersion":1,"clientVersion":"test","clientKind":"desktop"}),
        )
        .await
    }

    async fn finish(self) {
        if let Some(gateway) = self.gateway {
            gateway.shutdown().await.unwrap();
        }
        assert!(!self.service.gateway_status().unwrap().running);
        assert_eq!(self.service.gateway_status().unwrap().connections, 0);
        assert!(self
            .service
            .gateway_status()
            .unwrap()
            .session_connections
            .is_empty());
        assert!(self
            .service
            .gateway_status()
            .unwrap()
            .certificate_fingerprint
            .is_none());
        self.core.shutdown().await;
        drop(self.api);
        drop(self.service);
        drop(self.core);
        drop(self.client);
        drop(self.home);
    }
}

async fn rpc(socket: &mut Socket, id: u64, method: &str, params: Value) -> Value {
    rpc_meta(socket, id, method, params, None).await
}

async fn rpc_meta(
    socket: &mut Socket,
    id: u64,
    method: &str,
    params: Value,
    meta: Option<Value>,
) -> Value {
    let mut request = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
    if let Some(meta) = meta {
        request["meta"] = meta;
    }
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(message) = socket.next().await {
            match message.unwrap() {
                Message::Text(text) => {
                    let value: Value = serde_json::from_str(&text).unwrap();
                    if value["id"] == id {
                        return value;
                    }
                }
                Message::Close(_) => panic!("socket closed before response"),
                _ => {}
            }
        }
        panic!("socket ended before response")
    })
    .await
    .unwrap()
}

async fn closed(socket: &mut Socket) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(message) = socket.next().await {
            if matches!(message, Ok(Message::Close(_)) | Err(_)) {
                break;
            }
        }
    })
    .await
    .expect("revoked socket must close promptly");
}

#[tokio::test]
async fn tls_deactivation_receipts_replay_and_recover_under_small_frame_limits() {
    use tiktools_collaboration::{Capability, ResourceKind};
    let _env = ENV.lock().await;
    let fixture = Fixture::configured(true, |c| {
        c.limits.frame_bytes = 4096;
        c.limits.params_bytes = 1024;
    })
    .await;
    fixture.core.workflow_save(json!({"id":"welcome","name":"Large owner definition","enabled":true,"body":"private-definition".repeat(4096)})).unwrap();
    fixture
        .core
        .workflow_save(json!({"id":"private","name":"Hidden","enabled":true}))
        .unwrap();
    let credentials = fixture
        .native(Grant {
            capabilities: [
                Capability::WorkflowsRead,
                Capability::WorkflowsToggle,
                Capability::WorkflowsEdit,
                Capability::CommandsRead,
            ]
            .into(),
            resources: [(ResourceKind::Workflow, ["welcome".into()].into())].into(),
            ..Grant::default()
        })
        .await;
    let mut commands = fixture.socket(&credentials.access_token).await;
    let hello = fixture.hello(&mut commands).await;
    assert_eq!(hello["result"]["features"]["mutations"], true);
    assert_eq!(hello["result"]["features"]["workflowDisable"], true);
    assert_eq!(hello["result"]["features"]["workflowSave"], false);
    let mut events = fixture.socket(&credentials.access_token).await;
    fixture.hello(&mut events).await;
    let subscribe = rpc(
        &mut events,
        2,
        "collaboration.subscribe",
        json!({"domains":["commands"]}),
    )
    .await;
    let id = &subscribe["result"]["subscriptionId"];
    let baseline = rpc(
        &mut events,
        3,
        "collaboration.snapshot",
        json!({"subscriptionId":id}),
    )
    .await;
    assert!(baseline["result"]["workflows"]
        .as_array()
        .unwrap()
        .is_empty());
    let meta = json!({"commandId":"disable-over-tls","hostInstanceId":fixture.service.instance_id(),"expectedRevision":"1"});
    assert_eq!(
        rpc_meta(
            &mut commands,
            2,
            "workflows.disable",
            json!({"id":"private"}),
            Some(meta.clone())
        )
        .await["error"]["code"],
        "forbidden"
    );
    let first = rpc_meta(
        &mut commands,
        3,
        "workflows.disable",
        json!({"id":"welcome"}),
        Some(meta.clone()),
    )
    .await;
    assert_eq!(first["result"], json!({"id":"welcome","enabled":false}));
    assert_eq!(first["meta"]["resourceVersions"][0]["revision"], "2");
    assert!(serde_json::to_vec(&first).unwrap().len() < 4096);
    let event = stream_event(&mut events).await;
    assert_eq!(event["topic"], "command.changed");
    assert_eq!(event["commandId"], "disable-over-tls");
    assert_eq!(event["resource"]["revision"], "2");
    assert!(!event.to_string().contains("private-definition"));
    fixture.core.workflow_set_enabled("welcome", true).unwrap();
    commands.close(None).await.unwrap();
    drop(commands);
    let mut retry = fixture.socket(&credentials.access_token).await;
    fixture.hello(&mut retry).await;
    let replay = rpc_meta(
        &mut retry,
        2,
        "workflows.disable",
        json!({"id":"welcome"}),
        Some(meta),
    )
    .await;
    assert_eq!(replay["result"], first["result"]);
    assert_eq!(replay["meta"], first["meta"]);
    assert_eq!(
        fixture.core.workflow_get("welcome").unwrap()["enabled"],
        true
    );
    let result = rpc(
        &mut retry,
        3,
        "collaboration.commands.get",
        json!({"commandId":"disable-over-tls"}),
    )
    .await;
    assert_eq!(result["result"]["result"], first["result"]);
    assert_eq!(result["result"]["principalId"], credentials.principal_id);
    let deletion = rpc_meta(&mut retry,4,"workflows.delete",json!({"id":"welcome"}),Some(json!({"commandId":"delete-over-tls","hostInstanceId":fixture.service.instance_id(),"expectedRevision":"3"}))).await;
    assert_eq!(deletion["result"], json!({"id":"welcome","deleted":true}));
    assert_eq!(
        stream_event(&mut events).await["commandId"],
        "delete-over-tls"
    );
    audit_revision(&fixture, "welcome", "4").await;
    let audit = fixture
        .service
        .audit(tiktools_collaboration::AuditQuery {
            after: 0,
            limit: Some(100),
        })
        .unwrap();
    let imported: Vec<_> = audit
        .iter()
        .filter_map(|entry| entry.details.as_ref())
        .filter(|detail| detail.command_id.as_deref() == Some("disable-over-tls"))
        .collect();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].principal_id, credentials.principal_id);
    assert_eq!(imported[0].session_id, credentials.session_id);
    assert_eq!(
        fixture.core.workflow_get("private").unwrap()["enabled"],
        true
    );
    events.close(None).await.unwrap();
    retry.close(None).await.unwrap();
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_tls_protocol_permissions_rotation_and_revocation_share_one_runtime() {
    let _env = ENV.lock().await;
    let fixture = Fixture::new(true).await;
    assert!(Client::new()
        .get(format!("{}/remote/v1/info", fixture.origin))
        .send()
        .await
        .is_err());
    let info = fixture
        .client
        .get(format!("{}/remote/v1/info", fixture.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(info.headers()["cache-control"], "no-store");
    let info: Value = info.json().await.unwrap();
    assert_eq!(info["certificateFingerprint"].as_str().unwrap().len(), 64);
    assert_eq!(
        serde_json::to_value(fixture.service.gateway_status().unwrap()).unwrap()
            ["certificateFingerprint"],
        info["certificateFingerprint"]
    );
    assert!(
        serde_json::to_value(fixture.service.gateway_status().unwrap())
            .unwrap()
            .get("generation")
            .is_none()
    );
    let a = fixture.native(Grant::viewer()).await;
    let b = fixture.native(Grant::viewer()).await;
    let mut incompatible = fixture.socket(&a.access_token).await;
    assert_eq!(
        rpc(
            &mut incompatible,
            1,
            "collaboration.hello",
            json!({"protocolVersion":2,"clientVersion":"test","clientKind":"desktop"})
        )
        .await["error"]["code"],
        "protocol_incompatible"
    );
    closed(&mut incompatible).await;
    drop(incompatible);
    let mut unnegotiated = fixture.socket(&a.access_token).await;
    assert_eq!(
        rpc(&mut unnegotiated, 1, "live.status", json!({})).await["error"]["code"],
        "protocol_incompatible"
    );
    closed(&mut unnegotiated).await;
    let mut socket_a = fixture.socket(&a.access_token).await;
    let hello = fixture.hello(&mut socket_a).await;
    assert_eq!(hello["result"]["hostId"], fixture.service.host_id());
    assert_eq!(hello["result"]["principalId"], a.principal_id);
    assert!(hello["result"]["connectionId"].as_str().is_some());
    assert_eq!(
        rpc(&mut socket_a, 2, "live.status", json!({})).await["result"]["connected"],
        false
    );
    assert_eq!(
        rpc(&mut socket_a, 3, "system.shutdown", json!({"role":"owner"})).await["error"]["code"],
        "forbidden"
    );
    assert!(!fixture.core.is_shutdown());
    let methods = rpc(&mut socket_a, 4, "rpc.discover", json!({})).await;
    assert!(!methods["result"]["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "collaboration.configure"));
    fixture
        .service
        .set_grant(GrantSet {
            principal_id: a.principal_id.clone(),
            expected_revision: "1".into(),
            grant: Grant::default(),
        })
        .unwrap();
    assert_eq!(
        rpc(&mut socket_a, 5, "live.status", json!({})).await["error"]["code"],
        "forbidden"
    );
    let mut socket_b = fixture.socket(&b.access_token).await;
    fixture.hello(&mut socket_b).await;
    assert_eq!(
        fixture
            .service
            .gateway_status()
            .unwrap()
            .session_connections
            .get(&a.session_id),
        Some(&1)
    );
    assert_eq!(
        fixture
            .service
            .gateway_status()
            .unwrap()
            .session_connections
            .get(&b.session_id),
        Some(&1)
    );
    fixture
        .service
        .revoke_session(&a.session_id, "local-owner")
        .unwrap();
    closed(&mut socket_a).await;
    assert_eq!(
        rpc(&mut socket_b, 2, "live.status", json!({})).await["result"]["connected"],
        false
    );
    let refreshed: Credentials = fixture
        .client
        .post(format!("{}/remote/v1/session/refresh", fixture.origin))
        .header("x-tiktools-client", "native")
        .json(&json!({"refreshToken":b.refresh_token}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    closed(&mut socket_b).await;
    let mut renewed = fixture.socket(&refreshed.access_token).await;
    fixture.hello(&mut renewed).await;
    assert_eq!(
        rpc(&mut renewed, 2, "collaboration.me", json!({})).await["result"]["sessionId"],
        b.session_id
    );
    let reused = fixture
        .client
        .post(format!("{}/remote/v1/session/refresh", fixture.origin))
        .header("x-tiktools-client", "native")
        .json(&json!({"refreshToken":b.refresh_token}))
        .send()
        .await
        .unwrap();
    assert_eq!(reused.status(), StatusCode::UNAUTHORIZED);
    closed(&mut renewed).await;
    drop(socket_a);
    drop(socket_b);
    drop(unnegotiated);
    drop(renewed);
    fixture.finish().await;
}

#[tokio::test]
async fn tls_points_lost_ack_recovers_original_outcome_without_reapplying_or_exposing_hidden_viewers(
) {
    use tiktools_collaboration::{Capability, ResourceKind};
    let _env = ENV.lock().await;
    let fixture = Fixture::configured(true, |c| {
        c.limits.frame_bytes = 4096;
        c.limits.params_bytes = 1024;
    })
    .await;
    let grant = Grant {
        capabilities: [
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(ResourceKind::Viewer, ["alice".into()].into())].into(),
        max_points_delta: Some(5.0),
        ..Grant::default()
    };
    let credentials = fixture.native(grant).await;
    let reader = fixture
        .native(Grant {
            capabilities: [Capability::PointsRead].into(),
            resources: [(ResourceKind::Viewer, ["alice".into()].into())].into(),
            ..Grant::default()
        })
        .await;
    let mut first = fixture.socket(&credentials.access_token).await;
    let hello = fixture.hello(&mut first).await;
    assert_eq!(
        hello["result"]["subscriptionDomains"],
        json!([
            "workflows",
            "points",
            "automation",
            "competitions",
            "commands"
        ])
    );
    let meta =
        json!({"commandId":"points-over-tls","hostInstanceId":fixture.service.instance_id()});
    // Send once, wait for independently committed audit evidence, and discard
    // the response without reading it. Reconnection must query this exact ID.
    first.send(Message::Text(json!({"jsonrpc":"2.0","id":2,"method":"points.adjust","params":{"uniqueId":"alice","delta":4},"meta":meta}).to_string().into())).await.unwrap();
    audit_revision(&fixture, "alice", "1").await;
    first.close(None).await.unwrap();
    drop(first);
    let mut recovered = fixture.socket(&credentials.access_token).await;
    fixture.hello(&mut recovered).await;
    let outcome = rpc(
        &mut recovered,
        2,
        "collaboration.commands.get",
        json!({"commandId":"points-over-tls"}),
    )
    .await;
    assert_eq!(outcome["result"]["principalId"], credentials.principal_id);
    assert_eq!(outcome["result"]["result"]["totalPoints"], 4.0);
    assert_eq!(outcome["result"]["resourceVersions"][0]["kind"], "viewer");
    let replay = rpc_meta(
        &mut recovered,
        3,
        "points.adjust",
        json!({"uniqueId":"alice","delta":4}),
        Some(meta.clone()),
    )
    .await;
    assert_eq!(replay["result"]["totalPoints"], 4.0);
    assert_eq!(replay["meta"]["resourceVersions"][0]["revision"], "1");
    assert_eq!(fixture.core.points_viewer("alice").unwrap()["points"], 4.0);
    fixture.core.points_adjust("alice", 2.0).unwrap();
    fixture.core.points_adjust("bob", 100.0).unwrap();
    audit_revision(&fixture, "alice", "2").await;
    let board = rpc(&mut recovered, 4, "points.leaderboard", json!({"limit":1})).await;
    assert_eq!(board["result"]["viewers"].as_array().unwrap().len(), 1);
    assert_eq!(board["result"]["viewers"][0]["uniqueId"], "alice");
    assert_eq!(board["result"]["viewers"][0]["points"], 6.0);
    assert_eq!(board["meta"]["resourceVersions"][0]["revision"], "2");
    assert_eq!(
        rpc(
            &mut recovered,
            5,
            "points.viewer.get",
            json!({"uniqueId":"bob"})
        )
        .await["error"]["code"],
        "forbidden"
    );
    let mut readonly = fixture.socket(&reader.access_token).await;
    fixture.hello(&mut readonly).await;
    assert_eq!(rpc_meta(&mut readonly,2,"points.adjust",json!({"uniqueId":"alice","delta":1}),Some(json!({"commandId":"read-cannot-write","hostInstanceId":fixture.service.instance_id()}))).await["error"]["code"],"forbidden");
    assert_eq!(
        rpc(
            &mut readonly,
            3,
            "points.viewer.get",
            json!({"uniqueId":"alice"})
        )
        .await["result"]["viewer"]["points"],
        6.0
    );
    let again = rpc(
        &mut recovered,
        6,
        "collaboration.commands.get",
        json!({"commandId":"points-over-tls"}),
    )
    .await;
    assert_eq!(again["result"]["result"]["totalPoints"], 4.0);
    assert_eq!(
        fixture
            .service
            .audit(tiktools_collaboration::AuditQuery {
                after: 0,
                limit: Some(100)
            })
            .unwrap()
            .iter()
            .filter_map(|e| e.details.as_ref())
            .filter(|a| a.command_id.as_deref() == Some("points-over-tls"))
            .count(),
        1
    );
    readonly.close(None).await.unwrap();
    recovered.close(None).await.unwrap();
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_credentials_require_origin_csrf_and_single_use_socket_ticket() {
    let _env = ENV.lock().await;
    let fixture = Fixture::new(true).await;
    let invitation = fixture
        .service
        .create_invitation(InviteCreate {
            grant: Grant::viewer(),
        })
        .unwrap();
    let exchange = "b".repeat(64);
    let pending:Value=fixture.client.post(format!("{}/remote/v1/pair/redeem",fixture.origin)).header("origin",&fixture.origin)
        .json(&json!({"inviteId":invitation.invite_id,"secret":invitation.secret,"clientLabel":"Browser","clientKind":"browser","exchangeSecretHash":tiktools_collaboration::hash_secret(&exchange)})).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    fixture
        .service
        .decide_request(pending["requestId"].as_str().unwrap(), true)
        .unwrap();
    let payload = json!({"requestId":pending["requestId"],"exchangeSecret":exchange});
    let wrong_mode = fixture
        .client
        .post(format!("{}/remote/v1/pair/status", fixture.origin))
        .header("x-tiktools-client", "native")
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(wrong_mode.status(), StatusCode::UNAUTHORIZED);
    let approved = fixture
        .client
        .post(format!("{}/remote/v1/pair/status", fixture.origin))
        .header("origin", &fixture.origin)
        .json(&payload)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let cookie = approved.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        cookie.contains("HttpOnly")
            && cookie.contains("Secure")
            && cookie.contains("SameSite=Strict")
    );
    let approved: Value = approved.json().await.unwrap();
    assert!(approved.get("accessToken").is_none());
    assert!(approved.get("refreshToken").is_none());
    let cookie = cookie.split(';').next().unwrap().to_owned();
    let info: Value = fixture
        .client
        .get(format!("{}/remote/v1/info", fixture.origin))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(info["browserApp"], cfg!(feature = "browser-app"));
    let anonymous = fixture
        .client
        .get(format!("{}/remote/v1/app", fixture.origin))
        .send()
        .await
        .unwrap();
    #[cfg(feature = "browser-app")]
    {
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
        let app = fixture
            .client
            .get(format!("{}/remote/v1/app", fixture.origin))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        assert!(app.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html"));
        let html = app.text().await.unwrap();
        assert!(html.contains("name=\"tiktools-transport\" content=\"remote-web\""));
        let script = format!(
            "/assets/{}",
            html.split("src=\"/assets/")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
        );
        let unauthorized_asset = fixture
            .client
            .get(format!("{}{}", fixture.origin, script))
            .send()
            .await
            .unwrap();
        assert_eq!(unauthorized_asset.status(), StatusCode::UNAUTHORIZED);
        let asset = fixture
            .client
            .get(format!("{}{}", fixture.origin, script))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        assert!(asset.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/javascript"));
        assert!(!asset.bytes().await.unwrap().is_empty());
        let source_map = fixture
            .client
            .get(format!("{}{}.map", fixture.origin, script))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(source_map.status(), StatusCode::NOT_FOUND);
        let native = fixture.native(Grant::viewer()).await;
        let native_app = fixture
            .client
            .get(format!("{}/remote/v1/app", fixture.origin))
            .bearer_auth(native.access_token)
            .send()
            .await
            .unwrap();
        assert_eq!(native_app.status(), StatusCode::FORBIDDEN);
    }
    #[cfg(not(feature = "browser-app"))]
    assert_eq!(anonymous.status(), StatusCode::NOT_FOUND);
    let bootstrap = fixture
        .client
        .get(format!("{}/remote/v1/session", fixture.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let bootstrap: Value = bootstrap.json().await.unwrap();
    let csrf = bootstrap["csrfToken"].as_str().unwrap();
    let forbidden = fixture
        .client
        .post(format!("{}/remote/v1/ws-ticket", fixture.origin))
        .header("cookie", &cookie)
        .header("origin", &fixture.origin)
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let wrong_origin = fixture
        .client
        .get(format!("{}/remote/v1/session", fixture.origin))
        .header("cookie", &cookie)
        .header("origin", "https://untrusted.invalid")
        .send()
        .await
        .unwrap();
    assert_eq!(wrong_origin.status(), StatusCode::FORBIDDEN);
    let ticket = fixture
        .client
        .post(format!("{}/remote/v1/ws-ticket", fixture.origin))
        .header("cookie", &cookie)
        .header("origin", &fixture.origin)
        .header("x-csrf-token", csrf)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let ticket = ticket.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let cookies = format!("{cookie}; {ticket}");
    let request = fixture.ws_request(&[
        ("origin", fixture.origin.clone()),
        ("cookie", cookies.clone()),
    ]);
    let (mut socket, _) = tokio_tungstenite::connect_async_tls_with_config(
        request,
        None,
        false,
        fixture.connector.clone(),
    )
    .await
    .unwrap();
    let hello = fixture.hello(&mut socket).await;
    assert_eq!(
        hello["result"]["features"]["browserApp"],
        cfg!(feature = "browser-app")
    );
    let reused = fixture.ws_request(&[("origin", fixture.origin.clone()), ("cookie", cookies)]);
    assert!(tokio_tungstenite::connect_async_tls_with_config(
        reused,
        None,
        false,
        fixture.connector.clone()
    )
    .await
    .is_err());
    let logout = fixture
        .client
        .post(format!("{}/remote/v1/session/logout", fixture.origin))
        .header("cookie", &cookie)
        .header("origin", &fixture.origin)
        .header("x-csrf-token", csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::OK);
    #[cfg(feature = "browser-app")]
    {
        let revoked = fixture
            .client
            .get(format!("{}/remote/v1/app", fixture.origin))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
    }
    closed(&mut socket).await;
    drop(socket);
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn boundary_rejects_wrong_host_cookie_bearer_mixing_and_excess_connections() {
    let _env = ENV.lock().await;
    let fixture = Fixture::new(false).await;
    assert_eq!(
        fixture
            .client
            .get(format!("{}/remote/v1/info", fixture.origin))
            .header("host", "untrusted.invalid")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        fixture
            .client
            .get(format!(
                "{}/remote/v1/info?token=not-allowed",
                fixture.origin
            ))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let a = fixture.native(Grant::viewer()).await;
    let b = fixture.native(Grant::viewer()).await;
    let c = fixture.native(Grant::viewer()).await;
    let mixed = fixture
        .client
        .get(format!("{}/remote/v1/session", fixture.origin))
        .bearer_auth(&a.access_token)
        .header("cookie", "unused=1")
        .send()
        .await
        .unwrap();
    assert_eq!(mixed.status(), StatusCode::FORBIDDEN);
    let mut sockets = Vec::new();
    for _ in 0..4 {
        let mut socket = fixture.socket(&a.access_token).await;
        fixture.hello(&mut socket).await;
        sockets.push(socket);
    }
    let excess = fixture.ws_request(&[("authorization", format!("Bearer {}", a.access_token))]);
    assert!(
        matches!(tokio_tungstenite::connect_async(excess).await,Err(tokio_tungstenite::tungstenite::Error::Http(response)) if response.status().as_u16()==429)
    );
    for _ in 0..4 {
        let mut socket = fixture.socket(&b.access_token).await;
        fixture.hello(&mut socket).await;
        sockets.push(socket);
    }
    let excess = fixture.ws_request(&[("authorization", format!("Bearer {}", c.access_token))]);
    assert!(
        matches!(tokio_tungstenite::connect_async(excess).await,Err(tokio_tungstenite::tungstenite::Error::Http(response)) if response.status().as_u16()==429)
    );
    assert_eq!(fixture.service.gateway_status().unwrap().connections, 8);
    for socket in &mut sockets {
        let _ = socket.close(None).await;
    }
    drop(sockets);
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn configured_lower_limits_are_advertised_and_enforced_on_the_wire() {
    let _env = ENV.lock().await;
    let fixture = Fixture::configured(false, |config| {
        config.limits.connections = 2;
        config.limits.connections_per_principal = 1;
        config.limits.frame_bytes = 8192;
        config.limits.params_bytes = 1024;
        config.limits.pending_calls = 2;
    })
    .await;
    let a = fixture.native(Grant::viewer()).await;
    let b = fixture.native(Grant::viewer()).await;
    let mut socket_a = fixture.socket(&a.access_token).await;
    let hello = fixture.hello(&mut socket_a).await;
    assert_eq!(hello["result"]["limits"]["connections"], 2);
    assert_eq!(hello["result"]["limits"]["paramsBytes"], 1024);
    let excess = fixture.ws_request(&[("authorization", format!("Bearer {}", a.access_token))]);
    assert!(
        matches!(tokio_tungstenite::connect_async(excess).await,Err(tokio_tungstenite::tungstenite::Error::Http(response)) if response.status().as_u16()==429)
    );
    let mut socket_b = fixture.socket(&b.access_token).await;
    fixture.hello(&mut socket_b).await;
    assert_eq!(
        rpc(
            &mut socket_a,
            2,
            "live.status",
            json!({"extra":"x".repeat(1200)})
        )
        .await["error"]["code"],
        "too_large"
    );
    assert_eq!(
        rpc(&mut socket_a, 3, "live.status", json!({})).await["result"]["connected"],
        false
    );
    socket_a
        .send(Message::Text("x".repeat(8193).into()))
        .await
        .unwrap();
    closed(&mut socket_a).await;
    assert_eq!(
        rpc(&mut socket_b, 2, "live.status", json!({})).await["result"]["connected"],
        false
    );
    drop(socket_a);
    drop(socket_b);
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_does_not_create_a_database_for_local_only_startup() {
    let _env = ENV.lock().await;
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("TIKTOOLS_HOME", home.path());
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.path().join("builtins"));
    std::env::set_var("TIKTOOLS_DEV_PLUGINS_DIR", home.path().join("dev-plugins"));
    std::env::set_var("TIKTOOLS_PLUGIN_DATA_DIR", home.path().join("plugin-data"));
    let core = Arc::new(AppCore::new(Arc::new(Emitter)));
    let api = Arc::new(ControlApi::new(Arc::clone(&core)));
    let monitor = tokio::spawn(tiktools_remote::supervise(Arc::clone(&api)));
    tokio::time::sleep(Duration::from_millis(650)).await;
    assert!(!home
        .path()
        .join("data/collaboration/collaboration.db")
        .exists());
    let status = api
        .execute_value(&json!({"id":1,"method":"collaboration.status","params":{}}))
        .await
        .result
        .unwrap();
    assert_eq!(status["gateway"]["available"], true);
    assert_eq!(status["gateway"]["running"], false);
    core.shutdown().await;
    tokio::time::timeout(Duration::from_secs(2), monitor)
        .await
        .unwrap()
        .unwrap();
    drop(api);
    drop(core);
    drop(home);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_resource_restrictions_do_not_expose_another_creator_or_unscoped_history() {
    let _env = ENV.lock().await;
    let fixture = Fixture::new(false).await;
    let mut grant = Grant::viewer();
    grant.resources.insert(
        tiktools_collaboration::ResourceKind::Live,
        ["allowed-creator".into()].into(),
    );
    let credentials = fixture.native(grant).await;
    let mut socket = fixture.socket(&credentials.access_token).await;
    let hello = fixture.hello(&mut socket).await;
    assert_eq!(
        rpc(&mut socket, 5, "ui.feed.browser", json!({"after":"0"})).await["error"]["code"],
        "forbidden"
    );
    assert_eq!(
        hello["result"]["resourceRestrictions"]["live"][0],
        "allowed-creator"
    );
    assert_eq!(
        rpc(&mut socket, 2, "live.status", json!({})).await["error"]["code"],
        "forbidden"
    );
    assert_eq!(
        rpc(
            &mut socket,
            3,
            "ui.feed.backfill",
            json!({"version":1,"after":0})
        )
        .await["error"]["code"],
        "forbidden"
    );
    let discovered = rpc(&mut socket, 4, "rpc.discover", json!({})).await;
    assert!(!discovered["result"]["methods"]
        .as_array()
        .unwrap()
        .iter()
        .any(|method| method["name"] == "ui.feed.backfill" || method["name"] == "ui.feed.browser"));
    drop(socket);
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_restores_configuration_restarts_limits_and_disables_only_remote_access() {
    let _env = ENV.lock().await;
    let mut fixture = Fixture::new(true).await;
    let original = fixture.native(Grant::viewer()).await;
    fixture.gateway.take().unwrap().shutdown().await.unwrap();
    let monitor = tokio::spawn(tiktools_remote::supervise(Arc::clone(&fixture.api)));
    async fn running(fixture: &Fixture, expected: bool) {
        let desired = fixture.service.configuration_snapshot().unwrap().1;
        tokio::time::timeout(Duration::from_secs(6), async {
            loop {
                let status = fixture.service.gateway_status().unwrap();
                // A closing listener can still report the old running state;
                // readiness belongs to the requested configuration generation.
                if status.running == expected
                    && (!expected
                        || status.configuration_revision.as_deref() == Some(desired.as_str()))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
    running(&fixture, true).await;
    let mut socket = fixture.socket(&original.access_token).await;
    fixture.hello(&mut socket).await;
    assert_eq!(
        rpc(&mut socket, 2, "live.status", json!({})).await["result"]["connected"],
        false
    );
    let mut configuration = fixture.service.configuration().unwrap();
    configuration.limits.connections = 1;
    configuration.limits.connections_per_principal = 1;
    assert!(fixture
        .api
        .execute_value(&json!({"id":3,"method":"collaboration.configure","params":configuration}))
        .await
        .is_ok());
    closed(&mut socket).await;
    drop(socket);
    running(&fixture, true).await;
    let mut updated = fixture.socket(&original.access_token).await;
    assert_eq!(
        fixture.hello(&mut updated).await["result"]["limits"]["connections"],
        1
    );
    configuration.enabled = false;
    assert!(fixture
        .api
        .execute_value(&json!({"id":4,"method":"collaboration.configure","params":configuration}))
        .await
        .is_ok());
    closed(&mut updated).await;
    drop(updated);
    running(&fixture, false).await;
    assert!(!fixture.core.is_shutdown());
    assert!(fixture
        .client
        .get(format!("{}/remote/v1/info", fixture.origin))
        .send()
        .await
        .is_err());
    configuration.enabled = true;
    assert!(fixture
        .api
        .execute_value(&json!({"id":5,"method":"collaboration.configure","params":configuration}))
        .await
        .is_ok());
    running(&fixture, true).await;
    assert_eq!(
        fixture
            .client
            .get(format!("{}/remote/v1/session", fixture.origin))
            .bearer_auth(&original.access_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let credentials = fixture.native(Grant::viewer()).await;
    let mut fresh = fixture.socket(&credentials.access_token).await;
    assert_eq!(
        fixture.hello(&mut fresh).await["result"]["hostId"],
        fixture.service.host_id()
    );
    assert!(!fixture.core.is_shutdown());
    fixture.core.shutdown().await;
    tokio::time::timeout(Duration::from_secs(7), monitor)
        .await
        .unwrap()
        .unwrap();
    closed(&mut fresh).await;
    drop(fresh);
    fixture.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reapplying_configuration_reloads_rotated_tls_files_at_the_same_paths() {
    let _env = ENV.lock().await;
    let mut fixture = Fixture::new(true).await;
    let credentials = fixture.native(Grant::viewer()).await;
    fixture.gateway.take().unwrap().shutdown().await.unwrap();
    let monitor = tokio::spawn(tiktools_remote::supervise(Arc::clone(&fixture.api)));
    async fn applied(fixture: &Fixture, revision: &str) {
        tokio::time::timeout(Duration::from_secs(6), async {
            loop {
                let status = fixture.service.gateway_status().unwrap();
                if status.running && status.configuration_revision.as_deref() == Some(revision) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
    let (configuration, previous_revision) = fixture.service.configuration_snapshot().unwrap();
    applied(&fixture, &previous_revision).await;
    let mut socket = fixture.socket(&credentials.access_token).await;
    fixture.hello(&mut socket).await;
    let next = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    std::fs::write(
        configuration.tls_certificate.as_ref().unwrap(),
        next.cert.pem(),
    )
    .unwrap();
    std::fs::write(
        configuration.tls_private_key.as_ref().unwrap(),
        next.signing_key.serialize_pem(),
    )
    .unwrap();
    let result = fixture
        .api
        .execute_value(&json!({"id":2,"method":"collaboration.configure","params":configuration}))
        .await
        .result
        .unwrap();
    let revision = result["configurationRevision"].as_str().unwrap();
    assert_ne!(previous_revision, revision);
    closed(&mut socket).await;
    drop(socket);
    applied(&fixture, revision).await;
    // The old trusted certificate cannot silently trust its replacement.
    assert!(fixture
        .client
        .get(format!("{}/remote/v1/info", fixture.origin))
        .send()
        .await
        .is_err());
    fixture.client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .add_root_certificate(reqwest::Certificate::from_pem(next.cert.pem().as_bytes()).unwrap())
        .build()
        .unwrap();
    let info: Value = fixture
        .client
        .get(format!("{}/remote/v1/info", fixture.origin))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let fingerprint: String = ring::digest::digest(&ring::digest::SHA256, next.cert.der().as_ref())
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(info["certificateFingerprint"], fingerprint);
    assert_eq!(
        fixture
            .service
            .gateway_status()
            .unwrap()
            .certificate_fingerprint
            .as_deref(),
        Some(fingerprint.as_str())
    );
    assert_eq!(info["hostId"], fixture.service.host_id());
    // TLS reload closes connections but does not revoke approved devices.
    assert_eq!(
        fixture
            .client
            .get(format!("{}/remote/v1/session", fixture.origin))
            .bearer_auth(&credentials.access_token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    fixture.core.shutdown().await;
    tokio::time::timeout(Duration::from_secs(7), monitor)
        .await
        .unwrap()
        .unwrap();
    fixture.finish().await;
}

#[cfg(feature = "browser-app")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires built dist/web, Bun and Playwright Chromium"]
async fn browser_controller_pairs_reads_live_updates_and_clears_on_revocation() {
    use std::{
        io::Write,
        path::Path,
        process::{Command, Stdio},
    };
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    async fn stage(child: &mut Child, dir: &Path, name: &str) {
        tokio::time::timeout(Duration::from_secs(25), async {
            while !dir.join(name).exists() {
                if dir.join("failure").exists() {
                    panic!(
                        "browser fixture failed: {}",
                        std::fs::read_to_string(dir.join("failure")).unwrap_or_default()
                    );
                }
                if let Some(status) = child.0.try_wait().unwrap() {
                    panic!("browser fixture exited: {status}");
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("browser fixture timed out");
    }
    let _env = ENV.lock().await;
    let fixture = Fixture::configured(true, |config| {
        config.limits.frame_bytes = 4096;
        config.limits.params_bytes = 1024;
    })
    .await;
    fixture.core.events.publish_live_event_from_creator(json!({"kind":"chat","author":"viewer1","text":format!("Long message {}", "🔥".repeat(2048))}), Some("creator"));
    fixture.core.events.publish_live_event_from_creator(
        json!({"kind":"chat","author":"viewer1","text":"Browser feed message"}),
        Some("creator"),
    );
    fixture
        .core
        .workflow_save(json!({"id":"welcome","name":"Host initial","enabled":true}))
        .unwrap();
    fixture
        .core
        .workflow_save(json!({"id":"other","name":"Other workflow","enabled":true}))
        .unwrap();
    fixture
        .core
        .workflow_save(json!({"id":"ui","name":"UI workflow","enabled":true}))
        .unwrap();
    let grant = Grant {
        capabilities: [
            tiktools_collaboration::Capability::WorkflowsRead,
            tiktools_collaboration::Capability::WorkflowsToggle,
            tiktools_collaboration::Capability::WorkflowsEdit,
            tiktools_collaboration::Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            tiktools_collaboration::ResourceKind::Workflow,
            ["welcome".into(), "other".into(), "ui".into()].into(),
        )]
        .into(),
        ..Grant::default()
    };
    let invitation = fixture
        .service
        .create_invitation(InviteCreate { grant })
        .unwrap();
    let replacement = fixture
        .service
        .create_invitation(InviteCreate {
            grant: Grant {
                capabilities: [tiktools_collaboration::Capability::LiveRead].into(),
                ..Grant::default()
            },
        })
        .unwrap();
    let signals = tempfile::tempdir().unwrap();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let mut child = Child(
        Command::new("bun")
            .arg(root.join("tests/helpers/remote-browser-controller-driver.ts"))
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let payload = json!({"origin":fixture.origin,"inviteId":invitation.invite_id,"secret":invitation.secret,"replacementId":replacement.invite_id,"replacementSecret":replacement.secret,"signals":signals.path()});
    let mut input = child.0.stdin.take().unwrap();
    serde_json::to_writer(&mut input, &payload).unwrap();
    input.write_all(b"\n").unwrap();
    drop(input);
    stage(&mut child, signals.path(), "requested").await;
    let request_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let requests = fixture.service.requests(Default::default()).unwrap();
            if let Some(request) = requests
                .items
                .into_iter()
                .find(|request| request.client_label == "Browser moderator")
            {
                break request.request_id;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    fixture.service.decide_request(&request_id, true).unwrap();
    stage(&mut child, signals.path(), "ready").await;
    fixture
        .core
        .workflow_save(json!({"id":"welcome","name":"Host updated","enabled":true}))
        .unwrap();
    stage(&mut child, signals.path(), "updated").await;
    stage(&mut child, signals.path(), "commands-complete").await;
    assert_eq!(
        fixture.core.workflow_get("welcome").unwrap()["enabled"],
        false
    );
    assert!(fixture.core.workflow_get("other").is_err());
    assert_eq!(fixture.core.workflow_get("ui").unwrap()["enabled"], false);
    stage(&mut child, signals.path(), "replacement-requested").await;
    let replacement_request_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let requests = fixture.service.requests(Default::default()).unwrap();
            if let Some(request) = requests
                .items
                .into_iter()
                .find(|request| request.client_label == "Browser replacement")
            {
                break request.request_id;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    fixture
        .service
        .decide_request(&replacement_request_id, true)
        .unwrap();
    stage(&mut child, signals.path(), "cookie-cleared").await;
    stage(&mut child, signals.path(), "replacement-ready").await;
    let session = fixture
        .service
        .sessions(Default::default())
        .unwrap()
        .items
        .into_iter()
        .find(|session| {
            session.client_kind == tiktools_collaboration::ClientKind::Browser
                && session.client_label == "Browser replacement"
                && !session.revoked
        })
        .unwrap();
    fixture
        .service
        .revoke_session(&session.session_id, "test-owner")
        .unwrap();
    stage(&mut child, signals.path(), "cleared").await;
    let status = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(status.success());
    fixture.finish().await;
}
