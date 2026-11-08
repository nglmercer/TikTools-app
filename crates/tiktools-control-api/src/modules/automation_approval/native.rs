//! The host owns durable identities for browser-driven approval operations.
//! These endpoints neither accept command metadata nor replay an uncertain send.
use crate::{
    error::ApiError, registry::ConcurrencyStrategy, router::ControlRouter, RequestContext,
    RequestMetadata,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::collaboration::Versioned;
pub use tiktools_core::control::{
    OwnerApprovalBlockReason, OwnerApprovalEligibility, OwnerApprovalIntent,
    OwnerApprovalOperation, OwnerApprovalOutcome, OwnerApprovalReceipt, OwnerApprovalRecord,
    OwnerApprovalReviewParams, OwnerApprovalReviewResult, OwnerApprovalReviewUnknown,
    OwnerApprovalState, OwnerApprovalStatus, OwnerApprovalStatusKind, OwnerApprovalSubject,
    OwnerApprovalSubmit,
};

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnerApprovalEmpty {}

fn owner(context: RequestContext, meta: Option<RequestMetadata>) -> Result<(), ApiError> {
    if !matches!(context, RequestContext::LocalOwner) {
        return Err(crate::context::forbidden());
    }
    if meta.is_some_and(|meta| {
        meta.command_id.is_some()
            || meta.host_instance_id.is_some()
            || meta.expected_revision.is_some()
    }) {
        return Err(ApiError::invalid_params(
            "native owner approval coordinator chooses command metadata",
        ));
    }
    Ok(())
}

pub(super) fn register(router: &mut ControlRouter) {
    router.register_versioned::<OwnerApprovalReviewParams, OwnerApprovalReviewResult, _, _>(
        "automation.approval.review",
        "Owner-only atomic saved source, execution manifest, current grants and approval state",
        false,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::super::blocking_task("automation.approval.review", move || {
                core.owner_approval_review(params.kind, &params.id)
            })
            .await?
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerApprovalEmpty, OwnerApprovalState, _, _>(
        "automation.approval.get",
        "Owner-only pending approval operation and recent durable metadata",
        false,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::super::blocking_task("automation.approval.get", move || {
                core.owner_approval_state()
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerApprovalSubmit, OwnerApprovalState, _, _>(
        "automation.approval.submit",
        "Persists a native owner intent before reviewed activation or revocation",
        true,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::super::blocking_task("automation.approval.submit", move || {
                core.owner_approval_submit(params)
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerApprovalEmpty, OwnerApprovalState, _, _>(
        "automation.approval.resolve",
        "Looks up the original owner approval outcome without dispatch",
        true,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::super::blocking_task("automation.approval.resolve", move || {
                core.owner_approval_resolve()
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerApprovalReviewUnknown, OwnerApprovalState, _, _>(
        "automation.approval.review_unknown",
        "Archives an explicitly reviewed unresolved owner approval without replay",
        true,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::super::blocking_task("automation.approval.review_unknown", move || {
                core.owner_approval_review_unknown(params)
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    for name in [
        "automation.approval.review",
        "automation.approval.get",
        "automation.approval.submit",
        "automation.approval.resolve",
        "automation.approval.review_unknown",
    ] {
        router.set_remote_policy(name, crate::policy::RemotePolicy::Deny);
    }
    for name in [
        "automation.approval.submit",
        "automation.approval.resolve",
        "automation.approval.review_unknown",
    ] {
        router.set_concurrency(name, ConcurrencyStrategy::ExclusiveTransition);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_owner_endpoints_reject_all_caller_managed_command_metadata() {
        for meta in [
            RequestMetadata {
                command_id: Some("chosen".into()),
                ..Default::default()
            },
            RequestMetadata {
                host_instance_id: Some("chosen".into()),
                ..Default::default()
            },
            RequestMetadata {
                expected_revision: Some("1".into()),
                ..Default::default()
            },
        ] {
            assert!(owner(RequestContext::LocalOwner, Some(meta)).is_err());
        }
        assert!(owner(RequestContext::LocalOwner, None).is_ok());
    }

    #[tokio::test]
    async fn cancelling_native_request_does_not_abandon_its_owned_blocking_operation() {
        let (entered, observed) = tokio::sync::oneshot::channel();
        let (resume, proceed) = std::sync::mpsc::sync_channel(0);
        let (completed, completion) = tokio::sync::oneshot::channel();
        let request = tokio::spawn(async move {
            super::super::super::blocking_task("owner approval cancellation fixture", move || {
                entered.send(()).unwrap();
                proceed
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                completed.send(()).unwrap();
            })
            .await
        });
        observed.await.unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        resume.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), completion)
            .await
            .unwrap()
            .unwrap();
    }
}
