//! Anonymous v2 discovery from the operator instance. Catalog metadata is not
//! an installation authorization or a source of registry trust.
use super::OperationError;
use crate::AppCore;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Leave ample room for RPC envelopes under the 1 MiB remote frame limit.
const MAX_CATALOG_RESULT_BYTES: usize = 768 * 1024;

#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogQuery {
    #[serde(default)]
    pub query: String,
    pub after: Option<String>,
    /// Exact decimal generation; JavaScript numbers cannot represent all i64s.
    pub generation: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudCatalogPage {
    pub generation: Option<String>,
    pub items: Vec<Value>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogSummariesQuery {
    pub query: Option<String>,
    pub target: Option<String>,
    pub generation: Option<String>,
    pub cursor: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogVersionsQuery {
    pub id: String,
    pub target: Option<String>,
    pub generation: Option<String>,
    pub cursor: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogPaginationCapabilities {
    pub schema_version: u32,
    pub summary_path: String,
    pub version_page_path: String,
    pub max_items: u32,
    pub max_bytes: u32,
    pub version_order: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogSummary {
    pub id: String,
    pub metadata_path: String,
    pub latest_stable_version: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogSummariesPage {
    pub generation: Option<String>,
    pub items: Vec<CloudCatalogSummary>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogAdvisory {
    pub severity: String,
    pub message: Option<String>,
    pub url: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogVersion {
    pub version: String,
    pub target: String,
    pub sha256: String,
    pub size: String,
    pub yanked: bool,
    pub key_revoked: bool,
    pub publisher_approved: bool,
    pub advisory: Option<CloudCatalogAdvisory>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogVersionsPage {
    pub generation: String,
    pub id: String,
    pub metadata_path: String,
    pub items: Vec<CloudCatalogVersion>,
    pub next_cursor: Option<String>,
}

/// Unified catalog query (v3). `kind` is restricted to the three served kinds.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogItemsQuery {
    #[serde(default)]
    pub query: String,
    pub kind: Option<String>,
    pub target: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudCatalogItemPage {
    pub items: Vec<Value>,
    pub next_cursor: Option<String>,
    /// `ready` or `noActiveGeneration` (no plugin generation is published yet,
    /// which is an empty registry and not an outage). Absent when the Cloud
    /// does not announce it or announces a value this host does not know.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_state: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudCatalogItemQuery {
    pub kind: String,
    pub id: String,
    pub target: Option<String>,
}

fn convert_catalog<T: serde::de::DeserializeOwned>(
    value: impl Serialize,
) -> Result<T, OperationError> {
    let value = serde_json::to_value(value)
        .map_err(|_| OperationError::internal("invalid catalog response"))?;
    serde_json::from_value(value).map_err(|_| OperationError::internal("invalid catalog response"))
}
fn catalog_error(error: cloud_client::SyncError) -> OperationError {
    match error {
        cloud_client::SyncError::Configuration => {
            OperationError::invalid("invalid cloud catalog query or cursor")
        }
        _ => OperationError::unavailable("cloud catalog is unavailable"),
    }
}
fn pagination_capability_error(error: cloud_client::SyncError) -> OperationError {
    match error {
        cloud_client::SyncError::Http(404) => OperationError {
            code: "capability_unavailable",
            message: "this Cloud instance does not expose bounded catalog pagination".into(),
            data: Some(json!({"reason":"bounded_catalog_unsupported"})),
        },
        other => catalog_error(other),
    }
}
fn configured_catalog() -> Result<cloud_client::CloudCatalog, OperationError> {
    let origin = tiktools_cloud_auth::resolve_cloud_origin()
        .ok_or_else(|| OperationError::unavailable("cloud is not configured"))?;
    cloud_client::CloudCatalog::new(&origin)
        .map_err(|_| OperationError::unavailable("cloud catalog is not configured"))
}

/// v3 items carry release sizes that may exceed JavaScript's safe integer range.
/// Convert every size to an exact string so the webview never rounds it.
fn project_items_v3(mut value: Value) -> Result<CloudCatalogItemPage, OperationError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| OperationError::internal("invalid catalog response"))?;
    let mut items = object
        .remove("items")
        .and_then(|v| match v {
            Value::Array(items) => Some(items),
            _ => None,
        })
        .ok_or_else(|| OperationError::internal("invalid catalog items"))?;
    for item in &mut items {
        if let Some(size) = item.pointer("/releaseRef/size").and_then(Value::as_i64) {
            item["releaseRef"]["size"] = Value::String(size.to_string());
        }
    }
    let next_cursor = match object.remove("nextCursor") {
        Some(Value::Null) | None => None,
        Some(Value::String(v)) => Some(v),
        _ => return Err(OperationError::internal("invalid catalog cursor")),
    };
    let data_state = match object.remove("dataState") {
        Some(Value::String(state)) if matches!(state.as_str(), "ready" | "noActiveGeneration") => {
            Some(state)
        }
        _ => None,
    };
    let result = CloudCatalogItemPage {
        items,
        next_cursor,
        data_state,
    };
    let bytes = serde_json::to_vec(&result)
        .map_err(|_| OperationError::internal("invalid catalog response"))?;
    if bytes.len() > MAX_CATALOG_RESULT_BYTES {
        return Err(OperationError::unavailable(
            "catalog page exceeds the transport limit; narrow the search or target filter",
        ));
    }
    Ok(result)
}

fn items_v3_error(error: cloud_client::SyncError) -> OperationError {
    match error {
        cloud_client::SyncError::Configuration => {
            OperationError::invalid("invalid cloud catalog query or cursor")
        }
        cloud_client::SyncError::Http(404) => OperationError::not_found("catalog item not found"),
        _ => OperationError::unavailable("cloud catalog is unavailable"),
    }
}

fn parse_generation(value: Option<&str>) -> Result<Option<i64>, OperationError> {
    value
        .map(|v| {
            if v.is_empty() || v.starts_with('0') || !v.bytes().all(|c| c.is_ascii_digit()) {
                return Err(OperationError::invalid(
                    "generation must be a positive decimal string",
                ));
            }
            v.parse::<i64>().ok().filter(|n| *n > 0).ok_or_else(|| {
                OperationError::invalid("generation must be a positive decimal string")
            })
        })
        .transpose()
}

// The transport has already validated the complete response before this
// projection. Convert every i64 exposed to JavaScript into an exact string.
fn project(mut value: Value) -> Result<CloudCatalogPage, OperationError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| OperationError::internal("invalid catalog response"))?;
    let generation = match object.remove("generation") {
        Some(Value::Null) => None,
        Some(Value::Number(n)) => Some(
            n.as_i64()
                .ok_or_else(|| OperationError::internal("invalid catalog generation"))?
                .to_string(),
        ),
        _ => return Err(OperationError::internal("invalid catalog generation")),
    };
    let mut items = object
        .remove("items")
        .and_then(|v| v.as_array().cloned())
        .ok_or_else(|| OperationError::internal("invalid catalog items"))?;
    for item in &mut items {
        let versions = item
            .get_mut("versions")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| OperationError::internal("invalid catalog versions"))?;
        for version in versions {
            let size = version
                .get("size")
                .and_then(Value::as_i64)
                .ok_or_else(|| OperationError::internal("invalid catalog size"))?;
            version["size"] = Value::String(size.to_string());
        }
    }
    let next_cursor = match object.remove("nextCursor") {
        Some(Value::Null) => None,
        Some(Value::String(v)) => Some(v),
        _ => return Err(OperationError::internal("invalid catalog cursor")),
    };
    let result = CloudCatalogPage {
        generation,
        items,
        next_cursor,
    };
    let bytes = serde_json::to_vec(&result)
        .map_err(|_| OperationError::internal("invalid catalog response"))?;
    if bytes.len() > MAX_CATALOG_RESULT_BYTES {
        return Err(OperationError::unavailable(
            "catalog page exceeds the transport limit; narrow the search or target filter",
        ));
    }
    Ok(result)
}

async fn fetch_catalog(
    origin: &str,
    query: CloudCatalogQuery,
) -> Result<CloudCatalogPage, OperationError> {
    let generation = parse_generation(query.generation.as_deref())?;
    if query.after.is_some() && generation.is_none() {
        return Err(OperationError::invalid(
            "catalog pagination requires the original generation",
        ));
    }
    let catalog = cloud_client::CloudCatalog::new(origin)
        .map_err(|_| OperationError::unavailable("cloud catalog is not configured"))?;
    let page = catalog
        .plugins_filtered(
            &query.query,
            query.after.as_deref(),
            generation,
            query.target.as_deref(),
        )
        .await
        .map_err(|error| match error {
            cloud_client::SyncError::Configuration => {
                OperationError::invalid("invalid cloud catalog query")
            }
            _ => OperationError::unavailable("cloud catalog is unavailable"),
        })?;
    project(page)
}

impl AppCore {
    pub async fn cloud_catalog_pagination_capabilities(
        &self,
    ) -> Result<CloudCatalogPaginationCapabilities, OperationError> {
        convert_catalog(
            configured_catalog()?
                .pagination_capabilities()
                .await
                .map_err(pagination_capability_error)?,
        )
    }
    pub async fn cloud_catalog_summaries(
        &self,
        query: CloudCatalogSummariesQuery,
    ) -> Result<CloudCatalogSummariesPage, OperationError> {
        let generation = parse_generation(query.generation.as_deref())?;
        let page = configured_catalog()?
            .plugin_summaries(
                query.query.as_deref(),
                query.target.as_deref(),
                generation,
                query.cursor.as_deref(),
            )
            .await
            .map_err(catalog_error)?;
        convert_catalog(page)
    }
    pub async fn cloud_catalog_versions(
        &self,
        query: CloudCatalogVersionsQuery,
    ) -> Result<CloudCatalogVersionsPage, OperationError> {
        let generation = parse_generation(query.generation.as_deref())?;
        let page = configured_catalog()?
            .plugin_version_page(
                &query.id,
                query.target.as_deref(),
                generation,
                query.cursor.as_deref(),
            )
            .await
            .map_err(catalog_error)?;
        convert_catalog(page)
    }
    /// Unified catalog page. Presentation only; install still needs the signed release.
    pub async fn cloud_catalog_items(
        &self,
        query: CloudCatalogItemsQuery,
    ) -> Result<CloudCatalogItemPage, OperationError> {
        let limit = query.limit.unwrap_or(25);
        let page = configured_catalog()?
            .items_v3(
                &query.query,
                query.kind.as_deref(),
                query.target.as_deref(),
                query.cursor.as_deref(),
                limit,
            )
            .await
            .map_err(items_v3_error)?;
        project_items_v3(page)
    }
    pub async fn cloud_catalog_item(
        &self,
        query: CloudCatalogItemQuery,
    ) -> Result<Value, OperationError> {
        configured_catalog()?
            .item_v3(&query.kind, &query.id, query.target.as_deref())
            .await
            .map_err(items_v3_error)
    }
    /// Read-only discovery, not proof that a package can be installed.
    pub async fn cloud_catalog_plugins(
        &self,
        query: CloudCatalogQuery,
    ) -> Result<CloudCatalogPage, OperationError> {
        let origin = tiktools_cloud_auth::resolve_cloud_origin()
            .ok_or_else(|| OperationError::unavailable("cloud is not configured"))?;
        fetch_catalog(&origin, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[test]
    fn pagination_fallback_is_only_for_explicit_missing_endpoint() {
        let absent = pagination_capability_error(cloud_client::SyncError::Http(404));
        assert_eq!(absent.code, "capability_unavailable");
        assert_eq!(
            absent.data.unwrap()["reason"],
            "bounded_catalog_unsupported"
        );
        for error in [
            cloud_client::SyncError::Network,
            cloud_client::SyncError::Protocol,
            cloud_client::SyncError::Http(500),
        ] {
            let failure = pagination_capability_error(error);
            assert_eq!(failure.code, "unavailable");
            assert!(failure.data.is_none());
        }
    }
    #[test]
    fn catalog_projection_keeps_large_integers_exact() {
        let page = project(json!({"generation":9007199254740993_i64,"nextCursor":null,"items":[{"id":"a","versions":[{"size":9007199254740995_i64}]}]})).unwrap();
        assert_eq!(page.generation.as_deref(), Some("9007199254740993"));
        assert_eq!(page.items[0]["versions"][0]["size"], "9007199254740995");
        assert_eq!(
            parse_generation(page.generation.as_deref()).unwrap(),
            Some(9007199254740993)
        );
        for invalid in ["", "0", "01", "-1", "1.0", "9223372036854775808"] {
            assert!(parse_generation(Some(invalid)).is_err());
        }
    }

    #[test]
    fn unified_items_keep_release_sizes_exact_and_reject_bad_cursors() {
        let page = project_items_v3(json!({
            "items": [{"identity": {"kind": "plugin", "id": "acme.gift"},
                       "releaseRef": {"type": "pluginRelease", "size": 9007199254740995_i64}}],
            "nextCursor": "eyJrIjoxfQ"
        }))
        .unwrap();
        assert_eq!(page.items[0]["releaseRef"]["size"], "9007199254740995");
        assert_eq!(page.next_cursor.as_deref(), Some("eyJrIjoxfQ"));
        assert_eq!(page.data_state, None, "absent stays absent");
        let empty = project_items_v3(
            json!({"items": [], "nextCursor": null, "dataState": "noActiveGeneration"}),
        )
        .unwrap();
        assert_eq!(empty.data_state.as_deref(), Some("noActiveGeneration"));
        assert_eq!(
            serde_json::to_value(&empty).unwrap()["dataState"],
            "noActiveGeneration"
        );
        let unknown =
            project_items_v3(json!({"items": [], "nextCursor": null, "dataState": "future"}))
                .unwrap();
        assert_eq!(unknown.data_state, None);
        assert!(serde_json::to_value(&unknown)
            .unwrap()
            .get("dataState")
            .is_none());
        assert!(project_items_v3(json!({"items": [], "nextCursor": 3})).is_err());
        assert!(project_items_v3(json!({"nextCursor": null})).is_err());
    }

    #[test]
    fn oversized_catalog_page_fails_without_truncating() {
        let error = project(json!({"generation":1,"nextCursor":null,"items":[{"id":"a","versions":[{"size":1}],"padding":"x".repeat(MAX_CATALOG_RESULT_BYTES)}]})).unwrap_err();
        assert_eq!(error.code, "unavailable");
        assert!(error.message.contains("narrow the search"));
    }

    #[tokio::test]
    async fn catalog_http_uses_configured_origin_and_pinned_cursor() {
        const GENERATION: i64 = 9007199254740993;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for page in 0..2 {
                let (mut socket, _) =
                    tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut request = Vec::new();
                loop {
                    let mut bytes = [0u8; 2048];
                    let n = socket.read(&mut bytes).await.unwrap();
                    assert!(n > 0);
                    request.extend_from_slice(&bytes[..n]);
                    if request.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                    assert!(request.len() < 16384);
                }
                requests.push(String::from_utf8(request).unwrap());
                let items: Vec<Value> = if page == 0 {
                    (0..50).map(|i| {
                    let id = format!("acme.p{i:02}");
                    json!({"id": id, "latestStableVersion":"1.0.0", "metadataPath":format!("/api/v1/registry/generations/{GENERATION}/files/index/ac/me/acme.p{i:02}.json"),
                      "versions":[{"version":"1.0.0","target":"any","sha256":"a".repeat(64),"size":9007199254740995_i64,"yanked":false,"keyRevoked":false,"publisherApproved":true,"advisory":null}]})
                }).collect()
                } else {
                    vec![]
                };
                let body = serde_json::to_vec(&json!({"generation":GENERATION,"items":items,"nextCursor":if page == 0 { Some("acme.p49") } else { None }})).unwrap();
                let headers = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                socket.write_all(headers.as_bytes()).await.unwrap();
                socket.write_all(&body).await.unwrap();
            }
            requests
        });
        let first = fetch_catalog(&origin, CloudCatalogQuery::default())
            .await
            .unwrap();
        assert_eq!(first.generation.as_deref(), Some("9007199254740993"));
        assert_eq!(first.items[0]["versions"][0]["size"], "9007199254740995");
        let invalid = CloudCatalogQuery {
            after: first.next_cursor.clone(),
            ..Default::default()
        };
        assert_eq!(
            fetch_catalog(&origin, invalid).await.unwrap_err().code,
            "invalid_params"
        );
        let second = fetch_catalog(
            &origin,
            CloudCatalogQuery {
                after: first.next_cursor,
                generation: first.generation,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(second.items.is_empty());
        let requests = task.await.unwrap();
        assert!(requests[0].starts_with("GET /api/v2/catalog/plugins?q="));
        assert!(requests[1].contains("after=acme.p49"));
        assert!(requests[1].contains("generation=9007199254740993"));
        assert!(!requests
            .iter()
            .any(|request| request.to_ascii_lowercase().contains("authorization:")));
    }
}
