//! Protobuf provenance for the TikTok automation contracts.
//!
//! Every TikTok automation-data field is normalized from one protobuf field of
//! the vendor message that carried the event. This module records that
//! projection (TikTools field <- source path + transform) and stamps it onto
//! the generated JSON Schema as `x-native-method` / `x-native-source`, so the
//! TypeScript registry and autocomplete can show where each value comes from.
//!
//! Raw schema facts come from the pinned signer registry: field numbers,
//! protobuf names, JSON names, logical types, and cardinality are read off
//! [`ttl_live_proto::FieldSchema`], never restated here. This table owns only
//! what the descriptors cannot know — the normalization semantics (which
//! source path feeds which TikTools field, and through which [`SourceTransform`]).
//! Generation fails loudly when a transform disagrees with the descriptor
//! (see `validate_projection`), so protobuf drift surfaces here instead of
//! shipping stale provenance.
//!
//! Two deliberate boundaries:
//!
//! * The runtime DTOs and their serialization are untouched: JSON output types
//!   stay authoritative from the normalized contract, and generation fails
//!   loudly if a projection's expected type drifts from what schemars emits.
//! * Transforms are declared, not inferred. The descriptor says what the
//!   source *is*; the table says what normalization *does* with it.

mod metadata;
mod projection;
mod schema;
#[cfg(test)]
mod tests;
mod validation;

pub use metadata::{
    field_for_path, try_field_for_path, try_path_metadata, PathMetadata, StaleProjection,
};
pub use projection::SourceTransform;
pub use schema::replace_tiktok_contract_schemas;
