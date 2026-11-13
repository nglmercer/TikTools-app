use super::OperationError;
#[cfg(feature = "persistence")]
use crate::services::runtime_variables::variable_state_key;
pub use crate::services::runtime_variables::{
    RuntimeVariable, RuntimeVariableKind, RuntimeVariablesReplace, RuntimeVariablesSnapshot,
};
use crate::{services::runtime_variables, AppCore};
use std::collections::BTreeMap;

impl AppCore {
    fn bind_variable_revision(
        &self,
        mut snapshot: RuntimeVariablesSnapshot,
    ) -> RuntimeVariablesSnapshot {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(self.variable_revision_binding.to_le_bytes());
        digest.update(snapshot.revision.as_bytes());
        snapshot.revision = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        snapshot
    }

    #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
    pub(crate) fn runtime_variable_secret_names(
        &self,
    ) -> Result<std::collections::BTreeSet<String>, OperationError> {
        self.db.runtime_variable_secret_names()
    }

    pub(crate) fn with_runtime_variables<R>(
        &self,
        write: bool,
        operation: impl FnOnce(&mut BTreeMap<String, String>) -> Result<R, OperationError>,
    ) -> Result<R, OperationError> {
        // The network dispatcher retains a shared emission lease until its
        // bounded request finishes. Classification cannot change to secret
        // after admission while that request is still allowed to emit.
        #[cfg(all(feature = "cloud-auth", feature = "persistence"))]
        let _emission = if write {
            Some(
                tiktools_cloud_sync::ProductEmissionFence::open(
                    self.db.paths().data.join("plugin-sync-emission"),
                )
                .map_err(|_| OperationError::unavailable("variable sync fence is unavailable"))?
                .try_untracked_edit()
                .map_err(|error| match error {
                    tiktools_cloud_sync::ProtocolFenceError::Busy => OperationError::conflict(
                        "a sync write is in flight; retry saving variables",
                    ),
                    _ => OperationError::unavailable("variable sync fence is unavailable"),
                })?,
            )
        } else {
            None
        };
        #[cfg(feature = "persistence")]
        {
            let (result, state) = self.db.with_runtime_variables(write, operation)?;
            if write {
                self.app_state.with_values(|values| {
                    values.retain(|key, _| !variable_state_key(key));
                    values.extend(state);
                });
            }
            Ok(result)
        }
        #[cfg(not(feature = "persistence"))]
        {
            let _ = write;
            self.app_state.with_values(|values| {
                // Commit the candidate only after all validation succeeds.
                let mut candidate = values.clone();
                let result = operation(&mut candidate)?;
                *values = candidate;
                Ok(result)
            })
        }
    }

    pub fn runtime_variables_list(&self) -> Result<RuntimeVariablesSnapshot, OperationError> {
        self.with_runtime_variables(false, |state| {
            runtime_variables::snapshot(state).map(|snapshot| self.bind_variable_revision(snapshot))
        })
    }

    pub fn runtime_variables_replace(
        &self,
        mut params: RuntimeVariablesReplace,
    ) -> Result<RuntimeVariablesSnapshot, OperationError> {
        self.with_runtime_variables(true, |state| {
            let current = runtime_variables::snapshot(state)?;
            if params.expected_revision != self.bind_variable_revision(current.clone()).revision {
                return Err(OperationError::conflict(
                    "host or runtime variables changed; reload before saving",
                ));
            }
            params.expected_revision = current.revision;
            runtime_variables::replace(state, params)
                .map(|snapshot| self.bind_variable_revision(snapshot))
        })
    }
}
