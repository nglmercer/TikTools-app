//! Application retry ownership: typed failures, bounded attempts, and a cooldown
//! shared by connection, browsing, and replacement requests on this client.
use crate::{lifecycle, ClientEvent, ConnectionState, TikTokError};
use std::{
    future::Future,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Default)]
pub(super) struct Cooldown(Arc<Mutex<Option<tokio::time::Instant>>>);

impl Cooldown {
    pub(super) async fn remaining(&self) -> Option<Duration> {
        self.0
            .lock()
            .await
            .and_then(|deadline| deadline.checked_duration_since(tokio::time::Instant::now()))
            .filter(|delay| !delay.is_zero())
    }

    pub(super) async fn extend(&self, delay: Duration) {
        let deadline = tokio::time::Instant::now() + delay;
        let mut current = self.0.lock().await;
        *current = Some(current.map_or(deadline, |previous| previous.max(deadline)));
    }

    pub(super) async fn wait(&self, stop: &CancellationToken) -> Result<(), TikTokError> {
        loop {
            let deadline = *self.0.lock().await;
            match deadline {
                Some(deadline) if deadline > tokio::time::Instant::now() => {
                    tokio::select! {
                        biased;
                        () = stop.cancelled() => return Err(TikTokError::Task("connection superseded".into())),
                        () = tokio::time::sleep_until(deadline) => {}
                    }
                }
                _ => return Ok(()),
            }
        }
    }
}

#[derive(Clone)]
pub(super) struct RetryContext {
    pub(super) cooldown: Cooldown,
    pub(super) policy: ttl_live_ws::ReconnectPolicy,
    pub(super) stop: CancellationToken,
    pub(super) events: lifecycle::EventSender,
    pub(super) state: lifecycle::LifecycleState,
    pub(super) generation: u64,
}

fn delay(
    error: &TikTokError,
    attempt: u32,
    policy: &ttl_live_ws::ReconnectPolicy,
    sample: f64,
) -> Option<Duration> {
    match error {
        TikTokError::RateLimited { retry_after } => {
            // Never shorten Retry-After. An unavailable header uses a conservative
            // 30/60 second fallback, with positive jitter to avoid synchronized retries.
            let fallback = Duration::from_secs(30 * (1u64 << attempt.saturating_sub(1).min(4)))
                .min(Duration::from_secs(300));
            Some(
                retry_after.unwrap_or(fallback).max(fallback)
                    + Duration::from_secs_f64(7.5 * sample.clamp(0.0, 1.0)),
            )
        }
        TikTokError::RetryableTransport(_) => {
            Some(policy.backoff_with_jitter(attempt, 0.25, sample))
        }
        _ => None,
    }
}

impl RetryContext {
    pub(super) async fn run<T, F, Fut>(&self, mut operation: F) -> Result<T, TikTokError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, TikTokError>>,
    {
        let mut attempt: u32 = 0;
        loop {
            if self.stop.is_cancelled() {
                return Err(TikTokError::Task("connection superseded".into()));
            }
            if let Some(wait) = self.cooldown.remaining().await {
                self.state
                    .transition(self.generation, ConnectionState::Reconnecting);
                self.events.send(
                    self.generation,
                    ClientEvent::Reconnecting {
                        attempt: u64::from(attempt.max(1)),
                        delay_ms: wait.as_millis().min(u128::from(u64::MAX)) as u64,
                    },
                );
            }
            self.cooldown.wait(&self.stop).await?;
            let result = lifecycle::interruptible(&self.stop, operation()).await;
            let error = match result {
                Ok(value) => return Ok(value),
                Err(error) => error,
            };
            attempt += 1;
            let Some(wait) = delay(&error, attempt, &self.policy, fastrand::f64()) else {
                return Err(error);
            };
            // Retain cooldown even after exhaustion or cancellation, so repeated
            // Connect clicks cannot immediately hit the same rate limit again.
            if matches!(error, TikTokError::RateLimited { .. }) {
                self.cooldown.extend(wait).await;
            }
            let limit = if matches!(error, TikTokError::RateLimited { .. }) {
                self.policy.max_attempts.min(2)
            } else {
                self.policy.max_attempts
            };
            if attempt > limit {
                return Err(error);
            }
            self.state
                .transition(self.generation, ConnectionState::Reconnecting);
            self.events.send(
                self.generation,
                ClientEvent::Reconnecting {
                    attempt: u64::from(attempt),
                    delay_ms: wait.as_millis().min(u128::from(u64::MAX)) as u64,
                },
            );
            tokio::select! {
                biased;
                () = self.stop.cancelled() => return Err(TikTokError::Task("connection superseded".into())),
                () = tokio::time::sleep(wait) => {}
            }
        }
    }
}

