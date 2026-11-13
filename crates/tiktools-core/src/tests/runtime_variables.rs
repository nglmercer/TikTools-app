use super::*;
use crate::control::{RuntimeVariable, RuntimeVariableKind as Kind, RuntimeVariablesReplace};
use serde_json::json;

fn variable(kind: Kind, value: serde_json::Value) -> RuntimeVariable {
    RuntimeVariable { kind, value }
}

#[tokio::test]
async fn typed_variables_keep_legacy_text_and_render_each_explicit_type() {
    let (core, _, root) = core_with_fake_plugins(Arc::new(FakePluginState::default()), &[]);
    core.globals_set("legacy", "true").unwrap();
    let first = core.runtime_variables_list().unwrap();
    assert_eq!(
        first.variables["legacy"],
        variable(Kind::Text, json!("true"))
    );
    let mut variables = first.variables.clone();
    variables.insert("literal".into(), variable(Kind::Text, json!("null")));
    variables.insert("port".into(), variable(Kind::Number, json!(46665)));
    variables.insert("enabled".into(), variable(Kind::Boolean, json!(false)));
    variables.insert("names".into(), variable(Kind::List, json!(["uno", "dos"])));
    variables.insert(
        "key".into(),
        variable(Kind::Secret, json!("local-test-secret")),
    );
    let result = core
        .runtime_variables_replace(RuntimeVariablesReplace {
            expected_revision: first.revision.clone(),
            variables: variables.clone(),
        })
        .unwrap();
    assert_ne!(result.revision, first.revision);
    assert_eq!(core.runtime_variables_list().unwrap(), result);
    let globals = core.globals_list().unwrap();
    assert_eq!(globals["literal"], "null");
    assert_eq!(globals["port"], "46665");
    assert_eq!(globals["enabled"], "false");
    assert_eq!(globals["names"], r#"["uno","dos"]"#);
    assert!(!format!("{result:?}").contains("local-test-secret"));
    let same = core
        .runtime_variables_replace(RuntimeVariablesReplace {
            expected_revision: result.revision.clone(),
            variables,
        })
        .unwrap();
    assert_eq!(same.revision, result.revision);
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn legacy_writes_preserve_secret_classification_and_delete_type_metadata() {
    let (core, _, root) = core_with_fake_plugins(Arc::new(FakePluginState::default()), &[]);
    let first = core.runtime_variables_list().unwrap();
    core.runtime_variables_replace(RuntimeVariablesReplace {
        expected_revision: first.revision,
        variables: [("token".into(), variable(Kind::Secret, json!("old-secret")))].into(),
    })
    .unwrap();
    core.globals_set("token", "new-secret").unwrap();
    assert!(core
        .app_state_set("runtime.variable.type.token", "text")
        .is_err());
    assert_eq!(
        core.runtime_variables_list().unwrap().variables["token"],
        variable(Kind::Secret, json!("new-secret"))
    );
    assert!(core.globals_delete("token").unwrap());
    assert!(!core.globals_delete("token").unwrap());
    core.globals_set("token", "ordinary").unwrap();
    assert_eq!(
        core.runtime_variables_list().unwrap().variables["token"].kind,
        Kind::Text
    );
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "persistence")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn variable_revision_prevents_lost_updates_across_independent_database_handles() {
    let (core, _, root) = core_with_fake_plugins(Arc::new(FakePluginState::default()), &[]);
    core.globals_set("port", "1").unwrap();
    let before = core.runtime_variables_list().unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let raw_revision = core
        .db
        .with_runtime_variables(false, |state| {
            crate::services::runtime_variables::snapshot(state)
        })
        .unwrap()
        .0
        .revision;
    let mut writers = Vec::new();
    for value in [2, 3] {
        let db = crate::db::DatabaseManager::new(core.db.paths().clone());
        let barrier = barrier.clone();
        let expected = raw_revision.clone();
        writers.push(std::thread::spawn(move || {
            barrier.wait();
            db.with_runtime_variables(true, |state| {
                crate::services::runtime_variables::replace(
                    state,
                    RuntimeVariablesReplace {
                        expected_revision: expected,
                        variables: [("port".into(), variable(Kind::Number, json!(value)))].into(),
                    },
                )
            })
        }));
    }
    barrier.wait();
    let results: Vec<_> = writers
        .into_iter()
        .map(|writer| writer.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let rejected = results.iter().find_map(|r| r.as_ref().err()).unwrap();
    assert_eq!(rejected.code, "conflict");
    let current = core.runtime_variables_list().unwrap();
    assert_ne!(current.revision, before.revision);
    assert!(matches!(
        current.variables["port"].value.as_i64(),
        Some(2 | 3)
    ));
    assert!(core
        .runtime_variables_replace(RuntimeVariablesReplace {
            expected_revision: before.revision,
            variables: before.variables
        })
        .is_err());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn another_host_lifetime_cannot_reuse_a_variable_revision_even_for_identical_data() {
    let (core, _, root) = core_with_fake_plugins(Arc::new(FakePluginState::default()), &[]);
    let original = core.runtime_variables_list().unwrap();
    let restarted = Arc::new(AppCore::with_paths(
        Arc::new(RecordingEmitter::default()),
        Arc::new(NoopMediaHost),
        core.db.paths().clone(),
    ));
    let current = restarted.runtime_variables_list().unwrap();
    assert_eq!(original.variables, current.variables);
    assert_ne!(original.revision, current.revision);
    let result = restarted.runtime_variables_replace(RuntimeVariablesReplace {
        expected_revision: original.revision,
        variables: [(
            "foreignDraft".into(),
            variable(Kind::Text, json!("do-not-save")),
        )]
        .into(),
    });
    assert_eq!(result.unwrap_err().code, "conflict");
    assert_eq!(restarted.runtime_variables_list().unwrap(), current);
    restarted.shutdown().await;
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn invalid_typed_replacement_preserves_every_existing_value() {
    let (core, _, root) = core_with_fake_plugins(Arc::new(FakePluginState::default()), &[]);
    core.globals_set("keep", "original").unwrap();
    let before = core.runtime_variables_list().unwrap();
    for bad in [
        variable(Kind::Number, json!("not a number")),
        variable(Kind::Boolean, json!(1)),
        variable(Kind::List, json!([1])),
        variable(Kind::Secret, json!(null)),
    ] {
        let result = core.runtime_variables_replace(RuntimeVariablesReplace {
            expected_revision: before.revision.clone(),
            variables: [("replacement".into(), bad)].into(),
        });
        assert!(result.is_err());
        assert_eq!(core.runtime_variables_list().unwrap(), before);
    }
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(all(feature = "cloud-auth", feature = "persistence"))]
#[tokio::test]
async fn variable_edits_cannot_overtake_an_admitted_cloud_emission() {
    let (core, _, root) = core_with_fake_plugins(Arc::new(FakePluginState::default()), &[]);
    core.globals_set("name", "public").unwrap();
    let before = core.runtime_variables_list().unwrap();
    let fence = tiktools_cloud_sync::ProductEmissionFence::open(
        core.db.paths().data.join("plugin-sync-emission"),
    )
    .unwrap();
    let emission = fence.try_emission().unwrap();
    let params = RuntimeVariablesReplace {
        expected_revision: before.revision.clone(),
        variables: [("name".into(), variable(Kind::Secret, json!("public")))].into(),
    };
    assert_eq!(
        core.runtime_variables_replace(params.clone())
            .unwrap_err()
            .code,
        "conflict"
    );
    assert!(core.globals_set("name", "changed").is_err());
    assert!(core.globals_delete("name").is_err());
    assert_eq!(core.runtime_variables_list().unwrap(), before);
    drop(emission);
    assert_eq!(
        core.runtime_variables_replace(params).unwrap().variables["name"].kind,
        Kind::Secret
    );
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn sqlite_failure_rolls_back_the_whole_variable_replacement_and_cache() {
    let (core, _, root) = core_with_fake_plugins(Arc::new(FakePluginState::default()), &[]);
    core.globals_set("keep", "original").unwrap();
    let before = core.runtime_variables_list().unwrap();
    let cache = core.app_state.read(None);
    let connection = rusqlite::Connection::open(core.db.points_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_variable BEFORE INSERT ON app_state WHEN NEW.key='runtime.variable.type.z' BEGIN SELECT RAISE(ABORT,'fixture rejects variable write'); END;").unwrap();
    assert!(core
        .runtime_variables_replace(RuntimeVariablesReplace {
            expected_revision: before.revision.clone(),
            variables: [
                ("a".into(), variable(Kind::Text, json!("new"))),
                ("z".into(), variable(Kind::Boolean, json!(true)))
            ]
            .into(),
        })
        .is_err());
    assert_eq!(core.runtime_variables_list().unwrap(), before);
    assert_eq!(core.app_state.read(None), cache);
    connection
        .execute_batch("DROP TRIGGER reject_variable")
        .unwrap();
    assert!(core.globals_delete("keep").unwrap());
    core.shutdown().await;
    let _ = std::fs::remove_dir_all(root);
}
