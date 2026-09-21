use std::{collections::BTreeMap, sync::RwLock};

use tiktools_plugin_api::sync::{recover_rwlock_read, recover_rwlock_write};

#[derive(Default)]
pub struct AppStateService {
    values: RwLock<BTreeMap<String, String>>,
}

impl AppStateService {
    pub fn read(&self, keys: Option<&[String]>) -> BTreeMap<String, String> {
        let values = recover_rwlock_read(&self.values, "app state");
        match keys {
            Some(keys) if !keys.is_empty() => keys
                .iter()
                .filter_map(|key| values.get(key).map(|value| (key.clone(), value.clone())))
                .collect(),
            _ => values.clone(),
        }
    }

    pub fn set(&self, key: String, value: String) {
        recover_rwlock_write(&self.values, "app state").insert(key, value);
    }
}
