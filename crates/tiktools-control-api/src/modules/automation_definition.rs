//! Host-owned identities for ordinary Owner definition writes. Recovery reads
//! the original ledger and never dispatches an uncertain operation again.
use crate::{
    error::ApiError, registry::ConcurrencyStrategy, router::ControlRouter, RequestContext,
    RequestMetadata,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::collaboration::Versioned;
pub use tiktools_core::control::{
    OwnerDefinitionAcknowledgement, OwnerDefinitionIntent, OwnerDefinitionOperation,
    OwnerDefinitionOutcome, OwnerDefinitionReceipt, OwnerDefinitionRecord,
    OwnerDefinitionReviewUnknown, OwnerDefinitionState, OwnerDefinitionSubmission,
    OwnerDefinitionSubmit,
};

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnerDefinitionEmpty {}

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
            "native owner definition coordinator chooses command metadata",
        ));
    }
    Ok(())
}

pub fn register(router: &mut ControlRouter) {
    router.register_versioned::<OwnerDefinitionEmpty, OwnerDefinitionState, _, _>(
        "automation.definition.get",
        "Owner-only pending definition write and recent durable metadata",
        false,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.definition.get", move || {
                core.owner_definition_state()
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerDefinitionSubmit, OwnerDefinitionSubmission, _, _>(
        "automation.definition.submit",
        "Persists a native Owner intent before one checked create, update, delete or enablement write",
        true,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.definition.submit", move || {
                core.owner_definition_submit(params)
            })
            .await?
            .map(|submission| Versioned::new(submission, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerDefinitionEmpty, OwnerDefinitionState, _, _>(
        "automation.definition.resolve",
        "Looks up the original Owner definition outcome without dispatch or historical source",
        true,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.definition.resolve", move || {
                core.owner_definition_resolve()
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerDefinitionReviewUnknown, OwnerDefinitionState, _, _>(
        "automation.definition.review_unknown",
        "Archives explicitly reviewed uncertainty for the original Owner definition identity",
        true,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.definition.review_unknown", move || {
                core.owner_definition_review_unknown(params)
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    for name in [
        "automation.definition.get",
        "automation.definition.submit",
        "automation.definition.resolve",
        "automation.definition.review_unknown",
    ] {
        router.set_remote_policy(name, crate::policy::RemotePolicy::Deny);
    }
    for name in [
        "automation.definition.submit",
        "automation.definition.resolve",
        "automation.definition.review_unknown",
    ] {
        router.set_concurrency(name, ConcurrencyStrategy::ExclusiveTransition);
    }
    // The submit union includes deletion. Generic CLI callers use this flag
    // to require --confirm before executing any potentially destructive call.
    router.set_flags("automation.definition.submit", true, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_definition_routes_reject_caller_managed_business_identity() {
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

    #[test]
    fn native_definition_mutations_are_exclusive_without_caller_managed_commands() {
        let mut router = ControlRouter::new();
        register(&mut router);
        let methods = router.metadata();
        assert_eq!(methods.len(), 4);
        for method in methods {
            assert!(!method.durable_commands);
            if method.name == "automation.definition.get" {
                assert!(!method.side_effect);
                assert!(method.concurrency.is_none());
            } else {
                assert!(method.side_effect);
                assert_eq!(
                    method.concurrency,
                    Some(ConcurrencyStrategy::ExclusiveTransition)
                );
            }
            assert_eq!(
                method.destructive,
                method.name == "automation.definition.submit"
            );
        }
    }
}
