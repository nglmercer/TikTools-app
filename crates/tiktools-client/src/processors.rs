//! Typed `processors.*` methods.

use super::TikToolsClient;
use tiktools_control_api::modules::processors::{
    ProcessorListResult, ProcessorOutcomeDto, ProcessorStatusResult, ProcessorTestParams,
};
use tiktools_control_api::modules::Empty;
use tiktools_control_api::ClientError;

/// RPC names this module covers. Each typed method below calls
/// through its constant, so the coverage list and the methods
/// cannot drift apart; the parity test pins the list to the registry.
pub(crate) const METHODS: &[&str] = &[PROCESSORS_LIST, PROCESSORS_STATUS, PROCESSORS_TEST];

const PROCESSORS_LIST: &str = "processors.list";
const PROCESSORS_STATUS: &str = "processors.status";
const PROCESSORS_TEST: &str = "processors.test";

impl TikToolsClient {
    /// Indexed event processors.
    /// RPC method `processors.list`.
    pub async fn processors_list(&self) -> Result<ProcessorListResult, ClientError> {
        self.call(PROCESSORS_LIST, Empty::default()).await
    }
    /// Processor health and metrics snapshot.
    /// RPC method `processors.status`.
    pub async fn processors_status(&self) -> Result<ProcessorStatusResult, ClientError> {
        self.call(PROCESSORS_STATUS, Empty::default()).await
    }
    /// Runs one processor against a sample event.
    /// RPC method `processors.test`.
    pub async fn processors_test(
        &self,
        params: ProcessorTestParams,
    ) -> Result<ProcessorOutcomeDto, ClientError> {
        self.call(PROCESSORS_TEST, params).await
    }
}
