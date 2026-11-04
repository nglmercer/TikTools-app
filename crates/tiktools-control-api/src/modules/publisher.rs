//! Cloud publisher operations: namespaces, signing keys, artifact uploads
//! and release submissions. Every response is nonsecret; the session token,
//! the backend password and private keys never cross this boundary.

use std::sync::Arc;

use tiktools_core::AppCore;
pub use tiktools_core::cloud_publisher::{
    PublisherCreateParams, PublisherEnrollParams, PublisherEnrollResult, PublisherIdParams,
    PublisherKeysResult, PublisherListResult, PublisherPublicationResult, PublisherReleaseParams,
    PublisherReleaseResult, PublisherSubmitParams, PublisherSummary, PublisherUploadParams,
    PublisherUploadResult,
};

use super::Empty;
use crate::{error::ApiError, router::ControlRouter};

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, PublisherListResult, _, _>(
        "publisher.list",
        "Lists the Cloud publisher namespaces the signed-in account belongs to",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.publisher_list().await.map_err(ApiError::from)
        },
    );
    router.register_typed::<PublisherCreateParams, PublisherSummary, _, _>(
        "publisher.create",
        "Requests a Cloud publisher namespace (starts pending admin approval)",
        true,
        |core: Arc<AppCore>, params: PublisherCreateParams| async move {
            core.publisher_create(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<PublisherIdParams, PublisherKeysResult, _, _>(
        "publisher.keys",
        "Lists the signing keys of one publisher, pending and approved",
        false,
        |core: Arc<AppCore>, params: PublisherIdParams| async move {
            core.publisher_keys(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<PublisherEnrollParams, PublisherEnrollResult, _, _>(
        "publisher.keys.enroll",
        "Enrolls a signing key: step-up, challenge, local key generation and registration",
        true,
        |core: Arc<AppCore>, params: PublisherEnrollParams| async move {
            core.publisher_enroll(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<PublisherUploadParams, PublisherUploadResult, _, _>(
        "publisher.uploads.send",
        "Uploads a local plugin artifact to object storage for a publisher",
        true,
        |core: Arc<AppCore>, params: PublisherUploadParams| async move {
            core.publisher_upload(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<PublisherSubmitParams, PublisherReleaseResult, _, _>(
        "publisher.releases.submit",
        "Submits an uploaded artifact as a plugin release for review",
        true,
        |core: Arc<AppCore>, params: PublisherSubmitParams| async move {
            core.publisher_submit(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<PublisherReleaseParams, PublisherReleaseResult, _, _>(
        "publisher.releases.get",
        "Fetches one plugin release for status review",
        false,
        |core: Arc<AppCore>, params: PublisherReleaseParams| async move {
            core.publisher_release(params).await.map_err(ApiError::from)
        },
    );
    router.register_typed::<PublisherReleaseParams, PublisherPublicationResult, _, _>(
        "publisher.publication.request",
        "Requests publication review for a release (needs a reviewer role)",
        true,
        |core: Arc<AppCore>, params: PublisherReleaseParams| async move {
            core.publisher_publish(params).await.map_err(ApiError::from)
        },
    );
}
