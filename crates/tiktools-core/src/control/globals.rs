//! `globals.*` operations: validated CRUD over the `globals.` app_state slice.
//!
//! Globals persist in the shared `app_state` store (memory plus the sqlite
//! table when `persistence` is on), so CLI, desktop, and headless hosts
//! observe one library. The render path snapshots through the same slice.

use std::collections::BTreeMap;

use crate::control::OperationError;
#[cfg(feature = "persistence")]
use crate::services::globals::GLOBALS_KEY_PREFIX;
use crate::services::globals::{
    bare_key, normalize_value, storage_key, validate_key, MAX_GLOBAL_KEYS,
};
use crate::AppCore;

impl AppCore {
    /// Every global as bare key → text value, ordered by key.
    pub fn globals_list(&self) -> Result<BTreeMap<String, String>, OperationError> {
        Ok(self.globals_snapshot())
    }

    /// One global value by bare key.
    pub fn globals_get(&self, key: &str) -> Result<String, OperationError> {
        validate_key(key)?;
        self.globals_snapshot()
            .remove(key)
            .ok_or_else(|| OperationError::not_found(format!("unknown global `{key}`")))
    }

    /// Creates or replaces one global; raw input normalizes for rendering.
    pub fn globals_set(&self, key: &str, raw: &str) -> Result<(String, String), OperationError> {
        validate_key(key)?;
        let value = normalize_value(raw)?;
        self.with_runtime_variables(true, |state| {
            use crate::services::runtime_variables::{REVISION_KEY, TYPE_PREFIX};
            let storage = storage_key(key);
            if !state.contains_key(&storage)
                && state
                    .keys()
                    .filter(|key| key.starts_with(crate::services::globals::GLOBALS_KEY_PREFIX))
                    .count()
                    >= MAX_GLOBAL_KEYS
            {
                return Err(OperationError::invalid("at most 128 globals"));
            }
            let type_key = format!("{TYPE_PREFIX}{key}");
            // A client using the legacy API cannot accidentally make a local
            // secret shareable by replacing its textual value.
            let kind = if state.get(&type_key).map(String::as_str) == Some("secret") {
                "secret"
            } else {
                "text"
            };
            if state.get(&storage) != Some(&value)
                || state.get(&type_key).map(String::as_str) != Some(kind)
            {
                state.insert(storage, value.clone());
                state.insert(type_key, kind.to_owned());
                state.insert(
                    REVISION_KEY.to_owned(),
                    format!("{:032x}", fastrand::u128(..)),
                );
            }
            Ok((key.to_owned(), value))
        })
    }

    /// Deletes one global; `Ok(false)` when the key did not exist.
    pub fn globals_delete(&self, key: &str) -> Result<bool, OperationError> {
        validate_key(key)?;
        self.with_runtime_variables(true, |state| {
            use crate::services::runtime_variables::{REVISION_KEY, TYPE_PREFIX};
            let existed = state.remove(&storage_key(key)).is_some();
            let metadata = state.remove(&format!("{TYPE_PREFIX}{key}")).is_some();
            if existed || metadata {
                state.insert(
                    REVISION_KEY.to_owned(),
                    format!("{:032x}", fastrand::u128(..)),
                );
            }
            Ok(existed)
        })
    }

    /// Authority is checked at the read boundary, including plugin-intent and
    /// declarative-HTTP rereads. Unsupported delegated access never loads the
    /// unrestricted snapshot, even if its caller forgot an earlier guard.
    pub(crate) fn automation_globals(
        &self,
        context: &crate::automation_runtime::context::ExecutionContext,
    ) -> Result<BTreeMap<String, String>, String> {
        self.execution_context_guard(context)?;
        Ok(self.globals_snapshot())
    }

    fn globals_snapshot(&self) -> BTreeMap<String, String> {
        #[cfg(feature = "persistence")]
        {
            match self.db.load_app_state_prefix(GLOBALS_KEY_PREFIX) {
                Ok(persisted) => {
                    return persisted
                        .into_iter()
                        .filter_map(|(key, value)| {
                            let key = bare_key(&key)?.to_owned();
                            value.as_str().map(|value| (key, value.to_owned()))
                        })
                        .collect();
                }
                Err(error) => tracing::warn!(%error, "could not load globals"),
            }
        }
        self.app_state
            .read(None)
            .into_iter()
            .filter_map(|(key, value)| bare_key(&key).map(|key| (key.to_owned(), value)))
            .collect()
    }
}
