//! `globals.*` operations: validated CRUD over the `globals.` app_state slice.
//!
//! Globals persist in the shared `app_state` store (memory plus the sqlite
//! table when `persistence` is on), so CLI, desktop, and headless hosts
//! observe one library. The render path snapshots through the same slice.

use std::collections::BTreeMap;

use crate::control::OperationError;
use crate::services::globals::{
    bare_key, normalize_value, storage_key, validate_key, GLOBALS_KEY_PREFIX, MAX_GLOBAL_KEYS,
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
        let snapshot = self.globals_snapshot();
        if !snapshot.contains_key(key) && snapshot.len() >= MAX_GLOBAL_KEYS {
            return Err(OperationError::invalid("at most 128 globals"));
        }
        self.app_state_set(&storage_key(key), &value)?;
        Ok((key.to_owned(), value))
    }

    /// Deletes one global; `Ok(false)` when the key did not exist.
    pub fn globals_delete(&self, key: &str) -> Result<bool, OperationError> {
        validate_key(key)?;
        let storage = storage_key(key);
        let existed = self.globals_snapshot().contains_key(key);
        self.app_state.remove(&storage);
        #[cfg(feature = "persistence")]
        if let Err(error) = self.db.delete_app_state(&storage) {
            tracing::warn!(%error, "could not delete persisted global");
        }
        Ok(existed)
    }

    /// Render-path snapshot. Infallible: actions render with whatever loads.
    pub(crate) fn automation_globals(&self) -> BTreeMap<String, String> {
        self.globals_snapshot()
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
