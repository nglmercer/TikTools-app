//! Single-record library storage in the `automation-profiles` collection.
//!
//! Mirrors the native sync transport used by `selection_sync` but targets the
//! one record stored under the `library` key instead of the plugin-selection
//! collection.
use super::*;

const LIBRARY_COLLECTION: &str = "automation-profiles";
const LIBRARY_KEY: &str = "library";
const LIBRARY_HEAD_PATH: &str = "api/v1/native/sync/head";
const LIBRARY_CHANGES_PATH: &str = "api/v1/native/sync/changes";
const LIBRARY_COLLECTION_PATH: &str = "api/v1/native/sync/collections/automation-profiles";
const LIBRARY_RECORD_PATH: &str = "api/v1/native/sync/collections/automation-profiles/library";
const LIBRARY_PAGE_LIMIT: usize = 16;

fn deserialize_schema_version_one<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let version = u32::deserialize(deserializer)?;
    if version == 1 {
        Ok(version)
    } else {
        Err(serde::de::Error::custom("unsupported schemaVersion"))
    }
}

fn deserialize_origin<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let origin = String::deserialize(deserializer)?;
    if origin == "ui" || origin == "agent" {
        Ok(origin)
    } else {
        Err(serde::de::Error::custom("invalid origin"))
    }
}

/// One page of the collection listing; `next_after` is the last key of the page.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryListing {
    records: Vec<LibraryEnvelope>,
    #[serde(default)]
    next_after: Option<String>,
}

/// The single automation-library record stored at `automation-profiles/library`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryRecord {
    #[serde(deserialize_with = "deserialize_schema_version_one")]
    pub schema_version: u32,
    #[serde(deserialize_with = "deserialize_origin")]
    pub origin: String,
    pub revision: String,
    pub snapshot: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

/// One change to the library record in the native sync change feed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryChange {
    pub collection: String,
    pub key: String,
    pub revision: i64,
    pub deleted: bool,
    pub value: Option<LibraryRecord>,
}

/// A page of the native sync change feed (the endpoint uses snake_case).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LibraryChanges {
    pub changes: Vec<LibraryChange>,
    pub cursor: i64,
    pub has_more: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryEnvelope {
    collection: String,
    key: String,
    revision: serde_json::Value,
    deleted: bool,
    value: Option<LibraryRecord>,
}

fn revision_token(revision: &serde_json::Value) -> Result<String, SyncError> {
    match revision {
        serde_json::Value::String(value) if !value.is_empty() => Ok(value.clone()),
        serde_json::Value::Number(number) if number.as_i64().is_some_and(|value| value >= 0) => {
            Ok(number.to_string())
        }
        _ => Err(SyncError::Protocol),
    }
}

impl Session {
    /// Current sync head, read from the same endpoint as selection sync.
    pub async fn library_head(&self) -> Result<i64, SyncError> {
        let head: Head = self.get(LIBRARY_HEAD_PATH, &[]).await?;
        if head.cursor < 0 {
            return Err(SyncError::Protocol);
        }
        Ok(head.cursor)
    }

    /// One change page restricted to this collection and key. The filters are
    /// sent to the endpoint when it supports them and applied again locally so
    /// the result is correct even if the endpoint ignores them.
    pub async fn library_changes(&self, cursor: i64) -> Result<LibraryChanges, SyncError> {
        if cursor < 0 {
            return Err(SyncError::Configuration);
        }
        let params = [
            ("cursor", cursor.to_string()),
            ("collection", LIBRARY_COLLECTION.to_owned()),
            ("key", LIBRARY_KEY.to_owned()),
        ];
        let page: LibraryChanges = self.get(LIBRARY_CHANGES_PATH, &params).await?;
        if page.cursor < cursor || (page.has_more && page.cursor == cursor) {
            return Err(SyncError::Protocol);
        }
        let changes = page
            .changes
            .into_iter()
            .filter(|change| change.collection == LIBRARY_COLLECTION && change.key == LIBRARY_KEY)
            .collect();
        Ok(LibraryChanges {
            changes,
            cursor: page.cursor,
            has_more: page.has_more,
        })
    }

