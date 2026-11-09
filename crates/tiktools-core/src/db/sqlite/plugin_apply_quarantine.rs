//! Startup observation of remote plugin applications whose runtime outcome
//! has not yet been finalized. This contains no Cloud credentials or source.
use std::collections::BTreeSet;

use super::{DatabaseError, DatabaseManager};

pub(super) const MAX_RETAINED: usize = 4096;

fn valid_token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl DatabaseManager {
    /// Identify plugins whose persisted source was changed but whose runtime
    /// outcome still needs native reconciliation. With Cloud support, validate
    /// all retained evidence before deciding whether a prepared/ready decision
    /// can permit startup. Finalized rows need an exactly matching witness.
    /// Without the Cloud decoder, any active evidence blocks startup globally.
    /// Corrupt rows, an orphan witness or unreadable storage also fail closed.
    pub(crate) fn plugin_apply_quarantine(&self) -> Result<BTreeSet<String>, DatabaseError> {
        let db = self.open(&self.automation_path())?;
        let mut statement = db.prepare(
            "SELECT i.intent_id,i.plugin_id,i.phase,i.payload_json,i.committed_generation,i.runtime_epoch,
                    f.intent_id,f.plugin_id,f.payload_json,f.committed_generation,f.runtime_epoch
             FROM plugin_sync_apply_intents i LEFT JOIN plugin_sync_apply_finalizations f ON f.intent_id=i.intent_id
             UNION ALL
             SELECT NULL,NULL,NULL,NULL,NULL,NULL,f.intent_id,f.plugin_id,f.payload_json,f.committed_generation,f.runtime_epoch
             FROM plugin_sync_apply_finalizations f LEFT JOIN plugin_sync_apply_intents i ON i.intent_id=f.intent_id
             WHERE i.intent_id IS NULL LIMIT ?1",
        )?;
        let mut rows = statement.query([(MAX_RETAINED + 1) as i64])?;
        #[cfg(feature = "cloud-auth")]
        let mut blocked = BTreeSet::new();
        #[cfg(not(feature = "cloud-auth"))]
        let blocked = BTreeSet::new();
        let mut count = 0usize;
        while let Some(row) = rows.next()? {
            count += 1;
            if count > MAX_RETAINED {
                return Err(DatabaseError::Invalid(
                    "too many plugin apply recoveries".into(),
                ));
            }
            let id: Option<String> = row.get(0)?;
            let id =
                id.ok_or_else(|| DatabaseError::Invalid("orphan plugin apply witness".into()))?;
            let plugin_id: String = row.get(1)?;
            let phase: String = row.get(2)?;
            let encoded: String = row.get(3)?;
            let generation: Option<i64> = row.get(4)?;
            let epoch: Option<String> = row.get(5)?;
            let witness_id: Option<String> = row.get(6)?;
            if phase == "finalized" {
                let witnessed = witness_id.as_deref() == Some(id.as_str())
                    && row.get::<_, Option<String>>(7)?.as_deref() == Some(plugin_id.as_str())
                    && row.get::<_, Option<String>>(8)?.as_deref() == Some(encoded.as_str())
                    && row.get::<_, Option<i64>>(9)? == generation
                    && row.get::<_, Option<String>>(10)? == epoch;
                if !witnessed
                    || !valid_token(&id)
                    || encoded.len() > 16 * 1024
                    || !epoch.as_deref().is_some_and(valid_token)
                    || !tiktools_plugin_api::manifest::is_valid_plugin_id(&plugin_id)
                {
                    return Err(DatabaseError::Invalid(
                        "invalid plugin apply finalization witness".into(),
                    ));
                }
            } else if witness_id.is_some() {
                return Err(DatabaseError::Invalid(
                    "unexpected plugin apply finalization witness".into(),
                ));
            }
            #[cfg(feature = "cloud-auth")]
            {
                use super::plugin_sync_apply::{decode_stored, PluginApplyPhase};
                let value = decode_stored(&id, (plugin_id, phase, encoded, generation, epoch))?;
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
                    PluginApplyPhase::Finalized => {}
                }
            }
            #[cfg(not(feature = "cloud-auth"))]
            {
                // This binary cannot validate Cloud journal payloads, so no
                // plugin may run until a capable native host reconciles them.
                if phase != "finalized" {
                    return Err(DatabaseError::Invalid(
                        "plugin apply recovery requires Cloud support".into(),
                    ));
                }
                // A Cloud-disabled build cannot decode the full portable
                // contract. Validate the identity and generation at least;
                // both independently retained copies must already match.
                let value: serde_json::Value = serde_json::from_str(&encoded)?;
                let source = value.get("sourceGeneration").and_then(|v| v.as_i64());
                if value.get("intentId").and_then(|v| v.as_str()) != Some(id.as_str())
                    || value.get("pluginId").and_then(|v| v.as_str()) != Some(plugin_id.as_str())
                    || source.filter(|g| *g > 0).and_then(|g| g.checked_add(1)) != generation
                {
                    return Err(DatabaseError::Invalid(
                        "invalid finalized plugin apply".into(),
                    ));
                }
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
        let connection = reopened.open(&reopened.automation_path()).unwrap();
        connection
            .execute(
                "DELETE FROM plugin_sync_apply_intents WHERE plugin_id='other.plugin'",
                [],
            )
            .unwrap();
        // Merely changing an applied phase must not hide its uncertain result.
        connection.execute("UPDATE plugin_sync_apply_intents SET phase='finalized' WHERE plugin_id='applied.plugin'", []).unwrap();
        drop(connection);
        assert!(reopened.plugin_apply_quarantine().is_err());
        let connection = reopened.open(&reopened.automation_path()).unwrap();
        connection.execute("INSERT INTO plugin_sync_apply_finalizations SELECT intent_id,plugin_id,payload_json,committed_generation,runtime_epoch FROM plugin_sync_apply_intents WHERE plugin_id='applied.plugin'", []).unwrap();
        drop(connection);
        assert_eq!(
            reopened.plugin_apply_quarantine().unwrap(),
            BTreeSet::from(["acme.plugin".to_owned()])
        );
        let connection = reopened.open(&reopened.automation_path()).unwrap();
        connection
            .execute(
                "UPDATE plugin_sync_apply_finalizations SET payload_json='{}'",
                [],
            )
            .unwrap();
        drop(connection);
        assert!(reopened.plugin_apply_quarantine().is_err());
        let connection = reopened.open(&reopened.automation_path()).unwrap();
        connection
            .execute(
                "DELETE FROM plugin_sync_apply_intents WHERE plugin_id='applied.plugin'",
                [],
            )
            .unwrap();
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
        let connection = db.open(&db.automation_path()).unwrap();
        connection
            .execute("DELETE FROM plugin_sync_apply_intents", [])
            .unwrap();
        let id = "c".repeat(64);
        let epoch = "d".repeat(64);
        let body =
            serde_json::json!({"intentId":id,"pluginId":"other.plugin","sourceGeneration":1})
                .to_string();
        connection.execute("INSERT INTO plugin_sync_apply_intents VALUES(?1,'other.plugin','finalized',?2,2,?3)", rusqlite::params![id, body, epoch]).unwrap();
        connection.execute("INSERT INTO plugin_sync_apply_finalizations SELECT intent_id,plugin_id,payload_json,committed_generation,runtime_epoch FROM plugin_sync_apply_intents", []).unwrap();
        drop(connection);
        assert!(db.plugin_apply_quarantine().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
