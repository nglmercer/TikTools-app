//! Stage a local napi-rs native package into a plugin directory.
//!
//! Minimal alternative to `npm install` for TikTools `napi-vm` native
//! addons: pure Rust, no Node.js, no registry, no lifecycle scripts, no
//! shelling out to the `napi` CLI. The caller points at an already-built
//! package directory (a checkout, an extracted artifact set) and the
//! helper validates and copies it to the plugin's declared package root.
//!
//! What this does:
//!
//! - validates the package name and destination root with the same
//!   manifest validators the host enforces at discovery;
//! - requires `package.json` to name the declared package and every
//!   declared JavaScript entrypoint (`main`, `.` export targets, `types`)
//!   to exist; a package declaring none is native-only and needs just its
//!   binaries;
//! - honors the `files` allowlist when present, so build byproducts never
//!   leak into the plugin; `node_modules`, `.git`, and `target` trees are
//!   never traversed, with or without one;
//! - keeps every `.node` binary (no platform filtering) and refuses a
//!   package with none;
//! - refuses symlinks anywhere and verifies the staged tree stays inside
//!   the plugin directory.
//!
//! What this deliberately does not do: fetch from a registry or tarball,
//! run lifecycle scripts, generate loaders (the committed napi-rs output
//! stays the source of truth), hash binaries (the pack step covers
//! integrity through `checksums.json`), or mutate `plugin.json` — the
//! manifest declaration is the native-code authorization boundary and
//! stays author-owned.
//!
//! ```no_run
//! use std::path::Path;
//! use tiktools_plugin_sdk::native_stage::{stage_native_package, NativeStageRequest};
//!
//! let report = stage_native_package(&NativeStageRequest {
//!     package: "rdev-node",
//!     source: Path::new("../rdev-node"),
//!     plugin_dir: Path::new("./my-plugin"),
//!     root: "node_modules/rdev-node",
//!     overwrite: false,
//! })?;
//! println!("staged {} files", report.files.len());
//! # Ok::<(), tiktools_plugin_sdk::native_stage::NativeStageError>(())
//! ```

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tiktools_plugin_api::manifest::{is_safe_relative_path, is_valid_native_package_name};

/// Maximum staged files per package. Real native packages ship a handful
/// of loaders plus one `.node` per platform; the cap only bounds abuse.
const MAX_STAGE_FILES: usize = 4_096;
/// Maximum bytes per staged file. Native binaries are tens of megabytes;
/// anything larger is not a napi-rs artifact.
const MAX_STAGE_FILE_BYTES: u64 = 256 * 1024 * 1024;
/// Longest accepted destination root, mirroring the manifest limit for
/// native paths.
const MAX_STAGE_ROOT_LEN: usize = 512;
/// Directory names never staged without an explicit `files` allowlist: a
/// source checkout carries its own dependencies, VCS data, and build
/// output, none of which belongs in a plugin archive.
const EXCLUDED_DIRECTORIES: [&str; 3] = ["node_modules", ".git", "target"];
/// Export conditions whose `.` targets must exist for the staged package
/// to load under both Node.js and napi-vm guests.
const REQUIRED_EXPORT_CONDITIONS: [&str; 5] = ["import", "require", "default", "node", "types"];

/// One request to stage a native package into a plugin directory.
///
/// `source` is a local package directory holding `package.json` plus the
/// built artifacts; `plugin_dir` is the plugin root being assembled;
/// `root` is the declared package root relative to it
/// (`node_modules/rdev-node`). With `overwrite` unset, a non-empty
/// destination fails instead of being replaced.
#[derive(Debug, Clone)]
pub struct NativeStageRequest<'a> {
    pub package: &'a str,
    pub source: &'a Path,
    pub plugin_dir: &'a Path,
    pub root: &'a str,
    pub overwrite: bool,
}

/// What [`stage_native_package`] copied. Paths are relative to the plugin
/// directory with `/` separators, sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeStageReport {
    pub root: String,
    pub files: Vec<String>,
    pub node_binaries: Vec<String>,
}

/// Failures staging a native package. Every variant names the offending
/// path or value so build tooling can report it without a backtrace.
#[derive(Debug, Error)]
pub enum NativeStageError {
    #[error("invalid native package name: {0}")]
    InvalidPackage(String),

    #[error("invalid native package root: {0}")]
    InvalidRoot(String),

