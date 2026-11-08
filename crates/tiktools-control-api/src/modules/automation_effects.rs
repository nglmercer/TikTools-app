//! Owner-only receipt inspection. There is intentionally no retry/replay RPC.
use crate::{error::ApiError, router::ControlRouter, RequestContext, RequestMetadata};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tiktools_core::collaboration::Versioned;
pub use tiktools_core::control::{EffectHistory, EffectKind, EffectReceipt, EffectState};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectGetParams {
    pub effect_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectHistoryParams {
    pub before: Option<String>,
    pub limit: u32,
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
impl EffectGetParams {
    pub fn validate(&self) -> Result<(), ApiError> {
        if digest(&self.effect_id) {
            Ok(())
        } else {
            Err(ApiError::invalid_params(
                "effectId must be a canonical effect identity",
            ))
        }
    }
}
impl EffectHistoryParams {
    pub fn validate(&self) -> Result<(), ApiError> {
        if !(1..=100).contains(&self.limit) || self.before.as_ref().is_some_and(|id| !digest(id)) {
            Err(ApiError::invalid_params(
                "invalid effect history cursor or limit",
            ))
        } else {
            Ok(())
        }
    }
}

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
            "effect inspection accepts no command metadata",
        ));
    }
    Ok(())
}

pub fn register(router: &mut ControlRouter) {
    router.register_versioned::<EffectGetParams, Option<EffectReceipt>, _, _>(
        "automation.effects.get",
        "Owner-only metadata receipt lookup; never retries an effect",
        false,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            params.validate()?;
            super::blocking_task("automation.effects.get", move || {
                core.automation_effect_receipt(&params.effect_id)
            })
            .await?
            .map(|receipt| Versioned::new(receipt, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    router.register_versioned::<EffectHistoryParams, EffectHistory, _, _>(
        "automation.effects.history",
        "Owner-only bounded metadata receipt history, including uncertain outcomes",
        false,
        |core, context, params, meta| async move {
            owner(context, meta)?;
            params.validate()?;
            super::blocking_task("automation.effects.history", move || {
                core.automation_effect_history(params.before.as_deref(), params.limit)
            })
            .await?
            .map(|page| Versioned::new(page, Vec::new()))
            .map_err(ApiError::from)
        },
    );
    for method in ["automation.effects.get", "automation.effects.history"] {
        router.set_remote_policy(method, crate::policy::RemotePolicy::Deny);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effect_inspection_has_strict_bounded_params_and_rejects_command_metadata() {
        assert!(EffectGetParams {
            effect_id: "a".repeat(64)
        }
        .validate()
        .is_ok());
        assert!(EffectGetParams {
            effect_id: "A".repeat(64)
        }
        .validate()
        .is_err());
        for limit in [0, 101, u32::MAX] {
            assert!(EffectHistoryParams {
                before: None,
                limit
            }
            .validate()
            .is_err());
        }
        assert!(EffectHistoryParams {
            before: Some("caller-key".into()),
            limit: 10
        }
        .validate()
        .is_err());
        assert!(
            serde_json::from_value::<EffectGetParams>(serde_json::json!({
                "effectId":"a".repeat(64),"replay":true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<EffectHistoryParams>(serde_json::json!({
                "before":null,"limit":10,"request":{}
            }))
            .is_err()
        );
        for metadata in [
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
            assert!(owner(RequestContext::LocalOwner, Some(metadata)).is_err());
        }
    }
}
