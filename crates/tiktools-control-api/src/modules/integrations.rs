//! Cloud integrations (webhook provider). Only `integrations.connect` returns a secret,
//! once; every other reply is nonsecret.

use std::sync::Arc;

pub use tiktools_core::cloud_integrations::{
    IntegrationConnectParams, IntegrationConnectResult, IntegrationConnection,
    IntegrationConnectionList, IntegrationDisconnectResult, IntegrationEvent, IntegrationEventList,
    IntegrationIdParams,
};
use tiktools_core::AppCore;

use super::Empty;
use crate::{error::ApiError, router::ControlRouter};

pub fn register(router: &mut ControlRouter) {
    router.register_typed::<Empty, IntegrationConnectionList, _, _>(
        "integrations.connections",
        "Lists the Cloud integration connections owned by the signed-in account",
        false,
        |core: Arc<AppCore>, _params: Empty| async move {
            core.integration_connections().await.map_err(ApiError::from)
        },
    );
    router.register_typed::<IntegrationConnectParams, IntegrationConnectResult, _, _>(
        "integrations.connect",
        "Creates a webhook connection; the secret is returned once and is not stored by the host",
        true,
        |core: Arc<AppCore>, params: IntegrationConnectParams| async move {
            core.integration_connect(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<IntegrationIdParams, IntegrationEventList, _, _>(
        "integrations.events",
        "Reads the newest webhook events of one owned connection, bounded to the reply budget",
        false,
        |core: Arc<AppCore>, params: IntegrationIdParams| async move {
            core.integration_events(params)
                .await
                .map_err(ApiError::from)
        },
    );
    router.register_typed::<IntegrationIdParams, IntegrationDisconnectResult, _, _>(
        "integrations.disconnect",
        "Revokes a webhook connection; later deliveries are refused and stored events stay readable",
        true,
        |core: Arc<AppCore>, params: IntegrationIdParams| async move {
            core.integration_disconnect(params).await.map_err(ApiError::from)
        },
    );
}
