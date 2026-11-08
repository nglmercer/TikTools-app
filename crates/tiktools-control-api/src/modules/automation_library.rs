//! Owner library transactions keep graph/catalog writes and original outcomes together.
use crate::{
    error::ApiError, registry::ConcurrencyStrategy, router::ControlRouter, RequestContext,
    RequestMetadata,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::collaboration::Versioned;
pub use tiktools_core::control::{
    owner_library_command_hash, owner_library_definition_digest, owner_library_digest,
    owner_library_versions_digest, validate_owner_library_submit, OwnerLibraryAcknowledgement,
    OwnerLibraryCatalog, OwnerLibraryEntry, OwnerLibraryIntent, OwnerLibraryMutation,
    OwnerLibraryOperation, OwnerLibraryOutcome, OwnerLibraryPack, OwnerLibraryProfile,
    OwnerLibraryReceipt, OwnerLibraryRecord, OwnerLibraryReviewUnknown, OwnerLibrarySnapshot,
    OwnerLibraryState, OwnerLibrarySubmission, OwnerLibrarySubmit, OWNER_LIBRARY_MAX_BYTES,
    OWNER_LIBRARY_MAX_CATALOG_BYTES, OWNER_LIBRARY_MAX_RECORDS,
};

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OwnerLibraryEmpty {}

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
            "native owner library coordinator chooses command metadata",
        ));
    }
    Ok(())
}

pub fn register(router: &mut ControlRouter) {
    router.register_versioned::<OwnerLibraryEmpty, OwnerLibrarySnapshot, _, _>(
        "automation.library.snapshot",
        "Coherent Owner graph, profile and custom template catalog with its reviewed digest",
        false,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.library.snapshot", move || {
                core.owner_library_snapshot()
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerLibraryEmpty, OwnerLibraryState, _, _>(
        "automation.library.get",
        "Owner-only pending library intent and recent metadata outcomes",
        false,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.library.get", move || core.owner_library_state())
                .await?
                .map(|state| Versioned::new(state, Vec::new()))
                .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerLibrarySubmit, OwnerLibrarySubmission, _, _>(
        "automation.library.submit",
        "One durable Owner graph/catalog transaction bound to its reviewed library snapshot",
        true,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.library.submit", move || {
                core.owner_library_submit(params)
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerLibraryEmpty, OwnerLibraryState, _, _>(
        "automation.library.resolve",
        "Read the original library outcome without source or another dispatch",
        true,
        |core, context, _, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.library.resolve", move || {
                core.owner_library_resolve()
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<OwnerLibraryReviewUnknown, OwnerLibraryState, _, _>(
        "automation.library.review_unknown",
        "Archive explicitly reviewed uncertainty for the original Owner library command",
        true,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            super::blocking_task("automation.library.review_unknown", move || {
                core.owner_library_review_unknown(params)
            })
            .await?
            .map(|state| Versioned::new(state, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    for name in [
        "automation.library.snapshot",
        "automation.library.get",
        "automation.library.submit",
        "automation.library.resolve",
        "automation.library.review_unknown",
    ] {
        router.set_remote_policy(name, crate::policy::RemotePolicy::Deny);
    }
    for name in [
        "automation.library.submit",
        "automation.library.resolve",
        "automation.library.review_unknown",
    ] {
        router.set_concurrency(name, ConcurrencyStrategy::ExclusiveTransition);
    }
    router.set_flags("automation.library.submit", true, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_routes_keep_host_owned_identity_and_classify_mutations() {
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
                expected_revision: Some("0".into()),
                ..Default::default()
            },
        ] {
            assert!(owner(RequestContext::LocalOwner, Some(meta)).is_err());
        }
        assert!(owner(RequestContext::LocalOwner, None).is_ok());
        let mut router = ControlRouter::new();
        register(&mut router);
        assert_eq!(router.metadata().len(), 5);
        for method in router.metadata() {
            let mutation = !matches!(
                method.name.as_str(),
                "automation.library.snapshot" | "automation.library.get"
            );
            assert_eq!(method.side_effect, mutation);
            assert_eq!(
                method.concurrency,
                mutation.then_some(ConcurrencyStrategy::ExclusiveTransition)
            );
            assert_eq!(
                method.destructive,
                method.name == "automation.library.submit"
            );
            assert!(!method.durable_commands);
        }
    }
}
