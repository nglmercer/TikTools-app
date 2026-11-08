//! Explicit local-owner approval of saved delegated definitions. The request
//! never supplies source, provenance, grants or a reusable execution permit.
use crate::{error::ApiError, router::ControlRouter, RequestContext, RequestMetadata};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tiktools_core::{
    collaboration::{CommandRequest, Versioned},
    control::AutomationKind,
};

mod native;
pub use native::*;

pub use tiktools_core::control::{
    ApprovalActivateParams, ApprovalEnablement, ApprovalRevokeParams,
};

/// A historical receipt for the exact post-transition binding, never a permit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalActivateReceipt {
    pub kind: AutomationKind,
    pub id: String,
    pub enabled: bool,
    pub approved: bool,
    pub policy_version: u32,
    pub manifest_digest: String,
    pub approval_generation: String,
}

/// Revocation fences future delegated work; it does not undo dispatched work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalRevokeReceipt {
    pub kind: AutomationKind,
    pub id: String,
    pub revoked: bool,
    pub approval_generation: String,
}

fn owner_request(
    context: RequestContext,
    metadata: Option<RequestMetadata>,
) -> Result<CommandRequest, ApiError> {
    if !matches!(context, RequestContext::LocalOwner) {
        return Err(crate::context::forbidden());
    }
    let metadata = metadata.unwrap_or_default();
    let request = CommandRequest {
        command_id: metadata
            .command_id
            .ok_or_else(|| ApiError::invalid_params("commandId is required"))?,
        host_instance_id: metadata
            .host_instance_id
            .ok_or_else(|| ApiError::invalid_params("hostInstanceId is required"))?,
        expected_revision: metadata
            .expected_revision
            .ok_or_else(|| ApiError::invalid_params("expectedRevision is required"))?,
    };
    request
        .validate()
        .map_err(|_| ApiError::invalid_params("invalid approval command metadata"))?;
    Ok(request)
}

fn receipt<T: DeserializeOwned>(
    result: Versioned<serde_json::Value>,
) -> Result<Versioned<T>, ApiError> {
    let Versioned {
        value,
        resource_versions,
        command,
        host_instance_id,
    } = result;
    Ok(Versioned {
        value: serde_json::from_value(value)
            .map_err(|_| ApiError::internal("invalid automation approval receipt"))?,
        resource_versions,
        command,
        host_instance_id,
    })
}

pub fn register(router: &mut ControlRouter) {
    native::register(router);
    router.register_versioned::<ApprovalActivateParams, ApprovalActivateReceipt, _, _>(
        "automation.approval.activate",
        "Owner-only reviewed delegated approval and explicit activation; authority remains delegated",
        true,
        |core, context, params, metadata| async move {
            let request = owner_request(context, metadata)?;
            if !params.acknowledge_effects {
                return Err(ApiError::new(
                    "confirmation_required",
                    "review and acknowledge the delegated effect and enable-state plan",
                ));
            }
            super::blocking_task("automation.approval.activate", move || {
                core.automation_approval_activate(request, params)
            })
            .await?
            .map_err(ApiError::from)
            .and_then(receipt)
        },
    );
    router.register_versioned::<ApprovalRevokeParams, ApprovalRevokeReceipt, _, _>(
        "automation.approval.revoke",
        "Owner-only durable revocation of future delegated execution",
        true,
        |core, context, params, metadata| async move {
            let request = owner_request(context, metadata)?;
            if !params.acknowledge_revocation {
                return Err(ApiError::new(
                    "confirmation_required",
                    "acknowledge delegated execution revocation",
                ));
            }
            super::blocking_task("automation.approval.revoke", move || {
                core.automation_approval_revoke(request, params)
            })
            .await?
            .map_err(ApiError::from)
            .and_then(receipt)
        },
    );
    for name in ["automation.approval.activate", "automation.approval.revoke"] {
        router.set_durable_commands(name, true);
        // Keep this explicit as well as the registration default. No remote
        // grant can authorize these methods or their result/error metadata.
        router.set_remote_policy(name, crate::policy::RemotePolicy::Deny);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_approval_requires_complete_durable_metadata() {
        for metadata in [
            None,
            Some(RequestMetadata::default()),
            Some(RequestMetadata {
                command_id: Some("approval-command".into()),
                host_instance_id: Some("host-instance".into()),
                expected_revision: None,
            }),
            Some(RequestMetadata {
                command_id: Some("approval-command".into()),
                host_instance_id: Some("host-instance".into()),
                expected_revision: Some("01".into()),
            }),
        ] {
            assert!(owner_request(RequestContext::LocalOwner, metadata).is_err());
        }
        assert!(owner_request(
            RequestContext::LocalOwner,
            Some(RequestMetadata {
                command_id: Some("approval-command".into()),
                host_instance_id: Some("host-instance".into()),
                expected_revision: Some("1".into()),
            }),
        )
        .is_ok());
    }

    #[test]
    fn approval_receipts_do_not_accept_source_or_authority() {
        let valid = serde_json::json!({
            "kind":"event","id":"draft","enabled":true,"approved":true,
            "policyVersion":1,"manifestDigest":"a".repeat(64),"approvalGeneration":"1"
        });
        assert!(serde_json::from_value::<ApprovalActivateReceipt>(valid.clone()).is_ok());
        for field in [
            "record",
            "source",
            "provenance",
            "grant",
            "permit",
            "authority",
        ] {
            let mut extra = valid.clone();
            extra[field] = serde_json::json!({});
            assert!(serde_json::from_value::<ApprovalActivateReceipt>(extra).is_err());
        }
    }
}
