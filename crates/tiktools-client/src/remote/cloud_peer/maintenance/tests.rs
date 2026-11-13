use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test(start_paused = true)]
async fn failed_cloud_refresh_retries_without_stopping_the_data_connection() {
    let calls = Arc::new(AtomicUsize::new(0));
    let stop = CancellationToken::new();
    let count = calls.clone();
    let worker = tokio::spawn(maintain(
        stop.clone(),
        || true,
        move || {
            let count = count.clone();
            async move {
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(network())
                } else {
                    Ok(())
                }
            }
        },
    ));
    for expected in 1..=3 {
        tokio::task::yield_now().await;
        tokio::time::advance(REFRESH_INTERVAL).await;
        tokio::task::yield_now().await;
        assert_eq!(calls.load(Ordering::SeqCst), expected);
        assert!(!stop.is_cancelled());
    }
    stop.cancel();
    worker.await.unwrap();
    tokio::time::advance(REFRESH_INTERVAL * 2).await;
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test(start_paused = true)]
async fn a_stalled_cloud_request_times_out_and_the_next_refresh_can_succeed() {
    let stop = CancellationToken::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let worker = tokio::spawn(maintain(
        stop.clone(),
        || true,
        move || {
            let attempt = count.fetch_add(1, Ordering::SeqCst);
            async move {
                if attempt == 0 {
                    std::future::pending::<()>().await;
                }
                Ok(())
            }
        },
    ));
    tokio::task::yield_now().await;
    tokio::time::advance(REFRESH_INTERVAL).await;
    tokio::task::yield_now().await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    tokio::time::advance(REFRESH_TIMEOUT).await;
    tokio::task::yield_now().await;
    assert!(!stop.is_cancelled());
    tokio::time::advance(REFRESH_INTERVAL).await;
    tokio::task::yield_now().await;
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    stop.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn disconnect_drops_pending_refresh_instead_of_waiting_for_cloud() {
    let stop = CancellationToken::new();
    let started = Arc::new(tokio::sync::Notify::new());
    let dropped = Arc::new(AtomicUsize::new(0));
    struct Pending(Arc<AtomicUsize>);
    impl Drop for Pending {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let notify = started.clone();
    let count = dropped.clone();
    let worker = tokio::spawn(maintain(
        stop.clone(),
        || true,
        move || {
            let guard = Pending(count.clone());
            let notify = notify.clone();
            async move {
                let _guard = guard;
                notify.notify_one();
                std::future::pending::<Result<(), ClientError>>().await
            }
        },
    ));
    started.notified().await;
    stop.cancel();
    worker.await.unwrap();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn maintenance_cannot_publish_for_another_account_origin_or_key() {
    let home = tempfile::tempdir().unwrap();
    let context = MaintenanceContext::new(
        Arc::new(CloudAuth::new(
            None,
            home.path().join("account.json"),
            Arc::new(tiktools_cloud_auth::MemoryTokenStore::new()),
        )),
        home.path().into(),
    );
    let origin = "https://cloud.example";
    let (_, data) = scoped(home.path(), origin, "alice");
    std::fs::create_dir_all(data.join("remote")).unwrap();
    let device = uuid::Uuid::new_v4().to_string();
    std::fs::write(
        data.join("remote/remote-controller.json"),
        serde_json::json!({"deviceId":device}).to_string(),
    )
    .unwrap();
    let endpoint = tiktools_peer::SecretKey::generate().public().to_string();
    let mut profile = RemoteProfile {
        version: 2,
        name: "test".into(),
        profile_id: uuid::Uuid::new_v4().to_string(),
        origin: origin.into(),
        host_id: uuid::Uuid::new_v4().to_string(),
        principal_id: uuid::Uuid::new_v4().to_string(),
        session_id: uuid::Uuid::new_v4().to_string(),
        client_kind: ClientKind::Desktop,
        trust: super::super::super::TlsTrust::System,
        credential_key: String::new(),
        refresh_attempt: None,
        peer: Some(PeerProfile {
            key_path: data.join("identity.key"),
            local_endpoint_id: endpoint.clone(),
            remote_address: EndpointAddr::new(tiktools_peer::SecretKey::generate().public()),
            relay_urls: vec![],
            cloud_host_id: uuid::Uuid::new_v4().to_string(),
            host_device_id: None,
        }),
    };
    assert_eq!(
        context
            .binding(profile.peer.as_ref().unwrap(), origin, "alice", &endpoint)
            .unwrap()
            .1,
        device
    );
    for (other_origin, other_account) in [(origin, "bob"), ("https://other.example", "alice")] {
        assert!(context
            .binding(
                profile.peer.as_ref().unwrap(),
                other_origin,
                other_account,
                &endpoint
            )
            .is_err());
    }
    profile.peer.as_mut().unwrap().local_endpoint_id =
        tiktools_peer::SecretKey::generate().public().to_string();
    assert!(context
        .binding(profile.peer.as_ref().unwrap(), origin, "alice", &endpoint)
        .is_err());
}

#[tokio::test(start_paused = true)]
async fn account_change_cancels_a_stalled_publication_promptly() {
    let stop = CancellationToken::new();
    let valid = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let dropped = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(tokio::sync::Notify::new());
    struct Pending(Arc<AtomicUsize>);
    impl Drop for Pending {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let state = valid.clone();
    let ready = started.clone();
    let count = dropped.clone();
    let lifetime = stop.clone();
    let worker = tokio::spawn(maintain(
        lifetime,
        move || state.load(Ordering::SeqCst),
        move || {
            let guard = Pending(count.clone());
            let ready = ready.clone();
            async move {
                let _guard = guard;
                ready.notify_one();
                std::future::pending::<Result<(), ClientError>>().await
            }
        },
    ));
    started.notified().await;
    valid.store(false, Ordering::SeqCst);
    tokio::time::advance(Duration::from_millis(500)).await;
    tokio::time::timeout(Duration::from_secs(1), worker)
        .await
        .unwrap()
        .unwrap();
    assert!(stop.is_cancelled());
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}
