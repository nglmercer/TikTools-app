//! Package entry points, subpaths and conditional exports.
use super::*;

pub(super) fn split_package(spec: &str) -> (&str, &str) {
    let name_end = if spec.starts_with('@') {
        spec.match_indices('/')
            .nth(1)
            .map_or(spec.len(), |(index, _)| index)
    } else {
        spec.find('/').unwrap_or(spec.len())
    };
    let (name, rest) = spec.split_at(name_end);
    (name, rest.trim_start_matches('/'))
}

pub(super) fn resolve_package(
    shared: &HostShared,
    from_dir: &Path,
    spec: &str,
    mode: Mode,
) -> Result<PathBuf, String> {
    let (name, subpath) = split_package(spec);
    if name.is_empty() || name.contains("..") || name.contains('\\') {
        return Err(format!("invalid module specifier `{spec}`"));
    }
    for dir in from_dir.ancestors() {
        if !dir.starts_with(&shared.root) {
            break;
        }
        let package_dir = dir.join("node_modules").join(name);
        if !package_dir.is_dir() {
            continue;
        }
        return resolve_in_package(&package_dir, subpath, mode, spec);
    }
    Err(format!("Cannot find module `{spec}`"))
}

fn resolve_in_package(
    package_dir: &Path,
    subpath: &str,
    mode: Mode,
    spec: &str,
) -> Result<PathBuf, String> {
    let package = read_package_json(&package_dir.join("package.json"));
    if let Some(exports) = package.as_ref().and_then(|json| json.get("exports")) {
        let key = if subpath.is_empty() {
            ".".to_owned()
        } else {
            format!("./{subpath}")
        };
        let target = exports_target(exports, &key, mode)
            .ok_or_else(|| format!("module `{spec}` is not exported by its package"))?;
        return resolve_path(&package_dir.join(target.trim_start_matches("./")));
    }
    if !subpath.is_empty() {
        return resolve_path(&package_dir.join(subpath));
    }
    if let Some(package) = &package {
        let field = match mode {
            Mode::Import => package
                .get("module")
                .and_then(serde_json::Value::as_str)
                .or_else(|| package.get("main").and_then(serde_json::Value::as_str)),
            Mode::Require => package.get("main").and_then(serde_json::Value::as_str),
        };
        if let Some(main) = field {
            if let Ok(found) = resolve_path(&package_dir.join(main)) {
                return Ok(found);
            }
        }
    }
    resolve_directory(package_dir)
}

fn condition_matches(key: &str, mode: Mode) -> bool {
    match key {
        "default" | "node" => true,
        "import" | "module" | "module-sync" => mode == Mode::Import,
        "require" => mode == Mode::Require,
        _ => false,
    }
}

fn condition_target(value: &serde_json::Value, mode: Mode) -> Option<String> {
    match value {
        serde_json::Value::String(target) => Some(target.clone()),
        serde_json::Value::Array(items) => {
            items.iter().find_map(|item| condition_target(item, mode))
        }
        serde_json::Value::Object(map) => map
            .iter()
            .filter(|(key, _)| condition_matches(key, mode))
            .find_map(|(_, value)| condition_target(value, mode)),
        _ => None,
    }
}

fn exports_target(exports: &serde_json::Value, key: &str, mode: Mode) -> Option<String> {
    match exports {
        serde_json::Value::String(_) | serde_json::Value::Array(_) => (key == ".")
            .then(|| condition_target(exports, mode))
            .flatten(),
        serde_json::Value::Object(map) => {
            let has_subpaths = map.keys().any(|key| key.starts_with('.'));
            if has_subpaths {
                if let Some(value) = map.get(key) {
                    return condition_target(value, mode);
                }
                // `./features/*` style patterns.
                map.iter().find_map(|(pattern, value)| {
                    let (prefix, suffix) = pattern.split_once('*')?;
                    let middle = key.strip_prefix(prefix)?.strip_suffix(suffix)?;
                    condition_target(value, mode).map(|target| target.replace('*', middle))
                })
            } else if key == "." {
                condition_target(exports, mode)
            } else {
                None
            }
        }
        _ => None,
    }
}
