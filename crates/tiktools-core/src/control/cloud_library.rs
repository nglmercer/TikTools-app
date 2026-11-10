//! Read-only Cloud library discovery. Cloud documents never become local
//! executable definitions without a separate native review and application.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::OperationError;
use crate::AppCore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CloudLibraryScope {
    Public,
    Mine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CloudLibraryKind {
    Template,
    Profile,
}
impl CloudLibraryKind {
    fn wire(self) -> &'static str {
        match self {
            Self::Template => "template",
            Self::Profile => "profile",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudLibraryQuery {
    pub scope: CloudLibraryScope,
    pub kind: Option<CloudLibraryKind>,
    pub query: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudLibraryRead {
    pub scope: CloudLibraryScope,
    pub id: String,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CloudLibraryStatus {
    Draft,
    Pending,
    Published,
    Rejected,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudLibraryDocumentMeta {
    pub id: String,
    pub kind: CloudLibraryKind,
    pub slug: String,
    /// Exact decimal string; never a JavaScript number.
    pub revision: String,
    pub title: String,
    pub sha256: String,
    pub status: CloudLibraryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudLibraryPage {
    pub items: Vec<CloudLibraryDocumentMeta>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudLibraryDocument {
    pub meta: CloudLibraryDocumentMeta,
    pub document: Value,
}

fn uuid(value: &str) -> Result<Uuid, OperationError> {
    let id =
        Uuid::parse_str(value).map_err(|_| OperationError::invalid("invalid Cloud document ID"))?;
    if id.to_string() != value {
        return Err(OperationError::invalid("invalid Cloud document ID"));
    }
    Ok(id)
}
fn revision(value: Option<&str>) -> Result<Option<i64>, OperationError> {
    value
        .map(|value| {
            if value.is_empty()
                || value.starts_with('0')
                || !value.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(OperationError::invalid("invalid Cloud revision"));
            }
            value
                .parse::<i64>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or_else(|| OperationError::invalid("invalid Cloud revision"))
        })
        .transpose()
}
fn project_meta(value: &mut Value) -> Result<(), OperationError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| OperationError::internal("invalid Cloud document"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| OperationError::internal("invalid Cloud document ID"))?;
    object.insert("id".into(), Value::String(id.to_string()));
    let revision = object
        .get("revision")
        .and_then(Value::as_i64)
        .filter(|n| *n > 0)
        .ok_or_else(|| OperationError::internal("invalid Cloud revision"))?;
    object.insert("revision".into(), Value::String(revision.to_string()));
    Ok(())
}
fn project_page(mut value: Value) -> Result<CloudLibraryPage, OperationError> {
    let items = value
        .get_mut("items")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| OperationError::internal("invalid Cloud document page"))?;
    for item in items {
        project_meta(item)?;
    }
    if let Some(cursor) = value.get_mut("nextCursor") {
        if !cursor.is_null() {
            let id = cursor
                .as_str()
                .and_then(|cursor| Uuid::parse_str(cursor).ok())
                .ok_or_else(|| OperationError::internal("invalid Cloud document cursor"))?;
            *cursor = Value::String(id.to_string());
        }
    }
    serde_json::from_value(value)
        .map_err(|_| OperationError::internal("invalid Cloud document page"))
}
fn project_document(mut value: Value) -> Result<CloudLibraryDocument, OperationError> {
    let document = value
        .as_object_mut()
        .and_then(|object| object.remove("document"))
        .ok_or_else(|| OperationError::internal("invalid Cloud document"))?;
    project_meta(&mut value)?;
    let meta = serde_json::from_value(value)
        .map_err(|_| OperationError::internal("invalid Cloud document"))?;
    Ok(CloudLibraryDocument { meta, document })
}
fn cloud_error(error: cloud_client::SyncError) -> OperationError {
    match error {
        cloud_client::SyncError::Configuration => {
            OperationError::invalid("invalid Cloud library request")
        }
        cloud_client::SyncError::Http(404) => OperationError::not_found("Cloud document not found"),
        _ => OperationError::unavailable("Cloud library is unavailable"),
    }
}
fn catalog() -> Result<cloud_client::CloudCatalog, OperationError> {
    let origin = tiktools_cloud_auth::resolve_cloud_origin()
        .ok_or_else(|| OperationError::unavailable("Cloud is not configured"))?;
    cloud_client::CloudCatalog::new(&origin).map_err(cloud_error)
}

async fn private_page(
    auth: &tiktools_cloud_auth::CloudAuth,
    kind: Option<&str>,
    query: Option<&str>,
    after: Option<Uuid>,
) -> Result<CloudLibraryPage, OperationError> {
    let session = auth
        .session()
        .await
        .ok_or_else(|| OperationError::unavailable("sign in to view private Cloud documents"))?;
    let result = session.library_documents_filtered(kind, query, after).await;
    auth.with_current_session(&session, || {
        result.map_err(cloud_error).and_then(project_page)
    })
    .map_err(|_| OperationError::conflict("Cloud session changed; refresh the library"))?
}

async fn private_document(
    auth: &tiktools_cloud_auth::CloudAuth,
    id: Uuid,
    revision: Option<i64>,
) -> Result<CloudLibraryDocument, OperationError> {
    let session = auth
        .session()
        .await
        .ok_or_else(|| OperationError::unavailable("sign in to view private Cloud documents"))?;
    let result = match revision {
        Some(revision) => session.library_version(id, revision).await,
        None => session.library_document(id).await,
    };
    auth.with_current_session(&session, || {
        result.map_err(cloud_error).and_then(project_document)
    })
    .map_err(|_| OperationError::conflict("Cloud session changed; refresh the library"))?
}

impl AppCore {
    /// Fetch one metadata page. Private data is returned only for the exact
    /// captured account session; account changes invalidate late responses.
    pub async fn cloud_library_list(
        &self,
        query: CloudLibraryQuery,
    ) -> Result<CloudLibraryPage, OperationError> {
        let after = query.after.as_deref().map(uuid).transpose()?;
        if query.query.as_ref().is_some_and(|q| q.len() > 100) {
            return Err(OperationError::invalid("invalid Cloud library query"));
        }
        let kind = query.kind.map(CloudLibraryKind::wire);
        match query.scope {
            CloudLibraryScope::Public => project_page(
                catalog()?
                    .documents_filtered(kind, query.query.as_deref(), after)
                    .await
                    .map_err(cloud_error)?,
            ),
            CloudLibraryScope::Mine => {
                private_page(self.cloud.auth(), kind, query.query.as_deref(), after).await
            }
        }
    }

    /// Fetch the exact requested Cloud revision as data only; no import or
    /// activation occurs. Public reads expose only published revisions.
    pub async fn cloud_library_read(
        &self,
        request: CloudLibraryRead,
    ) -> Result<CloudLibraryDocument, OperationError> {
        let id = uuid(&request.id)?;
        let revision = revision(request.revision.as_deref())?;
        match request.scope {
            CloudLibraryScope::Public => project_document(
                catalog()?
                    .document(id, revision)
                    .await
                    .map_err(cloud_error)?,
            ),
            CloudLibraryScope::Mine => private_document(self.cloud.auth(), id, revision).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use std::{path::PathBuf, sync::Arc};
    use tiktools_cloud_auth::{CloudAuth, Persistence, TokenStore, TokenStoreError};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::Notify,
    };

    struct Store;
    impl TokenStore for Store {
        fn persistence(&self) -> Persistence {
            Persistence::MemoryOnly
        }
        fn load(&self, _: &str, _: &str) -> Result<Option<SecretString>, TokenStoreError> {
            Ok(Some(SecretString::from("fixture-token".to_owned())))
        }
        fn store(
            &self,
            _: &str,
            _: &str,
            _: &cloud_client::Session,
        ) -> Result<(), TokenStoreError> {
            Ok(())
        }
        fn delete(&self, _: &str, _: &str) -> Result<(), TokenStoreError> {
            Ok(())
        }
    }
    struct TestDir(PathBuf);
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn typed_projection_preserves_revision_and_rejects_invalid_request_identifiers() {
        let id = "f2f3f4f5-0102-4304-8506-070809101112";
        let page = project_page(serde_json::json!({"items":[{
            "id":id,"kind":"template","slug":"greeting","revision":99,
            "title":"Greeting","sha256":"a".repeat(64),"status":"published"
        }],"nextCursor":null}))
        .unwrap();
        assert_eq!(page.items[0].revision, "99");
        assert_eq!(page.items[0].id, id);
        assert_eq!(uuid(id).unwrap().to_string(), id);
        assert!(uuid("F2F3F4F5-0102-4304-8506-070809101112").is_err());
        assert!(revision(Some("01")).is_err());
        assert!(revision(Some("0")).is_err());
        let doc = project_document(serde_json::json!({
            "id":id,"kind":"template","slug":"greeting","revision":1,
            "title":"Greeting","sha256":"a".repeat(64),"status":"published",
            "document":{"id":"greeting","title":"Greeting"}
        }))
        .unwrap();
        assert_eq!(doc.meta.revision, "1");
        assert_eq!(doc.document["id"], "greeting");
        let uppercase = id.to_ascii_uppercase();
        let page = project_page(serde_json::json!({"items":[{
            "id":uppercase,"kind":"template","slug":"greeting","revision":1,
            "title":"Greeting","sha256":"a".repeat(64),"status":"published"
        }],"nextCursor":uppercase}))
        .unwrap();
        assert_eq!(page.items[0].id, id);
        assert_eq!(page.next_cursor.as_deref(), Some(id));
        assert!(uuid(page.next_cursor.as_deref().unwrap()).is_ok());
        let doc = project_document(serde_json::json!({
            "id":uppercase,"kind":"template","slug":"greeting","revision":1,
            "title":"Greeting","sha256":"a".repeat(64),"status":"published",
            "document":{"id":"greeting","title":"Greeting"}
        }))
        .unwrap();
        assert_eq!(doc.meta.id, id);
    }

    #[tokio::test]
    async fn uppercase_response_ids_remain_usable_for_private_pagination() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let observed = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let server = {
            let observed = observed.clone();
            tokio::spawn(async move {
                while let Ok((mut socket, _)) = listener.accept().await {
                    let observed = observed.clone();
                    tokio::spawn(async move {
                        let mut request = Vec::new();
                        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                            let mut bytes = [0u8; 1024];
                            let Ok(n) = socket.read(&mut bytes).await else {
                                return;
                            };
                            if n == 0 || request.len() > 16 * 1024 {
                                return;
                            }
                            request.extend_from_slice(&bytes[..n]);
                        }
                        let line = String::from_utf8_lossy(&request)
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .to_owned();
                        let body = if line.starts_with("GET /api/v1/native/me ") {
                            serde_json::json!({"id":"alice"})
                        } else if line.starts_with("GET /api/v2/library/documents") {
                            observed.lock().unwrap().push(line.clone());
                            if line.contains("after=") {
                                serde_json::json!({"items":[],"nextCursor":null})
                            } else {
                                let ids: Vec<_> = (1..=50)
                                    .map(|n| {
                                        Uuid::from_u128(0xabcdef00000040008000000000000000_u128 + n)
                                            .to_string()
                                            .to_ascii_uppercase()
                                    })
                                    .collect();
                                let items: Vec<_> = ids.iter().map(|id| serde_json::json!({
                                    "id":id,"kind":"template","slug":"greeting","revision":1,
                                    "title":"Greeting","sha256":"a".repeat(64),"status":"draft"
                                })).collect();
                                serde_json::json!({"items":items,"nextCursor":ids.last()})
                            }
                        } else {
                            serde_json::json!({"error":"unexpected request"})
                        };
                        let bytes = serde_json::to_vec(&body).unwrap();
                        let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len());
                        let _ = socket.write_all(head.as_bytes()).await;
                        let _ = socket.write_all(&bytes).await;
                    });
                }
            })
        };
        let root = std::env::temp_dir().join(format!(
            "cloud-library-cursor-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let _dir = TestDir(root.clone());
        let sidecar = root.join("account.json");
        std::fs::write(
            &sidecar,
            serde_json::to_vec(&serde_json::json!({"origin":origin,"account_id":"alice"})).unwrap(),
        )
        .unwrap();
        let auth = CloudAuth::new(Some(origin), sidecar, Arc::new(Store));
        auth.restore().await.unwrap();
        let page = private_page(&auth, None, None, None).await.unwrap();
        let cursor = page.next_cursor.unwrap();
        assert_eq!(
            cursor,
            Uuid::from_u128(0xabcdef00000040008000000000000000_u128 + 50).to_string()
        );
        assert_eq!(page.items.last().unwrap().id, cursor);
        assert!(
            private_page(&auth, None, None, Some(uuid(&cursor).unwrap()))
                .await
                .unwrap()
                .items
                .is_empty()
        );
        let requests = observed.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].contains(&format!("after={cursor}")));
        server.abort();
    }

    #[tokio::test]
    async fn private_page_discards_a_response_after_same_account_relogin() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let server = {
            let seen = seen.clone();
            let release = release.clone();
            tokio::spawn(async move {
                while let Ok((mut socket, _)) = listener.accept().await {
                    let seen = seen.clone();
                    let release = release.clone();
                    tokio::spawn(async move {
                        let mut request = Vec::new();
                        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                            let mut bytes = [0u8; 1024];
                            let Ok(n) = socket.read(&mut bytes).await else {
                                return;
                            };
                            if n == 0 || request.len() > 16 * 1024 {
                                return;
                            }
                            request.extend_from_slice(&bytes[..n]);
                        }
                        let first = String::from_utf8_lossy(&request)
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .to_owned();
                        let (status, body) = if first.starts_with("GET /api/v1/native/me ") {
                            ("200 OK", "{\"id\":\"alice\"}")
                        } else if first.starts_with("GET /api/v2/library/documents") {
                            seen.notify_one();
                            release.notified().await;
                            ("200 OK", "{\"items\":[],\"nextCursor\":null}")
                        } else if first.starts_with("POST /api/v1/native/logout ") {
                            ("204 No Content", "")
                        } else {
                            ("404 Not Found", "{}")
                        };
                        let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                        let _ = socket.write_all(response.as_bytes()).await;
                    });
                }
            })
        };
        let root = std::env::temp_dir().join(format!(
            "cloud-library-test-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let _dir = TestDir(root.clone());
        let sidecar = root.join("account.json");
        std::fs::write(
            &sidecar,
            serde_json::to_vec(&serde_json::json!({"origin":origin,"account_id":"alice"})).unwrap(),
        )
        .unwrap();
        let auth = CloudAuth::new(Some(origin.clone()), sidecar.clone(), Arc::new(Store));
        auth.restore().await.unwrap();
        let request = {
            let auth = auth.clone();
            tokio::spawn(async move { private_page(&auth, None, None, None).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), seen.notified())
            .await
            .unwrap();
        auth.logout().await.unwrap();
        std::fs::write(
            &sidecar,
            serde_json::to_vec(&serde_json::json!({"origin":origin,"account_id":"alice"})).unwrap(),
        )
        .unwrap();
        auth.restore().await.unwrap();
        release.notify_one();
        assert_eq!(request.await.unwrap().unwrap_err().code, "conflict");
        server.abort();
    }
}
