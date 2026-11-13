use super::*;
use std::{fs, time::Duration};

#[test]
fn identity_survives_restart_and_corrupt_storage_is_not_replaced() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("peer/key");
    let first = load_or_create_key(&path).unwrap();
    assert_eq!(first.public(), load_or_create_key(&path).unwrap().public());
    fs::write(&path, b"invalid").unwrap();
    assert!(matches!(load_or_create_key(&path), Err(Error::Identity)));
    assert_eq!(fs::read(path).unwrap(), b"invalid");
}

#[cfg(unix)]
#[test]
fn identity_rejects_shared_files_and_symbolic_links() {
    use std::os::unix::{fs::symlink, fs::PermissionsExt};
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("key");
    load_or_create_key(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(load_or_create_key(&path), Err(Error::Identity)));
    let link = home.path().join("link");
    symlink(path, &link).unwrap();
    assert!(matches!(load_or_create_key(&link), Err(Error::Identity)));
}

pub(super) async fn pair() -> (PeerEndpoint, PeerEndpoint, PeerConnection, PeerConnection) {
    let host = PeerEndpoint::bind(SecretKey::generate(), &Config::default())
        .await
        .unwrap();
    let client = PeerEndpoint::bind(SecretKey::generate(), &Config::default())
        .await
        .unwrap();
    let (accepted, connected) = tokio::join!(host.accept(), client.connect(host.address()));
    let accepted = accepted.unwrap();
    let connected = connected.unwrap();
    assert_eq!(accepted.remote_id(), client.id());
    assert_eq!(connected.remote_id(), host.id());
    (host, client, accepted, connected)
}

#[tokio::test]
async fn direct_headless_frames_need_no_backend_or_relay() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (host, client, accepted, connected) = pair().await;
        let mut sending = connected.open().await.unwrap();
        sending.send(Frame::Text("command".into())).await.unwrap();
        let mut receiving = accepted.accept().await.unwrap();
        assert!(matches!(receiving.recv().await.unwrap(), Frame::Text(s) if s == "command"));
        receiving.send(Frame::Text("receipt".into())).await.unwrap();
        assert!(matches!(sending.recv().await.unwrap(), Frame::Text(s) if s == "receipt"));
        // No Cloud call exists on the data path; the same authenticated channel
        // continues to carry messages after the initial handshake.
        sending.send(Frame::Ping).await.unwrap();
        assert!(matches!(receiving.recv().await.unwrap(), Frame::Ping));
        host.close().await;
        client.close().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn invalid_relay_configuration_fails_before_network_use() {
    for relay in ["http://relay.example", "not a URL"] {
        assert!(matches!(
            PeerEndpoint::bind(
                SecretKey::generate(),
                &Config {
                    relays: vec![relay.into()]
                }
            )
            .await,
            Err(Error::Configuration)
        ));
    }
}

#[tokio::test]
async fn control_connection_limits_streams_before_application_allocation() {
    let (host, client, accepted, connected) = pair().await;
    let mut first = connected.open().await.unwrap();
    first.send(Frame::Ping).await.unwrap();
    let mut receiving = accepted.accept().await.unwrap();
    assert!(matches!(receiving.recv().await.unwrap(), Frame::Ping));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), connected.open())
            .await
            .is_err()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), connected.connection.open_uni())
            .await
            .is_err()
    );
    receiving.send(Frame::Pong).await.unwrap();
    assert!(matches!(first.recv().await.unwrap(), Frame::Pong));
    host.close().await;
    client.close().await;
}

#[test]
fn reopening_an_approved_identity_never_replaces_a_missing_file() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("missing.key");
    assert!(load_key(&path).is_err());
    assert!(!path.exists());
}
