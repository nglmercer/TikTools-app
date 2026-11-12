//! Bounded JSON encoding with separate owner-RPC and event budgets.
use super::*;

/// Stop serialization before a host-created JSON string exceeds its item
/// budget. The source value is owned by its producer, not by this transport.
pub(crate) fn encode_bounded(value: &impl serde::Serialize) -> Result<String, serde_json::Error> {
    encode_with_limit(value, MAX_OUTBOUND_BYTES)
}

pub(crate) fn encode_owner_rpc_bounded(
    value: &impl serde::Serialize,
) -> Result<String, serde_json::Error> {
    encode_with_limit(value, MAX_OWNER_RPC_BYTES)
}

pub(super) fn message_limit(value: &serde_json::Value) -> usize {
    if value.get("type").and_then(serde_json::Value::as_str) == Some("rpc-response") {
        MAX_OWNER_RPC_BYTES
    } else {
        MAX_OUTBOUND_BYTES
    }
}

pub(super) fn encode_with_limit(
    value: &impl serde::Serialize,
    limit: usize,
) -> Result<String, serde_json::Error> {
    struct Bounded {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::other(
                    "outbound payload exceeds transport limit",
                ));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Bounded {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut output, value)?;
    Ok(String::from_utf8(output.bytes).expect("JSON serializer produces UTF-8"))
}
