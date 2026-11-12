//! Canonical plugin-root containment, extension probing and file URL handling.
use super::*;

pub(super) fn contained(shared: &HostShared, path: &Path, spec: &str) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|_| format!("Cannot find module `{spec}` ({})", path.display()))?;
    if !canonical.starts_with(&shared.root) {
        return Err(format!("module `{spec}` escapes the plugin directory"));
    }
    Ok(canonical)
}

/// Resolves a path to a file: exact file, extension probing, then a
/// directory (`package.json` main, then `index`).
pub(super) fn resolve_path(base: &Path) -> Result<PathBuf, String> {
    if base.is_file() {
        return Ok(base.to_owned());
    }
    for ext in EXTENSIONS {
        let mut probe = base.as_os_str().to_owned();
        probe.push(format!(".{ext}"));
        let probe = PathBuf::from(probe);
        if probe.is_file() {
            return Ok(probe);
        }
    }
    if base.is_dir() {
        return resolve_directory(base);
    }
    Err(format!("Cannot find module `{}`", base.display()))
}

pub(super) fn resolve_directory(dir: &Path) -> Result<PathBuf, String> {
    if let Some(package) = read_package_json(&dir.join("package.json")) {
        if let Some(main) = package.get("main").and_then(serde_json::Value::as_str) {
            // `main` pointing at its own directory would recurse forever.
            let target = dir.join(main);
            if target != dir {
                if let Ok(found) = resolve_path(&target) {
                    return Ok(found);
                }
            }
        }
    }
    for ext in EXTENSIONS {
        let index = dir.join(format!("index.{ext}"));
        if index.is_file() {
            return Ok(index);
        }
    }
    Err(format!("Cannot find module `{}`", dir.display()))
}

pub(super) fn read_package_json(path: &Path) -> Option<serde_json::Value> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub(super) fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' | b':' => {
                url.push(byte as char);
            }
            other => url.push_str(&format!("%{other:02X}")),
        }
    }
    url
}

pub(super) fn path_from_url_or_path(value: &str) -> PathBuf {
    let Some(rest) = value.strip_prefix("file://") else {
        return PathBuf::from(value);
    };
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 3 <= bytes.len() {
            let decoded = std::str::from_utf8(&bytes[index + 1..index + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            if let Some(byte) = decoded {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    PathBuf::from(String::from_utf8_lossy(&out).into_owned())
}

pub(super) fn base_dir_of(shared: &HostShared, base: &str) -> PathBuf {
    let path = Path::new(base);
    if path.is_absolute() {
        path.parent()
            .map_or_else(|| shared.root.clone(), Path::to_owned)
    } else {
        shared.root.clone()
    }
}
