//! Startup observation of remote plugin applications whose runtime outcome
//! has not yet been finalized. This contains no Cloud credentials or source.
use std::collections::BTreeSet;

use super::{DatabaseError, DatabaseManager};

const MAX_ACTIVE: usize = 4096;

impl DatabaseManager {
    /// Identify plugins whose persisted source was changed but whose runtime
    /// outcome still needs native reconciliation. With Cloud support, validate
    /// all active evidence before deciding whether a prepared/ready decision
    /// can permit startup. Without its decoder, any active evidence blocks
    /// startup globally. Corrupt rows or unreadable storage also fail closed.
    pub(crate) fn plugin_apply_quarantine(&self) -> Result<BTreeSet<String>, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        let mut statement = db.prepare(
            "SELECT intent_id,plugin_id,phase,payload_json,committed_generation,runtime_epoch FROM plugin_sync_apply_intents
             WHERE phase IS NULL OR phase!='finalized' LIMIT ?1",
        )?;
        let mut rows = statement.query([(MAX_ACTIVE + 1) as i64])?;
        #[cfg(feature = "cloud-auth")]
        let mut blocked = BTreeSet::new();
        #[cfg(not(feature = "cloud-auth"))]
        let blocked = BTreeSet::new();
        let mut count = 0usize;
        while let Some(row) = rows.next()? {
            count += 1;
            if count > MAX_ACTIVE {
                return Err(DatabaseError::Invalid(
                    "too many plugin apply recoveries".into(),
                ));
            }
            #[cfg(feature = "cloud-auth")]
            {
                use super::plugin_sync_apply::{decode_stored, PluginApplyPhase};
                let id: String = row.get(0)?;
                let value = decode_stored(
                    &id,
                    (
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ),
                )?;
                if !tiktools_plugin_api::manifest::is_valid_plugin_id(&value.intent.plugin_id) {
                    return Err(DatabaseError::Invalid(
                        "invalid plugin apply recovery".into(),
                    ));
                }
                match value.phase {
                    PluginApplyPhase::Prepared | PluginApplyPhase::Ready => {}
                    PluginApplyPhase::RuntimePending | PluginApplyPhase::Applied => {
                        blocked.insert(value.intent.plugin_id);
                    }
                    PluginApplyPhase::Finalized => {
                        return Err(DatabaseError::Invalid(
                            "invalid plugin apply recovery".into(),
                        ))
                    }
                }
            }
            #[cfg(not(feature = "cloud-auth"))]
            {
                // This binary cannot validate Cloud journal payloads, so no
                // plugin may run until a capable native host reconciles them.
                let _ = row;
                return Err(DatabaseError::Invalid(
                    "plugin apply recovery requires Cloud support".into(),
                ));
            }
        }
        Ok(blocked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;

    #[cfg(feature = "cloud-auth")]
    fn payload(intent_id: &str, plugin_id: &str) -> String {
        use super::super::plugin_sync_apply::{PluginApplyIntent, PluginApplyReason};
        use tiktools_cloud_sync::{PluginSelection, V1Record};
        serde_json::to_string(&PluginApplyIntent {
            intent_id: intent_id.into(),
            origin: "https://cloud.example".into(),
            account_id: "alice".into(),
            config_generation: 1,
            plugin_id: plugin_id.into(),
            source_generation: 1,
            installed_version: "1.0.0".into(),
            expected_enabled: false,
            reviewed_pending: None,
            base_record: None,
            remote: Some(V1Record {
                collection: "plugin-selections".into(),
                key: plugin_id.into(),
                revision: 1,
                deleted: false,
                value: Some(PluginSelection {
                    schema_version: 1,
                    plugin_id: plugin_id.into(),
                    version: "1.0.0".into(),
                    enabled: true,
                }),
            }),
            reason: PluginApplyReason::Pulled,
        })
        .unwrap()
    }

    #[cfg(feature = "cloud-auth")]
    #[test]
    fn uncertain_runtime_is_quarantined_without_changing_saved_activation() {
        let root = std::env::temp_dir().join(format!(
            "plugin-apply-quarantine-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let db = DatabaseManager::new(paths);
        db.set_plugin_state("acme.plugin", true, true).unwrap();
        let connection = db.open(&db.automation_path()).unwrap();
        connection.execute("INSERT INTO plugin_sync_apply_intents(intent_id,plugin_id,phase,payload_json,committed_generation) VALUES(?1,'acme.plugin','runtime_pending',?2,2)", rusqlite::params!["a".repeat(64), payload(&"a".repeat(64), "acme.plugin")]).unwrap();
        connection.execute("INSERT INTO plugin_sync_apply_intents(intent_id,plugin_id,phase,payload_json) VALUES(?1,'other.plugin','ready',?2)", rusqlite::params!["b".repeat(64), payload(&"b".repeat(64), "other.plugin")]).unwrap();
        connection.execute("INSERT INTO plugin_sync_apply_intents(intent_id,plugin_id,phase,payload_json,committed_generation,runtime_epoch) VALUES(?1,'applied.plugin','applied',?2,2,?3)", rusqlite::params!["c".repeat(64), payload(&"c".repeat(64), "applied.plugin"), "d".repeat(64)]).unwrap();
        drop(connection);
        let reopened = DatabaseManager::new(db.paths().clone());
        assert_eq!(
            reopened.plugin_apply_quarantine().unwrap(),
            BTreeSet::from(["acme.plugin".to_owned(), "applied.plugin".to_owned()])
        );
        let snapshot = reopened.load_behavior_snapshot().unwrap();
        let plugin = snapshot["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == "acme.plugin")
            .unwrap();
        assert_eq!(plugin["enabled"], true);
        let connection = reopened.open(&reopened.automation_path()).unwrap();
        connection.execute("UPDATE plugin_sync_apply_intents SET committed_generation=2 WHERE plugin_id='other.plugin'", []).unwrap();
        drop(connection);
        assert!(reopened.plugin_apply_quarantine().is_err());
        let connection = reopened.open(&reopened.automation_path()).unwrap();
        connection.execute("UPDATE plugin_sync_apply_intents SET committed_generation=NULL,payload_json='{}' WHERE plugin_id='other.plugin'", []).unwrap();
        drop(connection);
        assert!(reopened.plugin_apply_quarantine().is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(not(feature = "cloud-auth"))]
    #[test]
    fn a_build_without_cloud_support_cannot_authorize_any_active_evidence() {
        let root = std::env::temp_dir().join(format!(
            "plugin-apply-no-cloud-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("absent"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        let db = DatabaseManager::new(paths);
        assert!(db.plugin_apply_quarantine().unwrap().is_empty());
        let connection = db.open(&db.automation_path()).unwrap();
        connection.execute("INSERT INTO plugin_sync_apply_intents(intent_id,plugin_id,phase,payload_json) VALUES(?1,'other.plugin','ready','{}')", ["b".repeat(64)]).unwrap();
        drop(connection);
        assert!(db.plugin_apply_quarantine().is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
