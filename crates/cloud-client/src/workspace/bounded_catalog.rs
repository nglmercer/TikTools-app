//! Strict, byte-bounded anonymous catalog pagination. Catalog metadata never
//! supplies the independent signatures or root pins required for installation.
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Deserializer, Serialize};

const MAX_PAGE: usize = 512 * 1024;
const MAX_CURSOR: usize = 2048;
const MAX_ITEMS: usize = 50;

fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogPageCapabilities {
    pub schema_version: u32,
    pub summary_path: String,
    pub version_page_path: String,
    pub max_items: u32,
    pub max_bytes: u32,
    pub version_order: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogSummary {
    pub id: String,
    pub metadata_path: String,
    #[serde(deserialize_with = "required_option")]
    pub latest_stable_version: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogSummaryPage {
    #[serde(deserialize_with = "required_option")]
    pub generation: Option<String>,
    pub items: Vec<CatalogSummary>,
    #[serde(deserialize_with = "required_option")]
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogAdvisory {
    pub severity: String,
    #[serde(deserialize_with = "required_option")]
    pub message: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogVersion {
    pub version: String,
    pub target: String,
    pub sha256: String,
    pub size: String,
    pub yanked: bool,
    pub key_revoked: bool,
    pub publisher_approved: bool,
    #[serde(deserialize_with = "required_option")]
    pub advisory: Option<CatalogAdvisory>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogVersionPage {
    pub generation: String,
    pub id: String,
    pub metadata_path: String,
    pub items: Vec<CatalogVersion>,
    #[serde(deserialize_with = "required_option")]
    pub next_cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Cursor {
    Summary {
        generation: i64,
        #[serde(deserialize_with = "required_option")]
        q: Option<String>,
        #[serde(deserialize_with = "required_option")]
        target: Option<String>,
        last: String,
    },
    Versions {
        generation: i64,
        id: String,
        #[serde(deserialize_with = "required_option")]
        target: Option<String>,
        version: String,
        last_target: String,
    },
}
fn cursor(value: &str) -> Result<Cursor, SyncError> {
    if value.is_empty() || value.len() > MAX_CURSOR {
        return Err(SyncError::Protocol);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| SyncError::Protocol)?;
    if URL_SAFE_NO_PAD.encode(&bytes) != value {
        return Err(SyncError::Protocol);
    }
    serde_json::from_slice(&bytes).map_err(|_| SyncError::Protocol)
}
fn decimal(value: &str) -> Option<i64> {
    if value.is_empty() || value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse::<i64>().ok().filter(|v| *v > 0)
}
fn path_valid(generation: i64, id: &str, actual: &str) -> bool {
    if !valid_catalog_id(id) {
        return false;
    }
    let chars: Vec<_> = id.chars().collect();
    let part = |start| {
        (start..start + 2)
            .map(|i| chars.get(i).copied().unwrap_or('_'))
            .collect::<String>()
    };
    let path = if part(2) == ".." {
        format!("index/{id}.json")
    } else {
        format!("index/{}/{}/{id}.json", part(0), part(2))
    };
    actual == format!("/api/v1/registry/generations/{generation}/files/{path}")
}
fn version_valid(version: &str) -> bool {
    version.len() <= 128 && semver::Version::parse(version).is_ok()
}
fn check_summary_cursor(
    value: &str,
    generation: i64,
    q: Option<&str>,
    target: Option<&str>,
) -> Result<String, SyncError> {
    match cursor(value)? {
        Cursor::Summary {
            generation: g,
            q: c_q,
            target: c_target,
            last,
        } if g == generation
            && c_q.as_deref() == q
            && c_target.as_deref() == target
            && valid_catalog_id(&last) =>
        {
            Ok(last)
        }
        _ => Err(SyncError::Protocol),
    }
}
fn check_version_cursor(
    value: &str,
    generation: i64,
    id: &str,
    target: Option<&str>,
) -> Result<(String, String), SyncError> {
    match cursor(value)? {
        Cursor::Versions {
            generation: g,
            id: c_id,
            target: c_target,
            version,
            last_target,
        } if g == generation
            && c_id == id
            && c_target.as_deref() == target
            && version_valid(&version)
            && valid_target(&last_target)
            && target.is_none_or(|v| last_target == v || last_target == "any") =>
        {
            Ok((version, last_target))
        }
        _ => Err(SyncError::Protocol),
    }
}
fn valid_advisory(advisory: &CatalogAdvisory) -> bool {
    matches!(
        advisory.severity.as_str(),
        "low" | "moderate" | "high" | "critical"
    ) && advisory.message.as_ref().is_some_and(|m| {
        !m.trim().is_empty() && m.len() <= 1024 && !m.chars().any(char::is_control)
    }) && advisory.url.as_ref().is_none_or(|u| {
        url::Url::parse(u).is_ok_and(|parsed| {
            parsed.scheme() == "https"
                && parsed.host().is_some()
                && parsed.username().is_empty()
                && parsed.password().is_none()
                && parsed.fragment().is_none()
        })
    })
}
fn valid_version(item: &CatalogVersion, target: Option<&str>) -> bool {
    version_valid(&item.version)
        && valid_target(&item.target)
        && target.is_none_or(|v| item.target == v || item.target == "any")
        && item.sha256.len() == 64
        && item
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && decimal(&item.size).is_some()
        && item.advisory.as_ref().is_none_or(valid_advisory)
}
impl CloudCatalog {
    async fn bounded_get<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, SyncError> {
        let url = self
            .origin
            .join(path)
            .map_err(|_| SyncError::Configuration)?;
        let response = self
            .client
            .get(url)
            .query(params)
            .send()
            .await
            .map_err(|_| SyncError::Network)?;
        decode_with_limit(response, MAX_PAGE).await
    }
    /// Discover bounded routes without adding fields to the legacy capabilities object.
    pub async fn pagination_capabilities(&self) -> Result<CatalogPageCapabilities, SyncError> {
        let value: CatalogPageCapabilities = self
            .bounded_get("api/v2/catalog/pagination-capabilities", &[])
            .await?;
        if value.schema_version != 1
            || value.summary_path != "/api/v2/catalog/plugin-summaries"
            || value.version_page_path != "/api/v2/catalog/plugins/{id}/version-pages"
            || value.max_items != 50
            || value.max_bytes != MAX_PAGE as u32
            || value.version_order != "ascii-version-target"
        {
            return Err(SyncError::Protocol);
        }
        Ok(value)
    }
    /// Read one generation-pinned summary page. Continuation must reuse all filters.
    pub async fn plugin_summaries(
        &self,
        q: Option<&str>,
        target: Option<&str>,
        generation: Option<i64>,
        next: Option<&str>,
    ) -> Result<CatalogSummaryPage, SyncError> {
        if q.is_some_and(|v| v.len() > 100)
            || target.is_some_and(|v| !valid_target(v))
            || generation.is_some_and(|v| v < 1)
        {
            return Err(SyncError::Configuration);
        }
        let after = match next {
            Some(value) => Some(
                check_summary_cursor(
                    value,
                    generation.ok_or(SyncError::Configuration)?,
                    q,
                    target,
                )
                .map_err(|_| SyncError::Configuration)?,
            ),
            None => None,
        };
        let mut params = Vec::new();
        if let Some(q) = q {
            params.push(("q", q.to_owned()));
        }
        if let Some(target) = target {
            params.push(("target", target.to_owned()));
        }
        if let Some(generation) = generation {
            params.push(("generation", generation.to_string()));
        }
        if let Some(next) = next {
            params.push(("cursor", next.to_owned()));
        }
        let page: CatalogSummaryPage = self
            .bounded_get("api/v2/catalog/plugin-summaries", &params)
            .await?;
        let Some(raw) = page.generation.as_deref() else {
            return if generation.is_none() && page.items.is_empty() && page.next_cursor.is_none() {
                Ok(page)
            } else {
                Err(SyncError::Protocol)
            };
        };
        let actual = decimal(raw).ok_or(SyncError::Protocol)?;
        if generation.is_some_and(|g| g != actual) || page.items.len() > MAX_ITEMS {
            return Err(SyncError::Protocol);
        }
        let mut previous = after.as_deref();
        for item in &page.items {
            if !path_valid(actual, &item.id, &item.metadata_path)
                || previous.is_some_and(|old| item.id.as_str() <= old)
                || q.is_some_and(|query| {
                    query.is_ascii() && !item.id.contains(&query.to_ascii_lowercase())
                })
                || item.latest_stable_version.as_ref().is_some_and(|v| {
                    !version_valid(v) || semver::Version::parse(v).is_ok_and(|v| !v.pre.is_empty())
                })
            {
                return Err(SyncError::Protocol);
            }
            previous = Some(&item.id);
        }
        if let Some(next) = &page.next_cursor {
            if page.items.is_empty()
                || Some(check_summary_cursor(next, actual, q, target)?.as_str()) != previous
            {
                return Err(SyncError::Protocol);
            }
        }
        Ok(page)
    }
    /// Read one version page sorted by ASCII version and target, not semver precedence.
    pub async fn plugin_version_page(
        &self,
        id: &str,
        target: Option<&str>,
        generation: Option<i64>,
        next: Option<&str>,
    ) -> Result<CatalogVersionPage, SyncError> {
        if !valid_catalog_id(id)
            || target.is_some_and(|v| !valid_target(v))
            || generation.is_some_and(|v| v < 1)
        {
            return Err(SyncError::Configuration);
        }
        let after = match next {
            Some(value) => Some(
                check_version_cursor(
                    value,
                    generation.ok_or(SyncError::Configuration)?,
                    id,
                    target,
                )
                .map_err(|_| SyncError::Configuration)?,
            ),
            None => None,
        };
        let mut params = Vec::new();
        if let Some(target) = target {
            params.push(("target", target.to_owned()));
        }
        if let Some(generation) = generation {
            params.push(("generation", generation.to_string()));
        }
        if let Some(next) = next {
            params.push(("cursor", next.to_owned()));
        }
        let page: CatalogVersionPage = self
            .bounded_get(
                &format!("api/v2/catalog/plugins/{id}/version-pages"),
                &params,
            )
            .await?;
        let actual = decimal(&page.generation).ok_or(SyncError::Protocol)?;
        if generation.is_some_and(|v| v != actual)
            || page.id != id
            || !path_valid(actual, id, &page.metadata_path)
            || page.items.len() > MAX_ITEMS
            || (after.is_none() && page.items.is_empty())
        {
            return Err(SyncError::Protocol);
        }
        let mut previous = after.as_ref().map(|(v, t)| (v.as_str(), t.as_str()));
        for item in &page.items {
            let key = (item.version.as_str(), item.target.as_str());
            if !valid_version(item, target) || previous.is_some_and(|old| key <= old) {
                return Err(SyncError::Protocol);
            }
            previous = Some(key);
        }
        if let Some(next) = &page.next_cursor {
            let key = check_version_cursor(next, actual, id, target)?;
            if page.items.is_empty() || previous != Some((key.0.as_str(), key.1.as_str())) {
                return Err(SyncError::Protocol);
            }
        }
        Ok(page)
    }
}
