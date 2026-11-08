use super::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

fn service() -> (tempfile::TempDir, Arc<CollaborationService>) {
    let directory = tempfile::tempdir().unwrap();
    let s = Arc::new(CollaborationService::open(&directory.path().join("private")).unwrap());
    s.configure(Configuration {
        enabled: true,
        advertised_origin: Some("http://127.0.0.1:8788".into()),
        ..Configuration::default()
    })
    .unwrap();
    (directory, s)
}
fn pair(s: &CollaborationService) -> Credentials {
    let invite = s
        .create_invitation(InviteCreate {
            grant: Grant::viewer(),
        })
        .unwrap();
    let exchange = secret().unwrap();
    let pending = s
        .redeem(
            Redeem {
                invite_id: invite.invite_id,
                secret: invite.secret,
                client_label: "Execution subject fixture".into(),
                client_kind: ClientKind::Desktop,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "execution-test",
        )
        .unwrap();
    s.decide_request(&pending.request_id, true).unwrap();
    s.collect(
        Collect {
            request_id: pending.request_id,
            exchange_secret: exchange,
        },
        "execution-test",
    )
    .unwrap()
}
fn subject(c: &Credentials) -> (String, String) {
    (c.principal_id.clone(), c.session_id.clone())
}

#[test]
fn context_intersections_keep_every_subject_and_reject_distinct_admission_locks_even_for_equal_host_labels(
) {
    let (_dir, s) = service();
    let a = pair(&s);
    let b = pair(&s);
    let first = s.execution_authorizations(&[subject(&a)]).unwrap();
    let second = s
        .execution_authorizations(&[subject(&b), subject(&a)])
        .unwrap();
    let joined = first.intersect(&second).unwrap();
    assert_eq!(joined.len(), 2);
    assert_eq!(joined.snapshot().unwrap().len(), 2);
    let empty = s.execution_authorizations(&[]).unwrap();
    assert_eq!(joined.intersect(&empty).unwrap().len(), 2);
    let (_other_dir, mut other) = service();
    // A copied or mislabeled service still owns another mutex. Equal host
    // labels must never claim that two admissions were atomic together.
    Arc::get_mut(&mut other).unwrap().host_id = s.host_id().into();
    let other_credentials = pair(&other);
    let foreign = other
        .execution_authorizations(&[subject(&other_credentials)])
        .unwrap();
    assert_eq!(s.host_id(), other.host_id());
    assert!(matches!(joined.intersect(&foreign), Err(Error::Forbidden)));
    s.revoke_session(&a.session_id, "local-owner").unwrap();
    assert!(matches!(first.intersect(&second), Err(Error::Unauthorized)));
    assert!(joined.snapshot().is_err());
}
fn activity(s: &CollaborationService, c: &Credentials) -> i64 {
    s.db()
        .unwrap()
        .query_row(
            "SELECT last_activity FROM sessions WHERE id=?1",
            [&c.session_id],
            |r| r.get(0),
        )
        .unwrap()
}

#[test]
fn saved_identity_is_independent_of_bearer_expiry_rotation_and_socket_lifetime_without_touching_activity(
) {
    let (_dir, s) = service();
    let c = pair(&s);
    let live = s.authenticate_live(&c.access_token, false, false).unwrap();
    s.db()
        .unwrap()
        .execute(
            "UPDATE sessions SET last_activity=?2,access_expires=0 WHERE id=?1",
            params![c.session_id, now() - 10_000],
        )
        .unwrap();
    assert!(s.authenticate(&c.access_token, false, false).is_err());
    let before = activity(&s, &c);
    let saved = s
        .execution_authorization(&c.principal_id, &c.session_id)
        .unwrap();
    let identity = saved.snapshot().unwrap();
    assert_eq!(identity.principal_id(), c.principal_id);
    assert_eq!(identity.session_id(), c.session_id);
    assert_eq!(identity.host_id(), s.host_id());
    assert_eq!(identity.grant_revision(), "1");
    assert_eq!(identity.configuration_revision(), "2");
    assert_eq!(identity.idle_expires_at(), before + IDLE_TTL);
    assert_eq!(activity(&s, &c), before);
    let refreshed = s.refresh(c.refresh_token.as_ref().unwrap()).unwrap();
    assert!(live.snapshot().is_err());
    drop(live);
    assert_eq!(refreshed.session_id, c.session_id);
    let before = activity(&s, &c);
    assert!(saved
        .snapshot()
        .unwrap()
        .grant()
        .capabilities
        .contains(&Capability::LiveRead));
    assert_eq!(activity(&s, &c), before);
    let socket = s
        .authenticate_live(&refreshed.access_token, false, false)
        .unwrap();
    drop(socket);
    assert!(saved.snapshot().is_ok());
}

#[test]
fn trusted_subjects_reconstruct_after_restart_and_observe_current_grants() {
    let (dir, s) = service();
    let c = pair(&s);
    let host = s.host_id().to_string();
    let instance = s.instance_id().to_string();
    let handle = s
        .execution_authorization(&c.principal_id, &c.session_id)
        .unwrap();
    assert!(handle.snapshot().is_ok());
    drop(handle);
    drop(s);
    let s = Arc::new(CollaborationService::open(&dir.path().join("private")).unwrap());
    assert_eq!(host, s.host_id());
    assert_ne!(instance, s.instance_id());
    let handle = s
        .execution_authorization(&c.principal_id, &c.session_id)
        .unwrap();
    let old = handle.snapshot().unwrap();
    s.set_grant(GrantSet {
        principal_id: c.principal_id.clone(),
        expected_revision: "1".into(),
        grant: Grant::default(),
    })
    .unwrap();
    let current = handle.snapshot().unwrap();
    assert_eq!(old.grant_revision(), "1");
    assert_eq!(current.grant_revision(), "2");
    assert!(current.grant().capabilities.is_empty());
    s.revoke_principal(&c.principal_id).unwrap();
    assert!(matches!(handle.snapshot(), Err(Error::Unauthorized)));
}

#[test]
fn exact_principal_session_and_host_binding_prevent_cross_identity_substitution() {
    let (_dir, s) = service();
    let a = pair(&s);
    let b = pair(&s);
    for (principal, session) in [
        (a.principal_id.clone(), b.session_id.clone()),
        (b.principal_id.clone(), a.session_id.clone()),
        (a.principal_id.clone(), Uuid::new_v4().to_string()),
        ("caller JSON".into(), a.session_id.clone()),
    ] {
        assert!(matches!(
            s.execution_authorization(&principal, &session),
            Err(Error::Unauthorized)
        ));
    }
    let (_other_dir, other) = service();
    assert!(matches!(
        other.execution_authorization(&a.principal_id, &a.session_id),
        Err(Error::Unauthorized)
    ));
    let all = s
        .execution_authorizations(&[subject(&b), subject(&a), subject(&b)])
        .unwrap();
    assert_eq!(all.len(), 2);
    assert!(!all.is_empty());
    let subjects = all.snapshot().unwrap();
    assert_eq!(subjects[0].principal_id(), b.principal_id);
    assert_eq!(subjects[1].principal_id(), a.principal_id);
    s.revoke_session(&a.session_id, "local-owner").unwrap();
    let ran = AtomicBool::new(false);
    assert!(all
        .with_current(|_| ran.store(true, Ordering::SeqCst))
        .is_err());
    assert!(!ran.load(Ordering::SeqCst));
    assert!(s
        .execution_authorization(&b.principal_id, &b.session_id)
        .is_ok());
}

#[test]
fn execution_does_not_keep_idle_sessions_alive_and_never_outlives_absolute_expiry_or_disable() {
    for expires in ["idle", "absolute", "disable"] {
        let (_dir, s) = service();
        let c = pair(&s);
        let handle = s
            .execution_authorization(&c.principal_id, &c.session_id)
            .unwrap();
        match expires {
            "idle" => {
                s.db()
                    .unwrap()
                    .execute(
                        "UPDATE sessions SET last_activity=?2 WHERE id=?1",
                        params![c.session_id, now() - IDLE_TTL],
                    )
                    .unwrap();
            }
            "absolute" => {
                s.db()
                    .unwrap()
                    .execute(
                        "UPDATE sessions SET absolute_expires=0 WHERE id=?1",
                        [&c.session_id],
                    )
                    .unwrap();
            }
            "disable" => {
                s.configure(Configuration::default()).unwrap();
            }
            _ => unreachable!(),
        }
        let ran = AtomicBool::new(false);
        assert!(handle
            .with_current(|_| ran.store(true, Ordering::SeqCst))
            .is_err());
        assert!(!ran.load(Ordering::SeqCst));
        if expires == "disable" {
            s.configure(Configuration {
                enabled: true,
                advertised_origin: Some("http://127.0.0.1:8788".into()),
                ..Configuration::default()
            })
            .unwrap();
            assert!(matches!(handle.snapshot(), Err(Error::Unauthorized)));
        }
    }
    let (_dir, s) = service();
    let c = pair(&s);
    let identity = s
        .execution_authorization(&c.principal_id, &c.session_id)
        .unwrap()
        .snapshot()
        .unwrap();
    let expired = ExecutionIdentity {
        absolute_expires_at: 0,
        ..identity
    };
    assert!(expired.check_deadline().is_err());
}

#[test]
fn batch_bounds_and_empty_sets_never_invent_a_remote_grant() {
    let (_dir, s) = service();
    let c = pair(&s);
    assert!(matches!(
        s.execution_authorizations(&vec![subject(&c); SUBJECTS + 1]),
        Err(Error::TooLarge)
    ));
    s.configure(Configuration::default()).unwrap();
    let empty = s.execution_authorizations(&[]).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert!(empty.snapshot().unwrap().is_empty());
    assert_eq!(empty.with_current(|values| values.len()).unwrap(), 0);
}

#[test]
fn malformed_oversized_or_inconsistent_stored_grants_cannot_authorize_a_saved_subject() {
    for change in ["json", "unknown", "size", "revision", "kind"] {
        let (_dir, s) = service();
        let c = pair(&s);
        let handle = s
            .execution_authorization(&c.principal_id, &c.session_id)
            .unwrap();
        let db = s.db().unwrap();
        match change {
            "json" => {
                db.execute(
                    "UPDATE principals SET grant_json=?2 WHERE id=?1",
                    params![c.principal_id, "invalid-fixture-sensitive-json"],
                )
                .unwrap();
            }
            "unknown" => {
                db.execute(
                    "UPDATE principals SET grant_json=?2 WHERE id=?1",
                    params![
                        c.principal_id,
                        "{\"capabilities\":[\"unknown-fixture-sensitive-capability\"]}"
                    ],
                )
                .unwrap();
            }
            "size" => {
                db.execute(
                    "UPDATE principals SET grant_json=?2 WHERE id=?1",
                    params![c.principal_id, "s".repeat(GRANT_BYTES + 1)],
                )
                .unwrap();
            }
            "revision" => {
                db.execute(
                    "UPDATE principals SET revision=0 WHERE id=?1",
                    [&c.principal_id],
                )
                .unwrap();
            }
            "kind" => {
                db.execute(
                    "UPDATE sessions SET kind='agent' WHERE id=?1",
                    [&c.session_id],
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        drop(db);
        let ran = AtomicBool::new(false);
        let error = handle
            .with_current(|_| ran.store(true, Ordering::SeqCst))
            .err()
            .unwrap();
        assert!(!ran.load(Ordering::SeqCst));
        assert!(!error.to_string().contains("fixture-sensitive"), "{change}");
    }
}

#[test]
fn combined_grant_budget_rejects_many_individually_valid_subjects_before_running_the_closure() {
    let (_dir, s) = service();
    let c = pair(&s);
    let mut grant = Grant::viewer();
    for field in 0..7 {
        grant.settings_fields.insert(
            format!("plugin-{field}"),
            (0..256)
                .map(|n| format!("{n:03}-{}", "x".repeat(216)))
                .collect(),
        );
    }
    grant.validate().unwrap();
    let encoded = serde_json::to_string(&grant).unwrap();
    assert!(encoded.len() < GRANT_BYTES);
    assert!(encoded.len() * 11 > BATCH_BYTES);
    s.set_grant(GrantSet {
        principal_id: c.principal_id.clone(),
        expected_revision: "1".into(),
        grant,
    })
    .unwrap();
    let mut subjects = vec![subject(&c)];
    {
        let mut db = s.db().unwrap();
        let tx = db.transaction().unwrap();
        for _ in 0..10 {
            let c = issue(&tx, &c.principal_id, ClientKind::Desktop).unwrap();
            subjects.push(subject(&c));
        }
        tx.commit().unwrap();
    }
    assert!(matches!(
        s.execution_authorizations(&subjects),
        Err(Error::TooLarge)
    ));
}

#[test]
fn one_batch_admission_serializes_with_grant_disable_and_revoke_commits() {
    for mutation in ["grant", "disable", "revoke"] {
        let (_dir, s) = service();
        let a = pair(&s);
        let b = pair(&s);
        let handle = s
            .execution_authorizations(&[subject(&a), subject(&b)])
            .unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let captured = handle.clone();
        let admission = thread::spawn(move || {
            captured.with_current(|identities| {
                assert_eq!(identities.len(), 2);
                entered_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                identities.iter().for_each(|i| i.check_deadline().unwrap());
            })
        });
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let writer = s.clone();
        let commit = thread::spawn(move || {
            started_tx.send(()).unwrap();
            match mutation {
                "grant" => writer
                    .set_grant(GrantSet {
                        principal_id: a.principal_id,
                        expected_revision: "1".into(),
                        grant: Grant::default(),
                    })
                    .unwrap(),
                "disable" => {
                    writer.configure(Configuration::default()).unwrap();
                }
                "revoke" => writer.revoke_session(&a.session_id, "local-owner").unwrap(),
                _ => unreachable!(),
            }
            done_tx.send(()).unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(
            done_rx.recv_timeout(Duration::from_millis(30)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        release_tx.send(()).unwrap();
        admission.join().unwrap().unwrap();
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        commit.join().unwrap();
        if mutation == "grant" {
            let values = handle.snapshot().unwrap();
            assert_eq!(values[0].grant_revision(), "2");
            assert!(values[0].grant().capabilities.is_empty());
            assert_eq!(values[1].grant_revision(), "1");
        } else {
            assert!(handle.snapshot().is_err());
        }
    }
}

#[test]
fn failed_revocation_transaction_does_not_revoke_execution_subjects() {
    let (_dir, s) = service();
    let c = pair(&s);
    let handle = s
        .execution_authorization(&c.principal_id, &c.session_id)
        .unwrap();
    s.db().unwrap().execute_batch("CREATE TRIGGER reject_execution_revoke BEFORE INSERT ON audit WHEN NEW.action='session.revoke' BEGIN SELECT RAISE(ABORT,'fixture rollback'); END;").unwrap();
    assert!(s.revoke_session(&c.session_id, "local-owner").is_err());
    assert!(handle.snapshot().is_ok());
}
