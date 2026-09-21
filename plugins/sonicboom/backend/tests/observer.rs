//! Observer integration: the real backend binary receives a framed
//! `live.ui-event` chat delivery and POSTs speech to the configured
//! server. A std-only TCP stub stands in for the SonicBoom server so no
//! test-only HTTP dependency leaks into the package.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;

fn frame(value: &serde_json::Value) -> Vec<u8> {
    let body = serde_json::to_vec(value).unwrap();
    let mut out = (body.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(&body);
    out
}

fn read_frame(stdout: &mut std::process::ChildStdout) -> serde_json::Value {
    let mut len = [0_u8; 4];
    stdout.read_exact(&mut len).unwrap();
    let len = u32::from_le_bytes(len) as usize;
    let mut body = vec![0_u8; len];
    stdout.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// Minimal HTTP/1.1 stub: answers `GET /v1/voices` and records
/// `POST /api/tts/play` requests, then reports them over the channel.
fn stub_server(listener: TcpListener, report: mpsc::Sender<(String, String)>) {
    for stream in listener.incoming().take(4) {
        let Ok(mut stream) = stream else { break };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut raw = Vec::new();
        let mut chunk = [0_u8; 4096];
        // Read until the header terminator, then the declared body.
        while let Ok(read) = stream.read(&mut chunk) {
            if read == 0 {
                break;
            }
            raw.extend_from_slice(&chunk[..read]);
            if raw.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let text = String::from_utf8_lossy(&raw).into_owned();
        let Some(header_end) = text.find("\r\n\r\n") else {
            break;
        };
        let head = &text[..header_end];
        let mut lines = head.lines();
        let request_line = lines.next().unwrap_or_default().to_owned();
        let content_length: usize = lines
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.trim().eq_ignore_ascii_case("content-length") {
                    value.trim().parse().ok()
                } else {
                    None
                }
            })
            .next()
            .unwrap_or(0);
        let mut body = text
            .as_bytes()
            .get(header_end + 4..)
            .unwrap_or_default()
            .to_vec();
        while body.len() < content_length {
            let Ok(read) = stream.read(&mut chunk) else {
                break;
            };
            if read == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..read]);
        }
        if request_line.starts_with("GET /v1/voices") {
            let payload = r#"[{"id":"M1","name":"M1"}]"#;
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                )
                .as_bytes(),
            );
        } else if request_line.starts_with("POST /api/tts/play") {
            let _ = report.send((request_line, String::from_utf8_lossy(&body).into_owned()));
            let payload = r#"{"ok":true}"#;
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                )
                .as_bytes(),
            );
        } else {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    }
}

#[test]
fn chat_event_speaks_through_the_configured_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (report, received) = mpsc::channel();
    let server = std::thread::spawn(move || stub_server(listener, report));

    let root = std::env::temp_dir().join(format!("sonicboom-observer-test-{}", std::process::id()));
    let plugin_dir = root.join("sonicboom.server");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("settings.json"),
        serde_json::json!({
            "serverUrl": format!("http://127.0.0.1:{port}"),
            "apiToken": "",
            "tts": {
                "enabled": true,
                "allowAllUsers": true,
                "commentMode": "any",
                "defaultVoice": "M1",
                "language": "en",
            },
        })
        .to_string(),
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_sonicboom-backend");
    let mut child = std::process::Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("TIKTOOLS_PLUGIN_ID", "sonicboom.server")
        .env("TIKTOOLS_PLUGIN_DATA_DIR", &root)
        .env("TIKTOOLS_PLUGIN_DIRECTORY", &plugin_dir)
        .spawn()
        .unwrap();

    let request = serde_json::json!({
        "protocolVersion": 1,
        "id": "observer-1",
        "method": "call",
        "payload": {
            "type": "event",
            "event": {
                "topic": "live.ui-event",
                "data": {
                    "event": {
                        "kind": "chat",
                        "author": "ada",
                        "nickname": "Ada",
                        "text": "hello world",
                    },
                },
            },
        },
    });
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&frame(&request))
        .unwrap();
    let response = read_frame(child.stdout.as_mut().unwrap());
    assert_eq!(response["ok"], true);
    assert_eq!(response["id"], "observer-1");

    let (request_line, body) = received
        .recv_timeout(Duration::from_secs(20))
        .expect("backend must POST speech to the server");
    assert!(
        request_line.contains("voice=M1") && request_line.contains("lang=en"),
        "speech URL carries voice/lang: {request_line}"
    );
    assert_eq!(body, "hello world");

    // A replay of the same line inside the dedup window must not speak.
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&frame(&request))
        .unwrap();
    let response = read_frame(child.stdout.as_mut().unwrap());
    assert_eq!(response["ok"], true);
    assert!(
        received.recv_timeout(Duration::from_millis(500)).is_err(),
        "replayed chat must not speak twice"
    );

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&root);
    drop(server);
}
