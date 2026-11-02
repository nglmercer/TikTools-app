use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::json;
use tiktools_plugin_api::{manifest::PLUGIN_TARGETS, PluginManifest, PluginRuntimeKind};
use tiktools_plugin_package::{pack, sign_archive, PackOutput, PackRequest, SigningKey};

use crate::{
    args::{print_json, Args, CommandResult},
    targets::{executable_suffix, host_target, rust_triple},
    validate::{load_manifest, manifest_path},
};

const PACK_USAGE: &str = "tiktools-plugin pack [dir] [--entry <built-entry>] [--out-dir dist] [--target <plugin-target>] [--sign <key.json>]\ntiktools-plugin pack --manifest <plugin.json> --entry <built-entry> --output <file.plugin> [--target <plugin-target>]";
const BUILD_USAGE: &str = "tiktools-plugin build [dir] [--target <plugin-target> | --all-targets] [--release] [--out-dir dist] [--sign <key.json>]";

pub(crate) fn pack_command(raw: &[String]) -> CommandResult {
    let args = Args::parse(raw);
    if args.has("--help") {
        println!("{PACK_USAGE}");
        return Ok(());
    }
    let key = args
        .path("--sign")
        .map(|path| SigningKey::load(&path))
        .transpose()?;
    let output = if let Some(manifest) = args.path("--manifest") {
        let output = args.required("--output", PACK_USAGE)?;
        pack(&PackRequest {
            manifest,
            entry: args.path("--entry"),
            output: PathBuf::from(output),
            target: args.flag("--target").map(ToOwned::to_owned),
        })?
    } else {
        let manifest_path = manifest_path(args.positional(0));
        pack_directory(
            &manifest_path,
            args.path("--entry"),
            &out_dir(&args, &manifest_path),
            args.flag("--target"),
        )?
    };
    let output = match &key {
        Some(key) => sign_archive(&output.path, key)?,
        None => output,
    };
    print_json(&describe(&output, key.is_some()));
    Ok(())
}

pub(crate) fn build_command(raw: &[String]) -> CommandResult {
    let args = Args::parse(raw);
    if args.has("--help") {
        println!("{BUILD_USAGE}");
        return Ok(());
    }
    let manifest_path = manifest_path(args.positional(0));
    let manifest = load_manifest(&manifest_path)?;
    let directory = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let key = args
        .path("--sign")
        .map(|path| SigningKey::load(&path))
        .transpose()?;
    let targets: Vec<Option<String>> = if args.has("--all-targets") {
        PLUGIN_TARGETS
            .iter()
            .map(|target| Some((*target).to_owned()))
            .collect()
    } else {
        vec![args.flag("--target").map(ToOwned::to_owned)]
    };
    let release = args.has("--release");
    let out = out_dir(&args, &manifest_path);
    let mut built = Vec::new();
    for target in targets {
        let entry = build_entry(&directory, &manifest, target.as_deref(), release)?;
        let packaged_target = target.clone().filter(|_| {
            matches!(
                manifest.runtime,
                PluginRuntimeKind::Native | PluginRuntimeKind::Process
            ) || !manifest.native_addons.is_empty()
        });
        let output = pack_directory(&manifest_path, entry, &out, packaged_target.as_deref())?;
        let output = match &key {
            Some(key) => sign_archive(&output.path, key)?,
            None => output,
        };
        built.push(describe(&output, key.is_some()));
    }
    print_json(&json!({ "packages": built }));
    Ok(())
}

