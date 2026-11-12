use super::*;

impl CloudPublisher {
    /// Submits an uploaded artifact as a plugin release for review.
    pub(crate) async fn submit(
        &self,
        auth: &CloudAuth,
        params: PublisherSubmitParams,
    ) -> Result<PublisherReleaseResult, crate::control::OperationError> {
        use crate::control::OperationError;
        if params.plugin_id.trim().is_empty() || params.plugin_id.len() > 256 {
            return Err(OperationError::invalid(
                "plugin id must be 1-256 characters",
            ));
        }
        if semver::Version::parse(&params.version).is_err() {
            return Err(OperationError::invalid(
                "version must be semantic (e.g. 1.2.3)",
            ));
        }
        let (origin, token) = self.credentials(auth).await?;
        let release: PublisherReleaseView = self
            .post_json(
                &origin,
                &token,
                "/api/v1/plugins/releases",
                None,
                &ReleaseSubmitRequest {
                    upload_id: params.upload_id,
                    publisher_key_id: params.publisher_key_id,
                    plugin_id: params.plugin_id,
                    version: params.version,
                    target: if params.target.trim().is_empty() {
                        default_target()
                    } else {
                        params.target
                    },
                },
            )
            .await?;
        Ok(PublisherReleaseResult { release })
    }

    /// Fetches one release for status review.
    pub(crate) async fn release(
        &self,
        auth: &CloudAuth,
        params: PublisherReleaseParams,
    ) -> Result<PublisherReleaseResult, crate::control::OperationError> {
        let (origin, token) = self.credentials(auth).await?;
        let release: PublisherReleaseView = self
            .get_json(
                &origin,
                &token,
                &format!("/api/v1/plugins/releases/{}", params.id),
            )
            .await?;
        Ok(PublisherReleaseResult { release })
    }

    /// Requests publication review for a release. Requires a reviewer role;
    /// the backend enforces it and reports promotion progress.
    pub(crate) async fn publish(
        &self,
        auth: &CloudAuth,
        params: PublisherReleaseParams,
    ) -> Result<PublisherPublicationResult, crate::control::OperationError> {
        let (origin, token) = self.credentials(auth).await?;
        let result: PublisherPublicationResult = self
            .post_json(
                &origin,
                &token,
                &format!("/api/v1/plugins/releases/{}/publish", params.id),
                None,
                &serde_json::json!({}),
            )
            .await?;
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReleaseSubmitRequest {
    upload_id: String,
    publisher_key_id: String,
    plugin_id: String,
    version: String,
    target: String,
}
