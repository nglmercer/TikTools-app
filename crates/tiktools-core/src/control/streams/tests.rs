use super::*;
use crate::{ipc::messages::HostMessage, paths::AppPaths, HostEmitter, NoopMediaHost};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
struct Fixture {
    core: Option<Arc<AppCore>>,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tt-stream-core-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = AppPaths {
            data: root.join("data"),
            plugins: root.join("plugins"),
            plugin_data: root.join("plugin-data"),
            builtin_plugins: root.join("builtin"),
            development_plugins: None,
            logs: root.join("logs"),
            temp: root.join("temp"),
            root: root.clone(),
        };
        paths.ensure_directories().unwrap();
        for path in [
            paths.points_database(),
            paths.automation_database(),
            paths.analytics_database(),
        ] {
            std::fs::File::create(path).unwrap();
        }
        Self {
            core: Some(Arc::new(AppCore::with_paths(
                Arc::new(Emitter),
                Arc::new(NoopMediaHost),
                paths,
            ))),
            root,
        }
    }
    fn core(&self) -> &Arc<AppCore> {
        self.core.as_ref().unwrap()
    }
    fn auth(&self, grant: Grant) -> Authorization {
        let s = self.core().collaboration().unwrap();
        let mut config = s.configuration().unwrap();
        config.enabled = true;
        config.advertised_origin = Some("http://127.0.0.1:8788".into());
        s.configure(config).unwrap();
        let invitation = s.create_invitation(InviteCreate { grant }).unwrap();
        let exchange = "a".repeat(64);
        let request = s
            .redeem(
                Redeem {
                    invite_id: invitation.invite_id,
                    secret: invitation.secret,
                    client_label: "Controller".into(),
                    client_kind: ClientKind::Desktop,
                    exchange_secret_hash: hash_secret(&exchange),
                },
                "source",
            )
            .unwrap();
        s.decide_request(&request.request_id, true).unwrap();
        let credentials = s
            .collect(
                Collect {
                    request_id: request.request_id,
                    exchange_secret: exchange,
                },
                "source",
            )
            .unwrap();
        s.authenticate_live(&credentials.access_token, false, false)
            .unwrap()
    }
    fn subscribe(&self, auth: &Authorization, limits: &RemoteLimits) -> SubscribeResult {
        self.core()
            .collaboration_subscribe(
                auth.clone(),
                "conn",
                SubscribeParams {
                    domains: [StreamDomain::Workflows].into(),
                    feed: false,
                    points_limit: None,
                },
                limits,
            )
            .unwrap()
    }
    fn snapshot(
        &self,
        auth: &Authorization,
        sub: &SubscribeResult,
        token: Option<String>,
        frame: usize,
    ) -> SnapshotPage {
        self.core()
            .collaboration_snapshot(
                auth.clone(),
                "conn",
                SnapshotParams {
                    subscription_id: sub.subscription_id.clone(),
                    page_token: token,
                },
                frame,
            )
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.core.take());
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn grant() -> Grant {
    Grant {
        capabilities: [Capability::WorkflowsRead].into(),
        ..Grant::default()
    }
}

