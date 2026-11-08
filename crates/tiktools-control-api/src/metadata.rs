//! Version metadata travels beside the domain result, never in method params.
use serde::{Deserialize, Serialize};
use tiktools_core::collaboration::ResourceVersion;

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_instance_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMetadata {
    pub resource_versions: Vec<ResourceVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<tiktools_core::collaboration::CommandReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_instance_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallResult<T> {
    pub result: T,
    pub meta: Option<ResponseMetadata>,
}
