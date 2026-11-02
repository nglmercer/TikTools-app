use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use tiktools_plugin_package::PackageArchive;

use crate::{
    args::{print_json, Args, CommandResult},
    pack_cmd::pack_directory,
    validate::{load_manifest, manifest_path},
};

const USAGE: &str = "\
tiktools-plugin dev stage [dir] --into <dev-plugins-dir> [--entry <built-entry>]
tiktools-plugin dev watch [dir] --into <dev-plugins-dir> [--interval-ms 1000] [--once]";
const IGNORED: [&str; 6] = [
    "target",
    "node_modules",
    "dist",
    "dist-plugin",
    ".git",
    "ui/dist",
];

pub(crate) fn run(raw: &[String]) -> CommandResult {
    let Some((verb, rest)) = raw.split_first() else {
        println!("{USAGE}");
        return Ok(());
    };
    let args = Args::parse(rest);
    let manifest = manifest_path(args.positional(0));
    let into = PathBuf::from(args.required("--into", USAGE)?);
    match verb.as_str() {
        "stage" => {
            let staged = stage(&manifest, &into, args.path("--entry"))?;
            print_json(&json!({ "staged": staged.display().to_string() }));
            Ok(())
        }
        "watch" => watch(&manifest, &into, &args),
        other => Err(format!("unknown dev verb `{other}`\n{USAGE}").into()),
    }
}

pub(crate) fn stage(
    manifest: &Path,
    into: &Path,
    entry: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let plugin = load_manifest(manifest)?;
    let scratch = into.join(format!(".devkit-{}-{}", std::process::id(), unique()));
    let packed = pack_directory(manifest, entry, &scratch, None);
    let result = packed.and_then(|output| {
        let extract = scratch.join("extract");
        fs::create_dir_all(&extract)?;
        let extracted = PackageArchive::open(&output.path)?.extract(&extract)?;
        let destination = into.join(&plugin.id);
        if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        fs::rename(&extracted.root, &destination)?;
        Ok(destination)
    });
    let _ = fs::remove_dir_all(&scratch);
    result
}

fn watch(manifest: &Path, into: &Path, args: &Args) -> CommandResult {
    let directory = manifest
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let interval = Duration::from_millis(
        args.flag("--interval-ms")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1000),
    );
    let mut last = None;
    loop {
        let stamp = newest_source(&directory)?;
        if last != Some(stamp) {
            last = Some(stamp);
            match rebuild(&directory).and_then(|()| stage(manifest, into, None)) {
                Ok(staged) => {
                    print_json(&json!({ "staged": staged.display().to_string(), "at": unique() }))
                }
                Err(error) => eprintln!("rebuild failed: {error}"),
            }
            if args.has("--once") {
                return Ok(());
            }
        }
        std::thread::sleep(interval);
    }
}

fn rebuild(directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = if directory.join("Cargo.toml").is_file() {
        let mut command = Command::new("cargo");
        command.arg("build");
        command
    } else if directory.join("package.json").is_file() {
        let mut command = Command::new("bun");
        command.args(["run", "build"]);
        command
    } else {
        return Ok(());
    };
    let status = command.current_dir(directory).status()?;
    if !status.success() {
        return Err(format!("build exited with {status}").into());
    }
    let manifest = load_manifest(&directory.join("plugin.json"))?;
    if directory.join("Cargo.toml").is_file() && !directory.join(&manifest.entry).is_file() {
        let built = directory
            .join("target/debug")
            .join(Path::new(&manifest.entry).file_name().unwrap_or_default());
        for candidate in [built.clone(), built.with_extension("exe")] {
            if candidate.is_file() {
                let destination = directory.join(&manifest.entry);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&candidate, destination)?;
                break;
            }
        }
    }
    Ok(())
}

fn newest_source(directory: &Path) -> Result<u128, std::io::Error> {
    let mut newest = 0;
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path
                .strip_prefix(directory)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if IGNORED.iter().any(|ignored| relative == *ignored) {
                continue;
            }
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                pending.push(path);
            } else if let Ok(modified) = metadata.modified() {
                let stamp = modified
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_millis())
                    .unwrap_or_default();
                newest = newest.max(stamp);
            }
        }
    }
    Ok(newest)
}

fn unique() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}
