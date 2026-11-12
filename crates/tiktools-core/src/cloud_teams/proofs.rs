use super::*;

/// Same canonical form the server digests: sorted keys, no insignificant whitespace.
pub fn canonical_json(value: &Value) -> String {
    fn write(value: &Value, out: &mut String) {
        match value {
            Value::Object(map) => {
                out.push('{');
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for (index, key) in keys.into_iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&serde_json::to_string(key).unwrap_or_default());
                    out.push(':');
                    write(&map[key], out);
                }
                out.push('}');
            }
            Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write(item, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }
    let mut out = String::new();
    write(value, &mut out);
    out
}

pub(super) fn binding_digest(binding: &Value) -> String {
    Sha256::digest(canonical_json(binding).as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Message signed for a challenge. Mirrors the server's `proof_message`.
pub fn proof_message(purpose: &str, nonce: &[u8], binding_sha256: &str) -> Vec<u8> {
    format!(
        "{PROOF_DOMAIN}\n{purpose}\n{}\n{binding_sha256}",
        URL_SAFE_NO_PAD.encode(nonce)
    )
    .into_bytes()
}
