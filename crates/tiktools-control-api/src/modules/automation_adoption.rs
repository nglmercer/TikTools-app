//! Local-owner host journal. Remote discovery does not grant these operations.
use crate::{
    error::ApiError, registry::ConcurrencyStrategy, router::ControlRouter, RequestContext,
    RequestMetadata,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::collaboration::Versioned;
pub use tiktools_core::control::{OwnerAdoptionReview, OwnerAdoptionState, OwnerAdoptionSubmit};
#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnerAdoptionEmpty {}

fn owner(context: RequestContext, meta: Option<RequestMetadata>) -> Result<(), ApiError> {
    if !matches!(context, RequestContext::LocalOwner) {
        return Err(crate::context::forbidden());
    }
    if meta.is_some_and(|m| {
        m.command_id.is_some() || m.host_instance_id.is_some() || m.expected_revision.is_some()
    }) {
        return Err(ApiError::invalid_params(
            "owner adoption coordinator chooses command metadata",
        ));
    }
    Ok(())
}
pub fn register(router: &mut ControlRouter) {
    router.register_versioned::<OwnerAdoptionEmpty, OwnerAdoptionState, _, _>(
        "automation.adoption.get",
        "Owner-only durable adoption intent and recent metadata",
        false,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.adoption.get", move || {
                core.owner_adoption_state()
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerAdoptionSubmit, OwnerAdoptionState, _, _>(
        "automation.adoption.submit",
        "Persists native owner intent before reviewed authority transfer",
        true,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.adoption.submit", move || {
                core.owner_adoption_submit(params)
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerAdoptionEmpty, OwnerAdoptionState, _, _>(
        "automation.adoption.resolve",
        "Queries original owner adoption outcome without dispatch",
        true,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.adoption.resolve", move || {
                core.owner_adoption_resolve()
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerAdoptionReview, OwnerAdoptionState, _, _>(
        "automation.adoption.review_unknown",
        "Archives an explicitly reviewed unresolved owner intent without replay",
        true,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.adoption.review_unknown", move || {
                core.owner_adoption_review_unknown(params)
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    for name in [
        "automation.adoption.submit",
        "automation.adoption.resolve",
        "automation.adoption.review_unknown",
    ] {
        router.set_concurrency(name, ConcurrencyStrategy::ExclusiveTransition);
    }
}
