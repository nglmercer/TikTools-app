//! Owner library pages and additive public catalog forks.
use super::OperationError;
use super::{
    capabilities::Feature,
    errors::{map_failure, map_fork_failure},
    transport::{credentials, gated_call},
    validation::{canonical_id, check_cursor},
};
use super::{
    CatalogForkParams, CatalogForkResult, LibraryItemPage, LibraryItemsQuery, LIBRARY_LIMITS,
};
use crate::cloud_http::{self, decode, NoContent};
use reqwest::Method;
use serde_json::{json, Value};
use tiktools_cloud_auth::CloudAuth;

/// Lists the signed-in owner's templates and profiles, in any status.
pub async fn list_library_items(
    auth: &CloudAuth,
    query: LibraryItemsQuery,
) -> Result<LibraryItemPage, OperationError> {
    if query.query.chars().count() > 100 || query.query.chars().any(char::is_control) {
        return Err(OperationError::invalid("search text is not valid"));
    }
    if let Some(kind) = &query.kind {
        if kind != "template" && kind != "profile" {
            return Err(OperationError::invalid(
                "library kind must be template or profile",
            ));
        }
    }
    check_cursor(query.cursor.as_deref())?;
    let limit = query.limit.unwrap_or(25);
    if !(1..=50).contains(&limit) {
        return Err(OperationError::invalid("limit must be between 1 and 50"));
    }
    let (origin, token) = credentials(auth).await?;
    let mut params: Vec<(&str, String)> = vec![("limit", limit.to_string())];
    if !query.query.is_empty() {
        params.push(("q", query.query.clone()));
    }
    if let Some(kind) = &query.kind {
        params.push(("kind", kind.clone()));
    }
    if let Some(cursor) = &query.cursor {
        params.push(("cursor", cursor.clone()));
    }
    let request = cloud_http::authorized(
        Method::GET,
        format!("{origin}/api/v3/library/items"),
        &token,
    )?
    .query(&params);
    let value =
        cloud_http::send_json(request, LIBRARY_LIMITS, NoContent::Unexpected, map_failure).await?;
    let items = value
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| OperationError::internal("cloud sent an unexpected library page"))?;
    let next_cursor = match value.get("nextCursor") {
        Some(Value::String(cursor)) => Some(cursor.clone()),
        Some(Value::Null) | None => None,
        _ => return Err(OperationError::internal("cloud sent an unexpected cursor")),
    };
    Ok(LibraryItemPage { items, next_cursor })
}

/// Copies a public template or profile into the caller's library as a private
/// draft. Additive: nothing existing is changed.
pub async fn fork_catalog_document(
    auth: &CloudAuth,
    params: CatalogForkParams,
) -> Result<CatalogForkResult, OperationError> {
    let id = canonical_id(&params.id)?;
    if params.revision.is_some_and(|revision| revision < 1) {
        return Err(OperationError::invalid("revision must be positive"));
    }
    let body = params
        .revision
        .map(|revision| json!({ "revision": revision }));
    let value = gated_call(
        auth,
        Feature::Fork,
        Method::POST,
        format!("/api/v2/catalog/documents/{}/fork", id),
        &[],
        body,
        map_fork_failure,
    )
    .await?;
    let result: CatalogForkResult = decode(value)?;
    if result.status != "draft" || result.revision < 1 || result.fork_of.revision < 1 {
        return Err(OperationError::internal(
            "cloud sent an unexpected fork reply",
        ));
    }
    Ok(result)
}
