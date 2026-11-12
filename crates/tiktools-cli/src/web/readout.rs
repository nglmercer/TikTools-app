use super::*;

pub(super) fn pairing_file(port: u16) -> PathBuf {
    tiktools_core::paths::AppPaths::from_environment()
        .temp
        .join(format!("tiktools-web-pairing-{port}.json"))
}

pub(super) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

pub(super) fn write_pairing_file(path: &Path, snapshot: &PairingSnapshot) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let result = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut file = options.open(&tmp)?;
        file.write_all(
            serde_json::to_vec(snapshot)
                .map_err(std::io::Error::other)?
                .as_slice(),
        )?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub(super) fn publish_pairing_code(path: &Path, code: &str) {
    let snapshot = PairingSnapshot {
        code: code.to_owned(),
        issued_at_unix: now_unix(),
        expires_in_secs: PAIRING_TTL_SECS,
        pid: std::process::id(),
    };
    if let Err(error) = write_pairing_file(path, &snapshot) {
        eprintln!("warning: could not publish pairing code for `web --print-code`: {error}");
    }
}

pub(super) fn load_pairing_snapshot(path: &Path) -> Result<PairingSnapshot, String> {
    let bytes = std::fs::read(path).map_err(|_| "no active pairing code".to_string())?;
    let snapshot: PairingSnapshot =
        serde_json::from_slice(&bytes).map_err(|_| "no active pairing code".to_string())?;
    if snapshot.code.len() != 6 || !snapshot.code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("no active pairing code".to_string());
    }
    Ok(snapshot)
}

pub(super) fn snapshot_remaining(snapshot: &PairingSnapshot, now: u64) -> Option<u64> {
    snapshot
        .issued_at_unix
        .saturating_add(snapshot.expires_in_secs)
        .checked_sub(now)
        .filter(|remaining| *remaining > 0)
}

fn server_listening(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(500),
    )
    .is_ok()
}

/// Readout for a separate CLI process. Fails closed when the file is missing,
/// malformed, expired, or the adapter is no longer listening.
pub(super) fn current_pairing_code_at(path: &Path, port: u16) -> Result<(String, u64), String> {
    let snapshot = load_pairing_snapshot(path)?;
    let remaining = snapshot_remaining(&snapshot, now_unix())
        .ok_or_else(|| "pairing code expired".to_string())?;
    if !server_listening(port) {
        return Err("local browser adapter is not running".to_string());
    }
    Ok((snapshot.code, remaining))
}

pub(super) fn current_pairing_code(port: u16) -> Result<(String, u64), String> {
    current_pairing_code_at(&pairing_file(port), port)
}