#[test]
fn points_pages_remain_immutable_while_reset_and_resource_delivery_advance() {
    let f = Fixture::new();
    let auth = f.auth(Grant {
        capabilities: [Capability::PointsRead].into(),
        ..Grant::default()
    });
    // Commit through the real legacy writer, then hold a paginated baseline.
    for n in 0..20 {
        f.core()
            .points_adjust(&format!("viewer-{n:02}"), 5.0)
            .unwrap();
    }
    f.core().points.shutdown();
    let sub = f
        .core()
        .collaboration_subscribe(
            auth.clone(),
            "conn",
            SubscribeParams {
                domains: [StreamDomain::Points].into(),
                feed: false,
                points_limit: None,
            },
            &RemoteLimits::default(),
        )
        .unwrap();
    let first = f.snapshot(&auth, &sub, None, 4096);
    assert!(first.next_page_token.is_some());
    // Simulate the committed reset writer path while the retained pages exist.
    let mut db = f.core().db.open_points_writer().unwrap();
    f.core()
        .db
        .apply_points_batch(
            &mut db,
            &[crate::services::PointsWriteOp::Reset {
                seq: 21,
                unique_id: Some("viewer-19".into()),
            }],
        )
        .unwrap();
    f.core().deliver_coordination().unwrap();
    assert!(f
        .core()
        .collaboration_stream_next(&auth, "conn")
        .unwrap()
        .is_none());
    let mut page = first.clone();
    let mut count = 0;
    loop {
        assert!(serde_json::to_vec(&page).unwrap().len() < 4096 - 256);
        assert!(page.workflows.is_empty());
        assert!(page.points.iter().all(|v| v["points"] == 5.0));
        assert!(page
            .resource_versions
            .iter()
            .all(|v| v.kind == ResourceKind::Viewer && v.revision == "1"));
        assert_eq!(page.snapshot_id, first.snapshot_id);
        count += page.points.len();
        let Some(next) = page.next_page_token else {
            break;
        };
        page = f.snapshot(&auth, &sub, Some(next), 4096);
    }
    assert_eq!(count, 20);
    let mut saw_reset = false;
    while let Some(event) = f.core().collaboration_stream_next(&auth, "conn").unwrap() {
        assert_eq!(event.topic, "points.changed");
        if event.resource.id == "viewer-19" && event.resource.revision == "2" {
            saw_reset = true;
        }
        f.core()
            .collaboration_stream_sent(&auth, "conn", &event)
            .unwrap();
    }
    assert!(saw_reset);
}
#[test]
fn captures_racing_with_legacy_writes_and_late_outbox_publication_have_coherent_watermarks() {
    let f = Fixture::new();
    let auth = f.auth(grant());
    f.core()
        .workflow_save(json!({"id":"welcome","name":"Counter","counter":1}))
        .unwrap();
    f.core().deliver_coordination().unwrap();
    let sub = f.subscribe(&auth, &RemoteLimits::default());
    let writer = Arc::clone(f.core());
    let thread = std::thread::spawn(move || {
        for n in 2..=101 {
            writer
                .workflow_save_versioned(
                    json!({"id":"welcome","name":"Counter","counter":n}),
                    Some(&(n - 1).to_string()),
                )
                .unwrap();
            std::thread::yield_now();
            writer.deliver_coordination().unwrap();
        }
    });
    for _ in 0..80 {
        let page = f.snapshot(&auth, &sub, None, 1024 * 1024);
        let revision: u64 = page.resource_versions[0].revision.parse().unwrap();
        let sequence: u64 = page.through_sequence.parse().unwrap();
        assert_eq!(page.workflows[0]["graph"]["counter"], revision);
        // The original version predates this subscription. At most one next
        // write can be committed but not yet published at the capture barrier.
        assert!(
            revision > sequence && revision <= sequence + 2,
            "revision {revision} / sequence {sequence}"
        );
        if let Some(event) = f.core().collaboration_stream_next(&auth, "conn").unwrap() {
            assert!(event.sequence.parse::<u64>().unwrap() > sequence);
            f.core()
                .collaboration_stream_sent(&auth, "conn", &event)
                .unwrap();
        }
        std::thread::yield_now();
    }
    thread.join().unwrap();
    let page = f.snapshot(&auth, &sub, None, 1024 * 1024);
    assert_eq!(page.resource_versions[0].revision, "101");
    assert_eq!(page.through_sequence, "100");
}
#[test]
fn snapshots_filter_before_reading_hidden_bodies_and_pages_are_immutable_during_writes() {
    let f = Fixture::new();
    let mut permissions = grant();
    permissions.resources.insert(
        ResourceKind::Workflow,
        (0..12).map(|n| format!("allowed-{n:02}")).collect(),
    );
    let auth = f.auth(permissions);
    for n in 0..12 {
        f.core()
            .workflow_save(
                json!({"id":format!("allowed-{n:02}"),"name":"Original","body":"x".repeat(800)}),
            )
            .unwrap();
    }
    f.core()
        .workflow_save(json!({"id":"hidden","name":"Secret","body":"x".repeat(1024*1024)}))
        .unwrap();
    let db = rusqlite::Connection::open(f.core().db.automation_path()).unwrap();
    db.execute(
        "UPDATE automation_workflows SET graph_json='invalid JSON, private body' WHERE id='hidden'",
        [],
    )
    .unwrap();
    drop(db);
    let limits = RemoteLimits {
        frame_bytes: 4096,
        params_bytes: 1024,
        ..RemoteLimits::default()
    };
    let sub = f.subscribe(&auth, &limits);
    let first = f.snapshot(&auth, &sub, None, limits.frame_bytes);
    assert!(first.next_page_token.is_some());
    f.core().workflow_set_enabled("allowed-11", true).unwrap();
    f.core().deliver_coordination().unwrap();
    assert!(f
        .core()
        .collaboration_stream_next(&auth, "conn")
        .unwrap()
        .is_none());
    let mut page = first.clone();
    let mut total = 0;
    loop {
        assert!(serde_json::to_vec(&page).unwrap().len() < limits.frame_bytes - 256);
        total += page.workflows.len();
        assert!(page
            .workflows
            .iter()
            .all(|row| row["name"] == "Original" && row["enabled"] == false));
        assert_eq!(page.snapshot_id, first.snapshot_id);
        assert_eq!(page.through_sequence, first.through_sequence);
        assert!(page.resource_versions.iter().all(|v| v.revision == "1"));
        let Some(next) = page.next_page_token else {
            break;
        };
        page = f.snapshot(&auth, &sub, Some(next), limits.frame_bytes);
    }
    assert_eq!(total, 12);
    let mut changed = false;
    while let Some(event) = f.core().collaboration_stream_next(&auth, "conn").unwrap() {
        assert_ne!(event.resource.id, "hidden");
        if event.resource.id == "allowed-11" && event.resource.revision == "2" {
            changed = true;
        }
        f.core()
            .collaboration_stream_sent(&auth, "conn", &event)
            .unwrap();
    }
    assert!(changed);
    let scoped = f
        .core()
        .workflow_list_scoped(&auth.snapshot().unwrap().grant, 128 * 1024)
        .unwrap();
    assert_eq!(scoped.value.len(), 12);
}
#[test]
fn stale_pages_and_queued_events_fail_after_grant_changes_or_snapshot_replacement() {
    let f = Fixture::new();
    let auth = f.auth(grant());
    let sub = f.subscribe(&auth, &RemoteLimits::default());
    let first = f.snapshot(&auth, &sub, None, 1024 * 1024);
    f.core()
        .workflow_save(json!({"id":"welcome","name":"One"}))
        .unwrap();
    f.core().deliver_coordination().unwrap();
    let event = f
        .core()
        .collaboration_stream_next(&auth, "conn")
        .unwrap()
        .unwrap();
    let current = f.snapshot(&auth, &sub, None, 1024 * 1024);
    assert_ne!(first.snapshot_id, current.snapshot_id);
    assert_eq!(
        f.core()
            .collaboration_stream_validate(
                &auth,
                "conn",
                &sub.subscription_id,
                &sub.grant_revision,
                Some(&first.snapshot_id)
            )
            .unwrap_err()
            .code,
        "resync_required"
    );
    let identity = auth.snapshot().unwrap();
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: Grant {
                capabilities: [Capability::WorkflowsRead].into(),
                resources: [(ResourceKind::Workflow, ["other".into()].into())].into(),
                ..Grant::default()
            },
        })
        .unwrap();
    assert_eq!(
        f.core()
            .collaboration_stream_event_authorize(&auth, "conn", &event)
            .unwrap_err()
            .code,
        "resync_required"
    );
    assert_eq!(
        f.core()
            .collaboration_stream_validate(
                &auth,
                "conn",
                &sub.subscription_id,
                &sub.grant_revision,
                Some(&current.snapshot_id)
            )
            .unwrap_err()
            .code,
        "resync_required"
    );
}
#[test]
fn oversized_allowed_records_fail_before_returning_a_partial_or_unbounded_baseline() {
    let f = Fixture::new();
    let auth = f.auth(grant());
    let sub = f.subscribe(&auth, &RemoteLimits::default());
    f.snapshot(&auth, &sub, None, 1024 * 1024);
    f.core()
        .workflow_save(json!({"id":"large","name":"Large","body":"x".repeat(5000)}))
        .unwrap();
    f.core().deliver_coordination().unwrap();
    assert_eq!(
        f.core()
            .collaboration_snapshot(
                auth.clone(),
                "conn",
                SnapshotParams {
                    subscription_id: sub.subscription_id.clone(),
                    page_token: None
                },
                4096
            )
            .unwrap_err()
            .code,
        "too_large"
    );
    // Failed recovery cannot leave the previous baseline marked current or
    // release pending events as though a new complete capture had succeeded.
    assert!(f
        .core()
        .collaboration_stream_next(&auth, "conn")
        .unwrap()
        .is_none());
    let recovered = f.snapshot(&auth, &sub, None, 1024 * 1024);
    assert_eq!(recovered.resource_versions[0].revision, "1");
    assert_eq!(
        f.core()
            .workflow_get_scoped(&grant(), "large", 2048)
            .unwrap_err()
            .code,
        "too_large"
    );
    assert_eq!(
        f.core()
            .workflow_list_scoped(&grant(), 2048)
            .unwrap_err()
            .code,
        "too_large"
    );
}

