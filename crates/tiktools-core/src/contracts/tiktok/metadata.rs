use ttl_live_proto::{FieldKind, FieldSchema};

/// A projection path that no longer resolves against the pinned registry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("stale TikTok contract projection: {method}.{path} ({reason})")]
pub struct StaleProjection {
    method: String,
    path: String,
    reason: String,
}

/// A projection path resolved against the pinned registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMetadata {
    /// The leaf field's descriptor, carrying its authoritative `number`,
    /// `name`, `json_name`, `value_kind`, and `cardinality`.
    pub field: &'static FieldSchema,
    /// Dotted protobuf JSON path (`gift.diamond_count` -> `gift.diamondCount`),
    /// built from each segment's descriptor `json_name`.
    pub json_path: String,
}

/// Resolves a dotted protobuf path against the current registry.
///
/// Nested segments (e.g. `gift` in `gift.name`) must be message-typed and are
/// followed through [`ttl_live_proto::schema_by_name`]. Any drift — unknown
/// method, renamed field, scalar in the middle of the path, missing nested
/// descriptor — is an error so generation fails instead of stamping stale
/// provenance.
pub fn try_path_metadata(method: &str, path: &str) -> Result<PathMetadata, StaleProjection> {
    let stale = |reason: String| StaleProjection {
        method: method.to_owned(),
        path: path.to_owned(),
        reason,
    };
    let mut schema = ttl_live_proto::schema_for_method(method)
        .ok_or_else(|| stale("unknown method (not in the pinned registry)".to_owned()))?;
    let segments: Vec<&str> = path.split('.').collect();
    let mut json_segments = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            return Err(stale("empty path segment".to_owned()));
        }
        let field = schema
            .fields
            .iter()
            .find(|field| field.name == *segment)
            .ok_or_else(|| stale(format!("{segment:?} is not a field of {}", schema.name)))?;
        json_segments.push(field.json_name);
        if index + 1 == segments.len() {
            return Ok(PathMetadata {
                field,
                json_path: json_segments.join("."),
            });
        }
        match field.kind {
            FieldKind::Message(qualified) => {
                schema = ttl_live_proto::schema_by_name(qualified)
                    .ok_or_else(|| stale(format!("nested type {qualified} has no descriptor")))?;
            }
            _ => {
                return Err(stale(format!(
                    "{segment:?} is a scalar; cannot resolve a nested path through it"
                )));
            }
        }
    }
    Err(stale("empty path".to_owned()))
}

/// Resolves a dotted protobuf path to its leaf field descriptor.
///
/// Convenience wrapper over [`try_path_metadata`] for callers that only need
/// the leaf [`FieldSchema`].
pub fn try_field_for_path(
    method: &str,
    path: &str,
) -> Result<&'static ttl_live_proto::FieldSchema, StaleProjection> {
    try_path_metadata(method, path).map(|metadata| metadata.field)
}

/// [`try_field_for_path`] that panics loudly on drift.
pub fn field_for_path(method: &str, path: &str) -> &'static ttl_live_proto::FieldSchema {
    try_field_for_path(method, path).unwrap_or_else(|error| panic!("{error}"))
}
