//! Host platform vocabulary, native package pins and exact host-binary selection. Foreign
//! targets and ambiguous matches never become guessed selections.

use super::{
    MAX_NATIVE_LIB_BINARY_LEN, MAX_NATIVE_LIB_REPO_LEN, MAX_NATIVE_LIB_TAG_LEN,
    MAX_NATIVE_LIB_VERSION_LEN, MAX_NATIVE_PACKAGE_LEN,
};

pub fn current_platform() -> String {
    match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        platform => platform,
    }
    .to_owned()
}

/// Node.js architecture vocabulary (`x64`, `arm64`, `ia32`): the same
/// names napi-rs artifact filenames use.
pub fn current_arch() -> String {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        architecture => architecture,
    }
    .to_owned()
}

/// Whether this binary links musl libc. Compile-time detection is the
/// reliable determination here: a process cannot change its libc at
/// runtime, so the libc that will `dlopen` a `.node` file is fixed when
/// TikTools itself is built.
pub fn is_musl() -> bool {
    cfg!(target_env = "musl")
}

pub fn current_target() -> String {
    let platform = current_platform();
    let arch = current_arch();
    let abi = match std::env::consts::OS {
        "windows" => "msvc",
        "linux" => {
            if is_musl() {
                "musl"
            } else {
                "gnu"
            }
        }
        "macos" => "darwin",
        other => other,
    };
    format!("{platform}-{arch}-{abi}")
}

/// The host target in napi-rs `platformArchABI` vocabulary
/// (`linux-x64-gnu`, `win32-x64-msvc`, `darwin-arm64`, ...). This is the
/// canonical infix host binary selection matches on: it encodes the real
/// platform, architecture, and Linux libc, including the un-suffixed
/// darwin triples.
pub fn current_napi_target() -> String {
    let platform = current_platform();
    let arch = current_arch();
    match std::env::consts::OS {
        "macos" => format!("{platform}-{arch}"),
        _ => current_target(),
    }
}

/// Canonical napi-rs target triples (`{binary}.{target}.node`) the
/// native-library providers know. `--target` validation and lockfile
/// pinning accept exactly these: a target outside this list is a
/// configuration error, never a guessed asset name.
pub const NAPI_TARGETS: &[&str] = &[
    "win32-x64-msvc",
    "win32-ia32-msvc",
    "win32-arm64-msvc",
    "darwin-x64",
    "darwin-arm64",
    "linux-x64-gnu",
    "linux-x64-musl",
    "linux-arm64-gnu",
    "linux-arm64-musl",
    "linux-arm-gnueabihf",
];

/// Whether `target` is a known napi-rs target triple.
pub fn is_known_napi_target(target: &str) -> bool {
    NAPI_TARGETS.contains(&target)
}

/// A bare package specifier guests can require (`rdev-node`,
/// `@scope/pkg`). Mirrors the host module loader's constraints — dot
/// segments, absolute paths, and URL-like requests are never valid package
/// names — so a declared alias is always requirable.
pub fn is_valid_native_package_name(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_NATIVE_PACKAGE_LEN
        || value.starts_with('.')
        || value.starts_with('/')
        || value.starts_with('#')
        || value.contains('\\')
        || value.contains(':')
        || value.contains('\0')
        || value.chars().any(char::is_whitespace)
    {
        return false;
    }
    let parts: Vec<_> = value.split('/').collect();
    // The alias names the package root, never a subpath: unscoped names
    // carry no slash, scoped names carry exactly one.
    let package_end = if value.starts_with('@') { 2 } else { 1 };
    if parts.len() != package_end
        || parts.iter().any(|part| part.is_empty())
        || parts.iter().any(|part| matches!(*part, "." | ".."))
    {
        return false;
    }
    if value.starts_with('@') && parts[0].len() <= 1 {
        return false;
    }
    true
}

