use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use tiktools_plugin_package::{
    KeyRole, KeysDocument, PackageArchive, Sha256Digest, SigningKey, TrustStore, TrustedKey,
};
use tiktools_plugin_registry::{Advisory, AdvisorySeverity, IndexWriter};

use crate::{
    args::{print_json, Args, CommandResult},
    keys::load_trusted,
    validate::publish_issues,
};

const USAGE: &str = "\
tiktools-plugin index init <dir> --name <registry> --root-key <root.key.json> --index-key <index.key.json> [--delegate <publisher.pub.json>]... [--description text] [--api url]
tiktools-plugin index delegate <dir> --root-key <root.key.json> --delegate <publisher.pub.json>...
tiktools-plugin index add <dir> <file.plugin>... --index-key <key.json> [--channel stable|beta] [--base-url url]
tiktools-plugin index yank <dir> <id@version> --index-key <key.json> [--undo]
tiktools-plugin index advise <dir> <id@version> --index-key <key.json> --severity low|moderate|high|critical --message text [--url url] | --clear
tiktools-plugin index revoke <dir> <sha256> --index-key <key.json>
tiktools-plugin index rebuild <dir> --index-key <key.json>";

const PUBLISH_USAGE: &str = "tiktools-plugin publish <file.plugin>... --registry <dir> --index-key <key.json> [--channel stable|beta] [--base-url url] [--legacy-id]";

pub(crate) fn index(raw: &[String]) -> CommandResult {
    let Some((verb, rest)) = raw.split_first() else {
        println!("{USAGE}");
        return Ok(());
    };
    let args = Args::parse(rest);
    let directory = PathBuf::from(args.positional(0).ok_or(USAGE)?);
    match verb.as_str() {
        "init" => {
            let root_key = SigningKey::load(&PathBuf::from(args.required("--root-key", USAGE)?))?;
            let index_key = SigningKey::load(&PathBuf::from(args.required("--index-key", USAGE)?))?;
            let mut delegations = vec![TrustedKey::new(index_key.public_key(), KeyRole::Index)];
            for path in args.flags("--delegate") {
                delegations.push(load_trusted(Path::new(&path))?);
            }
            let name = args.required("--name", USAGE)?;
            IndexWriter::open(&directory, &name, Some(index_key)).init(
                &root_key,
                delegations,
                args.flag("--description").map(ToOwned::to_owned),
                args.flag("--api").map(ToOwned::to_owned),
            )?;
            print_json(&json!({
                "registry": directory.display().to_string(),
                "rootFingerprint": root_key.public_key().fingerprint()?.to_string(),
                "next": "serve this directory over HTTPS and share the root fingerprint with users",
            }));
        }
        "delegate" => {
            let root_key = SigningKey::load(&PathBuf::from(args.required("--root-key", USAGE)?))?;
            let keys_path = directory.join("keys.json");
            let mut document = KeysDocument::parse(&fs::read(&keys_path)?)?;
            for path in args.flags("--delegate") {
                let key = load_trusted(Path::new(&path))?;
                document
                    .keys
                    .retain(|existing| existing.key.key_id != key.key.key_id);
                document.keys.push(key);
            }
            let bytes = document.to_bytes();
            fs::write(&keys_path, &bytes)?;
            fs::write(
                directory.join("keys.json.sig"),
                root_key.sign_detached(&bytes).to_bytes(),
            )?;
            print_json(&json!({ "keys": document.keys.len() }));
        }
        "add" => {
            let writer = writer(&directory, &args)?;
            let mut added = Vec::new();
            for archive in args.rest(1) {
                added.push(writer.add(
                    Path::new(archive),
                    args.flag("--channel").unwrap_or("stable"),
                    args.flag("--base-url"),
                )?);
            }
            print_json(&json!({ "added": added }));
        }
        "yank" => {
            let (id, version) = id_version(args.positional(1))?;
            writer(&directory, &args)?.yank(&id, &version, !args.has("--undo"))?;
            print_json(&json!({ "id": id, "version": version, "yanked": !args.has("--undo") }));
        }
        "advise" => {
            let (id, version) = id_version(args.positional(1))?;
            let advisory = if args.has("--clear") {
                None
            } else {
                Some(Advisory {
                    severity: match args.required("--severity", USAGE)?.as_str() {
                        "low" => AdvisorySeverity::Low,
                        "moderate" => AdvisorySeverity::Moderate,
                        "high" => AdvisorySeverity::High,
                        "critical" => AdvisorySeverity::Critical,
                        other => return Err(format!("unknown severity `{other}`").into()),
                    },
                    message: args.required("--message", USAGE)?,
                    url: args.flag("--url").map(ToOwned::to_owned),
                })
            };
            writer(&directory, &args)?.advise(&id, &version, advisory)?;
            print_json(&json!({ "id": id, "version": version }));
        }
        "revoke" => {
            let digest = args
                .positional(1)
                .and_then(Sha256Digest::parse)
                .ok_or("revoke needs a sha256 digest")?;
            writer(&directory, &args)?.revoke_digest(&digest)?;
            print_json(&json!({ "revoked": digest.to_string() }));
        }
        "rebuild" => {
            writer(&directory, &args)?.rebuild()?;
            print_json(&json!({ "rebuilt": directory.display().to_string() }));
        }
        other => return Err(format!("unknown index verb `{other}`\n{USAGE}").into()),
    }
    Ok(())
}

