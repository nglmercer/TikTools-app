//! Typed coverage for the Cloud teams RPCs registered by the control API.
//!
//! These are host-only remote-control operations; the control plane exposes
//! them so agents can discover them, and the CLI verifies that the typed
//! client covers every registered method.

pub(crate) const METHODS: &[&str] = &[
    "teams.people.search",
    "teams.accessible.list",
    "teams.commands.run",
    "teams.invitations.redeem",
    "teams.host.status",
    "teams.host.start",
    "teams.host.stop",
    "teams.hosts.list",
    "teams.hosts.create",
    "teams.hosts.get",
    "teams.hosts.update",
    "teams.devices.list",
    "teams.devices.enroll",
    "teams.devices.revoke",
    "teams.invitations.list",
    "teams.invitations.create",
    "teams.invitations.revoke",
    "teams.requests.list",
    "teams.requests.decide",
    "teams.grants.list",
    "teams.grants.update",
    "teams.grants.revoke",
    "teams.sessions.list",
    "teams.sessions.revoke",
];
