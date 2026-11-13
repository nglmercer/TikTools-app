//! Atomic typed/legacy global slice updates. Independent hosts share SQLite's
//! transaction boundary; failure never publishes only part of a replacement.
use super::{util::now, DatabaseManager};
use crate::{
    control::OperationError,
    services::runtime_variables::{variable_state_key, MAX_VARIABLE_STATE_BYTES},
};
use rusqlite::{params, TransactionBehavior};
use std::collections::BTreeMap;

fn storage_error(error: impl Into<super::DatabaseError>) -> OperationError {
    let error = error.into();
    tracing::warn!(
        sqlite_code = error.sqlite_extended_code(),
        "runtime variable storage operation failed"
    );
    OperationError::unavailable("runtime variables storage is unavailable")
}

impl DatabaseManager {
    #[cfg(feature = "cloud-auth")]
    pub(crate) fn runtime_variable_secret_names(
        &self,
    ) -> Result<std::collections::BTreeSet<String>, OperationError> {
        use crate::services::runtime_variables::TYPE_PREFIX;
        let connection = self.open(&self.points_path()).map_err(storage_error)?;
        let mut statement = connection
            .prepare("SELECT key,value FROM app_state WHERE key LIKE 'runtime.variable.type.%'")
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage_error)?;
        let mut names = std::collections::BTreeSet::new();
        let mut bytes = 0usize;
        for row in rows {
            let (key, tag) = row.map_err(storage_error)?;
            bytes = bytes.saturating_add(key.len()).saturating_add(tag.len());
            if bytes > 65_536 {
                return Err(OperationError::unavailable(
                    "runtime variable metadata exceeds the storage limit",
                ));
            }
            let Some(name) = key.strip_prefix(TYPE_PREFIX) else {
                continue;
            };
            match tag.as_str() {
                "secret" => {
                    names.insert(name.to_owned());
                }
                "text" | "number" | "boolean" | "list" => {}
                _ => {
                    return Err(OperationError::unavailable(
                        "runtime variable type metadata is invalid",
                    ))
                }
            }
        }
        Ok(names)
    }

    pub(crate) fn with_runtime_variables<R>(
        &self,
        write: bool,
        operation: impl FnOnce(&mut BTreeMap<String, String>) -> Result<R, OperationError>,
    ) -> Result<(R, BTreeMap<String, String>), OperationError> {
        let mut connection = self.open(&self.points_path()).map_err(storage_error)?;
        let transaction = connection
            .transaction_with_behavior(if write {
                TransactionBehavior::Immediate
            } else {
                TransactionBehavior::Deferred
            })
            .map_err(storage_error)?;
        let mut state = BTreeMap::new();
        {
            let mut statement = transaction.prepare("SELECT key,value FROM app_state WHERE key LIKE 'globals.%' OR key LIKE 'runtime.variable.%'").map_err(storage_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(storage_error)?;
            let mut bytes = 0usize;
            for row in rows {
                let (key, value) = row.map_err(storage_error)?;
                bytes = bytes.saturating_add(key.len()).saturating_add(value.len());
                if bytes > MAX_VARIABLE_STATE_BYTES {
                    return Err(OperationError::unavailable(
                        "runtime variables exceed the storage limit",
                    ));
                }
                if variable_state_key(&key) {
                    state.insert(key, value);
                }
            }
        }
        let previous = state.clone();
        let result = operation(&mut state)?;
        if !write && state != previous {
            return Err(OperationError::unavailable(
                "runtime variable read attempted a write",
            ));
        }
        if write {
            for key in previous.keys().filter(|key| !state.contains_key(*key)) {
                transaction
                    .execute("DELETE FROM app_state WHERE key=?", [key])
                    .map_err(storage_error)?;
            }
            for (key, value) in &state {
                if previous.get(key) == Some(value) {
                    continue;
                }
                if !variable_state_key(key) {
                    return Err(OperationError::invalid(
                        "invalid runtime variable state key",
                    ));
                }
                transaction.execute("INSERT INTO app_state(key,value,updated_at) VALUES(?,?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at", params![key,value,now()]).map_err(storage_error)?;
            }
        }
        transaction.commit().map_err(storage_error)?;
        Ok((result, state))
    }
}
