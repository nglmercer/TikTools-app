//! Fixtures shared by the registry test modules.
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tiktools_plugin_package::Sha256Digest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new(name: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tiktools-registry-{name}-{}-{suffix}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::AcqRel)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(crate) fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Minimal static file server: serves a directory tree with `ETag`,
/// `If-None-Match` and `Range` support, and counts the requests it receives.
/// The Cloud registry mock is the same server serving the generation tree.
pub(crate) struct StaticServer {
    pub(crate) base: String,
    pub(crate) root: PathBuf,
    pub(crate) requests: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl StaticServer {
    pub(crate) async fn start(root: PathBuf) -> Self {
        let serve_root = root.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = requests.clone();
        let task = tokio::spawn(async move {
            let root = serve_root;
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let root = root.clone();
                let counter = counter.clone();
                tokio::spawn(async move {
                    let mut buffer = vec![0_u8; 16 * 1024];
                    let Ok(read) = socket.read(&mut buffer).await else {
                        return;
                    };
                    counter.fetch_add(1, Ordering::AcqRel);
                    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .split('?')
                        .next()
                        .unwrap_or("/")
                        .trim_start_matches('/')
                        .to_owned();
                    let header = |name: &str| {
                        request.lines().find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.trim()
                                .eq_ignore_ascii_case(name)
                                .then(|| value.trim().to_owned())
                        })
                    };
                    let response = match fs::read(root.join(&path)) {
                        Ok(body) => {
                            let etag = format!("\"{}\"", Sha256Digest::of_bytes(&body));
                            if header("if-none-match").as_deref() == Some(etag.as_str()) {
                                (304, "Not Modified", Vec::new(), etag, None)
                            } else if let Some(start) = header("range")
                                .and_then(|range| {
                                    range.strip_prefix("bytes=").map(ToOwned::to_owned)
                                })
                                .and_then(|range| range.trim_end_matches('-').parse::<usize>().ok())
                                .filter(|start| *start < body.len())
                            {
                                let total = body.len();
                                (
                                    206,
                                    "Partial Content",
                                    body[start..].to_vec(),
                                    etag,
                                    Some(format!("bytes {start}-{}/{total}", total - 1)),
                                )
                            } else {
                                (200, "OK", body, etag, None)
                            }
                        }
                        Err(_) => (404, "Not Found", b"missing".to_vec(), String::new(), None),
                    };
                    let (status, reason, body, etag, range) = response;
                    let mut head = format!(
                        "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n",
                        body.len()
                    );
                    if !etag.is_empty() {
                        head.push_str(&format!("etag: {etag}\r\naccept-ranges: bytes\r\n"));
                    }
                    if let Some(range) = range {
                        head.push_str(&format!("content-range: {range}\r\n"));
                    }
                    head.push_str("\r\n");
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                });
            }
        });
        Self {
            base,
            root,
            requests,
            task,
        }
    }

    pub(crate) fn stop(self) {
        self.task.abort();
    }
}
