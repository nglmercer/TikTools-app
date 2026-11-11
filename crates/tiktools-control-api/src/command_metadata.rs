//! One reading of the request metadata for command-style mutations.
//!
//! Modules choose a shape (`versioned` compare-and-set or `additive`) and
//! receive either a complete durable command with the trusted caller, or the
//! local-owner fallback. The caller always comes from the authenticated
//! [`RequestContext`]; metadata can only name the command, never the principal.
use tiktools_core::{
    collaboration::{AdditiveCommandRequest, CommandRequest},
    control::CommandCaller,
};

use crate::{error::ApiError, RequestContext, RequestMetadata};

/// Outcome of resolving metadata plus context for one mutation.
pub(crate) enum CommandDispatch<R> {
    /// Complete durable metadata and the authenticated caller.
    Durable { caller: CommandCaller, request: R },
    /// Local owner without a `commandId`: the pre-durable envelope.
    Local { expected_revision: Option<String> },
}

/// Which complete metadata a mutation requires.
#[derive(Clone, Copy)]
enum Shape {
    Versioned,
    Additive,
}

/// Trusted caller for a durable command. Remote contexts need a persisted
/// authorization, which only an authenticated transport can construct.
pub(crate) fn command_caller(context: RequestContext) -> Result<CommandCaller, ApiError> {
    match context {
        RequestContext::LocalOwner => Ok(CommandCaller::LocalOwner),
        RequestContext::Remote(session) => Ok(CommandCaller::Remote(session.authorization()?)),
    }
}

/// Durable versioned command: `commandId`, `hostInstanceId` and
/// `expectedRevision` together. `remote_rejection` is the public error for a
/// remote caller that sent no durable metadata.
pub(crate) fn versioned_command(
    context: RequestContext,
    metadata: Option<RequestMetadata>,
    remote_rejection: fn() -> ApiError,
) -> Result<CommandDispatch<CommandRequest>, ApiError> {
    resolve(
        context,
        metadata,
        remote_rejection,
        Shape::Versioned,
        |parts| CommandRequest {
            command_id: parts.command_id,
            host_instance_id: parts.host_instance_id,
            expected_revision: parts.expected_revision.unwrap_or_default(),
        },
    )
}

/// Durable additive command: `commandId` and `hostInstanceId`, never a
/// precondition. A stray `expectedRevision` keeps being ignored here (the
/// router already rejects it for additive methods).
pub(crate) fn additive_command(
    context: RequestContext,
    metadata: Option<RequestMetadata>,
    remote_rejection: fn() -> ApiError,
) -> Result<CommandDispatch<AdditiveCommandRequest>, ApiError> {
    resolve(
        context,
        metadata,
        remote_rejection,
        Shape::Additive,
        |parts| AdditiveCommandRequest {
            command_id: parts.command_id,
            host_instance_id: parts.host_instance_id,
        },
    )
}

struct Parts {
    command_id: String,
    host_instance_id: String,
    expected_revision: Option<String>,
}

