//! Cloud integration operations for the host API. The webhook secret crosses this
//! boundary only in the `connect` reply; every other reply is nonsecret.

use super::OperationError;
use crate::{
    cloud_integrations::{
        IntegrationConnectParams, IntegrationConnectResult, IntegrationConnectionList,
        IntegrationDisconnectResult, IntegrationEventList, IntegrationIdParams,
    },
    AppCore,
};

impl AppCore {
    pub async fn integration_connections(
        &self,
    ) -> Result<IntegrationConnectionList, OperationError> {
        crate::cloud_integrations::list(self.cloud.auth()).await
    }

    pub async fn integration_connect(
        &self,
        params: IntegrationConnectParams,
    ) -> Result<IntegrationConnectResult, OperationError> {
        crate::cloud_integrations::connect(self.cloud.auth(), params).await
    }

    pub async fn integration_events(
        &self,
        params: IntegrationIdParams,
    ) -> Result<IntegrationEventList, OperationError> {
        crate::cloud_integrations::events(self.cloud.auth(), params).await
    }

    pub async fn integration_disconnect(
        &self,
        params: IntegrationIdParams,
    ) -> Result<IntegrationDisconnectResult, OperationError> {
        crate::cloud_integrations::disconnect(self.cloud.auth(), params).await
    }
}