pub(crate) fn pack_directory(
    manifest_path: &Path,
    entry: Option<PathBuf>,
    out_dir: &Path,
    target: Option<&str>,
) -> Result<PackOutput, Box<dyn std::error::Error>> {
    let manifest = load_manifest(manifest_path)?;
    let directory = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let entry = match entry {
        Some(entry) => Some(entry),
        None if manifest.runtime == PluginRuntimeKind::Declarative && manifest.entry.is_empty() => {
            None
        }
        None => Some(existing_entry(&directory, &manifest)?),
    };
    let file_name = format!(
        "{}-{}{}.plugin",
        manifest.id,
        manifest.version,
        target
            .map(|target| format!("-{target}"))
            .unwrap_or_default()
    );
    fs::create_dir_all(out_dir)?;
    Ok(pack(&PackRequest {
        manifest: manifest_path.to_path_buf(),
        entry,
        output: out_dir.join(file_name),
        target: target.map(ToOwned::to_owned),
    })?)
}

fn existing_entry(directory: &Path, manifest: &PluginManifest) -> Result<PathBuf, String> {
    [
        directory.join(&manifest.entry),
        directory.join(format!("{}.exe", manifest.entry)),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .ok_or_else(|| {
        format!(
            "entry `{}` does not exist; build it first or pass --entry",
            manifest.entry
        )
    })
}

fn build_entry(
    directory: &Path,
    manifest: &PluginManifest,
    target: Option<&str>,
    release: bool,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    if directory.join("Cargo.toml").is_file() && manifest.runtime != PluginRuntimeKind::NapiVm {
        return cargo_build(directory, manifest, target, release).map(Some);
    }
    if directory.join("package.json").is_file() {
        run(Command::new("bun")
            .args(["run", "build"])
            .current_dir(directory))?;
        return Ok(Some(existing_entry(directory, manifest)?));
    }
    if manifest.runtime == PluginRuntimeKind::Declarative && manifest.entry.is_empty() {
        return Ok(None);
    }
    Ok(Some(existing_entry(directory, manifest)?))
}

fn cargo_build(
    directory: &Path,
    manifest: &PluginManifest,
    target: Option<&str>,
    release: bool,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut command = Command::new("cargo");
    command.arg("build").current_dir(directory);
    if release {
        command.arg("--release");
    }
    let triple = match target {
        Some(target) => {
            Some(rust_triple(target).ok_or_else(|| format!("unknown target {target}"))?)
        }
        None => None,
    };
    if let Some(triple) = triple {
        command.args(["--target", triple]);
    }
    run(&mut command)?;
    let profile = if release { "release" } else { "debug" };
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| directory.join("target"));
    let output_dir = match triple {
        Some(triple) => target_dir.join(triple).join(profile),
        None => target_dir.join(profile),
    };
    let plugin_target = target.map(ToOwned::to_owned).unwrap_or_else(host_target);
    let file_name = Path::new(&manifest.entry)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(file_name)
        .to_owned();
    let candidates: Vec<PathBuf> = if manifest.runtime == PluginRuntimeKind::Native {
        let library = stem.replace('-', "_");
        vec![
            output_dir.join(format!("{library}.dll")),
            output_dir.join(format!("lib{library}.so")),
            output_dir.join(format!("lib{library}.dylib")),
        ]
    } else {
        let stem = file_name.trim_end_matches(".exe");
        vec![
            output_dir.join(format!("{stem}{}", executable_suffix(&plugin_target))),
            output_dir.join(stem),
        ]
    };
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "cargo built the project but `{stem}` was not found in {}",
                output_dir.display()
            )
            .into()
        })
}

fn run(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let status = command.status()?;
    if !status.success() {
        return Err(format!("{command:?} exited with {status}").into());
    }
    Ok(())
}

fn out_dir(args: &Args, manifest_path: &Path) -> PathBuf {
    args.path("--out-dir").unwrap_or_else(|| {
        manifest_path
            .parent()
            .map(|parent| parent.join("dist-plugin"))
            .unwrap_or_else(|| PathBuf::from("dist-plugin"))
    })
}

pub(crate) fn describe(output: &PackOutput, signed: bool) -> serde_json::Value {
    json!({
        "path": output.path.display().to_string(),
        "id": output.manifest.id,
        "version": output.manifest.version,
        "targets": output.manifest.targets,
        "sha256": output.digest.to_string(),
        "size": output.size,
        "signed": signed,
    })
}
