use super::*;

impl CloudPublisher {
    /// Uploads a local `.plugin` artifact: reserves the upload, PUTs the
    /// bytes to object storage, then completes the session. Returns the
    /// upload id for `releases.submit`.
    pub(crate) async fn upload(
        &self,
        auth: &CloudAuth,
        params: PublisherUploadParams,
    ) -> Result<PublisherUploadResult, crate::control::OperationError> {
        use crate::control::OperationError;
        let (origin, token) = self.credentials(auth).await?;
        let publisher_id = self
            .resolve_publisher_id(&origin, &token, &params.publisher_id)
            .await?;
        use std::io::Read;
        let file = std::fs::File::open(&params.path).map_err(|error| {
            OperationError::not_found(format!("could not read {}: {error}", params.path))
        })?;
        // Bound the read itself, not just the validation after allocation.
        let mut bytes = Vec::new();
        file.take(MAX_ARTIFACT_SIZE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| OperationError::unavailable("could not read artifact"))?;
        if bytes.is_empty() || bytes.len() as i64 > MAX_ARTIFACT_SIZE {
            return Err(OperationError::invalid(
                "artifact must be non-empty and at most 128 MiB",
            ));
        }
        let sha256 = Sha256Digest::of_bytes(&bytes).to_string();
        // Deterministic per distinct upload so a retry reuses the reservation
        // instead of creating a duplicate. Lowercase hex fits the backend's
        // idempotency-key charset.
        let idempotency =
            Sha256Digest::of_bytes(format!("{}:{}:{sha256}", publisher_id, bytes.len()).as_bytes())
                .to_string();
        let created: UploadCreateResponse = self
            .post_json(
                &origin,
                &token,
                "/api/v1/plugins/uploads",
                Some(&idempotency),
                &UploadCreateRequest {
                    publisher_id,
                    size: bytes.len() as i64,
                    sha256,
                },
            )
            .await?;
        let upload_url = safe_upload_url(&created.url)
            .ok_or_else(|| OperationError::invalid("cloud returned an unsafe upload URL"))?;
        // The signed URL is a bearer capability: no Authorization header, and a
        // transport error is reported without it.
        let mut put = cloud_http::client()?.put(upload_url);
        for (name, value) in &created.headers {
            put = put.header(name.as_str(), value.as_str());
        }
        // Only the status is used; the reply body is dropped unread.
        let put_status = put
            .body(bytes)
            .timeout(UPLOAD_TIMEOUT)
            .send()
            .await
            .map_err(|_| OperationError::unavailable("artifact upload failed"))?
            .status();
        if !put_status.is_success() {
            return Err(OperationError::unavailable(format!(
                "object storage refused the artifact ({put_status})"
            )));
        }
        #[derive(Deserialize)]
        struct CompleteResponse {
            #[allow(dead_code)]
            id: String,
            status: String,
        }
        let completed: CompleteResponse = self
            .post_json(
                &origin,
                &token,
                &format!("/api/v1/plugins/uploads/{}/complete", created.id),
                None,
                &serde_json::json!({}),
            )
            .await?;
        Ok(PublisherUploadResult {
            upload_id: created.id,
            status: completed.status,
        })
    }
}

pub(super) fn safe_upload_url(raw: &str) -> Option<reqwest::Url> {
    let url = reqwest::Url::parse(raw).ok()?;
    let loopback = url.host_str().is_some_and(|host| {
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    });
    if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadCreateRequest {
    publisher_id: String,
    size: i64,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadCreateResponse {
    id: String,
    url: String,
    headers: std::collections::BTreeMap<String, String>,
}
