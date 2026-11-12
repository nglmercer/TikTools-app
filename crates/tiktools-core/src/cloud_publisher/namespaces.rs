use super::*;

impl CloudPublisher {
    /// Accepts a publisher UUID or a namespace and returns the UUID. The CLI
    /// works in human namespaces; the backend keys everything by id.
    pub(super) async fn resolve_publisher_id(
        &self,
        origin: &str,
        token: &NativeToken,
        input: &str,
    ) -> Result<String, crate::control::OperationError> {
        use crate::control::OperationError;
        let trimmed = input.trim();
        if is_uuid(trimmed) {
            return Ok(trimmed.to_owned());
        }
        let publishers: Vec<PublisherSummary> =
            self.get_json(origin, token, "/api/v1/publishers").await?;
        publishers
            .into_iter()
            .find(|publisher| publisher.namespace == trimmed)
            .map(|publisher| publisher.id)
            .ok_or_else(|| {
                OperationError::not_found(format!("no publisher named `{trimmed}` in your account"))
            })
    }

    /// Namespaces the signed-in account belongs to.
    pub(crate) async fn list(
        &self,
        auth: &CloudAuth,
    ) -> Result<PublisherListResult, crate::control::OperationError> {
        let (origin, token) = self.credentials(auth).await?;
        let publishers: Vec<PublisherSummary> =
            self.get_json(&origin, &token, "/api/v1/publishers").await?;
        Ok(PublisherListResult { publishers })
    }

    /// Requests a publisher namespace. Starts `pending`; a Cloud admin
    /// approves it before anything can be published under it.
    pub(crate) async fn create(
        &self,
        auth: &CloudAuth,
        params: PublisherCreateParams,
    ) -> Result<PublisherSummary, crate::control::OperationError> {
        use crate::control::OperationError;
        let namespace = params.namespace.trim().to_owned();
        if !valid_namespace(&namespace) {
            return Err(OperationError::invalid(
                "namespace must be 2-63 lowercase letters, digits or hyphens",
            ));
        }
        let (origin, token) = self.credentials(auth).await?;
        self.post_json(
            &origin,
            &token,
            "/api/v1/publishers",
            None,
            &serde_json::json!({ "namespace": namespace }),
        )
        .await
    }

    /// Deletes a publisher namespace the caller owns. Accepts the namespace
    /// or the publisher id. The backend refuses (409) unless the namespace
    /// has no other members, keys, uploads, releases, transfers or step-up
    /// records.
    pub(crate) async fn delete(
        &self,
        auth: &CloudAuth,
        params: PublisherIdParams,
    ) -> Result<PublisherDeleteResult, crate::control::OperationError> {
        let (origin, token) = self.credentials(auth).await?;
        let publisher_id = self
            .resolve_publisher_id(&origin, &token, &params.publisher_id)
            .await?;
        self.delete_empty(
            &origin,
            &token,
            &format!("/api/v1/publishers/{publisher_id}"),
        )
        .await?;
        Ok(PublisherDeleteResult {
            publisher_id,
            deleted: true,
        })
    }
}

pub(super) fn is_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes[8] == b'-'
        && bytes[13] == b'-'
        && bytes[18] == b'-'
        && bytes[23] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 8 | 13 | 18 | 23) || byte.is_ascii_hexdigit())
}

pub(super) fn valid_namespace(value: &str) -> bool {
    let bytes = value.as_bytes();
    (2..=63).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes[bytes.len() - 1] != b'-'
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}