pub(crate) fn publish(raw: &[String]) -> CommandResult {
    let args = Args::parse(raw);
    let directory = PathBuf::from(args.required("--registry", PUBLISH_USAGE)?);
    let archives = args.rest(0);
    if archives.is_empty() {
        return Err(PUBLISH_USAGE.into());
    }
    let keys = KeysDocument::parse(&fs::read(directory.join("keys.json"))?)?;
    let mut store = TrustStore::new();
    store.extend_delegations(&keys, "registry");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    for archive in archives {
        let inspection = PackageArchive::open(archive)?.inspect()?;
        let issues = publish_issues(&inspection.manifest, Path::new(""), args.has("--legacy-id"))
            .into_iter()
            .filter(|issue| !issue.contains("changelog"))
            .collect::<Vec<_>>();
        if !issues.is_empty() {
            return Err(format!(
                "{archive} is not ready to publish:\n- {}",
                issues.join("\n- ")
            )
            .into());
        }
        let (Some(signature), Some(checksums)) = (&inspection.signature, &inspection.checksums)
        else {
            return Err(format!(
                "{archive} must be signed (`tiktools-plugin sign`) before publishing"
            )
            .into());
        };
        store
            .verify_package(signature, checksums, &inspection.manifest.id, now)
            .map_err(|error| {
                format!("{archive}: {error}; delegate the publisher key with `index delegate`")
            })?;
    }
    let mut forwarded = vec!["add".to_owned(), directory.display().to_string()];
    forwarded.extend(archives.iter().cloned());
    for flag in ["--index-key", "--channel", "--base-url"] {
        if let Some(value) = args.flag(flag) {
            forwarded.push(flag.to_owned());
            forwarded.push(value.to_owned());
        }
    }
    index(&forwarded)
}

fn writer(directory: &Path, args: &Args) -> Result<IndexWriter, Box<dyn std::error::Error>> {
    let key = SigningKey::load(&PathBuf::from(args.required("--index-key", USAGE)?))?;
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("config.json"))?)?;
    let name = config
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or("config.json has no registry name; run `index init` first")?
        .to_owned();
    Ok(IndexWriter::open(directory, name, Some(key)))
}

fn id_version(value: Option<&str>) -> Result<(String, String), String> {
    value
        .and_then(|value| value.split_once('@'))
        .map(|(id, version)| (id.to_owned(), version.to_owned()))
        .ok_or_else(|| format!("expected <id@version>\n{USAGE}"))
}
