use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use tiktools_plugin_package::{
    sign_archive, KeyId, KeyRole, PackageArchive, SigningKey, TrustStore, TrustedKey,
};

use crate::args::{print_json, Args, CommandResult};

const KEYGEN_USAGE: &str =
    "tiktools-plugin keygen --id <key-id> [--publisher <name>] [--role root|index|publisher] [--out <dir>]";
const SIGN_USAGE: &str = "tiktools-plugin sign <file.plugin>... --key <key.json>";
const VERIFY_USAGE: &str = "tiktools-plugin verify <file.plugin> [--trust <public-key.json>]...";

pub(crate) fn keygen(raw: &[String]) -> CommandResult {
    let args = Args::parse(raw);
    let id = KeyId::parse(&args.required("--id", KEYGEN_USAGE)?)?;
    let publisher = args.flag("--publisher").map(ToOwned::to_owned);
    let role = match args.flag("--role") {
        Some("root") => KeyRole::Root,
        Some("index") => KeyRole::Index,
        Some("publisher") => KeyRole::Publisher,
        Some(other) => return Err(format!("unknown role `{other}`\n{KEYGEN_USAGE}").into()),
        None if publisher.is_some() => KeyRole::Publisher,
        None => KeyRole::Root,
    };
    if role == KeyRole::Publisher && publisher.is_none() {
        return Err(format!("publisher keys need --publisher\n{KEYGEN_USAGE}").into());
    }
    let key = SigningKey::generate(id.clone(), publisher.clone())?;
    let out = args.path("--out").unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&out)?;
    let private = out.join(format!("{id}.key.json"));
    let public = out.join(format!("{id}.pub.json"));
    if private.exists() && !args.has("--force") {
        return Err(format!("{} exists; pass --force to overwrite", private.display()).into());
    }
    fs::write(&private, key.to_json())?;
    let mut trusted = TrustedKey::new(key.public_key(), role);
    trusted.publisher = publisher;
    let mut public_bytes = serde_json::to_vec_pretty(&trusted)?;
    public_bytes.push(b'\n');
    fs::write(&public, public_bytes)?;
    print_json(&json!({
        "keyId": id.to_string(),
        "role": role,
        "privateKey": private.display().to_string(),
        "publicKey": public.display().to_string(),
        "fingerprint": key.public_key().fingerprint()?.to_string(),
        "warning": "keep the private key file secret and out of version control",
    }));
    Ok(())
}

pub(crate) fn sign(raw: &[String]) -> CommandResult {
    let args = Args::parse(raw);
    let key = SigningKey::load(&PathBuf::from(args.required("--key", SIGN_USAGE)?))?;
    let archives = args.rest(0);
    if archives.is_empty() {
        return Err(SIGN_USAGE.into());
    }
    let mut signed = Vec::new();
    for archive in archives {
        let output = sign_archive(Path::new(archive), &key)?;
        signed.push(crate::pack_cmd::describe(&output, true));
    }
    print_json(&json!({ "signed": signed, "keyId": key.key_id().to_string() }));
    Ok(())
}

pub(crate) fn load_trusted(path: &Path) -> Result<TrustedKey, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)
        .map_err(|error| format!("{}: {error}", path.display()))?)
}

pub(crate) fn verify(raw: &[String]) -> CommandResult {
    let args = Args::parse(raw);
    let path = args.positional(0).ok_or(VERIFY_USAGE)?;
    let archive = PackageArchive::open(path)?;
    let inspection = archive.inspect()?;
    let staging = std::env::temp_dir().join(format!(
        "tiktools-plugin-verify-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    fs::create_dir_all(&staging)?;
    let extracted = archive.extract(&staging);
    let _ = fs::remove_dir_all(&staging);
    extracted?;
    let mut store = TrustStore::new();
    for trusted in args.flags("--trust") {
        store.insert(load_trusted(Path::new(&trusted))?, "local");
    }
    let signature = match (&inspection.signature, &inspection.checksums) {
        (Some(signature), Some(checksums)) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or_default();
            match store.verify_package(signature, checksums, &inspection.manifest.id, now) {
                Ok(verified) => {
                    json!({"status": "verified", "publisher": verified.publisher, "keyId": verified.key_id})
                }
                Err(error) => {
                    json!({"status": "untrusted", "publisher": signature.publisher, "keyId": signature.key_id, "reason": error.to_string()})
                }
            }
        }
        _ => json!({"status": "unsigned"}),
    };
    print_json(&json!({
        "id": inspection.manifest.id,
        "version": inspection.manifest.version,
        "sha256": archive.digest()?.to_string(),
        "checksums": "valid",
        "signature": signature,
    }));
    Ok(())
}
