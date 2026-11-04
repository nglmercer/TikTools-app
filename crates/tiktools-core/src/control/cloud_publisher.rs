//! Headless Cloud publisher operations. Everything crossing this boundary
//! is nonsecret (namespace and key metadata, release status); the session
//! token and the backend password stay inside native code, and private
//! signing keys never leave their owner-only files.

use super::OperationError;
use crate::{
    cloud_publisher::{
        PublisherCreateParams, PublisherEnrollParams, PublisherEnrollResult, PublisherIdParams,
        PublisherKeysResult, PublisherListResult, PublisherPublicationResult, PublisherReleaseParams,
        PublisherReleaseResult, PublisherSubmitParams, PublisherSummary, PublisherUploadParams,
        PublisherUploadResult,
    },
    AppCore,
};

impl AppCore {
    fn publisher(&self) -> &crate::cloud_publisher::CloudPublisher {
        &self.publisher
    }

    fn auth(&self) -> &tiktools_cloud_auth::CloudAuth {
        self.cloud.auth()
    }

    pub async fn publisher_list(&self) -> Result<PublisherListResult, OperationError> {
        self.publisher().list(self.auth()).await
    }

    pub async fn publisher_create(
        &self,
        params: PublisherCreateParams,
    ) -> Result<PublisherSummary, OperationError> {
        self.publisher().create(self.auth(), params).await
    }

    pub async fn publisher_keys(
        &self,
        params: PublisherIdParams,
    ) -> Result<PublisherKeysResult, OperationError> {
        self.publisher().keys(self.auth(), params).await
    }

    pub async fn publisher_enroll(
        &self,
        params: PublisherEnrollParams,
    ) -> Result<PublisherEnrollResult, OperationError> {
        self.publisher().enroll(self.auth(), params).await
    }

    pub async fn publisher_upload(
        &self,
        params: PublisherUploadParams,
    ) -> Result<PublisherUploadResult, OperationError> {
        self.publisher().upload(self.auth(), params).await
    }

    pub async fn publisher_submit(
        &self,
        params: PublisherSubmitParams,
    ) -> Result<PublisherReleaseResult, OperationError> {
        self.publisher().submit(self.auth(), params).await
    }

    pub async fn publisher_release(
        &self,
        params: PublisherReleaseParams,
    ) -> Result<PublisherReleaseResult, OperationError> {
        self.publisher().release(self.auth(), params).await
    }

    pub async fn publisher_publish(
        &self,
        params: PublisherReleaseParams,
    ) -> Result<PublisherPublicationResult, OperationError> {
        self.publisher().publish(self.auth(), params).await
    }
}
