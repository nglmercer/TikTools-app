//! `web` domain spec: the local browser adapter.
//!
//! The adapter itself is feature-gated (`local-web`) in `crate::web`, but
//! the domain is part of the CLI surface in every build — the dispatcher
//! recognizes `web` unconditionally and points at the missing feature when
//! the binary does not carry it — so the spec must exist unconditionally
//! for help, generated docs and the registry consistency tests.

use super::spec::{DomainSpec, VerbSpec};

pub(crate) const U_PRINT_CODE: &str = "web --print-code [--port N]";
pub(crate) const U_RESET_SESSION: &str = "web --reset-session [--port N]";

pub(crate) const SPEC: DomainSpec = DomainSpec {
    domain: "web",
    summary: "local browser adapter (requires the local-web feature)",
    verbs: &[
        VerbSpec::new(
            "web",
            "web --assets <directory> [--port N] [--join-origin URL] [--multi-session]",
            "serve the local browser adapter; host --ipc must be running",
        ),
        VerbSpec::new(
            "print-code",
            U_PRINT_CODE,
            "print the active pairing code without restarting",
        ),
        VerbSpec::new(
            "reset-session",
            U_RESET_SESSION,
            "revoke every paired browser and print a fresh code",
        )
        .destructive(),
    ],
};