fn resolve<R>(
    context: RequestContext,
    metadata: Option<RequestMetadata>,
    remote_rejection: fn() -> ApiError,
    shape: Shape,
    build: impl FnOnce(Parts) -> R,
) -> Result<CommandDispatch<R>, ApiError> {
    let metadata = metadata.unwrap_or_default();
    let Some(command_id) = metadata.command_id else {
        if matches!(context, RequestContext::Remote(_)) {
            return Err(remote_rejection());
        }
        if metadata.host_instance_id.is_some() {
            return Err(ApiError::invalid_params(
                "commandId is required with hostInstanceId",
            ));
        }
        return Ok(CommandDispatch::Local {
            expected_revision: metadata.expected_revision,
        });
    };
    let host_instance_id = metadata
        .host_instance_id
        .ok_or_else(|| ApiError::invalid_params("hostInstanceId is required"))?;
    if matches!(shape, Shape::Versioned) && metadata.expected_revision.is_none() {
        return Err(ApiError::invalid_params("expectedRevision is required"));
    }
    let request = build(Parts {
        command_id,
        host_instance_id,
        expected_revision: metadata.expected_revision,
    });
    Ok(CommandDispatch::Durable {
        caller: command_caller(context)?,
        request,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{context::RemoteCapability, RemoteSession};

    fn meta(
        command: Option<&str>,
        host: Option<&str>,
        revision: Option<&str>,
    ) -> Option<RequestMetadata> {
        Some(RequestMetadata {
            command_id: command.map(str::to_owned),
            host_instance_id: host.map(str::to_owned),
            expected_revision: revision.map(str::to_owned),
        })
    }

    fn remote() -> RequestContext {
        RequestContext::Remote(
            RemoteSession::new(
                "principal",
                "session",
                [RemoteCapability::LiveRead],
                Duration::from_secs(60),
            )
            .unwrap(),
        )
    }

    fn remote_rejected() -> ApiError {
        ApiError::new("remote_rejected", "remote rejection")
    }

    /// Compact view of a resolution for the table below.
    fn outcome<R>(result: Result<CommandDispatch<R>, ApiError>) -> String {
        match result {
            Ok(CommandDispatch::Durable { caller, .. }) => match caller {
                CommandCaller::LocalOwner => "durable:local".into(),
                CommandCaller::Remote(_) => "durable:remote".into(),
            },
            Ok(CommandDispatch::Local { expected_revision }) => {
                format!("local:{}", expected_revision.unwrap_or_else(|| "-".into()))
            }
            Err(error) => format!("{}:{}", error.code, error.message),
        }
    }

    #[test]
    fn metadata_table_for_versioned_and_additive_commands() {
        let host_required = "invalid_params:hostInstanceId is required";
        let revision_required = "invalid_params:expectedRevision is required";
        let command_required = "invalid_params:commandId is required with hostInstanceId";
        // (label, local metadata, versioned outcome, additive outcome)
        let local: Vec<(&str, Option<RequestMetadata>, String, String)> = vec![
            ("absent", None, "local:-".into(), "local:-".into()),
            (
                "empty",
                meta(None, None, None),
                "local:-".into(),
                "local:-".into(),
            ),
            (
                "revision only",
                meta(None, None, Some("r1")),
                "local:r1".into(),
                "local:r1".into(),
            ),
            (
                "complete",
                meta(Some("c1"), Some("h1"), Some("r1")),
                "durable:local".into(),
                "durable:local".into(),
            ),
            (
                "command and host without revision",
                meta(Some("c1"), Some("h1"), None),
                revision_required.into(),
                "durable:local".into(),
            ),
            (
                "command only",
                meta(Some("c1"), None, Some("r1")),
                host_required.into(),
                host_required.into(),
            ),
            (
                "command only without revision",
                meta(Some("c1"), None, None),
                host_required.into(),
                host_required.into(),
            ),
            (
                "isolated hostInstanceId",
                meta(None, Some("h1"), None),
                command_required.into(),
                command_required.into(),
            ),
            (
                "isolated hostInstanceId with revision",
                meta(None, Some("h1"), Some("r1")),
                command_required.into(),
                command_required.into(),
            ),
        ];
        for (label, metadata, versioned, additive) in local {
            assert_eq!(
                outcome(versioned_command(
                    RequestContext::LocalOwner,
                    metadata.clone(),
                    remote_rejected
                )),
                versioned,
                "versioned: {label}"
            );
            assert_eq!(
                outcome(additive_command(
                    RequestContext::LocalOwner,
                    metadata,
                    remote_rejected
                )),
                additive,
                "additive: {label}"
            );
        }
    }

    #[test]
    fn remote_callers_never_take_the_local_fallback_or_a_json_identity() {
        let rejected = "remote_rejected:remote rejection";
        for (label, metadata) in [
            ("absent", None),
            ("empty", meta(None, None, None)),
            ("revision only", meta(None, None, Some("r1"))),
            ("isolated hostInstanceId", meta(None, Some("h1"), None)),
        ] {
            assert_eq!(
                outcome(versioned_command(
                    remote(),
                    metadata.clone(),
                    remote_rejected
                )),
                rejected,
                "versioned: {label}"
            );
            assert_eq!(
                outcome(additive_command(remote(), metadata, remote_rejected)),
                rejected,
                "additive: {label}"
            );
        }
        // Partial metadata is reported before any authorization is consulted.
        assert_eq!(
            outcome(versioned_command(
                remote(),
                meta(Some("c1"), Some("h1"), None),
                remote_rejected
            )),
            "invalid_params:expectedRevision is required"
        );
        assert_eq!(
            outcome(additive_command(
                remote(),
                meta(Some("c1"), None, None),
                remote_rejected
            )),
            "invalid_params:hostInstanceId is required"
        );
        // Complete metadata from a session without persisted authorization
        // cannot become a caller: JSON never supplies one.
        assert_eq!(
            outcome(versioned_command(
                remote(),
                meta(Some("c1"), Some("h1"), Some("r1")),
                remote_rejected
            )),
            "unsupported_remote:persistent authorization is required"
        );
        assert_eq!(
            outcome(additive_command(
                remote(),
                meta(Some("c1"), Some("h1"), None),
                remote_rejected
            )),
            "unsupported_remote:persistent authorization is required"
        );
    }

    #[test]
    fn additive_command_carries_no_precondition_and_ignores_a_stray_revision() {
        let Ok(CommandDispatch::Durable { request, .. }) = additive_command(
            RequestContext::LocalOwner,
            meta(Some("c1"), Some("h1"), Some("ignored")),
            remote_rejected,
        ) else {
            panic!("complete additive metadata must resolve");
        };
        assert_eq!(request.command_id, "c1");
        assert_eq!(request.host_instance_id, "h1");
    }
}