/// An exact version pin (`1.0.1`, optionally with `-prerelease` or
/// `+build` suffixes). Ranges (`^1.0.0`, `~1.0`, `*`, `latest`) are
/// never valid: declarative native libraries resolve to exactly one
/// immutable artifact.
pub fn is_valid_native_lib_version(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_NATIVE_LIB_VERSION_LEN {
        return false;
    }
    let core = value.split(['-', '+']).next().unwrap_or_default();
    let mut parts = core.split('.');
    let valid_core = matches!((parts.next(), parts.next(), parts.next(), parts.next()), (
        Some(major),
        Some(minor),
        Some(patch),
        None,
    ) if !major.is_empty()
        && !minor.is_empty()
        && !patch.is_empty()
        && major.bytes().all(|byte| byte.is_ascii_digit())
        && minor.bytes().all(|byte| byte.is_ascii_digit())
        && patch.bytes().all(|byte| byte.is_ascii_digit()));
    if !valid_core {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

/// A `owner/name` repository slug. Both segments are non-empty and carry
/// no whitespace, slashes, or URL-significant characters, so the slug
/// interpolates safely into a release-download URL path.
pub fn is_valid_native_lib_repo(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_NATIVE_LIB_REPO_LEN {
        return false;
    }
    let mut parts = value.split('/');
    let valid = matches!((parts.next(), parts.next(), parts.next()), (
        Some(owner),
        Some(name),
        None,
    ) if !owner.is_empty() && !name.is_empty());
    if !valid {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
}

/// A release tag (`v1.0.1`). Non-empty, slash-free, and limited to URL
/// path-safe characters so it interpolates into a download URL.
pub fn is_valid_native_lib_tag(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_NATIVE_LIB_TAG_LEN {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+'))
}

/// A napi binary stem (`node-rdev`) used to build per-target asset
/// names (`{stem}.{target}.node`). Same discipline as tags: URL-safe,
/// slash-free, non-empty.
pub fn is_valid_native_lib_binary(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_NATIVE_LIB_BINARY_LEN {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// Failures selecting the host binary from a declared native package
/// root. Selection is exact: foreign targets are ignored, and anything
/// but one match fails the load instead of guessing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NativeBinarySelectError {
    #[error("native package `{root}` has no binary for {target}")]
    NoHostBinary { root: String, target: String },
    #[error("native package `{root}` has {count} binaries for {target}; expected exactly one")]
    MultipleHostBinaries {
        root: String,
        target: String,
        count: usize,
    },
    #[error("native package root is not a directory: {0}")]
    NotADirectory(String),
    #[error("cannot inspect native package root: {0}")]
    Unreadable(String),
}

/// Selects the exact `.node` binary for this host from a declared native
/// package root.
///
/// napi-rs artifacts are named `<binary>.<platformArchABI>.node`
/// (`node-rdev.linux-x64-gnu.node`, `node-rdev.darwin-arm64.node`, ...),
/// so the host suffix comes from [`current_napi_target`], which already
/// encodes the real platform, architecture, and Linux libc. Foreign
/// targets never match and the same-platform libc twin never matches;
/// zero or several matches fail instead of guessing. Only top-level
/// regular files ending in `.node` are considered.
pub fn select_host_native_binary(
    package_root: &std::path::Path,
) -> Result<std::path::PathBuf, NativeBinarySelectError> {
    if !package_root.is_dir() {
        return Err(NativeBinarySelectError::NotADirectory(
            package_root.display().to_string(),
        ));
    }
    let target = current_napi_target();
    let suffix = format!(".{target}.node");
    let entries = std::fs::read_dir(package_root)
        .map_err(|_| NativeBinarySelectError::Unreadable(package_root.display().to_string()))?;
    let mut matches = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|_| NativeBinarySelectError::Unreadable(package_root.display().to_string()))?;
        if !entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.len() > suffix.len() && name.ends_with(suffix.as_str()) {
            matches.push(entry.path());
        }
    }
    matches.sort();
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(NativeBinarySelectError::NoHostBinary {
            root: package_root.display().to_string(),
            target,
        }),
        count => Err(NativeBinarySelectError::MultipleHostBinaries {
            root: package_root.display().to_string(),
            target,
            count,
        }),
    }
}
