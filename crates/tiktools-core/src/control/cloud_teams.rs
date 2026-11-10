//! Cloud teams for remote control. Invitation tokens cross this boundary only in
//! the invitation-create reply; every other reply is nonsecret.

use super::OperationError;
use crate::{
    cloud_teams::{
        TeamCreateParams, TeamDecisionParams, TeamDevice, TeamDeviceList, TeamDeviceParams,
        TeamEnrollParams, TeamGrantList, TeamGrantRevokeParams, TeamGrantUpdateParams, TeamHost,
        TeamHostList, TeamHostParams, TeamInvitationCreateParams, TeamInvitationCreated,
        TeamInvitationList, TeamInvitationParams, TeamRequestList, TeamSessionList,
        TeamSessionParams, TeamUpdateParams,
    },
    AppCore,
};
use serde_json::Value;

impl AppCore {
    pub async fn teams_hosts(&self) -> Result<TeamHostList, OperationError> {
        crate::cloud_teams::hosts(self.cloud.auth()).await
    }
    pub async fn teams_host_create(
        &self,
        params: TeamCreateParams,
    ) -> Result<TeamHost, OperationError> {
        crate::cloud_teams::create_host(self.cloud.auth(), params).await
    }
    pub async fn teams_host_get(&self, params: TeamHostParams) -> Result<TeamHost, OperationError> {
        crate::cloud_teams::get_host(self.cloud.auth(), params).await
    }
    pub async fn teams_host_update(
        &self,
        params: TeamUpdateParams,
    ) -> Result<TeamHost, OperationError> {
        crate::cloud_teams::update_host(self.cloud.auth(), params).await
    }
    pub async fn teams_devices(&self) -> Result<TeamDeviceList, OperationError> {
        crate::cloud_teams::devices(self.cloud.auth()).await
    }
    pub async fn teams_device_enroll(
        &self,
        params: TeamEnrollParams,
    ) -> Result<TeamDevice, OperationError> {
        self.teams.enroll(self.cloud.auth(), params).await
    }
    pub async fn teams_device_revoke(
        &self,
        params: TeamDeviceParams,
    ) -> Result<Value, OperationError> {
        crate::cloud_teams::revoke_device(self.cloud.auth(), params).await
    }
    pub async fn teams_invitations(
        &self,
        host_id: String,
    ) -> Result<TeamInvitationList, OperationError> {
        crate::cloud_teams::invitations(self.cloud.auth(), &host_id).await
    }
    pub async fn teams_invitation_create(
        &self,
        params: TeamInvitationCreateParams,
    ) -> Result<TeamInvitationCreated, OperationError> {
        crate::cloud_teams::create_invitation(self.cloud.auth(), params).await
    }
    pub async fn teams_invitation_revoke(
        &self,
        params: TeamInvitationParams,
    ) -> Result<Value, OperationError> {
        crate::cloud_teams::revoke_invitation(self.cloud.auth(), params).await
    }
    pub async fn teams_requests(&self, host_id: String) -> Result<TeamRequestList, OperationError> {
        crate::cloud_teams::requests(self.cloud.auth(), &host_id).await
    }
    pub async fn teams_request_decide(
        &self,
        params: TeamDecisionParams,
    ) -> Result<Value, OperationError> {
        crate::cloud_teams::decide_request(self.cloud.auth(), params).await
    }
    pub async fn teams_grants(&self, host_id: String) -> Result<TeamGrantList, OperationError> {
        crate::cloud_teams::grants(self.cloud.auth(), &host_id).await
    }
    pub async fn teams_grant_update(
        &self,
        params: TeamGrantUpdateParams,
    ) -> Result<Value, OperationError> {
        crate::cloud_teams::update_grant(self.cloud.auth(), params).await
    }
    pub async fn teams_grant_revoke(
        &self,
        params: TeamGrantRevokeParams,
    ) -> Result<Value, OperationError> {
        crate::cloud_teams::revoke_grant(self.cloud.auth(), params).await
    }
    pub async fn teams_sessions(&self, host_id: String) -> Result<TeamSessionList, OperationError> {
        crate::cloud_teams::sessions(self.cloud.auth(), &host_id).await
    }
    pub async fn teams_session_revoke(
        &self,
        params: TeamSessionParams,
    ) -> Result<Value, OperationError> {
        crate::cloud_teams::revoke_session(self.cloud.auth(), params).await
    }
}
