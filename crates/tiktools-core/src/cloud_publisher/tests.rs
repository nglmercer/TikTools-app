use super::*;

#[test]
fn backend_failures_keep_a_short_single_line_message_and_the_status_mapping() {
    let reply = |status: StatusCode, body: &str| Reply {
        status,
        body: body.as_bytes().to_vec(),
    };
    let long = format!(r#"{{"error":"{}"}}"#, "x".repeat(5000));
    let error = backend_failure(&reply(StatusCode::BAD_REQUEST, &long));
    assert_eq!(error.code, "invalid_params");
    assert_eq!(error.message.chars().count(), MAX_BACKEND_MESSAGE_CHARS);
    let multiline = backend_failure(&reply(
        StatusCode::NOT_FOUND,
        r#"{"error":"gone\nsecond line\u0007"}"#,
    ));
    assert_eq!(multiline.code, "not_found");
    assert_eq!(multiline.message, "gonesecond line");
    // A large or malformed body falls back to the status, never to its text.
    let fallback = backend_failure(&reply(StatusCode::BAD_GATEWAY, ""));
    assert_eq!(
        fallback.message,
        "cloud rejected the request (502 Bad Gateway)"
    );
    assert_eq!(
        backend_failure(&reply(StatusCode::CONFLICT, "<html>")).code,
        "conflict"
    );
}

#[test]
fn typed_replies_reject_an_empty_204_and_unreadable_bodies() {
    let reply = |status: StatusCode, body: &str| Reply {
        status,
        body: body.as_bytes().to_vec(),
    };
    let ok: Result<serde_json::Value, _> = decode_reply(reply(StatusCode::OK, r#"{"a":1}"#));
    assert_eq!(ok.unwrap(), serde_json::json!({"a": 1}));
    assert!(decode_reply::<serde_json::Value>(reply(StatusCode::NO_CONTENT, "")).is_err());
    assert!(decode_reply::<serde_json::Value>(reply(StatusCode::OK, "")).is_err());
    assert!(decode_reply::<serde_json::Value>(reply(StatusCode::OK, "nope")).is_err());
}

#[test]
fn upload_url_requires_tls_or_loopback_and_has_no_embedded_credentials() {
    for good in [
        "https://store.example/object?token=secret",
        "http://127.0.0.1:5000/x",
        "http://[::1]:5000/x",
    ] {
        assert!(safe_upload_url(good).is_some());
    }
    for bad in [
        "http://example.com/x",
        "http://localhost/x",
        "https://user:pass@example.com/x",
        "https://example.com/x#frag",
        "file:///tmp/file",
    ] {
        assert!(safe_upload_url(bad).is_none());
    }
}

#[cfg(unix)]
#[test]
fn enrollment_reuses_publisher_bound_key_without_overwriting() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let dir = std::env::temp_dir().join(format!("publisher-key-test-{}", uuid_for_test()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("keys").join("key.json");
    let first =
        load_or_create_signing_key(&path, KeyId::parse("key-1").unwrap(), "tiktools").unwrap();
    let before = std::fs::read(&path).unwrap();
    let second =
        load_or_create_signing_key(&path, KeyId::parse("key-1").unwrap(), "tiktools").unwrap();
    assert_eq!(first.public_key(), second.public_key());
    assert_eq!(second.publisher(), Some("tiktools"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(load_or_create_signing_key(&path, KeyId::parse("key-1").unwrap(), "other").is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(load_or_create_signing_key(&path, KeyId::parse("key-1").unwrap(), "tiktools").is_err());
    let link = dir.join("keys/link.json");
    symlink(&path, &link).unwrap();
    assert!(load_or_create_signing_key(&link, KeyId::parse("key-1").unwrap(), "tiktools").is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
fn uuid_for_test() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

#[test]
fn valid_namespace_mirrors_backend_rules() {
    assert!(valid_namespace("acme"));
    assert!(valid_namespace("tiktools-2"));
    assert!(!valid_namespace("A"));
    assert!(!valid_namespace("a"));
    assert!(!valid_namespace("-acme"));
    assert!(!valid_namespace("acme-"));
    assert!(!valid_namespace("acme_plugin"));
    assert!(!valid_namespace("acme.plugin"));
}

#[test]
fn proof_message_matches_backend_domain() {
    assert_eq!(
        proof_message("pub", "chal", "nonce", "key-1", "cGsta2V5"),
        "tiktools-cloud-publisher-key-v1\npub\nchal\nnonce\nkey-1\ncGsta2V5"
    );
}

#[test]
fn base64_matches_standard_vectors() {
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    assert_eq!(
        base64_encode(&[0xff; 32]),
        "//////////////////////////////////////////8="
    );
}

#[tokio::test]
async fn full_publisher_flow_against_mock_backend() {
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    use secrecy::SecretString;
    use tiktools_cloud_auth::{CloudAuth, MemoryTokenStore, TokenStore};

    type RequestLog = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;
    #[derive(Clone)]
    struct Log(RequestLog);

    fn json(status: u16, body: &str) -> Vec<u8> {
        let reason = match status {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            409 => "Conflict",
            _ => "Error",
        };
        format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    fn route(
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
        log: &Log,
        origin: &str,
    ) -> Vec<u8> {
        let body_json: serde_json::Value =
            serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
        log.0
            .lock()
            .unwrap()
            .push((method.to_owned(), path.to_owned(), body.to_vec()));
        if path != "/store/object"
            && headers.get("authorization").map(String::as_str) != Some("Bearer test-token")
        {
            return json(401, r#"{"error":"unauthorized"}"#);
        }
        let release = |status: &str| {
            format!(
                r#"{{"id":"rel-1","publisherId":"pub-1","uploadId":"upload-1","publisherKeyId":"rec-1","pluginId":"acme.tts","version":"1.2.3","target":"any","status":"{status}","yanked":false}}"#
            )
        };
        match (method, path) {
            ("GET", "/api/v1/native/me") => json(200, r#"{"id":"user-1"}"#),
            ("GET", "/api/v1/native/me/export") => json(
                200,
                r#"{"schemaVersion":1,"profile":{"id":"user-1","email":"user@example.com","name":"Test User"},"cloudRoles":[],"publisherMemberships":[],"publisherKeysRequested":[],"releasesSubmitted":[],"uploadsRequested":[],"reviewsRecorded":[],"publicationRequests":[],"ownershipTransfers":[],"auditEventsByActor":[],"yanksRecorded":[],"advisoriesRecorded":[],"sync":{"schemaVersion":1,"cursor":0,"records":[]}}"#,
            ),
            ("DELETE", "/api/v1/publishers/pub-1") => {
                b"HTTP/1.1 204 No Content\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                    .to_vec()
            }
            ("DELETE", "/api/v1/publishers/00000000-0000-4000-8000-0000000000bb") => {
                json(409, r#"{"error":"publisher still has other members"}"#)
            }
            ("GET", "/api/v1/publishers") => json(
                200,
                r#"[{"id":"pub-1","namespace":"acme","status":"approved","role":"owner"}]"#,
            ),
            ("POST", "/api/v1/publishers") => {
                let namespace = body_json
                    .get("namespace")
                    .and_then(|value| value.as_str())
                    .unwrap_or("?");
                json(
                    201,
                    &format!(
                        r#"{{"id":"pub-2","namespace":"{namespace}","status":"pending","role":"owner"}}"#
                    ),
                )
            }
            ("GET", "/api/v1/publishers/pub-1/keys") => json(200, "[]"),
            ("POST", "/api/v1/step-up/password") => {
                if body_json.get("password").and_then(|value| value.as_str())
                    != Some("correct-horse")
                {
                    return json(401, r#"{"error":"bad password"}"#);
                }
                json(200, r#"{"grantId":"grant-1","expiresIn":300}"#)
            }
            ("POST", "/api/v1/publishers/pub-1/keys/challenges") => json(
                200,
                r#"{"challengeId":"chal-1","nonce":"n0nce","expiresIn":300}"#,
            ),
            ("POST", "/api/v1/publishers/pub-1/keys") => {
                let key_id = body_json
                    .get("keyId")
                    .and_then(|value| value.as_str())
                    .unwrap_or("?");
                let public_key = body_json
                    .get("publicKey")
                    .and_then(|value| value.as_str())
                    .unwrap_or("?");
                json(
                    200,
                    &format!(
                        r#"{{"id":"rec-1","keyId":"{key_id}","publicKey":"{public_key}","fingerprint":"fp-1","status":"pending"}}"#
                    ),
                )
            }
            ("POST", "/api/v1/plugins/uploads") => {
                if body_json.get("size").and_then(|value| value.as_i64()) != Some(4)
                    || body_json.get("sha256").and_then(|value| value.as_str())
                        != Some("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08")
                {
                    return json(400, r#"{"error":"bad reservation"}"#);
                }
                json(
                    200,
                    &format!(
                        r#"{{"id":"upload-1","url":"{origin}/store/object","headers":{{}},"expiresIn":299}}"#
                    ),
                )
            }
            ("PUT", "/store/object") => json(200, "{}"),
            ("POST", "/api/v1/plugins/uploads/upload-1/complete") => {
                json(200, r#"{"id":"upload-1","status":"complete"}"#)
            }
            ("POST", "/api/v1/plugins/releases") => json(200, &release("submitted")),
            ("GET", "/api/v1/plugins/releases/rel-1") => json(200, &release("submitted")),
            ("POST", "/api/v1/plugins/releases/rel-1/publish") => json(
                200,
                r#"{"jobId":"job-1","status":"requested","processingAvailable":true}"#,
            ),
            _ => json(400, r#"{"error":"unexpected"}"#),
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let log = Log(Arc::new(Mutex::new(Vec::new())));
    let routes = log.clone();
    let serve_origin = origin.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut head = Vec::new();
            let mut byte = [0_u8; 1];
            while head.len() < 65536 {
                if stream.read_exact(&mut byte).is_err() {
                    break;
                }
                head.push(byte[0]);
                if head.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let head = String::from_utf8_lossy(&head).into_owned();
            let mut lines = head.lines();
            let mut parts = lines.next().unwrap_or("").split_whitespace();
            let method = parts.next().unwrap_or("").to_owned();
            let path = parts.next().unwrap_or("/").to_owned();
            let mut headers = HashMap::new();
            for line in lines {
                if line.is_empty() {
                    break;
                }
                if let Some((key, value)) = line.split_once(':') {
                    headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
                }
            }
            let length: usize = headers
                .get("content-length")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let mut body = vec![0_u8; length.min(8 * 1024 * 1024)];
            if length > 0 {
                let _ = stream.read_exact(&mut body);
            }
            let reply = route(&method, &path, &headers, &body, &routes, &serve_origin);
            let _ = stream.write_all(&reply);
        }
    });

    let dir = std::env::temp_dir().join(format!(
        "tt-publisher-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let sidecar = dir.join("cloud-account.json");
    std::fs::write(
        &sidecar,
        format!(r#"{{"origin":"{origin}","account_id":"user-1"}}"#),
    )
    .unwrap();
    let store: Arc<dyn TokenStore> = Arc::new(MemoryTokenStore::new());
    let session =
        cloud_client::Session::connect_secret(&origin, SecretString::from("test-token".to_owned()))
            .await
            .unwrap();
    store.store(&origin, "user-1", &session).unwrap();
    let auth = CloudAuth::new(Some(origin.clone()), sidecar, store);
    let status = auth.restore().await.unwrap();
    assert_eq!(status.email.as_deref(), Some("user@example.com"));

    let data = dir.join("data");
    let paths = crate::paths::AppPaths {
        root: dir.clone(),
        data: data.clone(),
        plugins: dir.join("plugins"),
        plugin_data: dir.join("plugin-data"),
        builtin_plugins: dir.join("builtin"),
        development_plugins: None,
        logs: dir.join("logs"),
        temp: dir.join("temp"),
    };
    let publisher = CloudPublisher::new(&paths);

    let list = publisher.list(&auth).await.unwrap();
    assert_eq!(list.publishers.len(), 1);
    assert_eq!(list.publishers[0].namespace, "acme");

    let created = publisher
        .create(
            &auth,
            PublisherCreateParams {
                namespace: "newns".to_owned(),
            },
        )
        .await
        .unwrap();
    assert_eq!(created.status, "pending");

    let keys = publisher
        .keys(
            &auth,
            PublisherIdParams {
                publisher_id: "acme".to_owned(),
            },
        )
        .await
        .unwrap();
    assert!(keys.keys.is_empty());

    let enrolled = publisher
        .enroll(
            &auth,
            PublisherEnrollParams {
                publisher_id: "acme".to_owned(),
                password: "correct-horse".to_owned(),
                key_id: Some("acme-1".to_owned()),
            },
        )
        .await
        .unwrap();
    assert_eq!(enrolled.record.status, "pending");
    assert_eq!(enrolled.record.key_id, "acme-1");
    let key_bytes = std::fs::read(&enrolled.key_file).unwrap();
    let loaded = SigningKey::from_json(&key_bytes).unwrap();
    assert_eq!(loaded.key_id().as_str(), "acme-1");
    assert_eq!(loaded.publisher(), Some("acme"));
    let retried = publisher
        .enroll(
            &auth,
            PublisherEnrollParams {
                publisher_id: "acme".into(),
                password: "correct-horse".into(),
                key_id: Some("acme-1".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(&retried.key_file).unwrap(), key_bytes);
    assert_eq!(retried.record.public_key, enrolled.record.public_key);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&enrolled.key_file)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    let artifact = dir.join("acme.tts.plugin");
    std::fs::write(&artifact, b"test").unwrap();
    let uploaded = publisher
        .upload(
            &auth,
            PublisherUploadParams {
                publisher_id: "acme".to_owned(),
                path: artifact.to_string_lossy().into_owned(),
            },
        )
        .await
        .unwrap();
    assert_eq!(uploaded.upload_id, "upload-1");
    assert_eq!(uploaded.status, "complete");

    let submitted = publisher
        .submit(
            &auth,
            PublisherSubmitParams {
                upload_id: "upload-1".to_owned(),
                publisher_key_id: "rec-1".to_owned(),
                plugin_id: "acme.tts".to_owned(),
                version: "1.2.3".to_owned(),
                target: String::new(),
            },
        )
        .await
        .unwrap();
    assert_eq!(submitted.release.target, "any");
    assert!(!submitted.release.yanked);

    let fetched = publisher
        .release(
            &auth,
            PublisherReleaseParams {
                id: "rel-1".to_owned(),
            },
        )
        .await
        .unwrap();
    assert_eq!(fetched.release.id, "rel-1");

    let publication = publisher
        .publish(
            &auth,
            PublisherReleaseParams {
                id: "rel-1".to_owned(),
            },
        )
        .await
        .unwrap();
    assert_eq!(publication.job_id, "job-1");

    // Deleting by namespace resolves to the publisher id before DELETE.
    let deleted = publisher
        .delete(
            &auth,
            PublisherIdParams {
                publisher_id: "acme".to_owned(),
            },
        )
        .await
        .unwrap();
    assert_eq!(deleted.publisher_id, "pub-1");
    assert!(deleted.deleted);

    // A conflict from the backend is returned as an error and nothing is removed.
    let busy = publisher
        .delete(
            &auth,
            PublisherIdParams {
                publisher_id: "00000000-0000-4000-8000-0000000000bb".to_owned(),
            },
        )
        .await;
    assert!(
        busy.is_err(),
        "a publisher with other members must not be deleted"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn uuid_detection_accepts_only_canonical_ids() {
    assert!(is_uuid("38af3e2c-e851-4d56-9f71-75c5437d5c34"));
    assert!(!is_uuid("acme"));
    assert!(!is_uuid(""));
    assert!(!is_uuid("38af3e2c-e851-4d56-9f71-75c5437d5c3"));
    assert!(!is_uuid("38af3e2c_e851_4d56_9f71_75c5437d5c34"));
}

#[test]
fn signing_round_trip_matches_wire_format() {
    let key = SigningKey::generate(KeyId::parse("key-1").unwrap(), None).unwrap();
    let raw = key.public_key().raw().unwrap();
    assert_eq!(raw.len(), 32);
    assert_eq!(base64_encode(&raw).len(), 44);
    let proof = proof_message("pub", "chal", "nonce", "key-1", &base64_encode(&raw));
    let signature = key.sign(proof.as_bytes());
    assert_eq!(signature.len(), 64);
    key.public_key()
        .verify(proof.as_bytes(), &signature)
        .unwrap();
}
