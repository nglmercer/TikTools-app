//! Typed `rpc.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::rpc::{DiscoverResult, SchemaParams};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;
use tiktools_control_api::MethodMeta;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[RPC_DISCOVER, RPC_SCHEMA];

const RPC_DISCOVER: &str = "rpc.discover";
const RPC_SCHEMA: &str = "rpc.schema";

impl TikToolsClient {
    /// Lists every method with descriptions and JSON Schemas.
    /// RPC method `rpc.discover`.
    pub async fn rpc_discover(&self) -> Result<DiscoverResult, ClientError> {
        self.call(RPC_DISCOVER, Empty::default()).await
    }
    /// Returns one method metadata entry with its schemas.
    /// RPC method `rpc.schema`.
    pub async fn rpc_schema(&self, params: SchemaParams) -> Result<MethodMeta, ClientError> {
        self.call(RPC_SCHEMA, params).await
    }
}
