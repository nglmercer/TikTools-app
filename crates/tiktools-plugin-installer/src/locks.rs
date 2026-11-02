use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use crate::InstallError;

#[derive(Debug, Clone, Default)]
pub(crate) struct KeyedLocks {
    held: Arc<Mutex<BTreeSet<String>>>,
}

pub(crate) struct KeyedGuard {
    held: Arc<Mutex<BTreeSet<String>>>,
    key: String,
}

impl KeyedLocks {
    pub(crate) fn try_lock(&self, key: &str) -> Result<KeyedGuard, InstallError> {
        let mut held = self
            .held
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !held.insert(key.to_owned()) {
            return Err(InstallError::busy(key));
        }
        Ok(KeyedGuard {
            held: self.held.clone(),
            key: key.to_owned(),
        })
    }

    pub(crate) fn is_locked(&self, key: &str) -> bool {
        self.held
            .lock()
            .map(|held| held.contains(key))
            .unwrap_or(false)
    }
}

impl Drop for KeyedGuard {
    fn drop(&mut self) {
        if let Ok(mut held) = self.held.lock() {
            held.remove(&self.key);
        }
    }
}
