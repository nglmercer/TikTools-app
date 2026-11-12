//! Durable ticket preparation, guarded emission and response publication.
use super::super::{
    check_eligibility, ProductUploadAdmission, WorkspaceCoordError, WorkspaceCoordinator,
    WorkspaceEligibility,
};
use cloud_client::{
    Session, SyncError, WorkspaceEdit, WorkspaceKey, WorkspaceState, WorkspaceStateError,
};

impl WorkspaceCoordinator {
    pub(super) async fn push_workspace_edit(
        &self,
        session: &Session,
        key: &WorkspaceKey,
        eligibility: Option<&WorkspaceEligibility>,
        product: Option<&ProductUploadAdmission<'_>>,
        attempts: &mut usize,
    ) -> Result<bool, WorkspaceCoordError> {
        // Product dispatch needs an authenticated fresh observation of
        // enforced epoch policy before preparing any uncertain request.
        // A restored epoch or a GET from an older server is not enough.
        let product_epoch = if product.is_some() {
            check_eligibility(eligibility, session)?;
            let epoch = session.workspace_write_epoch().await?;
            check_eligibility(eligibility, session)?;
            if epoch.epoch_required() != Some(true) {
                return Err(WorkspaceCoordError::Ineligible);
            }
            Some(epoch)
        } else {
            None
        };
        let (ticket, prepared_proof) = if let Some(product) = product {
            self.admit_product(
                key,
                eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                product,
                |proof| {
                    let ticket = self.update_product(
                        eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                        |state| {
                            Ok(state.prepare_product_push_fenced(
                                proof,
                                product_epoch
                                    .as_ref()
                                    .ok_or(WorkspaceCoordError::Ineligible)?,
                            )?)
                        },
                    )?;
                    Ok((ticket, Some(proof.clone())))
                },
            )?
        } else {
            (self.update(|state| Ok(state.prepare_push(key)?))?, None)
        };
        *attempts += 1;
        // Take the cross-process product fence after durable prepare and
        // before emission. A denied fence leaves this ticket Unconfirmed.
        // The product worker imposes a deadline and cancellation drops
        // the lease; the durable ticket retains any uncertain outcome.
        let emission_guard = product
            .map(|product| product.source.acquire_emission_guard())
            .transpose()?;
        // The source can change between durable preparation and emission
        // admission. Recheck the original receipt under its native source
        // fence after obtaining the cooperating emission lease. A stale
        // source leaves the already prepared ticket unconfirmed.
        if let Some(product) = product {
            self.admit_product_proof(
                prepared_proof
                    .as_ref()
                    .ok_or(WorkspaceCoordError::Ineligible)?,
                eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                product,
                |_| Ok(()),
            )?;
        }
        // Configuration reads may block. Do them before taking the
        // account's owned session permit. The v2 protocol lease held by
        // the product caller excludes cooperating consent changes.
        check_eligibility(eligibility, session)?;
        // Admit the exact captured CloudAuth session before the last
        // eligibility check. A later account transition waits for the
        // request future to finish or be dropped; its account mutex is
        // never held over network I/O. The lease does not prevent a
        // cancelled request already received by the server from committing.
        let session_guard = if product.is_some() {
            let guard = eligibility
                .and_then(|eligibility| eligibility.emission.as_ref())
                .ok_or(WorkspaceCoordError::Ineligible)?;
            Some(guard()?)
        } else {
            None
        };
        // This final check must not perform filesystem I/O while logout
        // waits for session_guard. It uses only nonblocking identity and
        // cancellation/deadline checks supplied by the product caller.
        if product.is_some()
            && !eligibility
                .and_then(|eligibility| eligibility.emission_current.as_ref())
                .ok_or(WorkspaceCoordError::Ineligible)?()
        {
            return Err(WorkspaceCoordError::Ineligible);
        }
        let response = match (ticket.edit(), product.is_some()) {
            (WorkspaceEdit::Put(value), true) => {
                session
                    .workspace_put_fenced(
                        &key.collection,
                        &key.key,
                        value.clone(),
                        ticket.base_revision(),
                        ticket
                            .write_epoch()
                            .ok_or(WorkspaceCoordError::Ineligible)?,
                    )
                    .await
            }
            (WorkspaceEdit::Delete, true) => {
                session
                    .workspace_delete_fenced(
                        &key.collection,
                        &key.key,
                        ticket
                            .base_revision()
                            .ok_or(WorkspaceStateError::NeedsRead)?,
                        ticket
                            .write_epoch()
                            .ok_or(WorkspaceCoordError::Ineligible)?,
                    )
                    .await
            }
            (WorkspaceEdit::Put(value), false) => {
                session
                    .workspace_put(
                        &key.collection,
                        &key.key,
                        value.clone(),
                        ticket.base_revision(),
                    )
                    .await
            }
            (WorkspaceEdit::Delete, false) => {
                session
                    .workspace_delete(
                        &key.collection,
                        &key.key,
                        ticket
                            .base_revision()
                            .ok_or(WorkspaceStateError::NeedsRead)?,
                    )
                    .await
            }
        };
        // Drop before rechecking or publishing: a queued logout owns the
        // next transition, and reacquiring its gate here could deadlock.
        drop(session_guard);
        drop(emission_guard);
        // The response belongs to the captured product session and
        // consent. If either changed while it was in flight, keep the
        // durable ticket unconfirmed for an explicit later review.
        if product.is_some() {
            check_eligibility(eligibility, session)?;
        }
        match response {
            Ok(record) => {
                let acknowledge = |state: &mut WorkspaceState| {
                    state.acknowledge(&ticket, record.clone())?;
                    Ok(())
                };
                if product.is_some() {
                    self.update_product(
                        eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                        acknowledge,
                    )?;
                } else {
                    self.update(acknowledge)?;
                }
                return Ok(true);
            }
            Err(SyncError::Conflict) => {
                let conflict = |state: &mut WorkspaceState| {
                    state.mark_conflict(&ticket)?;
                    Ok(())
                };
                if product.is_some() {
                    self.update_product(
                        eligibility.ok_or(WorkspaceCoordError::Ineligible)?,
                        conflict,
                    )?;
                } else {
                    self.update(conflict)?;
                }
            }
            // A failed/ambiguous response leaves the durable original
            // Unconfirmed. Product recovery must fence the old request
            // with a server barrier before relying on any subsequent GET.
            Err(error) => return Err(error.into()),
        }
        Ok(false)
    }
}
