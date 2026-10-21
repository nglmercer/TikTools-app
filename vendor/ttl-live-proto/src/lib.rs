//! Generated Prost bindings for the TikTok Webcast Protobuf **v3** schema.
//!
//! The `.proto` sources under `proto/v3/` are vendored verbatim from
//! [`isaackogan/TikTok-Webcast-Protobuf`], pinned to the commit recorded in the
//! crate's `UPSTREAM` file. They are **AGPL-3.0-only**, unlike the rest of this
//! workspace; see `README.md` in this crate.
//!
//! This crate is deliberately thin: it exposes the generated types plus the one
//! decode entry point the transport needs. Normalisation into a stable event API
//! lives in `ttl-live-events`, so consumers never depend on the generated layout.
//!
//! ```no_run
//! # fn main() -> Result<(), prost::DecodeError> {
//! let payload: &[u8] = &[];
//! let batch = ttl_live_proto::decode_event_batch(payload)?;
//! for message in &batch.messages {
//!     println!("{} ({} bytes)", message.method, message.payload.len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [`isaackogan/TikTok-Webcast-Protobuf`]: https://github.com/isaackogan/TikTok-Webcast-Protobuf

use prost::Message;

/// Generated modules, rooted at the upstream package tree (`webcast.*`).
pub mod v3 {
    include!(concat!(env!("OUT_DIR"), "/v3.rs"));
}

pub mod registry;

pub use registry::{
    schema_by_name, schema_for_method, schemas, FieldCardinality, FieldKind, FieldSchema,
    FieldValueKind, MessageSchema, GENERATED_SCHEMA_MESSAGE_COUNT, GENERATED_WEBCAST_METHOD_COUNT,
};

pub use v3::webcast;
pub use v3::webcast::model::message as messages;
pub use v3::webcast::shared::message::{BaseProtoMessage, ProtoMessageFetchResult};

/// The upstream commit the vendored schemas were generated from.
pub const UPSTREAM_COMMIT: &str = "cf7bcd49d59926b44c1c4e2632df5558bf3e8169";

/// Decodes one decompressed WebSocket batch into a [`ProtoMessageFetchResult`].
///
/// `payload` is the already-gunzipped `LiveMessage::payload` produced by
/// `ttl-live-ws`; this function does no decompression of its own.
pub fn decode_event_batch(payload: &[u8]) -> Result<ProtoMessageFetchResult, prost::DecodeError> {
    ProtoMessageFetchResult::decode(payload)
}
