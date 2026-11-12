//! Bounded source reads and ES module/CommonJS classification.
use super::*;

fn nearest_package_type(shared: &HostShared, path: &Path) -> Option<String> {
    for dir in path.parent()?.ancestors() {
        if !dir.starts_with(&shared.root) {
            break;
        }
        if let Some(package) = read_package_json(&dir.join("package.json")) {
            return package
                .get("type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
        }
    }
    None
}

/// Lightweight ESM sniff: a top-level `import`/`export` statement or
/// `import.meta`. Dynamic `import(...)` alone does not make a file ESM.
pub(super) fn looks_like_esm(source: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        if line.starts_with("import.meta") {
            return true;
        }
        if let Some(rest) = line.strip_prefix("import") {
            return rest.starts_with(|c: char| {
                c.is_whitespace() || c == '{' || c == '*' || c == '"' || c == '\''
            }) && !rest.trim_start().starts_with('(');
        }
        if let Some(rest) = line.strip_prefix("export") {
            return rest.starts_with(|c: char| c.is_whitespace() || c == '{' || c == '*');
        }
        line.contains("import.meta")
    })
}

pub(super) fn is_esm_file(shared: &HostShared, path: &Path, source: &str) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("mjs") => true,
        Some("cjs") => false,
        _ => match nearest_package_type(shared, path).as_deref() {
            Some("module") => true,
            Some("commonjs") => false,
            _ => looks_like_esm(source),
        },
    }
}

pub(super) fn read_source(path: &Path) -> Result<String, String> {
    let length = fs::metadata(path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?
        .len();
    if length > MAX_SOURCE_BYTES {
        return Err(format!(
            "`{}` is larger than the {MAX_SOURCE_BYTES} byte source limit",
            path.display()
        ));
    }
    fs::read_to_string(path).map_err(|error| format!("cannot read `{}`: {error}", path.display()))
}