#[test]
fn automation_pages_keep_same_family_ids_private_and_immutable_across_owner_changes() {
    use crate::control::AutomationKind;
    let f = Fixture::new();
    for kind in [AutomationKind::Event, AutomationKind::Action] {
        f.core().automation_create(kind,json!({"id":"same","name":"Status","enabled":true,"config":{"secret":"owner-only"},"actionIds":["private"]})).unwrap();
    }
    let mut ids = std::collections::BTreeSet::from(["event:same".into(), "action:same".into()]);
    for n in 0..24 {
        let id = format!("extra-{n:02}");
        ids.insert(format!("event:{id}"));
        f.core()
            .automation_create(
                AutomationKind::Event,
                json!({"id":id,"name":"label".repeat(32),"enabled":true}),
            )
            .unwrap();
    }
    f.core()
        .automation_create(
            AutomationKind::Event,
            json!({"id":"hidden","name":"Hidden secret","enabled":true}),
        )
        .unwrap();
    f.core().deliver_coordination().unwrap();
    let auth = f.auth(Grant {
        capabilities: [Capability::AutomationRead, Capability::CommandsRead].into(),
        resources: [(ResourceKind::Automation, ids)].into(),
        ..Grant::default()
    });
    let sub = f
        .core()
        .collaboration_subscribe(
            auth.clone(),
            "conn",
            SubscribeParams {
                domains: [StreamDomain::Automation, StreamDomain::Commands].into(),
                feed: false,
                points_limit: None,
            },
            &RemoteLimits::default(),
        )
        .unwrap();
    let first = f.snapshot(&auth, &sub, None, 4096);
    assert!(first.next_page_token.is_some());
    f.core()
        .automation_set_enabled(AutomationKind::Event, "same", false)
        .unwrap();
    let mut pages = vec![first];
    while let Some(token) = pages.last().unwrap().next_page_token.clone() {
        pages.push(f.snapshot(&auth, &sub, Some(token), 4096));
    }
    assert!(pages.len() > 1);
    let records: Vec<_> = pages.iter().flat_map(|p| p.automation.iter()).collect();
    assert_eq!(records.len(), 26);
    assert_eq!(records.iter().filter(|r| r["id"] == "same").count(), 2);
    assert!(records
        .iter()
        .all(|r| r["enabled"] == true && r["definitionRedacted"] == true));
    assert!(!serde_json::to_string(&pages)
        .unwrap()
        .contains("owner-only"));
    assert!(!serde_json::to_string(&pages)
        .unwrap()
        .contains("Hidden secret"));
    assert!(pages
        .iter()
        .flat_map(|p| p.resource_versions.iter())
        .all(|v| v.revision == "1"));
    f.core().deliver_coordination().unwrap();
    let event = f
        .core()
        .collaboration_stream_next(&auth, "conn")
        .unwrap()
        .unwrap();
    assert_eq!(event.topic, "automation.changed");
    assert_eq!(
        event.data,
        json!({"id":"same","kind":"event","change":"disabled"})
    );
    assert_eq!(event.resource.revision, "2");
    let identity = auth.snapshot().unwrap();
    f.core()
        .collaboration()
        .unwrap()
        .set_grant(GrantSet {
            principal_id: identity.principal_id,
            expected_revision: identity.grant_revision,
            grant: Grant {
                capabilities: [Capability::AutomationRead].into(),
                resources: [(ResourceKind::Automation, ["action:same".into()].into())].into(),
                ..Grant::default()
            },
        })
        .unwrap();
    assert_eq!(
        f.core()
            .collaboration_stream_event_authorize(&auth, "conn", &event)
            .unwrap_err()
            .code,
        "resync_required"
    );
    let fresh = f
        .core()
        .collaboration_subscribe(
            auth.clone(),
            "new",
            SubscribeParams {
                domains: [StreamDomain::Automation].into(),
                feed: false,
                points_limit: None,
            },
            &RemoteLimits::default(),
        )
        .unwrap();
    let baseline = f
        .core()
        .collaboration_snapshot(
            auth,
            "new",
            SnapshotParams {
                subscription_id: fresh.subscription_id,
                page_token: None,
            },
            8192,
        )
        .unwrap();
    assert_eq!(baseline.automation.len(), 1);
    assert_eq!(baseline.automation[0]["kind"], "action");
}

#[test]
fn automation_only_commands_and_empty_scopes_obey_global_creator_and_read_permissions() {
    let f = Fixture::new();
    for (index, (capabilities, resources, allowed)) in [
        (
            vec![Capability::AutomationRead, Capability::CommandsRead],
            vec![],
            true,
        ),
        (
            vec![Capability::AutomationRead, Capability::CommandsRead],
            vec![(ResourceKind::Automation, Default::default())],
            true,
        ),
        (
            vec![Capability::AutomationRead, Capability::CommandsRead],
            vec![(ResourceKind::Live, ["creator".into()].into())],
            false,
        ),
        (vec![Capability::CommandsRead], vec![], false),
    ]
    .into_iter()
    .enumerate()
    {
        let auth = f.auth(Grant {
            capabilities: capabilities.into_iter().collect(),
            resources: resources.into_iter().collect(),
            ..Grant::default()
        });
        let result = f.core().collaboration_subscribe(
            auth,
            &format!("scope-test-{index}"),
            SubscribeParams {
                domains: [StreamDomain::Commands].into(),
                feed: false,
                points_limit: None,
            },
            &RemoteLimits::default(),
        );
        assert_eq!(result.is_ok(), allowed, "{result:?}");
    }
}
