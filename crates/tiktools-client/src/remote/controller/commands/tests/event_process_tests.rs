//! Native process termination against a real TLS host. Credentials exist only
//! in a private fixture file; no real OS store, app runtime or LIVE is created.
use super::*;
use secrecy::ExposeSecret;
use tiktools_collaboration::{Grant, GrantSet};
use tiktools_core::control::AutomationKind;

fn event_grant(write: bool) -> Grant {
    let mut capabilities = [Capability::AutomationRead, Capability::CommandsRead]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    if write {
        capabilities.extend([Capability::AutomationToggle, Capability::AutomationEdit]);
    }
    Grant {
        capabilities,
        resources: [(
            ResourceKind::Automation,
            ["event:same".into(), "action:same".into()].into(),
        )]
        .into(),
        ..Grant::default()
    }
}
fn owner_event() -> Value {
    json!({"id":"same","name":"Owner event","enabled":true,"trigger":"tiktok.chat","actionIds":["same"]})
}
fn event_input(c: &Controller, operation: &str) -> Value {
    let s = c.status();
    let hello = s.hello.as_ref().unwrap();
    let page = c.automations(&s.epoch, &s.automation_revision, 0).unwrap();
    let row = page["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["summary"]["kind"] == "event" && row["summary"]["id"] == "same")
        .unwrap();
    json!({"epoch":s.epoch,"automationRevision":s.automation_revision,
        "connectionId":hello.connection_id,"hostInstanceId":hello.host_instance_id,
        "grantRevision":hello.grant_revision,"id":"same",
        "expectedRevision":row["version"]["revision"],"operation":operation})
}

#[tokio::test]
#[ignore = "private subprocess invoked by the native event process-death test"]
async fn native_event_process_fixture() {
    let root =
        std::path::PathBuf::from(std::env::var_os("TIKTOOLS_NATIVE_EVENT_FIXTURE_ROOT").unwrap());
    assert!(root.starts_with(std::env::temp_dir()));
    let manager = ProfileManager::new(
        root.clone(),
        Arc::new(FixtureFileStore(root.join("test-credentials.input"))),
    )
    .unwrap();
    let c = Controller::new(manager);
    c.connect("events").await.unwrap();
    let operation = std::env::var("TIKTOOLS_NATIVE_EVENT_FIXTURE_OPERATION").unwrap();
    c.manage("controller.automation.command", event_input(&c, &operation))
        .await
        .unwrap();
    panic!("native event crash boundary was not reached");
}

#[tokio::test]
async fn event_process_death_preserves_original_intents_and_recovers_receipts_without_replaying() {
    for operation in ["disable-event", "delete-event"] {
        for stage in [
            "before_intent",
            "after_intent",
            "before_send",
            "after_reply",
            "after_record",
        ] {
            let f = Fixture::new().await;
            f.core
                .automation_create(
                    AutomationKind::Action,
                    json!({"id":"same","name":"Owner action","enabled":true,"typeId":"core.log","config":{"message":"owner-only"}}),
                )
                .unwrap();
            f.core
                .automation_create(AutomationKind::Event, owner_event())
                .unwrap();
            let home = tempfile::tempdir().unwrap();
            let store = Arc::new(Store::default());
            let manager = ProfileManager::new(home.path().join("profiles"), store.clone()).unwrap();
            let profile = manager
                .install(
                    "events",
                    &f.endpoint,
                    f.pair(event_grant(true)).await,
                    ClientKind::Agent,
                )
                .await
                .unwrap();
            let root = manager.directory().to_path_buf();
            let secret = store.load(&profile.credential_key).unwrap().unwrap();
            let credential_file = root.join("test-credentials.input");
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut output = options.open(&credential_file).unwrap();
            serde_json::to_writer(
                &mut output,
                &json!({"key":profile.credential_key,"secret":secret.expose_secret()}),
            )
            .unwrap();
            output.sync_all().unwrap();
            drop(output);
            let fixture_root = root.clone();
            let status = tokio::task::spawn_blocking(move || {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--ignored",
                        "--exact",
                        "remote::controller::commands::tests::event_process_tests::native_event_process_fixture",
                    ])
                    .env("TIKTOOLS_NATIVE_EVENT_FIXTURE_ROOT", &fixture_root)
                    .env("TIKTOOLS_NATIVE_EVENT_FIXTURE_OPERATION", operation)
                    .env("TIKTOOLS_NATIVE_EVENT_CRASH_STAGE", stage)
                    .current_dir(&fixture_root)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .unwrap()
            })
            .await
            .unwrap();
            assert_eq!(status.code(), Some(97), "{operation}/{stage}");
            let credentials: Value =
                serde_json::from_slice(&std::fs::read(&credential_file).unwrap()).unwrap();
            store
                .save(
                    credentials["key"].as_str().unwrap(),
                    &SecretString::from(credentials["secret"].as_str().unwrap().to_owned()),
                )
                .unwrap();
            let journal = Journal::open(manager.directory(), &profile).unwrap().view();
            let pending = matches!(stage, "after_intent" | "before_send" | "after_reply");
            let committed = matches!(stage, "after_reply" | "after_record");
            assert_eq!(journal.pending.is_some(), pending, "{operation}/{stage}");
            assert_eq!(
                journal.recent.len(),
                usize::from(stage == "after_record"),
                "{operation}/{stage}"
            );
            let original = journal
                .pending
                .as_ref()
                .or_else(|| journal.recent.first().map(|r| &r.intent))
                .cloned();
            if let Some(original) = &original {
                assert_eq!(serde_json::to_value(original.operation).unwrap(), operation);
                assert_eq!(original.expected_revision.as_deref(), Some("1"));
                assert_eq!(original.resource_id, "same");
                assert_eq!(original.host_instance_id, f.service.instance_id());
                assert_eq!(original.operation.scope(), "event");
            }
            let event = f
                .core
                .automation_get_versioned(AutomationKind::Event, "same")
                .unwrap();
            assert_eq!(
                event.resource_versions[0].revision,
                if committed { "2" } else { "1" },
                "{operation}/{stage}"
            );
            if committed {
                match operation {
                    "disable-event" => {
                        assert_eq!(event.value.unwrap()["enabled"], false);
                        f.core
                            .automation_set_enabled(AutomationKind::Event, "same", true)
                            .unwrap();
                    }
                    "delete-event" => {
                        assert!(event.value.is_none());
                        f.core
                            .automation_create(AutomationKind::Event, owner_event())
                            .unwrap();
                    }
                    _ => unreachable!(),
                }
                // Reconnecting can read its receipt but cannot send another
                // mutation under these current grants. The retained result
                // must still describe the original write at revision two.
                let principal = f.service.principal(&profile.principal_id).unwrap();
                f.service
                    .set_grant(GrantSet {
                        principal_id: profile.principal_id.clone(),
                        expected_revision: principal.grant_revision,
                        grant: event_grant(false),
                    })
                    .unwrap();
            } else {
                assert_eq!(event.value.unwrap()["enabled"], true);
            }
            let c = Controller::new(manager);
            c.connect("events").await.unwrap();
            if committed {
                wait(&c, |s| {
                    s.command_journal
                        .as_ref()
                        .is_some_and(|j| j.pending.is_none() && !j.recent.is_empty())
                })
                .await;
                let recovered = c.status().command_journal.unwrap();
                assert_eq!(recovered.recent[0].intent, original.unwrap());
                assert_eq!(recovered.recent[0].outcome, Outcome::Succeeded);
                assert_eq!(recovered.recent[0].revision.as_deref(), Some("2"));
                let current = f
                    .core
                    .automation_get_versioned(AutomationKind::Event, "same")
                    .unwrap();
                assert_eq!(current.value.unwrap()["enabled"], true);
                assert_eq!(current.resource_versions[0].revision, "3");
            } else if pending {
                wait(&c, |s| {
                    !s.command_busy && s.command_error_code.as_deref() == Some("forbidden")
                })
                .await;
                let recovered = c.status().command_journal.unwrap();
                assert_eq!(recovered.pending, journal.pending);
                assert!(recovered.recent.is_empty());
                let current = f
                    .core
                    .automation_get_versioned(AutomationKind::Event, "same")
                    .unwrap();
                assert_eq!(current.value.unwrap()["enabled"], true);
                assert_eq!(current.resource_versions[0].revision, "1");
            } else {
                assert!(c.status().command_journal.unwrap().recent.is_empty());
                assert!(c.status().command_journal.unwrap().pending.is_none());
            }
            let status = c.status();
            let page = c
                .automations(&status.epoch, &status.automation_revision, 0)
                .unwrap();
            let rows = page["rows"].as_array().unwrap();
            assert_eq!(rows.len(), 2);
            let visible_event = rows
                .iter()
                .find(|r| r["summary"]["kind"] == "event")
                .unwrap();
            assert_eq!(visible_event["summary"]["enabled"], true);
            assert_eq!(
                visible_event["version"]["revision"],
                if committed { "3" } else { "1" }
            );
            assert!(!page.to_string().contains("owner-only"));
            let action = f
                .core
                .automation_get_versioned(AutomationKind::Action, "same")
                .unwrap();
            assert_eq!(action.value.unwrap()["enabled"], true);
            assert_eq!(action.resource_versions[0].revision, "1");
            assert!(f.core.automation_runs().is_empty());
            for name in ["data", "plugin-data", "plugins"] {
                assert!(!root.join(name).exists());
            }
            let path = root
                .join("commands")
                .join(format!("{}.journal", profile.profile_id));
            if let Ok(bytes) = std::fs::read(path) {
                let text = String::from_utf8(bytes).unwrap();
                assert!(!text.contains("owner-only"));
                assert!(!text.contains("accessToken"));
                assert!(!text.contains(secret.expose_secret()));
            }
            c.disconnect().await;
            f.finish().await;
        }
    }
}