pub(super) fn retry_after(
    value: Option<&reqwest::header::HeaderValue>,
    now: SystemTime,
) -> Option<Duration> {
    let text = value?.to_str().ok()?.trim();
    if let Ok(seconds) = text.parse::<u64>() {
        // Bound representable deadlines; long cooldowns are never retried early.
        return Some(Duration::from_secs(seconds.min(365 * 24 * 60 * 60)));
    }
    Some(
        httpdate::parse_http_date(text)
            .ok()?
            .duration_since(now)
            .unwrap_or_default()
            .min(Duration::from_secs(365 * 24 * 60 * 60)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn context() -> RetryContext {
        RetryContext {
            cooldown: Cooldown::default(),
            policy: ttl_live_ws::ReconnectPolicy::default(),
            stop: CancellationToken::new(),
            events: lifecycle::EventSender::new(),
            state: lifecycle::LifecycleState::new(),
            generation: 0,
        }
    }

    #[test]
    fn rate_limits_have_a_separate_floor_and_preserve_server_delay() {
        let policy = ttl_live_ws::ReconnectPolicy::default();
        assert_eq!(
            delay(
                &TikTokError::RateLimited { retry_after: None },
                1,
                &policy,
                0.0
            ),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            delay(
                &TikTokError::RateLimited { retry_after: None },
                2,
                &policy,
                0.0
            ),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            delay(
                &TikTokError::RateLimited {
                    retry_after: Some(Duration::from_secs(600))
                },
                1,
                &policy,
                0.0
            ),
            Some(Duration::from_secs(600))
        );
        for error in [
            TikTokError::InvalidCreator,
            TikTokError::EmptySession,
            TikTokError::Discovery("offline".into()),
            TikTokError::Transport("HTTP 403".into()),
            TikTokError::Signer("bad signature".into()),
        ] {
            assert_eq!(delay(&error, 1, &policy, 0.0), None);
        }
    }

    #[test]
    fn retry_after_supports_seconds_dates_and_invalid_headers() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let header = reqwest::header::HeaderValue::from_static("120");
        assert_eq!(
            retry_after(Some(&header), now),
            Some(Duration::from_secs(120))
        );
        let header = reqwest::header::HeaderValue::from_str(&httpdate::fmt_http_date(
            now + Duration::from_secs(90),
        ))
        .unwrap();
        assert_eq!(
            retry_after(Some(&header), now),
            Some(Duration::from_secs(90))
        );
        assert_eq!(
            retry_after(
                Some(&reqwest::header::HeaderValue::from_static("invalid")),
                now
            ),
            None
        );
    }

    #[tokio::test(start_paused = true)]
    async fn initial_429_retries_then_delivers_success_and_progress() {
        let context = context();
        let mut events = context.events.subscribe();
        let calls = AtomicUsize::new(0);
        let started = tokio::time::Instant::now();
        let value = context
            .run(|| async {
                if calls.fetch_add(1, Ordering::Relaxed) == 0 {
                    Err(TikTokError::RateLimited { retry_after: None })
                } else {
                    Ok(42)
                }
            })
            .await
            .unwrap();
        assert_eq!(value, 42);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert!(started.elapsed() >= Duration::from_secs(30));
        assert!(matches!(
            events.try_recv().unwrap(),
            ClientEvent::Reconnecting {
                attempt: 1,
                delay_ms: 30000..=37500
            }
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn exhaustion_is_bounded_and_replacements_share_cooldown() {
        let context = context();
        let calls = AtomicUsize::new(0);
        assert!(context
            .run(|| async {
                calls.fetch_add(1, Ordering::Relaxed);
                Err::<(), _>(TikTokError::RateLimited { retry_after: None })
            })
            .await
            .is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        let started = tokio::time::Instant::now();
        context.run(|| async { Ok(()) }).await.unwrap();
        assert!(started.elapsed() >= Duration::from_secs(120));
    }

    #[tokio::test(start_paused = true)]
    async fn disconnect_during_retry_wait_preserves_cooldown_and_stops_requests() {
        let context = context();
        let mut events = context.events.subscribe();
        let stop = context.stop.clone();
        let cooldown = context.cooldown.clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let task = tokio::spawn(async move {
            context
                .run(|| async {
                    calls.fetch_add(1, Ordering::Relaxed);
                    Err::<(), _>(TikTokError::RateLimited { retry_after: None })
                })
                .await
        });
        assert!(matches!(
            events.recv().await.unwrap(),
            ClientEvent::Reconnecting { .. }
        ));
        stop.cancel();
        assert!(matches!(task.await.unwrap(), Err(TikTokError::Task(_))));
        assert_eq!(observed.load(Ordering::Relaxed), 1);
        assert!(cooldown.remaining().await.is_some());
    }

    #[tokio::test(start_paused = true)]
    async fn disconnect_interrupts_cooldown_without_another_request() {
        let context = context();
        context.cooldown.extend(Duration::from_secs(600)).await;
        context.stop.cancel();
        let calls = AtomicUsize::new(0);
        assert!(context
            .run(|| async {
                calls.fetch_add(1, Ordering::Relaxed);
                Ok(())
            })
            .await
            .is_err());
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }
}

#[cfg(test)]
mod socket_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use ttl_sign_core::{
        BackendFuture, ClientIdentity, CookieJar, SignOutcome, SignedFetch, SignerBackend,
        TransportRequest,
    };

    struct Backend {
        address: std::net::SocketAddr,
        signatures: AtomicUsize,
    }
    impl SignerBackend for Backend {
        fn transport(&self, _: TransportRequest) -> BackendFuture<'_> {
            let attempt = self.signatures.fetch_add(1, Ordering::Relaxed) + 1;
            Box::pin(async move {
                SignOutcome::Ok(SignedFetch {
                    signed_url: format!("ws://{}/?room_id=1&attempt={attempt}", self.address),
                    cookies: CookieJar::parse("ttwid=fixture"),
                    user_agent: "fixture".into(),
                    protobuf: Vec::new(),
                })
            })
        }
        fn identity(&self) -> ClientIdentity {
            ClientIdentity::new("fixture")
        }
    }

    #[tokio::test(start_paused = true)]
    async fn real_handshake_429_waits_resigns_and_stops_on_403() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend = Backend {
            address: listener.local_addr().unwrap(),
            signatures: AtomicUsize::new(0),
        };
        let server = tokio::spawn(async move {
            for (attempt, status) in [(1, "429 Too Many Requests"), (2, "403 Forbidden")] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(stream.read_u8().await.unwrap());
                    assert!(request.len() < 8192);
                }
                assert!(String::from_utf8(request)
                    .unwrap()
                    .contains(&format!("attempt={attempt}")));
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        });
        let context = RetryContext {
            cooldown: Cooldown::default(),
            policy: ttl_live_ws::ReconnectPolicy::default(),
            stop: CancellationToken::new(),
            events: lifecycle::EventSender::new(),
            state: lifecycle::LifecycleState::new(),
            generation: 0,
        };
        let started = tokio::time::Instant::now();
        let result = context.run(|| crate::open_socket(&backend, "1")).await;
        assert!(matches!(result, Err(TikTokError::Transport(message)) if message.contains("403")));
        assert_eq!(backend.signatures.load(Ordering::Relaxed), 2);
        assert!(started.elapsed() >= Duration::from_secs(30));
        server.await.unwrap();
    }

    #[test]
    fn only_transient_socket_statuses_are_retryable() {
        for status in [400, 401, 403, 404] {
            assert!(matches!(
                TikTokError::from_socket(ttl_live_ws::WsError::HttpStatus(status)),
                TikTokError::Transport(_)
            ));
        }
        assert!(matches!(
            TikTokError::from_socket(ttl_live_ws::WsError::HttpStatus(429)),
            TikTokError::RateLimited { .. }
        ));
        assert!(matches!(
            TikTokError::from_socket(ttl_live_ws::WsError::HttpStatus(503)),
            TikTokError::RetryableTransport(_)
        ));
        assert!(matches!(
            TikTokError::from_socket(ttl_live_ws::WsError::Blocked200(None)),
            TikTokError::Transport(_)
        ));
    }
}
