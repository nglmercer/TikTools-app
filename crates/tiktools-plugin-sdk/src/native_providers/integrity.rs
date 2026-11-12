use super::*;

fn hex_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(ALPHABET[(byte >> 4) as usize] as char);
        text.push(ALPHABET[(byte & 0x0f) as usize] as char);
    }
    text
}

/// Hex SHA-256 digest rendered as a lockfile pin (`sha256:<hex>`).
pub(super) fn sha256_pin(bytes: &[u8]) -> String {
    format!("sha256:{}", hex_encode(&Sha256::digest(bytes)))
}

/// Verify `bytes` against a `sha256:<hex>` pin. Malformed pins fail
/// closed: a pin that cannot be parsed never matches.
pub(super) fn verify_pin(bytes: &[u8], pin: &str) -> bool {
    let Some(expected) = pin.strip_prefix("sha256:") else {
        return false;
    };
    expected.len() == 64 && hex_encode(&Sha256::digest(bytes)) == expected
}

/// Verify `bytes` against an npm `integrity` value
/// (`sha512-<base64>` or `sha256-<base64>`). Unknown algorithms and
/// malformed values fail closed.
pub(super) fn verify_integrity(bytes: &[u8], integrity: &str) -> bool {
    use base64::Engine;
    let Some((algorithm, encoded)) = integrity.split_once('-') else {
        return false;
    };
    let Ok(expected) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return false;
    };
    match algorithm {
        "sha512" => Sha512::digest(bytes).as_slice() == expected.as_slice(),
        "sha256" => Sha256::digest(bytes).as_slice() == expected.as_slice(),
        _ => false,
    }
}

pub(super) fn verify_asset(
    package: &str,
    locked: &LockedNativeLib,
    file: &str,
    bytes: &[u8],
) -> Result<(), NativeProviderError> {
    let Some(pin) = locked.assets.get(file) else {
        return Err(NativeProviderError::PinMissing {
            package: package.to_owned(),
            file: file.to_owned(),
        });
    };
    if !verify_pin(bytes, pin) {
        return Err(NativeProviderError::PinMismatch {
            package: package.to_owned(),
            file: file.to_owned(),
        });
    }
    Ok(())
}
