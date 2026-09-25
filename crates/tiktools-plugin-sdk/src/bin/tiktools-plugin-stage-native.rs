//! Stage a native package into a plugin directory.
//!
//! Two modes: local staging from a source checkout (the default), and
//! provider staging from a `nativeLibs` declaration plus its lockfile:
//!
//! ```text
//! tiktools-plugin-stage-native --package <name> --source <dir> --plugin-dir <dir> --root <rel> [--overwrite]
//! tiktools-plugin-stage-native --provider [npm|github|all] --manifest <plugin.json> --plugin-dir <dir>
//!     [--lockfile <path>] [--target <triple>] [--overwrite] [--pin] [--work-dir <dir>]
//!     [--registry <url>] [--github-base <url>]
//! ```
//!
//! `--pin` resolves the declarations against their providers and
//! (re)writes the lockfile without staging; without it, the lockfile is
//! replayed and every library is verified and staged. Both modes print a
//! JSON report. Provider mode needs the `providers` cargo feature.

use std::{env, error::Error, path::PathBuf};

use tiktools_plugin_sdk::native_stage::{stage_native_package, NativeStageRequest};

const USAGE: &str = "Usage:\n  tiktools-plugin-stage-native --package <name> --source <dir> --plugin-dir <dir> --root <rel> [--overwrite]\n  tiktools-plugin-stage-native --provider [npm|github|all] --manifest <plugin.json> --plugin-dir <dir> [--lockfile <path>] [--target <triple>] [--overwrite] [--pin] [--work-dir <dir>] [--registry <url>] [--github-base <url>]";

struct LocalOptions {
    package: String,
    source: PathBuf,
    plugin_dir: PathBuf,
    root: String,
    overwrite: bool,
}

#[cfg_attr(not(feature = "providers"), allow(dead_code))]
struct ProviderOptions {
    provider: String,
    manifest: PathBuf,
    plugin_dir: Option<PathBuf>,
    lockfile: Option<PathBuf>,
    target: Option<String>,
    overwrite: bool,
    pin: bool,
    work_dir: Option<PathBuf>,
    registry: Option<String>,
    github_base: Option<String>,
}

enum Mode {
    Local(LocalOptions),
    Provider(ProviderOptions),
}

fn main() -> Result<(), Box<dyn Error>> {
    let mode = match parse_args()? {
        Some(mode) => mode,
        None => {
            println!("{USAGE}");
            return Ok(());
        }
    };
    match mode {
        Mode::Local(options) => run_local(options),
        Mode::Provider(options) => run_provider(options),
    }
}

fn run_local(options: LocalOptions) -> Result<(), Box<dyn Error>> {
    let report = stage_native_package(&NativeStageRequest {
        package: &options.package,
        source: &options.source,
        plugin_dir: &options.plugin_dir,
        root: &options.root,
        overwrite: options.overwrite,
    })
    .map_err(|error| format!("staging failed: {error}"))?;
    println!(
        "{{\"root\":{},\"files\":{},\"nodeBinaries\":{}}}",
        serde_json_string(&report.root),
        serde_json_string_array(&report.files),
        serde_json_string_array(&report.node_binaries),
    );
    Ok(())
}