    /// Read the single library record together with its revision token.
    /// The server only serves single-key writes, so the record is read from the
    /// collection listing. `Ok(None)` means the record is absent or tombstoned.
    pub async fn library_get(&self) -> Result<Option<(LibraryRecord, String)>, SyncError> {
        let mut after: Option<String> = None;
        for _ in 0..LIBRARY_PAGE_LIMIT {
            let params = after
                .as_ref()
                .map(|key| vec![("after", key.clone())])
                .unwrap_or_default();
            let listing: LibraryListing = self.get(LIBRARY_COLLECTION_PATH, &params).await?;
            for envelope in listing.records {
                if envelope.collection != LIBRARY_COLLECTION {
                    return Err(SyncError::Protocol);
                }
                if envelope.key != LIBRARY_KEY {
                    continue;
                }
                if envelope.deleted {
                    return Ok(None);
                }
                let Some(record) = envelope.value else {
                    return Ok(None);
                };
                return Ok(Some((record, revision_token(&envelope.revision)?)));
            }
            match listing.next_after {
                Some(next) => after = Some(next),
                None => return Ok(None),
            }
        }
        Err(SyncError::Protocol)
    }

    /// Put the library record using the same compare-and-set headers as
    /// selection sync: `If-Match` with a quoted expected revision, or
    /// `If-None-Match: *` when no record is expected to exist. Returns the new
    /// revision token.
    pub async fn library_put(
        &self,
        record: &LibraryRecord,
        expected_precondition: Option<&str>,
    ) -> Result<String, SyncError> {
        let mut request = self
            .request(reqwest::Method::PUT, LIBRARY_RECORD_PATH)?
            .json(record);
        request = if let Some(expected) = expected_precondition {
            request.header(IF_MATCH, format!("\"{expected}\""))
        } else {
            request.header(IF_NONE_MATCH, "*")
        };
        let response = request.send().await.map_err(|_| SyncError::Network)?;
        let envelope: LibraryEnvelope = decode(response).await?;
        if envelope.collection != LIBRARY_COLLECTION
            || envelope.key != LIBRARY_KEY
            || envelope.deleted
            || envelope.value.is_none()
        {
            return Err(SyncError::Protocol);
        }
        revision_token(&envelope.revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> LibraryRecord {
        LibraryRecord {
            schema_version: 1,
            origin: "ui".to_owned(),
            revision: "rev-1".to_owned(),
            snapshot: json!({"events": [], "actions": []}),
            updated_at: Some(1_700_000_000),
        }
    }

    #[test]
    fn library_paths() {
        assert_eq!(LIBRARY_COLLECTION, "automation-profiles");
        assert_eq!(LIBRARY_KEY, "library");
        assert_eq!(LIBRARY_HEAD_PATH, "api/v1/native/sync/head");
        assert_eq!(LIBRARY_CHANGES_PATH, "api/v1/native/sync/changes");
        assert_eq!(
            LIBRARY_COLLECTION_PATH,
            "api/v1/native/sync/collections/automation-profiles"
        );
    }

    #[test]
    fn library_record_round_trip() {
        let record = sample();
        let encoded = serde_json::to_value(&record).unwrap();
        assert_eq!(encoded["schemaVersion"], 1);
        assert_eq!(encoded["origin"], "ui");
        assert_eq!(encoded["updatedAt"], 1_700_000_000u64);
        let decoded: LibraryRecord = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, record);
    }

    #[test]
    fn library_record_rejects_unknown_field() {
        let value = json!({
            "schemaVersion": 1,
            "origin": "ui",
            "revision": "rev-1",
            "snapshot": {},
            "updatedAt": null,
            "unexpected": true,
        });
        assert!(serde_json::from_value::<LibraryRecord>(value).is_err());
    }

    #[test]
    fn library_record_rejects_other_schema_version() {
        let value = json!({
            "schemaVersion": 2,
            "origin": "ui",
            "revision": "rev-1",
            "snapshot": {},
            "updatedAt": null,
        });
        assert!(serde_json::from_value::<LibraryRecord>(value).is_err());
    }

    #[test]
    fn library_record_rejects_invalid_origin() {
        let value = json!({
            "schemaVersion": 1,
            "origin": "server",
            "revision": "rev-1",
            "snapshot": {},
            "updatedAt": null,
        });
        assert!(serde_json::from_value::<LibraryRecord>(value).is_err());
    }
}
