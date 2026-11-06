//! Release tooling: verify signed artifacts using the same code as the client.
use std::{
    fs,
    io::{self, Read},
    path::Path,
};
use tiktools_app_update::{authenticate_manifest, verify_signature, Channel, MAX_UPDATE_BYTES};

fn read_bounded(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > limit {
        return Err(io::Error::other("file exceeds size limit"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::other("file exceeds size limit"));
    }
    Ok(bytes)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: verify-update <package|manifest> <file> <signature>".into());
    }
    let key = std::env::var("TIKTOOLS_UPDATE_PUBLIC_KEY")
        .map_err(|_| "missing TIKTOOLS_UPDATE_PUBLIC_KEY")?;
    let signature = read_bounded(Path::new(&args[2]), 16 * 1024)?;
    let bytes = read_bounded(
        Path::new(&args[1]),
        if args[0] == "manifest" {
            256 * 1024
        } else {
            MAX_UPDATE_BYTES
        },
    )?;
    match args[0].as_str() {
        "package" => verify_signature(&bytes, std::str::from_utf8(&signature)?, &key)?,
        "manifest" => {
            let current = semver::Version::parse("0.0.0")?;
            for target in ["linux-x86_64", "windows-x86_64"] {
                authenticate_manifest(&bytes, &signature, Channel::Beta, target, &current, &key)?;
            }
        }
        _ => return Err("unsupported verification mode".into()),
    }
    println!("Verified {}", args[0]);
    Ok(())
}
