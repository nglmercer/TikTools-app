//! Authenticated SonicBoom jobs transport. HTTP budgets bound individual
//! operations; the tracking budget bounds generation, independently.
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    Transport,
    RequestTimeout,
    InferenceDeadline,
    Http(u16),
    Protocol,
    Synthesis,
    Playback,
    Deadline,
    Cancelled,
}

impl Failure {
    pub fn guidance(&self) -> &'static str {
        match self {
            Self::Transport => "Connection failed; an accepted submission may still exist. Do not retry with a new idempotency key.",
            Self::RequestTimeout => "The individual HTTP request timed out; check connectivity and server health.",
            Self::InferenceDeadline => "SonicBoom inference deadline expired; check model performance and inference settings.",
            Self::Http(401 | 403 | 404) => "Check API token ownership and reload the provider voice catalog.",
            Self::Http(429) => "SonicBoom inference admission is full; reduce concurrency or incoming speech.",
            Self::Http(503) => "Provider or server unavailable; load its selected model and reload voices.",
            Self::Http(_) => "SonicBoom rejected the request; check the authenticated profile, language and model revision.",
            Self::Protocol => "Job status could not be recovered; check server compatibility. An accepted submission may still exist.",
            Self::Synthesis => "Synthesis failed; inspect the server job and verify profile, model revision and language.",
            Self::Playback => "Audio was generated but playback was not admitted; check output device and playback queue. The job retains generated audio.",
            Self::Deadline => "Client generation tracking deadline expired; server cancellation was requested.",
            Self::Cancelled => "Speech was cancelled after disconnect, settings change or shutdown.",
        }
    }
}

