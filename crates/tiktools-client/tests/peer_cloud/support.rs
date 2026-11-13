use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc, time::Duration};
use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore};
use tiktools_core::{ipc::messages::HostMessage, paths::AppPaths, AppCore, HostEmitter};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fixture {
    pub origin: String,
    pub cloud_control_path: std::path::PathBuf,
    pub browser_origin: String,
    pub owner_cookie: String,
    pub moderator_cookie: String,
}
impl Fixture {
    pub fn load() -> Self {
        let path = std::env::var_os("TIKTOOLS_PEER_ACCEPTANCE_CONFIG")
            .expect("run scripts/collaboration-cloud-lab.py to provide an isolated backend");
        let bytes = std::fs::read(path).expect("private fixture configuration must be readable");
        let fixture: Self = serde_json::from_slice(&bytes).expect("valid fixture configuration");
        let url = url::Url::parse(&fixture.origin).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        fixture
    }
    pub async fn set_cloud_online(&self, online: bool) {
        self.control_mode(if online { "online" } else { "offline" })
            .await;
    }
    async fn control_mode(&self, mode: &str) {
        assert!(self.cloud_control_path.is_absolute());
        let next = self.cloud_control_path.with_extension("next");
        let permissions = tokio::fs::metadata(&self.cloud_control_path)
            .await
            .unwrap()
            .permissions();
        tokio::fs::write(&next, mode).await.unwrap();
        tokio::fs::set_permissions(&next, permissions)
            .await
            .unwrap();
        // The proxy must never interpret a partially written mode as offline.
        tokio::fs::rename(next, &self.cloud_control_path)
            .await
            .unwrap();
    }
    pub async fn hold_controller_responses(&self) {
        self.control_mode("hold-controller").await;
        let state = self
            .cloud_control_path
            .with_file_name("cloud-control.state");
        tokio::time::timeout(Duration::from_secs(5), async {
            while tokio::fs::read_to_string(&state).await.ok().as_deref() != Some("hold-controller")
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("private TLS proxy must acknowledge the response barrier");
    }
    pub fn account(&self, data: &Path) -> Arc<CloudAuth> {
        Arc::new(CloudAuth::new(
            Some(self.origin.clone()),
            data.join("account.json"),
            Arc::new(MemoryTokenStore::new()),
        ))
    }
    /// Simulates the signed-in browser's approval through the real PKCE
    /// handoff endpoint. No token is injected into either native account.
    pub async fn browser_callback(&self, browser: &str, cookie: &str) -> String {
        let url = url::Url::parse(browser).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        let response = reqwest::Client::new()
            .post(format!("{}/api/v1/desktop/handoff", self.origin))
            .header("cookie", cookie)
            .header("origin", &self.browser_origin)
            .json(&json!({
                "redirectUri":params["redirectUri"],
                "codeChallenge":params["codeChallenge"], "state":params["state"]
            }))
            .send()
            .await
            .unwrap_or_else(|_| panic!("browser handoff request failed"));
        assert!(
            response.status().is_success(),
            "browser approval must succeed: {}",
            response.status()
        );
        let response: Value = response.json().await.unwrap();
        response["callbackUrl"]
            .as_str()
            .expect("callback URL required")
            .into()
    }
    pub async fn login(&self, account: &CloudAuth, cookie: &str) {
        let begin = account.begin_login().await.unwrap();
        let callback = self.browser_callback(&begin.browser_url, cookie).await;
        let (login, ()) = tokio::join!(account.complete_login(), deliver_callback(&callback));
        login.unwrap();
    }
}

pub async fn deliver_callback(url: &str) {
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .unwrap_or_else(|_| panic!("private loopback callback failed"));
    assert!(
        response.status().is_success(),
        "loopback callback must succeed"
    );
}

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
pub fn host(data: &Path, account: CloudAuth) -> Arc<AppCore> {
    let paths = AppPaths {
        root: data.into(),
        data: data.join("data"),
        plugins: data.join("plugins"),
        plugin_data: data.join("plugin-data"),
        builtin_plugins: data.join("no-builtins"),
        development_plugins: None,
        logs: data.join("logs"),
        temp: data.join("temp"),
    };
    paths.ensure_directories().unwrap();
    for path in [
        paths.points_database(),
        paths.automation_database(),
        paths.analytics_database(),
    ] {
        std::fs::File::create(path).unwrap();
    }
    Arc::new(AppCore::with_paths_and_cloud_auth(
        Arc::new(Emitter),
        Arc::new(tiktools_core::NoopMediaHost),
        paths,
        account,
    ))
}

pub async fn until(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(40), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("native host state must converge");
}

pub struct NoBearer;
impl tiktools_client::remote::profiles::CredentialStore for NoBearer {
    fn load(
        &self,
        _: &str,
    ) -> Result<Option<tiktools_client::remote::SecretString>, tiktools_client::ClientError> {
        panic!("P2P must not load host bearer credentials")
    }
    fn save(
        &self,
        _: &str,
        _: &tiktools_client::remote::SecretString,
    ) -> Result<(), tiktools_client::ClientError> {
        panic!("P2P must not save host bearer credentials")
    }
    fn delete(&self, _: &str) -> Result<(), tiktools_client::ClientError> {
        panic!("P2P must not delete host bearer credentials")
    }
}

pub async fn cloud_available(origin: &str, online: bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if reqwest::Client::new()
                .get(format!("{origin}/api/v2/cloud"))
                .timeout(Duration::from_millis(500))
                .send()
                .await
                .is_ok()
                == online
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
}
pub async fn reopen(
    profiles: &tiktools_client::remote::profiles::ProfileManager,
    name: &str,
) -> tiktools_client::remote::RemoteClient {
    tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            match profiles.open(name).await {
                Ok(remote) => break remote,
                Err(error)
                    if matches!(
                        error.code.as_str(),
                        "unavailable"
                            | "transport"
                            | "unauthorized"
                            | "approval_pending"
                            | "rate_limited"
                    ) =>
                {
                    tokio::time::sleep(Duration::from_secs(1)).await
                }
                Err(error) => panic!("profile reopen failed: {}", error.code),
            }
        }
    })
    .await
    .unwrap()
}
