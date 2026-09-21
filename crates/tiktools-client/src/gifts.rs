//! Typed `gifts.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::gifts::{GiftCatalogResult, GiftDebugParams, GiftDebugResult};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[GIFTS_LIST, GIFTS_DEBUG];

const GIFTS_LIST: &str = "gifts.list";
const GIFTS_DEBUG: &str = "gifts.debug";

impl TikToolsClient {
    /// Persisted gift catalog ordered by diamond count.
    /// RPC method `gifts.list`.
    pub async fn gifts_list(&self) -> Result<GiftCatalogResult, ClientError> {
        self.call(GIFTS_LIST, Empty::default()).await
    }
    /// Gift icon lookup plus the catalog size.
    /// RPC method `gifts.debug`.
    pub async fn gifts_debug(
        &self,
        params: GiftDebugParams,
    ) -> Result<GiftDebugResult, ClientError> {
        self.call(GIFTS_DEBUG, params).await
    }
}
