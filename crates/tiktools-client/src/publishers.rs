use super::TikToolsClient;
use tiktools_control_api::modules::publisher::{
    PublisherCreateParams, PublisherDeleteResult, PublisherEnrollParams, PublisherEnrollResult,
    PublisherIdParams, PublisherKeysResult, PublisherListResult, PublisherPublicationResult,
    PublisherReleaseParams, PublisherReleaseResult, PublisherSubmitParams, PublisherSummary,
    PublisherUploadParams, PublisherUploadResult,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    PUBLISHER_LIST,
    PUBLISHER_CREATE,
    PUBLISHER_DELETE,
    PUBLISHER_KEYS,
    PUBLISHER_KEYS_ENROLL,
    PUBLISHER_UPLOADS_SEND,
    PUBLISHER_RELEASES_SUBMIT,
    PUBLISHER_RELEASES_GET,
    PUBLISHER_PUBLICATION_REQUEST,
];

const PUBLISHER_LIST: &str = "publisher.list";
const PUBLISHER_CREATE: &str = "publisher.create";
const PUBLISHER_DELETE: &str = "publisher.delete";
const PUBLISHER_KEYS: &str = "publisher.keys";
const PUBLISHER_KEYS_ENROLL: &str = "publisher.keys.enroll";
const PUBLISHER_UPLOADS_SEND: &str = "publisher.uploads.send";
const PUBLISHER_RELEASES_SUBMIT: &str = "publisher.releases.submit";
const PUBLISHER_RELEASES_GET: &str = "publisher.releases.get";
const PUBLISHER_PUBLICATION_REQUEST: &str = "publisher.publication.request";

impl TikToolsClient {
    pub async fn publisher_list(&self) -> Result<PublisherListResult, ClientError> {
        self.call(PUBLISHER_LIST, Empty::default()).await
    }

    pub async fn publisher_create(
        &self,
        params: PublisherCreateParams,
    ) -> Result<PublisherSummary, ClientError> {
        self.call(PUBLISHER_CREATE, params).await
    }

    pub async fn publisher_delete(
        &self,
        params: PublisherIdParams,
    ) -> Result<PublisherDeleteResult, ClientError> {
        self.call(PUBLISHER_DELETE, params).await
    }

    pub async fn publisher_keys(
        &self,
        params: PublisherIdParams,
    ) -> Result<PublisherKeysResult, ClientError> {
        self.call(PUBLISHER_KEYS, params).await
    }

    pub async fn publisher_enroll(
        &self,
        params: PublisherEnrollParams,
    ) -> Result<PublisherEnrollResult, ClientError> {
        self.call(PUBLISHER_KEYS_ENROLL, params).await
    }

    pub async fn publisher_upload(
        &self,
        params: PublisherUploadParams,
    ) -> Result<PublisherUploadResult, ClientError> {
        self.call(PUBLISHER_UPLOADS_SEND, params).await
    }

    pub async fn publisher_submit(
        &self,
        params: PublisherSubmitParams,
    ) -> Result<PublisherReleaseResult, ClientError> {
        self.call(PUBLISHER_RELEASES_SUBMIT, params).await
    }

    pub async fn publisher_release(
        &self,
        params: PublisherReleaseParams,
    ) -> Result<PublisherReleaseResult, ClientError> {
        self.call(PUBLISHER_RELEASES_GET, params).await
    }

    pub async fn publisher_publish(
        &self,
        params: PublisherReleaseParams,
    ) -> Result<PublisherPublicationResult, ClientError> {
        self.call(PUBLISHER_PUBLICATION_REQUEST, params).await
    }
}
