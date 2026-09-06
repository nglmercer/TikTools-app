//! Build a checksummed plugin archive without requiring a JavaScript toolchain.

use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use tiktools_plugin_api::manifest::{is_safe_relative_path, PluginManifest};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

const USAGE: &str =
    "Usage: tiktools-plugin-pack --manifest <plugin.json> --entry <built-entry> --output <plugin.plugin> [--target <plugin-target>]";

const SUPPORTED_PLUGIN_TARGETS: [&str; 6] = [
    "win32-x64-msvc",
    "win32-arm64-msvc",
    "linux-x64-gnu",
    "linux-arm64-gnu",
    "darwin-x64-darwin",
    "darwin-arm64-darwin",
];

struct Options {
    manifest: PathBuf,
    entry: PathBuf,
    output: PathBuf,
    target: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = match parse_args()? {
        Some(options) => options,
        None => {
            println!("{USAGE}");
            return Ok(());
        }
    };
    package(options)
}

fn parse_args() -> Result<Option<Options>, Box<dyn Error>> {
    let mut manifest = None;
    let mut entry = None;
    let mut output = None;
    let mut target: Option<String> = None;
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--help" || argument == "-h" {
            return Ok(None);
        }
        if argument == "--target" {
            let value = args
                .next()
                .ok_or_else(|| invalid(format!("{argument} requires a value\n{USAGE}")))?;
            target = Some(value);
            continue;
        }
        if let Some(value) = argument.strip_prefix("--target=") {
            target = Some(value.to_owned());
            continue;
        }
        let slot = match argument.as_str() {
            "--manifest" => &mut manifest,
            "--entry" => &mut entry,
            "--output" => &mut output,
            value => return Err(invalid(format!("unknown argument {value}\n{USAGE}"))),
        };
        let value = args
            .next()
            .ok_or_else(|| invalid(format!("{argument} requires a value\n{USAGE}")))?;
        *slot = Some(PathBuf::from(value));
    }
    let target = target
        .map(|value| validate_plugin_target(&value))
        .transpose()?;
    Ok(Some(Options {
        manifest: manifest.ok_or_else(|| invalid(format!("--manifest is required\n{USAGE}")))?,
        entry: entry.ok_or_else(|| invalid(format!("--entry is required\n{USAGE}")))?,
        output: output.ok_or_else(|| invalid(format!("--output is required\n{USAGE}")))?,
        target,
    }))
}

fn validate_plugin_target(value: &str) -> Result<String, Box<dyn Error>> {
    let normalized = value.trim().to_owned();
    if normalized.is_empty() || normalized.len() > 64 || normalized.chars().any(char::is_whitespace)
    {
        return Err(invalid(format!(
            "unsupported --target {value}; expected one of {}",
            SUPPORTED_PLUGIN_TARGETS.join(", ")
        )));
    }
    if !SUPPORTED_PLUGIN_TARGETS.contains(&normalized.as_str()) {
        return Err(invalid(format!(
            "unsupported --target {value}; expected one of {}",
            SUPPORTED_PLUGIN_TARGETS.join(", ")
        )));
    }
    Ok(normalized)
}

