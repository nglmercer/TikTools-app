//! Transport unit tests.

use super::framing::{read_capped_line, LineOutcome, MAX_REQUEST_BYTES};
use super::server::{drain_events, serve_stream};
use crate::ControlApi;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// Drives one request plus one domain event through the NDJSON framing
/// over an in-memory duplex (no sockets needed).
#[tokio::test]
async fn ndjson_framing_carries_responses_and_events() {
    struct Emitter;
    impl tiktools_core::HostEmitter for Emitter {
        fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
    }
    // Isolated home so the core never touches real user data.
    let home = std::env::temp_dir().join(format!(
        "tiktools-transport-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);
    let core = std::sync::Arc::new(tiktools_core::AppCore::new(std::sync::Arc::new(Emitter)));
    let api = ControlApi::new(std::sync::Arc::clone(&core));
    let (client, server) = tokio::io::duplex(64 * 1024);
    let (server_read, server_write) = tokio::io::split(server);
    let server_task = tokio::spawn(async move {
        serve_stream(
            &api,
            tokio::io::BufReader::new(server_read),
            server_write,
            true,
        )
        .await
    });
    let (client_read, mut client_write) = tokio::io::split(client);
    client_write
        .write_all(b"{\"id\":1,\"method\":\"system.ping\"}\n")
        .await
        .unwrap();
    let mut lines = tokio::io::BufReader::new(client_read).lines();
    // Every wait below is bounded: a stuck server must fail the test,
    // never hang the suite.
    let response = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
        .await
        .expect("response line timed out")
        .unwrap()
        .expect("response line");
    assert!(
        response.contains("\"id\":1"),
        "unexpected response: {response}"
    );
    assert!(
        response.contains("\"ok\":true"),
        "unexpected response: {response}"
    );
    // The server is now inside its select loop (hence subscribed), so a
    // domain event published here interleaves as a JSON-RPC notification.
    core.events
        .publish_domain(tiktools_core::events::DomainEvent::LiveDisconnected);
    let event = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
        .await
        .expect("event line timed out")
        .unwrap()
        .expect("event line");
    assert!(
        event.contains("\"method\":\"event\""),
        "unexpected event: {event}"
    );
    assert!(
        event.contains("live.disconnected"),
        "unexpected event: {event}"
    );
    // Split halves share the duplex endpoint, so a bare drop would not
    // deliver EOF; an explicit shutdown closes the write side.
    client_write.shutdown().await.unwrap();
    let eof = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
        .await
        .expect("EOF timed out")
        .unwrap();
    assert!(eof.is_none());
    tokio::time::timeout(std::time::Duration::from_secs(10), server_task)
        .await
        .expect("server task hung after EOF")
        .expect("server task panicked")
        .unwrap();
    let _ = std::fs::remove_dir_all(&home);
}

/// Isolated core for tests that need gap recording without a server.
fn gap_test_core(tag: &str) -> (std::sync::Arc<tiktools_core::AppCore>, std::path::PathBuf) {
    struct Emitter;
    impl tiktools_core::HostEmitter for Emitter {
        fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
    }
    let home = std::env::temp_dir().join(format!(
        "tiktools-transport-gap-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);
    (
        std::sync::Arc::new(tiktools_core::AppCore::new(std::sync::Arc::new(Emitter))),
        home,
    )
}

#[tokio::test]
async fn drain_emits_gap_and_continues_after_reliable_lag() {
    use tiktools_core::events::DomainEvent;
    let (core, home) = gap_test_core("drain");
    let bus = tiktools_core::events::EventBus::new(1);
    let mut events = Some(bus.subscribe_domain());
    // Lag the reliable lane deterministically, then publish a marker
    // that must still arrive after the gap.
    for _ in 0..600 {
        bus.publish_domain(DomainEvent::LiveDisconnected);
    }
    bus.publish_domain(DomainEvent::LiveError {
        phase: "post-gap-marker".to_owned(),
        message: "m".to_owned(),
    });
    let mut output = Vec::new();
    drain_events(&mut output, &mut events, &core)
        .await
        .expect("drain succeeds");
    let text = String::from_utf8(output).expect("drain writes UTF-8 lines");
    let mut saw_gap = false;
    let mut saw_marker = false;
    for line in text.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("drain writes JSON lines");
        if value.get("method").and_then(|method| method.as_str()) == Some("event.gap") {
            saw_gap = true;
            assert_eq!(value["params"]["resync"], serde_json::json!(true));
            assert!(
                value["params"]["lost"].as_u64().unwrap_or_default() > 0,
                "gap must count the lost events: {line}"
            );
        }
        if line.contains("post-gap-marker") {
            saw_marker = true;
        }
    }
    assert!(saw_gap, "reliable lag must emit event.gap, not silent loss");
    assert!(
        saw_marker,
        "draining must continue past lag to later events"
    );
    assert_eq!(core.event_gap_stats().0, 1);
    assert!(
        events.is_some(),
        "the stream stays armed after a recoverable gap"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn drain_skips_lossy_bursts_without_gap_signal() {
    use tiktools_core::events::DomainEvent;
    let (core, home) = gap_test_core("lossy");
    let bus = tiktools_core::events::EventBus::new(1);
    let mut events = Some(bus.subscribe_domain());
    for index in 0..600 {
        bus.publish_domain(DomainEvent::LiveUiEvent {
            event: serde_json::json!({"n": index}),
        });
    }
    let mut output = Vec::new();
    drain_events(&mut output, &mut events, &core)
        .await
        .expect("drain succeeds");
    let text = String::from_utf8(output).expect("drain writes UTF-8 lines");
    assert!(
        !text.contains("event.gap"),
        "a lossy flood must never cause a gap signal: {text}"
    );
    assert_eq!(core.event_gap_stats().0, 0);
    let _ = std::fs::remove_dir_all(&home);
}

/// End to end: a reliable burst over a slow connection produces an
/// explicit gap, the connection stays alive, and later events still
/// arrive. Current-thread on purpose: the burst publishes without
/// yielding, so the server cannot interleave reads and the lag is
/// deterministic.
#[tokio::test(flavor = "current_thread")]
async fn burst_lags_connection_with_gap_but_stays_alive() {
    use tiktools_core::events::DomainEvent;
    struct Emitter;
    impl tiktools_core::HostEmitter for Emitter {
        fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
    }
    let home = std::env::temp_dir().join(format!(
        "tiktools-transport-burst-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);
    let core = std::sync::Arc::new(tiktools_core::AppCore::new(std::sync::Arc::new(Emitter)));
    let api = ControlApi::new(std::sync::Arc::clone(&core));
    // Tiny duplex: the server blocks writing the burst, guaranteeing
    // the connection subscription lags while the test is not reading.
    let (client, server) = tokio::io::duplex(1024);
    let (server_read, server_write) = tokio::io::split(server);
    let server_task = tokio::spawn(async move {
        serve_stream(
            &api,
            tokio::io::BufReader::new(server_read),
            server_write,
            true,
        )
        .await
    });
    let (client_read, mut client_write) = tokio::io::split(client);
    let mut lines = tokio::io::BufReader::new(client_read).lines();
    async fn next_line(
        lines: &mut tokio::io::Lines<
            tokio::io::BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
        >,
    ) -> Option<String> {
        tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("line timed out")
            .expect("read failed")
    }
    // Round trip first so the server is subscribed before the burst.
    client_write
        .write_all(b"{\"id\":1,\"method\":\"system.ping\"}\n")
        .await
        .unwrap();
    assert!(next_line(&mut lines).await.unwrap().contains("\"id\":1"));
    // No awaits: on this runtime the server cannot read mid-burst, so
    // 600 reliable events into a 256-capacity lane must lag it.
    for _ in 0..600 {
        core.events.publish_domain(DomainEvent::LiveDisconnected);
    }
    // The client receives an explicit gap notification.
    let mut saw_gap = false;
    for _ in 0..700 {
        let line = next_line(&mut lines).await.expect("gap line");
        if line.contains("\"method\":\"event.gap\"") {
            let value: serde_json::Value = serde_json::from_str(&line).expect("gap is JSON");
            assert_eq!(value["params"]["resync"], serde_json::json!(true));
            assert!(
                value["params"]["lost"].as_u64().unwrap_or_default() > 0,
                "gap must count lost events: {line}"
            );
            saw_gap = true;
            break;
        }
    }
    assert!(saw_gap, "burst must produce an event.gap notification");
    assert!(
        core.event_gap_stats().0 >= 1,
        "the gap must be recorded for system.health"
    );
    // The connection remains alive: RPC still works after the gap.
    client_write
        .write_all(b"{\"id\":2,\"method\":\"system.ping\"}\n")
        .await
        .unwrap();
    let mut saw_pong = false;
    for _ in 0..700 {
        let line = next_line(&mut lines).await.expect("post-gap line");
        if line.contains("\"id\":2") {
            assert!(line.contains("\"ok\":true"), "unexpected pong: {line}");
            saw_pong = true;
            break;
        }
    }
    assert!(saw_pong, "connection must stay alive after the gap");
    // Later events still arrive after the gap.
    core.events.publish_domain(DomainEvent::LiveError {
        phase: "post-gap-marker".to_owned(),
        message: "m".to_owned(),
    });
    let mut saw_marker = false;
    for _ in 0..700 {
        let line = next_line(&mut lines).await.expect("marker line");
        if line.contains("post-gap-marker") {
            saw_marker = true;
            break;
        }
    }
    assert!(saw_marker, "post-gap events must still arrive");
    client_write.shutdown().await.unwrap();
    // Drain to EOF so a server blocked on a full buffer can finish.
    while next_line(&mut lines).await.is_some() {}
    tokio::time::timeout(std::time::Duration::from_secs(10), server_task)
        .await
        .expect("server task hung after EOF")
        .expect("server task panicked")
        .unwrap();
    let _ = std::fs::remove_dir_all(&home);
}

#[tokio::test]
async fn overlong_lines_error_without_breaking_framing() {
    let input = format!(
        "{{\"id\":1,\"method\":\"{}}}\n{{\"id\":2,\"method\":\"system.ping\"}}\n",
        "x".repeat(MAX_REQUEST_BYTES)
    );
    let mut reader = tokio::io::BufReader::new(input.as_bytes());
    let mut line = Vec::new();
    assert!(matches!(
        read_capped_line(&mut reader, &mut line).await.unwrap(),
        Some(LineOutcome::TooLarge)
    ));
    // The recovery prefix survives so the `too_large` error carries id 1.
    assert_eq!(
        crate::RpcId::extract_from_prefix(&String::from_utf8_lossy(&line)),
        crate::RpcId::Number(1)
    );
    assert!(matches!(
        read_capped_line(&mut reader, &mut line).await.unwrap(),
        Some(LineOutcome::Ok)
    ));
    assert!(String::from_utf8_lossy(&line).contains("\"id\":2"));
}

#[tokio::test]
async fn oversized_request_errors_with_its_id_and_framing_survives() {
    struct Emitter;
    impl tiktools_core::HostEmitter for Emitter {
        fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
    }
    let home = std::env::temp_dir().join(format!(
        "tiktools-transport-oversized-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    std::env::set_var("TIKTOOLS_HOME", &home);
    let core = std::sync::Arc::new(tiktools_core::AppCore::new(std::sync::Arc::new(Emitter)));
    let api = ControlApi::new(std::sync::Arc::clone(&core));
    let (client, server) = tokio::io::duplex(4 * 1024 * 1024);
    let (server_read, server_write) = tokio::io::split(server);
    let server_task = tokio::spawn(async move {
        serve_stream(
            &api,
            tokio::io::BufReader::new(server_read),
            server_write,
            false,
        )
        .await
    });
    let (client_read, mut client_write) = tokio::io::split(client);
    let oversized = format!(
        "{{\"id\":9,\"method\":\"system.ping\",\"params\":{{\"blob\":\"{}\"}}}}\n",
        "y".repeat(MAX_REQUEST_BYTES)
    );
    client_write.write_all(oversized.as_bytes()).await.unwrap();
    client_write
        .write_all(b"{\"id\":10,\"method\":\"system.ping\"}\n")
        .await
        .unwrap();
    let mut lines = tokio::io::BufReader::new(client_read).lines();
    let first = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
        .await
        .expect("too_large line timed out")
        .unwrap()
        .expect("too_large line");
    assert!(
        first.contains("\"id\":9") && first.contains("too_large"),
        "oversized error must carry its id: {first}"
    );
    let second = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
        .await
        .expect("follow-up line timed out")
        .unwrap()
        .expect("follow-up line");
    assert!(
        second.contains("\"id\":10") && second.contains("\"ok\":true"),
        "framing must survive the oversized line: {second}"
    );
    client_write.shutdown().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), server_task)
        .await
        .expect("server task hung after EOF")
        .expect("server task panicked")
        .unwrap();
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(windows)]
#[test]
fn user_sid_string_is_pipe_safe() {
    let sid = super::security::current_user_sid_string().expect("user SID resolves");
    assert!(sid.starts_with("S-1-"), "unexpected SID string form: {sid}");
    assert!(
        sid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "SID must be pipe-name safe: {sid}"
    );
}

#[cfg(windows)]
#[test]
fn production_pipe_name_is_per_user_and_override_is_exact() {
    let previous = std::env::var("TIKTOOLS_IPC_NAME").ok();
    std::env::set_var("TIKTOOLS_IPC_NAME", "tiktools-test-pipe-1");
    assert_eq!(
        super::ipc_pipe_name().expect("override name"),
        r"\\.\pipe\tiktools-test-pipe-1"
    );
    std::env::remove_var("TIKTOOLS_IPC_NAME");
    let production = super::ipc_pipe_name().expect("production name");
    assert!(
        production.starts_with(r"\\.\pipe\tiktools-control-S-"),
        "production pipe must carry the user SID: {production}"
    );
    if let Some(value) = previous {
        std::env::set_var("TIKTOOLS_IPC_NAME", value);
    } else {
        std::env::remove_var("TIKTOOLS_IPC_NAME");
    }
}

#[cfg(windows)]
#[tokio::test]
async fn pipe_probe_sees_listeners_and_claim_refuses_second_owner() {
    use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let name = format!(
        r"\\.\pipe\tiktools-claim-test-{}-{unique}",
        std::process::id()
    );
    // A claimed instance stays connectable under its user-only ACL,
    // and the raw handle interoperates with the runtime.
    let server = super::security::claim_pipe_instance(&name)
        .await
        .expect("first claim succeeds");
    let _client = ClientOptions::new()
        .open(&name)
        .expect("owner connects under its own ACL");
    tokio::time::timeout(std::time::Duration::from_secs(5), server.connect())
        .await
        .expect("connect must complete")
        .expect("connect succeeds");
    drop(server);
    drop(_client);
    // Emulate the server accept loop: always keep one free listening
    // instance (each probe consumes one; the loop recreates it).
    let loop_name = name.clone();
    let listener = tokio::spawn(async move {
        loop {
            let next = ServerOptions::new()
                .first_pipe_instance(false)
                .create(&loop_name)
                .expect("loop instance");
            let _ = next.connect().await;
        }
    });
    assert!(
        super::security::probe_live_pipe_server(&name).await,
        "served name must probe live"
    );
    // A second claim for the same name fails instead of splitting
    // clients across two runtimes.
    let rival = super::security::claim_pipe_instance(&name).await;
    assert!(
        matches!(rival, Err(ref error) if error.kind() == std::io::ErrorKind::AddrInUse),
        "second claim must fail with AddrInUse: {rival:?}"
    );
    listener.abort();
    let _ = listener.await;
    // With every handle closed the name is gone: the probe reports
    // idle and the name is claimable again (crash recovery).
    assert!(
        !super::security::probe_live_pipe_server(&name).await,
        "released name must probe idle"
    );
    let _reclaimed = super::security::claim_pipe_instance(&name)
        .await
        .expect("released name is claimable again");
}