    #[error("plugin directory is not usable: {0}")]
    UnusableDirectory(String),

    #[error("native package root already exists: {0}")]
    AlreadyStaged(String),

    #[error("cannot read package manifest: {0}")]
    Manifest(String),

    #[error("package manifest names `{found}`, expected `{expected}`")]
    PackageNameMismatch { expected: String, found: String },

    #[error("package entrypoint is missing: {0}")]
    MissingEntrypoint(String),

    #[error("package has no native binaries: {0}")]
    NoNativeBinaries(String),

    #[error("symlinks are never staged: {0}")]
    SymlinkRefused(String),

    #[error("destination escapes the plugin directory: {0}")]
    Escape(String),

    #[error("package stages too many files: {0}")]
    TooManyFiles(String),

    #[error("staged file is too large: {0}")]
    FileTooLarge(String),

    #[error("staging I/O failed: {0}")]
    Io(String),
}

impl From<std::io::Error> for NativeStageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// Validates `request.source` and copies it to `plugin_dir/root`.
///
/// See the [module](self) documentation for the exact contract.
pub fn stage_native_package(
    request: &NativeStageRequest,
) -> Result<NativeStageReport, NativeStageError> {
    if !is_valid_native_package_name(request.package) {
        return Err(NativeStageError::InvalidPackage(request.package.to_owned()));
    }
    if request.root.len() > MAX_STAGE_ROOT_LEN || !is_safe_relative_path(request.root) {
        return Err(NativeStageError::InvalidRoot(request.root.to_owned()));
    }
    let plugin_root = fs::canonicalize(request.plugin_dir).map_err(|_| {
        NativeStageError::UnusableDirectory(request.plugin_dir.display().to_string())
    })?;
    if !plugin_root.is_dir() {
        return Err(NativeStageError::UnusableDirectory(
            request.plugin_dir.display().to_string(),
        ));
    }
    let destination = plugin_root.join(request.root);
    prepare_destination(&destination, request.overwrite)?;

    let manifest = read_package_manifest(request.source)?;
    let found = manifest
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if found != request.package {
        return Err(NativeStageError::PackageNameMismatch {
            expected: request.package.to_owned(),
            found: found.to_owned(),
        });
    }
    let entrypoints = required_entrypoints(&manifest)?;
    let allowlist = files_allowlist(&manifest);

    let mut selected = select_source_files(request.source, allowlist.as_ref())?;
    if selected.len() > MAX_STAGE_FILES {
        return Err(NativeStageError::TooManyFiles(
            request.source.display().to_string(),
        ));
    }
    for relative in &selected {
        let bytes = fs::metadata(request.source.join(relative))?.len();
        if bytes > MAX_STAGE_FILE_BYTES {
            return Err(NativeStageError::FileTooLarge(relative.clone()));
        }
    }
    let node_binaries: Vec<String> = selected
        .iter()
        .filter(|relative| relative.ends_with(".node"))
        .cloned()
        .collect();
    if node_binaries.is_empty() {
        return Err(NativeStageError::NoNativeBinaries(
            request.source.display().to_string(),
        ));
    }
    for entrypoint in &entrypoints {
        if !selected.iter().any(|relative| relative == entrypoint) {
            return Err(NativeStageError::MissingEntrypoint(entrypoint.clone()));
        }
    }

    selected.sort();
    fs::create_dir_all(&destination)?;
    let mut files = Vec::with_capacity(selected.len());
    for relative in &selected {
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(request.source.join(relative), &target)?;
        files.push(format!("{}/{}", request.root, relative));
    }
    // The manifest parser guarantees a safe-relative root, so the join
    // cannot escape lexically; the canonical check still runs so a
    // symlinked middle directory can never redirect the copy.
    let canonical = fs::canonicalize(&destination)?;
    if !canonical.starts_with(&plugin_root) {
        return Err(NativeStageError::Escape(destination.display().to_string()));
    }

    Ok(NativeStageReport {
        root: request.root.to_owned(),
        files,
        node_binaries: node_binaries
            .into_iter()
            .map(|relative| format!("{}/{}", request.root, relative))
            .collect(),
    })
}

