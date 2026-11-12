//! Bounded remote page ingestion and explicit cursor-expiry marking.
use super::super::{
    check_eligibility, ProductUploadAdmission, WorkspaceCoordError, WorkspaceCoordinator,
    WorkspaceEligibility,
};
use cloud_client::{Session, SyncError, WorkspaceState, WorkspaceStateError};

const MAX_PULL_PAGES: usize = 200;

impl WorkspaceCoordinator {
    pub(super) async fn pull_workspace_pages(
        &self,
        session: &Session,
        eligibility: Option<&WorkspaceEligibility>,
        product: Option<&ProductUploadAdmission<'_>>,
    ) -> Result<usize, WorkspaceCoordError> {
        let mut pulled = 0;
        let mut complete = false;
        for _ in 0..MAX_PULL_PAGES {
            check_eligibility(eligibility, session)?;
            let cursor = self.snapshot()?.state.cursor;
            check_eligibility(eligibility, session)?;
            let response = session.workspace_changes(cursor).await;
            check_eligibility(eligibility, session)?;
            let page = match response {
                Ok(page) => page,
                Err(SyncError::CursorExpired) => {
                    let mark = |state: &mut WorkspaceState| {
                        state.require_rebuild();
                        Ok(())
                    };
                    if product.is_some() {
                        self.update_product(
                            eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                            mark,
                        )?;
                    } else {
                        self.update(mark)?;
                    }
                    return Err(WorkspaceStateError::NeedsRebuild.into());
                }
                Err(error) => return Err(error.into()),
            };
            check_eligibility(eligibility, session)?;
            pulled += page.changes.len();
            let apply = |state: &mut WorkspaceState| {
                if state.cursor != cursor {
                    return Err(WorkspaceStateError::StaleReview.into());
                }
                Ok(state.apply_changes(page.clone())?)
            };
            let more = if product.is_some() {
                self.update_product(eligibility.ok_or(WorkspaceCoordError::Ineligible)?, apply)?
            } else {
                self.update(apply)?
            };
            if !more {
                complete = true;
                break;
            }
        }
        if !complete {
            return Err(SyncError::TooManyPages.into());
        }
        Ok(pulled)
    }
}
