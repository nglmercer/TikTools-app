use super::TikToolsClient;
use tiktools_control_api::modules::teams::{
    TeamCreateParams, TeamDecisionParams, TeamDevice, TeamDeviceList, TeamDeviceParams,
    TeamEnrollParams, TeamGrantList, TeamGrantRevokeParams, TeamGrantUpdateParams, TeamHost,
    TeamHostList, TeamHostParams, TeamInvitationCreateParams, TeamInvitationCreated,
    TeamInvitationList, TeamInvitationParams, TeamRequestList, TeamSessionList, TeamSessionParams,
    TeamUpdateParams,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    TEAMS_HOSTS_LIST,
    TEAMS_HOSTS_CREATE,
    TEAMS_HOSTS_GET,
    TEAMS_HOSTS_UPDATE,
    TEAMS_DEVICES_LIST,
    TEAMS_DEVICES_ENROLL,
    TEAMS_DEVICES_REVOKE,
    TEAMS_INVITATIONS_LIST,
    TEAMS_INVITATIONS_CREATE,
    TEAMS_INVITATIONS_REVOKE,
    TEAMS_REQUESTS_LIST,
    TEAMS_REQUESTS_DECIDE,
    TEAMS_GRANTS_LIST,
    TEAMS_GRANTS_UPDATE,
    TEAMS_GRANTS_REVOKE,
    TEAMS_SESSIONS_LIST,
    TEAMS_SESSIONS_REVOKE,
];

const TEAMS_HOSTS_LIST: &str = "teams.hosts.list";
const TEAMS_HOSTS_CREATE: &str = "teams.hosts.create";
const TEAMS_HOSTS_GET: &str = "teams.hosts.get";
const TEAMS_HOSTS_UPDATE: &str = "teams.hosts.update";
const TEAMS_DEVICES_LIST: &str = "teams.devices.list";
const TEAMS_DEVICES_ENROLL: &str = "teams.devices.enroll";
const TEAMS_DEVICES_REVOKE: &str = "teams.devices.revoke";
const TEAMS_INVITATIONS_LIST: &str = "teams.invitations.list";
const TEAMS_INVITATIONS_CREATE: &str = "teams.invitations.create";
const TEAMS_INVITATIONS_REVOKE: &str = "teams.invitations.revoke";
const TEAMS_REQUESTS_LIST: &str = "teams.requests.list";
const TEAMS_REQUESTS_DECIDE: &str = "teams.requests.decide";
const TEAMS_GRANTS_LIST: &str = "teams.grants.list";
const TEAMS_GRANTS_UPDATE: &str = "teams.grants.update";
const TEAMS_GRANTS_REVOKE: &str = "teams.grants.revoke";
const TEAMS_SESSIONS_LIST: &str = "teams.sessions.list";
const TEAMS_SESSIONS_REVOKE: &str = "teams.sessions.revoke";

impl TikToolsClient {
    pub async fn teams_hosts_list(&self) -> Result<TeamHostList, ClientError> {
        self.call(TEAMS_HOSTS_LIST, Empty::default()).await
    }
    pub async fn teams_hosts_create(
        &self,
        params: TeamCreateParams,
    ) -> Result<TeamHost, ClientError> {
        self.call(TEAMS_HOSTS_CREATE, params).await
    }
    pub async fn teams_hosts_get(&self, params: TeamHostParams) -> Result<TeamHost, ClientError> {
        self.call(TEAMS_HOSTS_GET, params).await
    }
    pub async fn teams_hosts_update(
        &self,
        params: TeamUpdateParams,
    ) -> Result<TeamHost, ClientError> {
        self.call(TEAMS_HOSTS_UPDATE, params).await
    }
    pub async fn teams_devices_list(&self) -> Result<TeamDeviceList, ClientError> {
        self.call(TEAMS_DEVICES_LIST, Empty::default()).await
    }
    pub async fn teams_devices_enroll(
        &self,
        params: TeamEnrollParams,
    ) -> Result<TeamDevice, ClientError> {
        self.call(TEAMS_DEVICES_ENROLL, params).await
    }
    pub async fn teams_devices_revoke(
        &self,
        params: TeamDeviceParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(TEAMS_DEVICES_REVOKE, params).await
    }
    pub async fn teams_invitations_list(
        &self,
        params: TeamHostParams,
    ) -> Result<TeamInvitationList, ClientError> {
        self.call(TEAMS_INVITATIONS_LIST, params).await
    }
    pub async fn teams_invitations_create(
        &self,
        params: TeamInvitationCreateParams,
    ) -> Result<TeamInvitationCreated, ClientError> {
        self.call(TEAMS_INVITATIONS_CREATE, params).await
    }
    pub async fn teams_invitations_revoke(
        &self,
        params: TeamInvitationParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(TEAMS_INVITATIONS_REVOKE, params).await
    }
    pub async fn teams_requests_list(
        &self,
        params: TeamHostParams,
    ) -> Result<TeamRequestList, ClientError> {
        self.call(TEAMS_REQUESTS_LIST, params).await
    }
    pub async fn teams_requests_decide(
        &self,
        params: TeamDecisionParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(TEAMS_REQUESTS_DECIDE, params).await
    }
    pub async fn teams_grants_list(
        &self,
        params: TeamHostParams,
    ) -> Result<TeamGrantList, ClientError> {
        self.call(TEAMS_GRANTS_LIST, params).await
    }
    pub async fn teams_grants_update(
        &self,
        params: TeamGrantUpdateParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(TEAMS_GRANTS_UPDATE, params).await
    }
    pub async fn teams_grants_revoke(
        &self,
        params: TeamGrantRevokeParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(TEAMS_GRANTS_REVOKE, params).await
    }
    pub async fn teams_sessions_list(
        &self,
        params: TeamHostParams,
    ) -> Result<TeamSessionList, ClientError> {
        self.call(TEAMS_SESSIONS_LIST, params).await
    }
    pub async fn teams_sessions_revoke(
        &self,
        params: TeamSessionParams,
    ) -> Result<serde_json::Value, ClientError> {
        self.call(TEAMS_SESSIONS_REVOKE, params).await
    }
}