/// Creates or clears the destination directory. Anything that is not a
/// directory (or a non-empty directory without `overwrite`) fails: staging
/// never merges into an unknown tree.
fn prepare_destination(destination: &Path, overwrite: bool) -> Result<(), NativeStageError> {
    let metadata = match fs::symlink_metadata(destination) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(NativeStageError::Io(error.to_string())),
        Ok(metadata) => metadata,
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(NativeStageError::SymlinkRefused(
            destination.display().to_string(),
        ));
    }
    if !overwrite && fs::read_dir(destination)?.next().is_some() {
        return Err(NativeStageError::AlreadyStaged(
            destination.display().to_string(),
        ));
    }
    if overwrite {
        fs::remove_dir_all(destination)?;
    }
    Ok(())
}

fn read_package_manifest(source: &Path) -> Result<serde_json::Value, NativeStageError> {
    let bytes = fs::read(source.join("package.json"))
        .map_err(|_| NativeStageError::Manifest(source.display().to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| NativeStageError::Manifest(source.display().to_string()))
}

/// Entrypoints the staged tree must contain: `main` when declared,
/// every `.` export target under a known condition, and the declared
/// types file when present. A package declaring no JavaScript entrypoint
/// is native-only (loaded purely through its alias) and needs just its
/// binaries.
fn required_entrypoints(manifest: &serde_json::Value) -> Result<Vec<String>, NativeStageError> {
    let mut entrypoints = BTreeSet::new();
    if let Some(main) = manifest.get("main").and_then(serde_json::Value::as_str) {
        entrypoints.insert(normalize_entrypoint(main)?);
    }
    if let Some(exports) = manifest.get("exports") {
        for target in export_targets(exports) {
            entrypoints.insert(normalize_entrypoint(&target)?);
        }
    }
    for key in ["types", "typings"] {
        if let Some(types) = manifest.get(key).and_then(serde_json::Value::as_str) {
            entrypoints.insert(normalize_entrypoint(types)?);
        }
    }
    Ok(entrypoints.into_iter().collect())
}

/// String targets under the `.` export, one nesting level deep through
/// condition objects and arrays. Unknown conditions are still collected:
/// staging must keep whatever the loader might resolve.
fn export_targets(exports: &serde_json::Value) -> Vec<String> {
    let dot = match exports {
        serde_json::Value::Object(entries) if entries.keys().any(|key| key.starts_with('.')) => {
            entries.get(".")
        }
        other => Some(other),
    };
    let mut targets = Vec::new();
    collect_export_targets(dot, &mut targets);
    return targets;

    fn collect_export_targets(value: Option<&serde_json::Value>, targets: &mut Vec<String>) {
        match value {
            Some(serde_json::Value::String(target)) => targets.push(target.clone()),
            Some(serde_json::Value::Array(entries)) => {
                for entry in entries {
                    collect_export_targets(Some(entry), targets);
                }
            }
            Some(serde_json::Value::Object(conditions)) => {
                for (condition, target) in conditions {
                    if REQUIRED_EXPORT_CONDITIONS.contains(&condition.as_str()) {
                        collect_export_targets(Some(target), targets);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Normalizes a manifest entrypoint (`./index.js`, `index.js`) to the
/// relative path the staged tree must contain. Absolute paths, traversal,
/// and empty values fail: they can never resolve inside the package.
fn normalize_entrypoint(value: &str) -> Result<String, NativeStageError> {
    let trimmed = value.strip_prefix("./").unwrap_or(value);
    if trimmed.is_empty() || !is_safe_relative_path(trimmed) {
        return Err(NativeStageError::MissingEntrypoint(value.to_owned()));
    }
    Ok(trimmed.to_owned())
}

/// The `files` allowlist when the manifest declares one. `package.json`
/// itself is always staged, matching npm behavior.
fn files_allowlist(manifest: &serde_json::Value) -> Option<Vec<String>> {
    let entries = manifest.get("files")?.as_array()?;
    if entries.is_empty() {
        return None;
    }
    let mut allowlist: Vec<String> = entries
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect();
    if !allowlist.iter().any(|entry| entry == "package.json") {
        allowlist.push("package.json".to_owned());
    }
    Some(allowlist)
}

/// Minimal `files` matching: exact relative paths, top-level `*.suffix`
/// patterns, and `directory`/`directory/` prefixes. Anything richer fails
/// closed (matches nothing) rather than guessing npm glob semantics.
fn allowlist_matches(allowlist: &[String], relative: &str) -> bool {
    allowlist.iter().any(|pattern| {
        let pattern = pattern.strip_prefix("./").unwrap_or(pattern);
        if pattern == relative {
            return true;
        }
        if let Some(suffix) = pattern.strip_prefix("*.") {
            return !relative.contains('/')
                && relative.ends_with(suffix)
                && relative.len() > suffix.len();
        }
        let prefix = pattern.strip_suffix('/').unwrap_or(pattern);
        !prefix.is_empty() && (relative == prefix || relative.starts_with(&format!("{prefix}/")))
    })
}

/// Walks the source tree and returns selected relative paths (`/`
/// separators). Symlinks fail the whole stage; dependency, VCS, and
/// build directories are never traversed.
fn select_source_files(
    source: &Path,
    allowlist: Option<&Vec<String>>,
) -> Result<Vec<String>, NativeStageError> {
    let mut selected = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(relative_dir) = stack.pop() {
        let absolute_dir = if relative_dir.as_os_str().is_empty() {
            source.to_path_buf()
        } else {
            source.join(&relative_dir)
        };
        let mut entries: Vec<_> = fs::read_dir(&absolute_dir)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                return Err(NativeStageError::SymlinkRefused(
                    entry.path().display().to_string(),
                ));
            }
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let relative = if relative_dir.as_os_str().is_empty() {
                file_name.clone()
            } else {
                format!(
                    "{}/{file_name}",
                    relative_dir.to_string_lossy().replace('\\', "/")
                )
            };
            if file_type.is_dir() {
                // Never traversed, with or without an allowlist: native
                // packages are flat by construction, and a source checkout
                // carries dependency trees full of symlinks that staging
                // must neither copy nor choke on.
                let excluded = relative
                    .split('/')
                    .any(|segment| EXCLUDED_DIRECTORIES.contains(&segment));
                if !excluded {
                    stack.push(PathBuf::from(relative));
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let wanted = match allowlist {
                Some(allowlist) => allowlist_matches(allowlist, &relative),
                None => true,
            };
            if wanted {
                selected.push(relative);
            }
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Scratch package source under the system temp dir, unique per test.
    fn fixture_source(label: &str) -> PathBuf {
        let id = FIXTURE_COUNTER.fetch_add(1, Ordering::AcqRel);
        let root = std::env::temp_dir().join(format!(
            "tiktools-native-stage-{label}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_source(root: &Path, name: &str, bytes: &[u8]) {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    /// Minimal napi-rs-shaped package: manifest, loaders, types, and two
    /// platform binaries. Callers mutate the manifest for negative tests.
    fn write_package(root: &Path) {
        write_source(
            root,
            "package.json",
            br#"{
  "name": "rdev-node",
  "version": "1.0.1",
  "main": "index.js",
  "types": "./index.d.ts",
  "exports": { ".": { "types": "./index.d.ts", "import": "./index.mjs", "require": "./index.js" } },
  "files": ["index.js", "index.mjs", "index.d.ts", "*.node"]
}"#,
        );
        write_source(root, "index.js", b"module.exports = {};");
        write_source(root, "index.mjs", b"export default {};");
        write_source(root, "index.d.ts", b"export {};");
        write_source(root, "node-rdev.linux-x64-gnu.node", b"fake-linux");
        write_source(root, "node-rdev.win32-x64-msvc.node", b"fake-win32");
    }

    fn stage_request<'a>(
        package: &'a str,
        source: &'a Path,
        plugin_dir: &'a Path,
        root: &'a str,
    ) -> NativeStageRequest<'a> {
        NativeStageRequest {
            package,
            source,
            plugin_dir,
            root,
            overwrite: false,
        }
    }

    #[test]
    fn stages_package_honoring_files_allowlist() {
        let source = fixture_source("happy");
        write_package(&source);
        // Build byproducts a checkout carries but `files` excludes.
        write_source(&source, "src/lib.rs", b"fn main() {}");
        fs::create_dir_all(source.join("node_modules/dep")).unwrap();
        write_source(&source, "node_modules/dep/index.js", b"{}");

        let plugin = fixture_source("happy-plugin");
        let report = stage_native_package(&stage_request(
            "rdev-node",
            &source,
            &plugin,
            "node_modules/rdev-node",
        ))
        .unwrap();
        assert_eq!(report.root, "node_modules/rdev-node");
        assert_eq!(
            report.files,
            vec![
                "node_modules/rdev-node/index.d.ts",
                "node_modules/rdev-node/index.js",
                "node_modules/rdev-node/index.mjs",
                "node_modules/rdev-node/node-rdev.linux-x64-gnu.node",
                "node_modules/rdev-node/node-rdev.win32-x64-msvc.node",
                "node_modules/rdev-node/package.json",
            ]
        );
        assert_eq!(
            report.node_binaries,
            vec![
                "node_modules/rdev-node/node-rdev.linux-x64-gnu.node",
                "node_modules/rdev-node/node-rdev.win32-x64-msvc.node",
            ]
        );
        assert!(plugin.join("node_modules/rdev-node/index.js").is_file());
        assert!(!plugin.join("node_modules/rdev-node/src").exists());
        assert!(!plugin.join("node_modules/rdev-node/node_modules").exists());

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }

    #[test]
    fn checkout_directories_are_never_traversed() {
        // Even with a `files` allowlist, dependency trees are pruned
        // before symlink checks: a source checkout nests symlinks under
        // node_modules that staging must neither copy nor choke on.
        let source = fixture_source("prune");
        write_package(&source);
        fs::create_dir_all(source.join("node_modules/dep")).unwrap();
        write_source(&source, "node_modules/dep/index.js", b"{}");
        #[cfg(unix)]
        std::os::unix::fs::symlink("index.js", source.join("node_modules/dep/link.js")).unwrap();

        let plugin = fixture_source("prune-plugin");
        let report = stage_native_package(&stage_request(
            "rdev-node",
            &source,
            &plugin,
            "node_modules/rdev-node",
        ))
        .unwrap();
        assert!(
            !report
                .files
                .iter()
                .any(|file| file.contains("node_modules/rdev-node/node_modules")),
            "{report:?}"
        );
        assert!(!plugin.join("node_modules/rdev-node/node_modules").exists());

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }

    #[test]
    fn without_files_skips_checkout_directories() {
        let source = fixture_source("no-files");
        write_package(&source);
        let manifest = fs::read_to_string(source.join("package.json")).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        value.as_object_mut().unwrap().remove("files");
        fs::write(
            source.join("package.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        write_source(&source, "stray.txt", b"stray");
        write_source(&source, "target/debug/lib.node", b"nope");
        write_source(&source, ".git/HEAD", b"ref");

        let plugin = fixture_source("no-files-plugin");
        let report = stage_native_package(&stage_request(
            "rdev-node",
            &source,
            &plugin,
            "node_modules/rdev-node",
        ))
        .unwrap();
        assert!(report.files.iter().any(|file| file.ends_with("stray.txt")));
        assert!(!report.files.iter().any(|file| file.contains("target/")));
        assert!(!report.files.iter().any(|file| file.contains(".git/")));

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }

    #[test]
    fn rejects_mismatched_package_name() {
        let source = fixture_source("name");
        write_package(&source);
        let plugin = fixture_source("name-plugin");
        let error = stage_native_package(&stage_request(
            "other-pkg",
            &source,
            &plugin,
            "node_modules/other-pkg",
        ))
        .unwrap_err();
        assert!(
            matches!(error, NativeStageError::PackageNameMismatch { .. }),
            "{error}"
        );

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }

    #[test]
    fn rejects_traversal_and_absolute_roots() {
        let source = fixture_source("roots");
        write_package(&source);
        let plugin = fixture_source("roots-plugin");
        for root in ["../escape", "/absolute", "", "nested/../../escape"] {
            let error = stage_native_package(&stage_request("rdev-node", &source, &plugin, root))
                .unwrap_err();
            assert!(
                matches!(error, NativeStageError::InvalidRoot(_)),
                "{root}: {error}"
            );
        }
        assert!(!plugin.join("escape").exists());

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }

    #[test]
    fn rejects_invalid_package_names() {
        let source = fixture_source("names");
        write_package(&source);
        let plugin = fixture_source("names-plugin");
        for package in ["", "../evil", "@scope", "has space", "a/b/c"] {
            let error = stage_native_package(&stage_request(
                package,
                &source,
                &plugin,
                "node_modules/pkg",
            ))
            .unwrap_err();
            assert!(
                matches!(error, NativeStageError::InvalidPackage(_)),
                "{package}: {error}"
            );
        }

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }

    #[test]
    fn requires_entrypoints_and_binaries() {
        // Missing export target.
        let source = fixture_source("entry");
        write_package(&source);
        fs::remove_file(source.join("index.mjs")).unwrap();
        let plugin = fixture_source("entry-plugin");
        let error = stage_native_package(&stage_request(
            "rdev-node",
            &source,
            &plugin,
            "node_modules/rdev-node",
        ))
        .unwrap_err();
        assert!(
            matches!(error, NativeStageError::MissingEntrypoint(_)),
            "{error}"
        );

        // No .node files at all.
        let bare = fixture_source("nobin");
        write_package(&bare);
        fs::remove_file(bare.join("node-rdev.linux-x64-gnu.node")).unwrap();
        fs::remove_file(bare.join("node-rdev.win32-x64-msvc.node")).unwrap();
        let bare_plugin = fixture_source("nobin-plugin");
        let error = stage_native_package(&stage_request(
            "rdev-node",
            &bare,
            &bare_plugin,
            "node_modules/rdev-node",
        ))
        .unwrap_err();
        assert!(
            matches!(error, NativeStageError::NoNativeBinaries(_)),
            "{error}"
        );

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
        fs::remove_dir_all(&bare).ok();
        fs::remove_dir_all(&bare_plugin).ok();
    }

    #[test]
    fn refuses_symlinks_and_existing_destinations() {
        let source = fixture_source("links");
        write_package(&source);
        #[cfg(unix)]
        std::os::unix::fs::symlink("index.js", source.join("evil.js")).unwrap();
        let plugin = fixture_source("links-plugin");
        #[cfg(unix)]
        {
            let error = stage_native_package(&stage_request(
                "rdev-node",
                &source,
                &plugin,
                "node_modules/rdev-node",
            ))
            .unwrap_err();
            assert!(
                matches!(error, NativeStageError::SymlinkRefused(_)),
                "{error}"
            );
            fs::remove_file(source.join("evil.js")).unwrap();
        }

        stage_native_package(&stage_request(
            "rdev-node",
            &source,
            &plugin,
            "node_modules/rdev-node",
        ))
        .unwrap();
        let error = stage_native_package(&stage_request(
            "rdev-node",
            &source,
            &plugin,
            "node_modules/rdev-node",
        ))
        .unwrap_err();
        assert!(
            matches!(error, NativeStageError::AlreadyStaged(_)),
            "{error}"
        );

        let mut retry = stage_request("rdev-node", &source, &plugin, "node_modules/rdev-node");
        retry.overwrite = true;
        stage_native_package(&retry).unwrap();

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }

    #[test]
    fn native_only_packages_need_no_javascript_entrypoint() {
        let source = fixture_source("native-only");
        write_source(
            &source,
            "package.json",
            br#"{"name": "rdev-node", "version": "1.0.1"}"#,
        );
        write_source(&source, "node-rdev.linux-x64-gnu.node", b"fake-linux");
        write_source(&source, "node-rdev.win32-x64-msvc.node", b"fake-win32");

        let plugin = fixture_source("native-only-plugin");
        let report = stage_native_package(&stage_request(
            "rdev-node",
            &source,
            &plugin,
            "node_modules/rdev-node",
        ))
        .unwrap();
        assert_eq!(report.files.len(), 3);
        assert_eq!(report.node_binaries.len(), 2);

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }

    #[test]
    fn scoped_packages_stage_under_nested_roots() {
        let source = fixture_source("scoped");
        write_package(&source);
        let manifest = fs::read_to_string(source.join("package.json")).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        value.as_object_mut().unwrap().insert(
            "name".to_owned(),
            serde_json::Value::String("@scope/rdev-node".to_owned()),
        );
        fs::write(
            source.join("package.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();

        let plugin = fixture_source("scoped-plugin");
        let report = stage_native_package(&stage_request(
            "@scope/rdev-node",
            &source,
            &plugin,
            "node_modules/@scope/rdev-node",
        ))
        .unwrap();
        assert_eq!(report.files.len(), 6);
        assert!(plugin
            .join("node_modules/@scope/rdev-node/index.js")
            .is_file());

        fs::remove_dir_all(&source).ok();
        fs::remove_dir_all(&plugin).ok();
    }
}