#[cfg(feature = "providers")]
fn run_provider(options: ProviderOptions) -> Result<(), Box<dyn Error>> {
    use tiktools_plugin_api::{NativeLibProvider, PluginManifest};
    use tiktools_plugin_sdk::native_providers::{
        fetch_and_stage_native_libs, pin_native_lib, provider_client, read_lockfile,
        resolve_target, write_lockfile, FetchAndStage, NativeLibsLockfile, ProviderEndpoints,
        LOCKFILE_VERSION,
    };

    let filter = match options.provider.as_str() {
        "all" => None,
        "npm" => Some(NativeLibProvider::Npm),
        "github" => Some(NativeLibProvider::Github),
        other => {
            return Err(format!("unknown provider {other}; expected npm, github, or all").into())
        }
    };
    let manifest_text = std::fs::read_to_string(&options.manifest)
        .map_err(|error| format!("cannot read {}: {error}", options.manifest.display()))?;
    let manifest = PluginManifest::from_json_str(&manifest_text)
        .map_err(|error| format!("invalid manifest {}: {error}", options.manifest.display()))?;
    let lockfile_path = options.lockfile.clone().unwrap_or_else(|| {
        options
            .manifest
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(tiktools_plugin_sdk::native_providers::NATIVE_LIBS_LOCKFILE)
    });
    let mut endpoints = ProviderEndpoints::officials();
    if let Some(registry) = options.registry {
        endpoints.npm_registry = registry;
    }
    if let Some(base) = options.github_base {
        endpoints.github_base = base;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("cannot start fetch runtime: {error}"))?;
    runtime
        .block_on(async {
            let client = provider_client()?;
            if options.pin {
                let mut lockfile = if lockfile_path.is_file() {
                    read_lockfile(&lockfile_path)?
                } else {
                    NativeLibsLockfile {
                        version: LOCKFILE_VERSION,
                        packages: std::collections::BTreeMap::new(),
                    }
                };
                let mut pinned = Vec::new();
                for declaration in manifest
                    .native_libs
                    .iter()
                    .filter(|entry| filter.is_none_or(|provider| entry.provider == provider))
                {
                    let locked = pin_native_lib(&client, &endpoints, declaration).await?;
                    lockfile
                        .packages
                        .insert(declaration.package.clone(), locked);
                    pinned.push(declaration.package.clone());
                }
                write_lockfile(&lockfile_path, &lockfile)?;
                println!(
                    "{}",
                    serde_json::json!({
                        "lockfile": lockfile_path.display().to_string(),
                        "packages": pinned,
                    })
                );
                return Ok::<(), tiktools_plugin_sdk::native_providers::NativeProviderError>(());
            }
            let plugin_dir = options
                .plugin_dir
                .ok_or_else(|| "--plugin-dir is required unless --pin is given".to_owned())
                .map_err(|message| {
                    tiktools_plugin_sdk::native_providers::NativeProviderError::Io {
                        path: String::new(),
                        message,
                    }
                })?;
            let lockfile = read_lockfile(&lockfile_path)?;
            let target = resolve_target(options.target.as_deref())?;
            let work_dir = options.work_dir.unwrap_or_else(std::env::temp_dir);
            let reports = fetch_and_stage_native_libs(&FetchAndStage {
                client: &client,
                endpoints: &endpoints,
                manifest: &manifest,
                lockfile: &lockfile,
                target: &target,
                plugin_dir: &plugin_dir,
                work_dir: &work_dir,
                overwrite: options.overwrite,
                provider_filter: filter,
            })
            .await?;
            println!(
                "{}",
                serde_json::json!({ "target": target, "reports": reports })
            );
            Ok::<(), tiktools_plugin_sdk::native_providers::NativeProviderError>(())
        })
        .map_err(|error| format!("provider staging failed: {error}"))?;
    Ok(())
}

#[cfg(not(feature = "providers"))]
fn run_provider(_options: ProviderOptions) -> Result<(), Box<dyn Error>> {
    Err("provider mode needs the providers feature (rebuild with --features providers)".into())
}

fn parse_args() -> Result<Option<Mode>, Box<dyn Error>> {
    let mut package = None;
    let mut source = None;
    let mut plugin_dir = None;
    let mut root = None;
    let mut overwrite = false;
    let mut provider = None;
    let mut manifest = None;
    let mut lockfile = None;
    let mut target = None;
    let mut pin = false;
    let mut work_dir = None;
    let mut registry = None;
    let mut github_base = None;
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--help" || argument == "-h" {
            return Ok(None);
        }
        if argument == "--overwrite" {
            overwrite = true;
            continue;
        }
        if argument == "--pin" {
            pin = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {argument}\n{USAGE}"))?;
        match argument.as_str() {
            "--package" => package = Some(value),
            "--source" => source = Some(PathBuf::from(value)),
            "--plugin-dir" => plugin_dir = Some(PathBuf::from(value)),
            "--root" => root = Some(value),
            "--provider" => provider = Some(value),
            "--manifest" => manifest = Some(PathBuf::from(value)),
            "--lockfile" => lockfile = Some(PathBuf::from(value)),
            "--target" => target = Some(value),
            "--work-dir" => work_dir = Some(PathBuf::from(value)),
            "--registry" => registry = Some(value),
            "--github-base" => github_base = Some(value),
            _ => return Err(format!("unknown argument {argument}\n{USAGE}").into()),
        }
    }
    if provider.is_some() || pin {
        if source.is_some() || root.is_some() || package.is_some() {
            return Err(format!(
                "--provider/--pin cannot be combined with --package/--source/--root\n{USAGE}"
            )
            .into());
        }
        let Some(manifest) = manifest else {
            return Err(format!("provider mode needs --manifest\n{USAGE}").into());
        };
        return Ok(Some(Mode::Provider(ProviderOptions {
            provider: provider.unwrap_or_else(|| "all".to_owned()),
            manifest,
            plugin_dir,
            lockfile,
            target,
            overwrite,
            pin,
            work_dir,
            registry,
            github_base,
        })));
    }
    match (package, source, plugin_dir, root) {
        (Some(package), Some(source), Some(plugin_dir), Some(root)) => {
            if manifest.is_some() || lockfile.is_some() || target.is_some() {
                return Err(
                    format!("local mode takes no --manifest/--lockfile/--target\n{USAGE}").into(),
                );
            }
            Ok(Some(Mode::Local(LocalOptions {
                package,
                source,
                plugin_dir,
                root,
                overwrite,
            })))
        }
        _ => Err(USAGE.into()),
    }
}

fn serde_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for c in value.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}

fn serde_json_string_array(values: &[String]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|value| serde_json_string(value))
        .collect();
    format!("[{}]", items.join(","))
}
