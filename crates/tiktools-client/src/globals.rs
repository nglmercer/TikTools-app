//! Typed `globals.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::globals::{
    GlobalsDeleteParams, GlobalsDeleteResult, GlobalsGetParams, GlobalsListResult,
    GlobalsSetParams, GlobalsValueResult, RuntimeVariablesReplace, RuntimeVariablesSnapshot,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[
    GLOBALS_LIST,
    GLOBALS_GET,
    GLOBALS_SET,
    GLOBALS_DELETE,
    GLOBALS_TYPED_LIST,
    GLOBALS_TYPED_REPLACE,
];

const GLOBALS_TYPED_LIST: &str = "globals.typed.list";
const GLOBALS_TYPED_REPLACE: &str = "globals.typed.replace";

const GLOBALS_LIST: &str = "globals.list";
const GLOBALS_GET: &str = "globals.get";
const GLOBALS_SET: &str = "globals.set";
const GLOBALS_DELETE: &str = "globals.delete";

impl TikToolsClient {
    /// Local typed variables and the revision required for an atomic edit.
    pub async fn globals_typed_list(&self) -> Result<RuntimeVariablesSnapshot, ClientError> {
        self.call(GLOBALS_TYPED_LIST, Empty {}).await
    }

    /// Save exactly the reviewed local revision. This does not sync values.
    pub async fn globals_typed_replace(
        &self,
        params: RuntimeVariablesReplace,
    ) -> Result<RuntimeVariablesSnapshot, ClientError> {
        self.call(GLOBALS_TYPED_REPLACE, params).await
    }

    /// Every runtime global (bare key to text value).
    /// RPC method `globals.list`.
    pub async fn globals_list(&self) -> Result<GlobalsListResult, ClientError> {
        self.call(GLOBALS_LIST, Empty {}).await
    }
    /// One runtime global by bare key.
    /// RPC method `globals.get`.
    pub async fn globals_get(
        &self,
        params: GlobalsGetParams,
    ) -> Result<GlobalsValueResult, ClientError> {
        self.call(GLOBALS_GET, params).await
    }
    /// Creates or replaces one runtime global.
    /// RPC method `globals.set`.
    pub async fn globals_set(
        &self,
        params: GlobalsSetParams,
    ) -> Result<GlobalsValueResult, ClientError> {
        self.call(GLOBALS_SET, params).await
    }
    /// Deletes one runtime global.
    /// RPC method `globals.delete`.
    pub async fn globals_delete(
        &self,
        params: GlobalsDeleteParams,
    ) -> Result<GlobalsDeleteResult, ClientError> {
        self.call(GLOBALS_DELETE, params).await
    }
}
