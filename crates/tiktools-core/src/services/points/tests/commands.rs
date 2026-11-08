use super::*;
use crate::{
    collaboration::AdditiveCommandRequest,
    control::PointsAdjustment,
    db::{CommandTransaction, DatabaseError},
};
use std::sync::mpsc;

fn command(id: &str) -> CommandTransaction<AdditiveCommandRequest> {
    CommandTransaction {
        principal_id: "local-owner".into(),
        session_id: "local-owner".into(),
        authorization: None,
        request: AdditiveCommandRequest {
            command_id: id.into(),
            host_instance_id: "instance".into(),
        },
        current_instance_id: "instance".into(),
        max_ids_per_principal: 100,
    }
}

#[test]
fn blocked_snapshot_rejects_command_before_acceptance_and_later_awards_preserve_the_committed_adjustment(
) {
    let (database, service, root) = temp_service("command-ordered");
    let service = Arc::new(service);
    let (reached, observed) = mpsc::sync_channel(0);
    let (resume, proceed) = mpsc::sync_channel(0);
    *service.before_submit.lock().unwrap() = Some(AdmissionPause {
        reached,
        resume: proceed,
    });
    let producer = service.clone();
    let thread = std::thread::spawn(move || {
        producer
            .award_points("alice", PointAction::Chat, AwardOptions::default())
            .unwrap()
    });
    observed.recv_timeout(Duration::from_secs(10)).unwrap();
    let operation = PointsAdjustment {
        expected_currency: None,
        unique_id: "alice".into(),
        delta: 2.0,
    };
    assert!(matches!(
        service.execute_command(&operation, &command("once")),
        Err(DatabaseError::PointsWriterUnavailable)
    ));
    assert!(database
        .command_resource("local-owner", "once")
        .unwrap()
        .is_none());
    resume.send(()).unwrap();
    thread.join().unwrap();
    let (result, fresh) = service
        .execute_command(&operation, &command("once"))
        .unwrap();
    assert!(fresh);
    assert_eq!(result.value["totalPoints"], 3.0);
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    service.shutdown();
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        4.0
    );
    assert_eq!(service.viewer("alice").unwrap()["points"], 4.0);
    assert_eq!(
        database
            .command_outcome("local-owner", "once")
            .unwrap()
            .unwrap()
            .result["totalPoints"],
        3.0
    );
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn uncommitted_or_permanently_failed_legacy_writes_never_become_a_command_acknowledgement() {
    let (database, service, root) = temp_service("command-drain");
    let (reached, observed) = mpsc::sync_channel(0);
    let (resume, proceed) = mpsc::sync_channel(0);
    *service.writer.before_apply.lock().unwrap() = Some(AdmissionPause {
        reached,
        resume: proceed,
    });
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    observed.recv_timeout(Duration::from_secs(10)).unwrap();
    let operation = PointsAdjustment {
        expected_currency: None,
        unique_id: "alice".into(),
        delta: 2.0,
    };
    assert!(matches!(
        service.execute_command(&operation, &command("pending")),
        Err(DatabaseError::PointsWriterUnavailable)
    ));
    assert!(database
        .command_resource("local-owner", "pending")
        .unwrap()
        .is_none());
    resume.send(()).unwrap();
    wait_for_writer_idle(&service);
    let db = database.open_points_writer().unwrap();
    db.execute_batch("CREATE TRIGGER reject_award BEFORE INSERT ON control_resource_outbox BEGIN SELECT RAISE(ABORT,'fixture'); END").unwrap();
    service
        .award_points("alice", PointAction::Chat, AwardOptions::default())
        .unwrap();
    wait_for_writer_idle(&service);
    db.execute_batch("DROP TRIGGER reject_award").unwrap();
    assert!(matches!(
        service.execute_command(&operation, &command("failed")),
        Err(DatabaseError::PointsWriterUnavailable)
    ));
    assert!(database
        .command_resource("local-owner", "failed")
        .unwrap()
        .is_none());
    assert_eq!(
        database.load_viewer_by_id("alice").unwrap().unwrap()["points"],
        1.0
    );
    service.shutdown();
    drop(db);
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn authorization_is_rechecked_after_the_sqlite_writer_has_actually_admitted_the_command() {
    use crate::collaboration::*;
    let (database, service, root) = temp_service("command-auth-admission");
    let service = Arc::new(service);
    let authority = CollaborationService::open(&root.join("collaboration")).unwrap();
    let mut config = authority.configuration().unwrap();
    config.enabled = true;
    config.advertised_origin = Some("http://127.0.0.1:8788".into());
    authority.configure(config).unwrap();
    let mut grant = Grant {
        capabilities: [Capability::PointsRead, Capability::PointsAdjust].into(),
        resources: [(ResourceKind::Viewer, ["alice".into()].into())].into(),
        ..Grant::default()
    };
    let invite = authority
        .create_invitation(InviteCreate {
            grant: grant.clone(),
        })
        .unwrap();
    let secret = "a".repeat(64);
    let pending = authority
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Queued agent".into(),
                client_kind: ClientKind::Agent,
                exchange_secret_hash: hash_secret(&secret),
            },
            "fixture",
        )
        .unwrap();
    authority.decide_request(&pending.request_id, true).unwrap();
    let credentials = authority
        .collect(
            Collect {
                request_id: pending.request_id,
                exchange_secret: secret,
            },
            "fixture",
        )
        .unwrap();
    let auth = authority
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap();
    let identity = auth.snapshot().unwrap();
    let command = CommandTransaction {
        principal_id: identity.principal_id,
        session_id: identity.session_id,
        authorization: Some(auth),
        request: AdditiveCommandRequest {
            command_id: "admitted".into(),
            host_instance_id: authority.instance_id().into(),
        },
        current_instance_id: authority.instance_id().into(),
        max_ids_per_principal: 100,
    };
    let (reached, observed) = mpsc::sync_channel(0);
    let (resume, proceed) = mpsc::sync_channel(0);
    *service.after_command_admission.lock().unwrap() = Some(AdmissionPause {
        reached,
        resume: proceed,
    });
    let worker = service.clone();
    let thread = std::thread::spawn(move || {
        worker.execute_command(
            &PointsAdjustment {
                expected_currency: None,
                unique_id: "alice".into(),
                delta: 2.0,
            },
            &command,
        )
    });
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    grant.capabilities.remove(&Capability::PointsAdjust);
    authority
        .set_grant(GrantSet {
            principal_id: credentials.principal_id.clone(),
            expected_revision: "1".into(),
            grant,
        })
        .unwrap();
    resume.send(()).unwrap();
    assert!(matches!(
        thread.join().unwrap(),
        Err(DatabaseError::CommandAuthorization(Error::Forbidden))
    ));
    assert!(database
        .command_resource(&credentials.principal_id, "admitted")
        .unwrap()
        .is_none());
    assert!(database.load_viewer_by_id("alice").unwrap().is_none());
    assert!(service.viewer("alice").is_none());
    service.shutdown();
    drop(authority);
    drop(service);
    drop(database);
    let _ = std::fs::remove_dir_all(root);
}
