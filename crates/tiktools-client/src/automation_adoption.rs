//! Typed local-owner adoption coordinator calls. Command IDs stay on the host.
use crate::{ClientError, TikToolsClient};
pub use tiktools_control_api::modules::automation_adoption::{
    OwnerAdoptionReview, OwnerAdoptionState, OwnerAdoptionSubmit,
};
use tiktools_control_api::modules::Empty;
pub(crate) const METHODS: &[&str] = &[GET, SUBMIT, RESOLVE, REVIEW];
const GET: &str = "automation.adoption.get";
const SUBMIT: &str = "automation.adoption.submit";
const RESOLVE: &str = "automation.adoption.resolve";
const REVIEW: &str = "automation.adoption.review_unknown";
impl TikToolsClient {
    pub async fn automation_adoption_get(&self) -> Result<OwnerAdoptionState, ClientError> {
        self.call(GET, Empty {}).await
    }
    pub async fn automation_adoption_submit(
        &self,
        params: OwnerAdoptionSubmit,
    ) -> Result<OwnerAdoptionState, ClientError> {
        self.call(SUBMIT, params).await
    }
    pub async fn automation_adoption_resolve(&self) -> Result<OwnerAdoptionState, ClientError> {
        self.call(RESOLVE, Empty {}).await
    }
    pub async fn automation_adoption_review_unknown(
        &self,
        params: OwnerAdoptionReview,
    ) -> Result<OwnerAdoptionState, ClientError> {
        self.call(REVIEW, params).await
    }
}
