//! Typed local-owner collaboration administration. Selecting a remote
//! transport never grants access to this owner-only surface.
use super::{ClientError, TikToolsClient};
use tiktools_control_api::modules::{collaboration::*, Empty, OkResult};

pub(crate) const METHODS: &[&str] = &[
    "collaboration.hello",
    "collaboration.me",
    "collaboration.subscribe",
    "collaboration.snapshot",
    "collaboration.resume",
    "collaboration.commands.get",
    "collaboration.status",
    "collaboration.configure",
    "collaboration.invites.create",
    "collaboration.invites.list",
    "collaboration.invites.revoke",
    "collaboration.requests.list",
    "collaboration.requests.approve",
    "collaboration.requests.reject",
    "collaboration.sessions.list",
    "collaboration.sessions.revoke",
    "collaboration.principals.list",
    "collaboration.principals.get",
    "collaboration.principals.revoke",
    "collaboration.grants.set",
    "collaboration.audit.list",
];

macro_rules! method {
    ($function:ident,$name:literal,$input:ty,$output:ty) => {
        pub async fn $function(&self, params: $input) -> Result<$output, ClientError> {
            self.call($name, params).await
        }
    };
    ($function:ident,$name:literal,$output:ty) => {
        pub async fn $function(&self) -> Result<$output, ClientError> {
            self.call($name, Empty::default()).await
        }
    };
}

impl TikToolsClient {
    pub async fn collaboration_configure_checked(
        &self,
        configuration: Configuration,
        expected_revision: String,
    ) -> Result<Status, ClientError> {
        self.call(
            "collaboration.configure",
            ConfigureInput {
                configuration,
                expected_revision: Some(expected_revision),
            },
        )
        .await
    }
    method!(
        collaboration_hello,
        "collaboration.hello",
        HelloParams,
        HelloResult
    );
    method!(collaboration_me, "collaboration.me", Authenticated);
    method!(
        collaboration_subscribe,
        "collaboration.subscribe",
        SubscribeParams,
        SubscribeResult
    );
    method!(
        collaboration_snapshot,
        "collaboration.snapshot",
        SnapshotParams,
        SnapshotPage
    );
    method!(
        collaboration_resume,
        "collaboration.resume",
        ResumeParams,
        ResumeResult
    );
    method!(
        collaboration_commands_get,
        "collaboration.commands.get",
        CommandGet,
        CommandOutcome
    );
    method!(collaboration_status, "collaboration.status", Status);
    method!(
        collaboration_configure,
        "collaboration.configure",
        Configuration,
        Status
    );
    method!(
        collaboration_invites_create,
        "collaboration.invites.create",
        InviteCreate,
        InviteSecret
    );
    method!(
        collaboration_invites_list,
        "collaboration.invites.list",
        ListQuery,
        InvitationsResult
    );
    method!(
        collaboration_invites_revoke,
        "collaboration.invites.revoke",
        IdParams,
        OkResult
    );
    method!(
        collaboration_requests_list,
        "collaboration.requests.list",
        ListQuery,
        RequestsResult
    );
    method!(
        collaboration_requests_approve,
        "collaboration.requests.approve",
        IdParams,
        OkResult
    );
    method!(
        collaboration_requests_reject,
        "collaboration.requests.reject",
        IdParams,
        OkResult
    );
    method!(
        collaboration_sessions_list,
        "collaboration.sessions.list",
        ListQuery,
        SessionsResult
    );
    method!(
        collaboration_sessions_revoke,
        "collaboration.sessions.revoke",
        IdParams,
        OkResult
    );
    method!(
        collaboration_principals_list,
        "collaboration.principals.list",
        ListQuery,
        PrincipalsResult
    );
    method!(
        collaboration_principals_get,
        "collaboration.principals.get",
        IdParams,
        Principal
    );
    method!(
        collaboration_principals_revoke,
        "collaboration.principals.revoke",
        IdParams,
        OkResult
    );
    method!(
        collaboration_grants_set,
        "collaboration.grants.set",
        GrantSet,
        OkResult
    );
    method!(
        collaboration_audit_list,
        "collaboration.audit.list",
        AuditQuery,
        AuditResult
    );
}
