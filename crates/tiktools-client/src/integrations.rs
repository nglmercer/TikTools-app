use super::TikToolsClient;
use tiktools_control_api::modules::integrations::{
    IntegrationConnectParams, IntegrationConnectResult, IntegrationConnectionList,
    IntegrationDisconnectResult, IntegrationEventList, IntegrationIdParams,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

pub(crate) const METHODS: &[&str] = &[
    INTEGRATIONS_CONNECTIONS,
    INTEGRATIONS_CONNECT,
    INTEGRATIONS_EVENTS,
    INTEGRATIONS_DISCONNECT,
];

const INTEGRATIONS_CONNECTIONS: &str = "integrations.connections";
const INTEGRATIONS_CONNECT: &str = "integrations.connect";
const INTEGRATIONS_EVENTS: &str = "integrations.events";
const INTEGRATIONS_DISCONNECT: &str = "integrations.disconnect";

impl TikToolsClient {
    pub async fn integrations_connections(&self) -> Result<IntegrationConnectionList, ClientError> {
        self.call(INTEGRATIONS_CONNECTIONS, Empty::default()).await
    }
    pub async fn integrations_connect(
        &self,
        params: IntegrationConnectParams,
    ) -> Result<IntegrationConnectResult, ClientError> {
        self.call(INTEGRATIONS_CONNECT, params).await
    }
    pub async fn integrations_events(
        &self,
        params: IntegrationIdParams,
    ) -> Result<IntegrationEventList, ClientError> {
        self.call(INTEGRATIONS_EVENTS, params).await
    }
    pub async fn integrations_disconnect(
        &self,
        params: IntegrationIdParams,
    ) -> Result<IntegrationDisconnectResult, ClientError> {
        self.call(INTEGRATIONS_DISCONNECT, params).await
    }
}