pub struct JobsClient {
    pub http: reqwest::Client,
    pub server: String,
    pub token: String,
}
impl JobsClient {
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.server))
            .bearer_auth(&self.token)
            .timeout(Duration::from_secs(10))
    }
    pub async fn submit(
        &self,
        provider: &str,
        voice: &str,
        language: &str,
        text: &str,
        key: &str,
    ) -> Result<String, Failure> {
        let (owner, profile) = voice.split_once(':').ok_or(Failure::Protocol)?;
        if owner != provider
            || profile.is_empty()
            || !matches!(provider, "pocket-tts" | "qwen3-gguf")
        {
            return Err(Failure::Protocol);
        }
        let path = format!("/api/tts/jobs?profile={}&provider={}&lang={}&format=wav&playback=enqueue&play_now=false",
            super::encode_param(profile), super::encode_param(provider), super::encode_param(language));
        // Only retry with the identical principal-scoped idempotency key.
        for attempt in 0..3 {
            let result = self
                .request(reqwest::Method::POST, &path)
                .header("Idempotency-Key", key)
                .header("Content-Type", "text/plain")
                .body(text.to_owned())
                .send()
                .await;
            match result {
                Ok(response) if response.status().is_success() => {
                    if let Ok(value) = response.json::<Value>().await {
                        if let Some(id) = value["id"]
                            .as_str()
                            .filter(|id| !id.is_empty() && id.len() <= 128)
                        {
                            return Ok(id.to_owned());
                        }
                    }
                    // The job may exist even if the accepted body was lost.
                    // Recover it with the same key rather than synthesizing anew.
                    if attempt == 2 {
                        return Err(Failure::Protocol);
                    }
                    tokio::time::sleep(Duration::from_millis(250 << attempt)).await;
                }
                Ok(response) if !matches!(response.status().as_u16(), 408 | 429 | 503) => {
                    return Err(Failure::Http(response.status().as_u16()))
                }
                Ok(response) if attempt == 2 => {
                    return Err(Failure::Http(response.status().as_u16()))
                }
                Err(error) if attempt == 2 => {
                    return Err(if error.is_timeout() {
                        Failure::RequestTimeout
                    } else {
                        Failure::Transport
                    })
                }
                _ => tokio::time::sleep(Duration::from_millis(250 << attempt)).await,
            }
        }
        Err(Failure::Transport)
    }
    pub async fn cancel(&self, id: &str) {
        let _ = self
            .request(
                reqwest::Method::POST,
                &format!("/api/tts/jobs/{}/cancel", super::encode_param(id)),
            )
            .send()
            .await;
    }
    #[cfg(test)]
    async fn track(
        &self,
        id: &str,
        budget: Duration,
        cancelled: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(String, Option<String>), Failure> {
        self.track_observed(id, budget, cancelled, |_| {}).await
    }
    #[cfg(test)]
    pub async fn track_observed(
        &self,
        id: &str,
        budget: Duration,
        cancelled: tokio::sync::watch::Receiver<bool>,
        on_stage: impl FnMut(&str),
    ) -> Result<(String, Option<String>), Failure> {
        self.track_with_duration(id, budget, cancelled, on_stage)
            .await
            .map(|(state, id, _)| (state, id))
    }
    pub async fn track_with_duration(
        &self,
        id: &str,
        budget: Duration,
        mut cancelled: tokio::sync::watch::Receiver<bool>,
        mut on_stage: impl FnMut(&str),
    ) -> Result<(String, Option<String>, Option<f64>), Failure> {
        if *cancelled.borrow() {
            self.cancel(id).await;
            return Err(Failure::Cancelled);
        }
        let tracking = async {
            let mut failures = 0;
            loop {
                let response = self
                    .request(
                        reqwest::Method::GET,
                        &format!("/api/tts/jobs/{}", super::encode_param(id)),
                    )
                    .send()
                    .await;
                let value = match response {
                    Ok(response) if response.status().is_success() => {
                        response.json::<Value>().await.ok()
                    }
                    Ok(response) if matches!(response.status().as_u16(), 401 | 403 | 404) => {
                        return Err(Failure::Http(response.status().as_u16()))
                    }
                    _ => None,
                };
                if let Some(value) = value {
                    failures = 0;
                    if value["deadline_exceeded"].as_bool() == Some(true) {
                        return Err(Failure::InferenceDeadline);
                    }
                    if let Some(stage) = value["stage"].as_str() {
                        on_stage(stage);
                    }
                    match value["stage"].as_str() {
                        Some("failed") => return Err(Failure::Synthesis),
                        Some("cancelled") => return Err(Failure::Cancelled),
                        Some("completed") => match value["playback"]["state"].as_str() {
                            Some(state @ ("queued" | "started")) => {
                                return Ok((
                                    state.to_owned(),
                                    value["playback"]["queue_id"].as_str().map(str::to_owned),
                                    value["audio_secs"]
                                        .as_f64()
                                        .filter(|v| v.is_finite() && *v >= 0.0),
                                ))
                            }
                            Some("failed" | "skipped") => return Err(Failure::Playback),
                            _ => {} // Synthesis completion precedes playback disposition.
                        },
                        Some(_) => {} // Server also exposes inference substages.
                        None => return Err(Failure::Protocol),
                    }
                } else {
                    failures += 1;
                    if failures >= 5 {
                        return Err(Failure::Transport);
                    }
                }
                tokio::time::sleep(Duration::from_millis(500 * (1 << failures.min(3)))).await;
            }
        };
        let result = tokio::select! {
            _ = cancelled.changed() => Err(Failure::Cancelled),
            result = tokio::time::timeout(budget, tracking) => result.unwrap_or(Err(Failure::Deadline)),
        };
        if result.is_err() {
            self.cancel(id).await;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn server(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (
        String,
        std::sync::mpsc::Receiver<String>,
        std::thread::JoinHandle<()>,
    ) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut raw = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let n = stream.read(&mut buffer).unwrap();
                    raw.extend_from_slice(&buffer[..n]);
                    if n == 0 || raw.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                tx.send(String::from_utf8_lossy(&raw).to_string()).unwrap();
                write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        (url, rx, thread)
    }
    #[tokio::test]
    async fn generated_duration_is_preserved_with_the_exact_playback_id() {
        let (server, _requests, thread) = server(vec![(
            "200 OK",
            r#"{"stage":"completed","audio_secs":12.5,"playback":{"state":"queued","queue_id":"audio-duration"}}"#,
        )]);
        let client = JobsClient {
            http: reqwest::Client::new(),
            server,
            token: String::new(),
        };
        let (_cancel, cancelled) = tokio::sync::watch::channel(false);
        let result = client
            .track_with_duration("job", Duration::from_secs(5), cancelled, |_| {})
            .await
            .unwrap();
        assert_eq!(
            result,
            ("queued".into(), Some("audio-duration".into()), Some(12.5))
        );
        thread.join().unwrap();
    }

    #[tokio::test]
    async fn clones_route_by_profile_and_track_playback_after_synthesis() {
        for provider in ["pocket-tts", "qwen3-gguf"] {
            let (server, requests, thread) = server(vec![
                ("202 Accepted", r#"{"id":"job-a","stage":"queued"}"#),
                ("200 OK", r#"{"id":"job-a","stage":"running"}"#),
                ("200 OK", r#"{"id":"job-a","stage":"completed"}"#),
                (
                    "200 OK",
                    r#"{"id":"job-a","stage":"completed","playback":{"state":"queued","queue_id":"audio-a"}}"#,
                ),
            ]);
            let client = JobsClient {
                http: reqwest::Client::new(),
                server,
                token: "owned-token".into(),
            };
            let id = client
                .submit(
                    provider,
                    &format!("{provider}:profile-a"),
                    "es",
                    "hello",
                    "scope-1",
                )
                .await
                .unwrap();
            let (_cancel, cancelled) = tokio::sync::watch::channel(false);
            assert_eq!(
                client
                    .track(&id, Duration::from_secs(5), cancelled)
                    .await
                    .unwrap(),
                ("queued".into(), Some("audio-a".into()))
            );
            thread.join().unwrap();
            let requests: Vec<_> = requests.try_iter().collect();
            assert!(requests[0].starts_with(&format!("POST /api/tts/jobs?profile=profile-a&provider={provider}&lang=es&format=wav&playback=enqueue&play_now=false HTTP/1.1")));
            assert!(requests[0]
                .to_lowercase()
                .contains("idempotency-key: scope-1"));
            assert!(requests[0]
                .to_lowercase()
                .contains("authorization: bearer owned-token"));
            assert_eq!(requests.len(), 4);
        }
    }
    #[tokio::test]
    async fn retry_uses_identical_idempotency_key() {
        for status in [
            "408 Request Timeout",
            "429 Too Many Requests",
            "503 Unavailable",
            "202 Accepted",
        ] {
            let (server, requests, thread) =
                server(vec![(status, "{}"), ("200 OK", r#"{"id":"original"}"#)]);
            let client = JobsClient {
                http: reqwest::Client::new(),
                server,
                token: String::new(),
            };
            assert_eq!(
                client
                    .submit("qwen3-gguf", "qwen3-gguf:p", "es", "hello", "retry-key")
                    .await
                    .unwrap(),
                "original"
            );
            thread.join().unwrap();
            let requests: Vec<_> = requests.try_iter().collect();
            assert_eq!(requests.len(), 2);
            for request in requests {
                assert!(request
                    .to_lowercase()
                    .contains("idempotency-key: retry-key"));
            }
        }
    }
    #[tokio::test]
    async fn pairing_is_checked_before_http() {
        let client = JobsClient {
            http: reqwest::Client::new(),
            server: "http://127.0.0.1:1".into(),
            token: String::new(),
        };
        assert_eq!(
            client
                .submit("qwen3-gguf", "pocket-tts:p", "es", "hello", "key")
                .await,
            Err(Failure::Protocol)
        );
    }
    #[tokio::test]
    async fn failed_playback_is_not_successful_synthesis_and_cancels_tracking() {
        let (server, requests, thread) = server(vec![
            (
                "200 OK",
                r#"{"stage":"completed","playback":{"state":"failed"}}"#,
            ),
            ("200 OK", "{}"),
        ]);
        let client = JobsClient {
            http: reqwest::Client::new(),
            server,
            token: String::new(),
        };
        let (_cancel, cancelled) = tokio::sync::watch::channel(false);
        assert_eq!(
            client.track("job", Duration::from_secs(5), cancelled).await,
            Err(Failure::Playback)
        );
        thread.join().unwrap();
        assert!(requests
            .try_iter()
            .last()
            .unwrap()
            .starts_with("POST /api/tts/jobs/job/cancel "));
    }
    #[tokio::test]
    async fn shutdown_cancels_even_if_requested_before_tracking_begins() {
        let (server, requests, thread) = server(vec![("200 OK", "{}")]);
        let client = JobsClient {
            http: reqwest::Client::new(),
            server,
            token: String::new(),
        };
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        cancel.send(true).unwrap();
        assert_eq!(
            client
                .track("abandoned", Duration::from_secs(180), cancelled)
                .await,
            Err(Failure::Cancelled)
        );
        thread.join().unwrap();
        assert!(requests
            .recv()
            .unwrap()
            .starts_with("POST /api/tts/jobs/abandoned/cancel "));
    }

    #[tokio::test]
    async fn transient_polling_failure_recovers_without_resubmitting() {
        let (server, requests, thread) = server(vec![
            ("503 Unavailable", "{}"),
            (
                "200 OK",
                r#"{"stage":"completed","playback":{"state":"queued","queue_id":"audio"}}"#,
            ),
        ]);
        let client = JobsClient {
            http: reqwest::Client::new(),
            server,
            token: String::new(),
        };
        let (_cancel, cancelled) = tokio::sync::watch::channel(false);
        assert!(client
            .track("job", Duration::from_secs(5), cancelled)
            .await
            .is_ok());
        thread.join().unwrap();
        assert!(requests
            .try_iter()
            .all(|request| request.starts_with("GET /api/tts/jobs/job ")));
    }

    #[tokio::test]
    async fn deadline_cancels_running_generation() {
        let (server, requests, thread) =
            server(vec![("200 OK", r#"{"stage":"running"}"#), ("200 OK", "{}")]);
        let client = JobsClient {
            http: reqwest::Client::new(),
            server,
            token: String::new(),
        };
        let (_cancel, cancelled) = tokio::sync::watch::channel(false);
        assert_eq!(
            client
                .track("job", Duration::from_millis(100), cancelled)
                .await,
            Err(Failure::Deadline)
        );
        thread.join().unwrap();
        assert!(requests
            .try_iter()
            .last()
            .unwrap()
            .starts_with("POST /api/tts/jobs/job/cancel "));
    }
}