fn package(options: Options) -> Result<(), Box<dyn Error>> {
    if options
        .output
        .extension()
        .and_then(|extension| extension.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("plugin"))
    {
        return Err(invalid("output must use the .plugin extension"));
    }

    let manifest_path = fs::canonicalize(&options.manifest)?;
    let package_directory = manifest_path
        .parent()
        .ok_or_else(|| invalid("manifest has no parent directory"))?;
    let manifest_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    let manifest = PluginManifest::from_value(manifest_value.clone())?;
    manifest.validate_compatibility()?;

    let entry_metadata = fs::symlink_metadata(&options.entry)?;
    if !entry_metadata.is_file() || entry_metadata.file_type().is_symlink() {
        return Err(invalid("entry must be a regular, non-symlink file"));
    }
    let entry_bytes = fs::read(&options.entry)?;
    let staged_entry = staged_entry_name(&manifest.entry, options.target.as_deref());

    let mut files = BTreeMap::new();
    let mut manifest_value = manifest_value;
    let packaged_object = manifest_value
        .as_object_mut()
        .ok_or_else(|| invalid("manifest must be a JSON object"))?;
    packaged_object.insert(
        "entry".to_owned(),
        serde_json::Value::String(staged_entry.clone()),
    );
    if let Some(target) = options.target.as_deref() {
        enforce_target_rules(&manifest, target)?;
        packaged_object.insert(
            "targets".to_owned(),
            serde_json::Value::Array(vec![serde_json::Value::String(target.to_owned())]),
        );
    }
    let mut manifest_bytes = serde_json::to_vec_pretty(&manifest_value)?;
    manifest_bytes.push(b'\n');
    insert_file(&mut files, "plugin.json", manifest_bytes)?;
    insert_file(&mut files, &staged_entry, entry_bytes)?;

    for directory in ["assets", "dist", "locales"] {
        let source = package_directory.join(directory);
        if source.exists() {
            collect_directory(&source, directory, &mut files)?;
        }
    }

    let checksums = files
        .iter()
        .map(|(path, bytes)| (path.clone(), sha256(bytes)))
        .collect::<BTreeMap<_, _>>();
    let mut checksums_bytes = serde_json::to_vec_pretty(&checksums)?;
    checksums_bytes.push(b'\n');
    insert_file(&mut files, "checksums.json", checksums_bytes)?;

    let output = absolutize(&options.output)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = output.with_extension("plugin.tmp");
    let _ = fs::remove_file(&temporary);
    write_archive(&temporary, &manifest.id, &files)?;
    if output.exists() {
        fs::remove_file(&output)?;
    }
    fs::rename(&temporary, &output)?;
    println!("Created {}", output.display());
    Ok(())
}

