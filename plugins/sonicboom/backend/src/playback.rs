//! Opt-in queue monitor. HTTP runs on its own bounded worker, never blocking
//! plugin event ingestion. Only allowlisted playback fields reach the WebView.
use crate::config::BackendConfig;
use serde_json::{json, Value};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

pub type Durations = Arc<Mutex<std::collections::BTreeMap<(String, String, String), f64>>>;
pub struct PlaybackMonitor {
    requests: Option<mpsc::SyncSender<(u64, BackendConfig)>>,
    snapshot: Arc<Mutex<(u64, Value)>>,
    identity: Option<(String, String)>,
    generation: u64,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl PlaybackMonitor {
    pub fn new(durations: Durations) -> Self {
        let (tx, rx) = mpsc::sync_channel::<(u64, BackendConfig)>(1);
        let snapshot = Arc::new(Mutex::new((0, json!({"state":"pending"}))));
        let shared = Arc::clone(&snapshot);
        let worker = std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            let Ok(http) = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(3))
                .build()
            else {
                return;
            };
            while let Ok((generation, config)) = rx.recv() {
                let mut value = runtime.block_on(fetch_status(&http, &config));
                if value["state"] == "ready" && value["durationSecs"].is_null() {
                    if let Some(id) = value["currentId"].as_str() {
                        let key = (
                            config.server_url.clone(),
                            config.api_token.clone(),
                            id.to_owned(),
                        );
                        if let Some(duration) = durations
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .get(&key)
                        {
                            value["durationSecs"] = json!(duration);
                        }
                    }
                }
                let mut latest = shared.lock().unwrap_or_else(|e| e.into_inner());
                if latest.0 == generation {
                    latest.1 = value;
                }
            }
        });
        Self {
            requests: Some(tx),
            snapshot,
            identity: None,
            generation: 0,
            worker: Some(worker),
        }
    }
}
impl PlaybackMonitor {
    pub fn refresh(&mut self, config: BackendConfig) -> Value {
        let identity = (config.server_url.clone(), config.api_token.clone());
        if self.identity.as_ref() != Some(&identity) {
            self.generation = self.generation.wrapping_add(1);
            self.identity = Some(identity);
            *self.snapshot.lock().unwrap_or_else(|e| e.into_inner()) =
                (self.generation, json!({"state":"pending"}));
        }
        if let Some(tx) = &self.requests {
            let _ = tx.try_send((self.generation, config));
        }
        self.snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .1
            .clone()
    }
}
impl Drop for PlaybackMonitor {
    fn drop(&mut self) {
        self.requests.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

async fn fetch_status(http: &reqwest::Client, config: &BackendConfig) -> Value {
    let result = async {
        let mut response = http
            .get(format!("{}/api/queue/status", config.server_url))
            .bearer_auth(&config.api_token)
            .send()
            .await
            .map_err(|_| "connection_failed")?;
        match response.status().as_u16() {
            200 => {}
            401 | 403 => return Err("unauthorized"),
            404 => return Err("unsupported"),
            _ => return Err("server_error"),
        }
        // Bound decoded bytes as well as Content-Length. Never echo response text.
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "connection_failed")? {
            if body.len() + chunk.len() > 65_536 {
                return Err("invalid_response");
            }
            body.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&body).map_err(|_| "invalid_response")?;
        sanitize_status(&value).ok_or("invalid_response")
    }
    .await;
    match result {
        Ok(value) => value,
        Err(code) => json!({"state":"error", "error":code}),
    }
}
fn finite_seconds(value: &Value) -> Option<f64> {
    value.as_f64().filter(|v| v.is_finite() && *v >= 0.0)
}
fn sanitize_status(value: &Value) -> Option<Value> {
    let id = match &value["current"] {
        Value::Null => None,
        Value::Object(current) => Some(
            current
                .get("id")?
                .as_str()?
                .chars()
                .take(128)
                .collect::<String>(),
        ),
        _ => return None,
    };
    Some(json!({
        "state":"ready", "observedAt":crate::now_millis(), "currentId":id,
        "queueLength":value["queue_length"].as_u64()?,
        "playing":value["is_playing"].as_bool()?, "paused":value["is_paused"].as_bool()?,
        // Older servers expose no duration/position: preserve absence, never
        // derive a claimed total from text length or wall-clock polling.
        "durationSecs":finite_seconds(&value["current"]["duration_secs"]).or_else(|| finite_seconds(&value["duration_secs"])),
        "positionSecs":finite_seconds(&value["position_secs"]),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn queue_requests_authenticate_and_never_expose_response_bodies() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            for (status, body) in [
                (
                    "200 OK",
                    r#"{"current":{"id":"current-a","path":"/private.wav"},"queue_length":1,"is_playing":true,"is_paused":false}"#,
                ),
                ("401 Unauthorized", "private-token-echo"),
            ] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 4096];
                let size = stream.read(&mut buffer).await.unwrap();
                let request = String::from_utf8_lossy(&buffer[..size]).to_lowercase();
                assert!(request.starts_with("get /api/queue/status "));
                assert!(request.contains("authorization: bearer test-token"));
                stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
        });
        let config = BackendConfig {
            server_url: format!("http://{address}"),
            api_token: "test-token".into(),
            ..Default::default()
        };
        let http = reqwest::Client::new();
        let ready = fetch_status(&http, &config).await;
        assert_eq!(ready["currentId"], "current-a");
        assert!(!ready.to_string().contains("private"));
        let error = fetch_status(&http, &config).await;
        assert_eq!(error, json!({"state":"error","error":"unauthorized"}));
        task.await.unwrap();
    }

    #[test]
    fn queue_snapshot_omits_paths_and_preserves_unknown_duration() {
        let value = json!({"current":{"id":"audio-a", "path":"/private/speech.wav"}, "queue_length":2, "is_playing":true, "is_paused":false});
        let status = sanitize_status(&value).unwrap();
        assert_eq!(status["currentId"], "audio-a");
        assert!(status["durationSecs"].is_null());
        assert!(!status.to_string().contains("private"));
        assert!(sanitize_status(&json!({"current":null})).is_none());
        let mut value = value;
        value["duration_secs"] = json!(12.5);
        value["position_secs"] = json!(3.0);
        assert_eq!(sanitize_status(&value).unwrap()["durationSecs"], 12.5);
        value["duration_secs"] = json!(-1);
        assert!(sanitize_status(&value).unwrap()["durationSecs"].is_null());
    }
    #[test]
    fn changing_principal_discards_previous_playback_snapshot() {
        let mut monitor = PlaybackMonitor::new(Default::default());
        let config = BackendConfig {
            server_url: "http://127.0.0.1:1".into(),
            ..Default::default()
        };
        monitor.refresh(config.clone());
        monitor.snapshot.lock().unwrap().1 = json!({"state":"ready","currentId":"old"});
        let changed = BackendConfig {
            api_token: "new-token".into(),
            ..config
        };
        assert_eq!(monitor.refresh(changed)["state"], "pending");
    }
}