fn collect_directory(
    directory: &Path,
    archive_prefix: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(invalid(format!(
                "asset tree contains a symlink: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            let relative = path
                .strip_prefix(directory)
                .map_err(|error| invalid(error.to_string()))?;
            let prefix = format!(
                "{}/{}",
                archive_prefix,
                relative.to_string_lossy().replace('\\', "/")
            );
            collect_directory(&path, &prefix, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(directory)
                .map_err(|error| invalid(error.to_string()))?;
            let archive_path = format!(
                "{}/{}",
                archive_prefix,
                relative.to_string_lossy().replace('\\', "/")
            );
            insert_file(files, &archive_path, fs::read(path)?)?;
        } else {
            return Err(invalid(format!(
                "unsupported asset entry: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn insert_file(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    bytes: Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    if !is_safe_relative_path(path) {
        return Err(invalid(format!("unsafe package path: {path}")));
    }
    if files.insert(path.to_owned(), bytes).is_some() {
        return Err(invalid(format!("duplicate package path: {path}")));
    }
    Ok(())
}

fn write_archive(
    path: &Path,
    plugin_id: &str,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (relative, bytes) in files {
        writer.start_file(format!("{plugin_id}/{relative}"), options)?;
        writer.write_all(bytes)?;
    }
    writer.finish()?;
    Ok(())
}

fn staged_entry_name(entry: &str, target: Option<&str>) -> String {
    // The executable suffix derives from the requested packaged target, not
    // the packager's own build platform, so cross-target builds keep `.exe`.
    let wants_exe = match target {
        Some(value) => value.starts_with("win32-"),
        None => cfg!(target_os = "windows"),
    };
    if wants_exe && !entry.to_ascii_lowercase().ends_with(".exe") {
        format!("{entry}.exe")
    } else {
        entry.to_owned()
    }
}

fn enforce_target_rules(manifest: &PluginManifest, _target: &str) -> Result<(), Box<dyn Error>> {
    use tiktools_plugin_api::PluginRuntimeKind;
    match manifest.runtime {
        PluginRuntimeKind::Native | PluginRuntimeKind::Process => Ok(()),
        PluginRuntimeKind::Wasm => Err(invalid(
            "--target must not be used for wasm plugins; keep targets empty for portable WASM",
        )),
    }
}

fn absolutize(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn supported_targets_validate() {
        for target in SUPPORTED_PLUGIN_TARGETS {
            assert_eq!(validate_plugin_target(target).unwrap(), target);
        }
        for unsupported in [
            "winx64",
            "windows64",
            "linux64",
            "macarm",
            "i686-pc-windows-msvc",
            "wasm32-unknown-unknown",
            "x86_64-unknown-linux-musl",
            "",
        ] {
            assert!(
                validate_plugin_target(unsupported).is_err(),
                "{unsupported} should be rejected"
            );
        }
    }

    #[test]
    fn staged_entry_suffix_follows_requested_target() {
        assert_eq!(
            staged_entry_name("plugin", Some("win32-x64-msvc")),
            "plugin.exe"
        );
        assert_eq!(
            staged_entry_name("plugin.exe", Some("win32-x64-msvc")),
            "plugin.exe"
        );
        assert_eq!(staged_entry_name("plugin", Some("linux-x64-gnu")), "plugin");
        assert_eq!(
            staged_entry_name("plugin", Some("darwin-arm64-darwin")),
            "plugin"
        );
    }

    fn test_manifest(runtime: &str) -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": 2,
            "id": "target-test",
            "name": "Target Test",
            "version": "1.0.0",
            "runtime": runtime,
            "entry": "target-test",
            "protocolVersion": 1,
        })
    }

    fn unique_dir(name: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("tiktools-pack-{name}-{suffix}"))
    }

    fn read_archive_entry(archive: &Path, entry: &str) -> Vec<u8> {
        let file = File::open(archive).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut contents = Vec::new();
        archive
            .by_name(entry)
            .unwrap()
            .read_to_end(&mut contents)
            .unwrap();
        contents
    }

    #[test]
    fn package_injects_target_without_touching_source_manifest() {
        let directory = unique_dir("inject");
        fs::create_dir_all(&directory).unwrap();
        let manifest_path = directory.join("plugin.json");
        let source = test_manifest("process");
        fs::write(&manifest_path, serde_json::to_vec_pretty(&source).unwrap()).unwrap();
        let entry_path = directory.join("built-entry");
        fs::write(&entry_path, b"binary").unwrap();
        let output = directory.join("target-test-1.0.0-linux-x64-gnu.plugin");

        package(Options {
            manifest: manifest_path.clone(),
            entry: entry_path,
            output: output.clone(),
            target: Some("linux-x64-gnu".to_owned()),
        })
        .unwrap();

        // Source manifest on disk is unchanged.
        let after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(after, source);

        // Packaged manifest carries exactly the requested target.
        let packaged: serde_json::Value =
            serde_json::from_slice(&read_archive_entry(&output, "target-test/plugin.json"))
                .unwrap();
        assert_eq!(
            packaged.get("targets").unwrap(),
            &serde_json::json!(["linux-x64-gnu"])
        );

        // Checksums cover the modified manifest and the staged entry.
        let checksums: serde_json::Value =
            serde_json::from_slice(&read_archive_entry(&output, "target-test/checksums.json"))
                .unwrap();
        let object = checksums.as_object().unwrap();
        assert!(object.contains_key("plugin.json"));
        assert!(object.contains_key("target-test"));

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn package_rejects_target_for_wasm() {
        let directory = unique_dir("wasm");
        fs::create_dir_all(&directory).unwrap();
        let manifest_path = directory.join("plugin.json");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&test_manifest("wasm")).unwrap(),
        )
        .unwrap();
        let entry_path = directory.join("plugin.wasm");
        fs::write(&entry_path, b"wasm").unwrap();

        let result = package(Options {
            manifest: manifest_path,
            entry: entry_path,
            output: directory.join("out.plugin"),
            target: Some("linux-x64-gnu".to_owned()),
        });
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&directory);
    }
}
