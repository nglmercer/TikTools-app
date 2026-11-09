use super::*;
use std::sync::{Arc, Barrier};

/// Synchronous store tests own their thread's clock. The marker prevents a
/// guard from being moved to another thread and restoring the wrong clock.
struct FrozenNow {
    previous: Option<i64>,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl FrozenNow {
    fn at(timestamp: i64) -> Self {
        Self {
            previous: TEST_NOW.with(|clock| clock.replace(Some(timestamp))),
            _thread: std::marker::PhantomData,
        }
    }

    fn set(&self, timestamp: i64) {
        TEST_NOW.with(|clock| clock.set(Some(timestamp)));
    }
}

impl Drop for FrozenNow {
    fn drop(&mut self) {
        TEST_NOW.with(|clock| clock.set(self.previous));
    }
}

fn service() -> (tempfile::TempDir, CollaborationService) {
    let dir = tempfile::tempdir().unwrap();
    let service = CollaborationService::open(&dir.path().join("private")).unwrap();
    service
        .configure(Configuration {
            enabled: true,
            advertised_origin: Some("http://127.0.0.1:8788".into()),
            ..Configuration::default()
        })
        .unwrap();
    (dir, service)
}

fn pending(s: &CollaborationService, client_kind: ClientKind) -> (PairingRequest, String) {
    let invitation = s
        .create_invitation(InviteCreate {
            grant: Grant::viewer(),
        })
        .unwrap();
    let exchange = secret().unwrap();
    let request = s
        .redeem(
            Redeem {
                invite_id: invitation.invite_id,
                secret: invitation.secret,
                client_label: "Moderator PC".into(),
                client_kind,
                exchange_secret_hash: hash_secret(&exchange),
            },
            "test-peer",
        )
        .unwrap();
    (request, exchange)
}

fn pair(s: &CollaborationService, client_kind: ClientKind) -> Credentials {
    let (request, exchange_secret) = pending(s, client_kind);
    s.decide_request(&request.request_id, true).unwrap();
    s.collect(
        Collect {
            request_id: request.request_id,
            exchange_secret,
        },
        "test-peer",
    )
    .unwrap()
}

#[test]
fn disabled_by_default_and_identity_persists_without_instance_reuse() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private");
    let first = CollaborationService::open(&path).unwrap();
    assert!(!first.status().unwrap().configuration.enabled);
    assert_eq!(
        first.status().unwrap().configuration.bind_address,
        "127.0.0.1:8788"
    );
    assert!(matches!(
        first.create_invitation(InviteCreate {
            grant: Grant::viewer()
        }),
        Err(Error::Disabled)
    ));
    let host = first.host_id().to_owned();
    let instance = first.instance_id().to_owned();
    drop(first);
    let second = CollaborationService::open(&path).unwrap();
    assert_eq!(second.host_id(), host);
    assert_ne!(second.instance_id(), instance);
}

#[test]
fn stale_configuration_cannot_disable_access_or_revoke_a_live_session() {
    let (_dir, s) = service();
    let credentials = pair(&s, ClientKind::Desktop);
    let invitation = s
        .create_invitation(InviteCreate {
            grant: Grant::viewer(),
        })
        .unwrap();
    let (configuration, revision) = s.configuration_snapshot().unwrap();
    let saved = s
        .configure_checked(configuration.clone(), Some(&revision))
        .unwrap();
    let before = s
        .audit(AuditQuery {
            after: 0,
            limit: Some(100),
        })
        .unwrap()
        .len();
    assert!(matches!(
        s.configure_checked(
            Configuration {
                enabled: false,
                ..configuration
            },
            Some(&revision)
        ),
        Err(Error::RevisionConflict)
    ));
    assert_eq!(
        s.status().unwrap().configuration_revision,
        saved.configuration_revision
    );
    assert!(s.status().unwrap().configuration.enabled);
    assert!(s
        .authenticate(&credentials.access_token, false, false)
        .is_ok());
    assert!(s
        .invitations(ListQuery::default())
        .unwrap()
        .items
        .iter()
        .any(|v| v.invite_id == invitation.invite_id && v.state == "created"));
    assert_eq!(
        s.audit(AuditQuery {
            after: 0,
            limit: Some(100)
        })
        .unwrap()
        .len(),
        before
    );
    for revision in ["01", "-1", "9223372036854775808"] {
        assert!(matches!(
            s.configure_checked(Configuration::default(), Some(revision)),
            Err(Error::Invalid(_))
        ));
    }
}

#[test]
fn simultaneous_owner_configuration_edits_have_exactly_one_winner() {
    let (_dir, service) = service();
    let s = Arc::new(service);
    let (configuration, revision) = s.configuration_snapshot().unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = [5, 6]
        .into_iter()
        .map(|rate| {
            let s = s.clone();
            let barrier = barrier.clone();
            let mut configuration = configuration.clone();
            let revision = revision.clone();
            std::thread::spawn(move || {
                configuration.limits.requests_per_second = rate;
                barrier.wait();
                s.configure_checked(configuration, Some(&revision))
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(Error::RevisionConflict)))
            .count(),
        1
    );
    assert_eq!(
        s.status().unwrap().configuration_revision,
        (revision.parse::<u64>().unwrap() + 1).to_string()
    );
}

#[test]
fn configuration_preconditions_are_wire_only_and_unknown_settings_stay_rejected() {
    let input: ConfigureInput =
        serde_json::from_value(serde_json::json!({"enabled":false,"expectedRevision":"0"}))
            .unwrap();
    assert_eq!(input.expected_revision.as_deref(), Some("0"));
    assert_eq!(
        serde_json::to_value(&input).unwrap()["expectedRevision"],
        "0"
    );
    assert!(serde_json::to_value(&input.configuration)
        .unwrap()
        .get("expectedRevision")
        .is_none());
    assert!(serde_json::from_value::<ConfigureInput>(
        serde_json::json!({"enabled":false,"expectedRevision":1})
    )
    .is_err());
    assert!(serde_json::from_value::<ConfigureInput>(
        serde_json::json!({"enabled":false,"unknown":true})
    )
    .is_err());
}

#[test]
fn owner_can_read_a_session_identity_directly_without_parsing_oversized_grants() {
    let (_dir, s) = service();
    let credentials = pair(&s, ClientKind::Desktop);
    let principal = s.principal(&credentials.principal_id).unwrap();
    assert_eq!(
        s.sessions(ListQuery::default()).unwrap().items[0].client_label,
        principal.client_label
    );
    assert!(matches!(s.principal("not-an-id"), Err(Error::Invalid(_))));
    assert!(matches!(
        s.principal(&uuid::Uuid::new_v4().to_string()),
        Err(Error::NotFound)
    ));
    s.db()
        .unwrap()
        .execute(
            "UPDATE principals SET grant_json=?1 WHERE id=?2",
            params!["invalid-json".repeat(50000), credentials.principal_id],
        )
        .unwrap();
    assert!(matches!(
        s.principal(&credentials.principal_id),
        Err(Error::TooLarge)
    ));
}

#[test]
fn a_second_store_owner_cannot_invalidate_pairing_or_keep_old_authorizations_alive() {
    let (dir, s) = service();
    let credentials = pair(&s, ClientKind::Desktop);
    let view = s
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap();
    let (request, exchange) = pending(&s, ClientKind::Agent);
    assert!(matches!(
        CollaborationService::open(&dir.path().join("private")),
        Err(Error::HostInUse)
    ));
    s.decide_request(&request.request_id, true).unwrap();
    assert!(s
        .collect(
            Collect {
                request_id: request.request_id,
                exchange_secret: exchange
            },
            "peer"
        )
        .is_ok());
    drop(s);
    assert!(view.snapshot().is_err());
    let reopened = CollaborationService::open(&dir.path().join("private")).unwrap();
    assert!(reopened
        .authenticate(&credentials.access_token, false, false)
        .is_ok());
}

#[test]
fn pairing_requires_secret_owner_approval_and_one_time_collection() {
    let (_dir, s) = service();
    let (request, exchange_secret) = pending(&s, ClientKind::Desktop);
    let input = Collect {
        request_id: request.request_id.clone(),
        exchange_secret: exchange_secret.clone(),
    };
    assert!(matches!(
        s.collect(input.clone(), "peer"),
        Err(Error::PairingPending)
    ));
    assert!(matches!(
        s.collect(
            Collect {
                request_id: request.request_id.clone(),
                exchange_secret: secret().unwrap()
            },
            "peer"
        ),
        Err(Error::PairingExpired)
    ));
    s.decide_request(&request.request_id, true).unwrap();
    let credentials = s.collect(input.clone(), "peer").unwrap();
    assert_eq!(
        s.authenticate(&credentials.access_token, false, true)
            .unwrap()
            .grant,
        Grant::viewer()
    );
    assert!(s
        .authenticate(&credentials.access_token, true, false)
        .is_err());
    assert!(matches!(
        s.collect(input, "peer"),
        Err(Error::PairingExpired)
    ));
    assert_eq!(s.sessions(ListQuery::default()).unwrap().items.len(), 1);
    let visible = serde_json::to_string(&(
        s.sessions(ListQuery::default()).unwrap().items,
        s.principals(ListQuery::default()).unwrap().items,
        s.invitations(ListQuery::default()).unwrap().items,
        s.requests(ListQuery::default()).unwrap().items,
        s.audit(AuditQuery {
            after: 0,
            limit: None,
        })
        .unwrap(),
    ))
    .unwrap();
    for value in [
        &exchange_secret,
        &credentials.access_token,
        credentials.refresh_token.as_ref().unwrap(),
    ] {
        assert!(!visible.contains(value));
    }
    let db = s.db().unwrap();
    let access_hash: String = db
        .query_row("SELECT access_hash FROM sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(access_hash, hash_secret(&credentials.access_token));
}

#[test]
fn invitations_and_collections_are_atomic_under_concurrent_attempts() {
    let (_dir, s) = service();
    let s = Arc::new(s);
    let invite = s
        .create_invitation(InviteCreate {
            grant: Grant::viewer(),
        })
        .unwrap();
    let exchange = secret().unwrap();
    let input = Redeem {
        invite_id: invite.invite_id,
        secret: invite.secret,
        client_label: "PC B".into(),
        client_kind: ClientKind::Desktop,
        exchange_secret_hash: hash_secret(&exchange),
    };
    let barrier = Arc::new(Barrier::new(3));
    let tasks: Vec<_> = (0..2)
        .map(|_| {
            let s = s.clone();
            let input = input.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                s.redeem(input, "peer")
            })
        })
        .collect();
    barrier.wait();
    let mut results: Vec<_> = tasks.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    let request = results.iter_mut().find_map(|r| r.as_mut().ok()).unwrap();
    s.decide_request(&request.request_id, true).unwrap();
    let input = Collect {
        request_id: request.request_id.clone(),
        exchange_secret: exchange,
    };
    let barrier = Arc::new(Barrier::new(3));
    let tasks: Vec<_> = (0..2)
        .map(|_| {
            let s = s.clone();
            let input = input.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                s.collect(input, "peer")
            })
        })
        .collect();
    barrier.wait();
    assert_eq!(
        tasks
            .into_iter()
            .map(|t| t.join().unwrap())
            .filter(|r| r.is_ok())
            .count(),
        1
    );
}

#[test]
fn browser_and_native_credentials_are_separate_and_revocation_is_independent() {
    let (_dir, s) = service();
    let a = pair(&s, ClientKind::Browser);
    let b = pair(&s, ClientKind::Agent);
    assert!(a.refresh_token.is_none());
    assert!(b.refresh_token.is_some());
    assert!(s.authenticate(&a.access_token, false, false).is_err());
    assert!(s.authenticate(&b.access_token, true, false).is_err());
    s.revoke_session(&a.session_id, "local-owner").unwrap();
    assert!(s.authenticate(&a.access_token, true, false).is_err());
    assert!(s.authenticate(&b.access_token, false, false).is_ok());
    s.revoke_principal(&b.principal_id).unwrap();
    assert!(s.authenticate(&b.access_token, false, false).is_err());
    assert!(s.refresh(b.refresh_token.as_ref().unwrap()).is_err());
}

#[test]
fn refresh_rotation_invalidates_old_access_and_reuse_revokes_only_that_session() {
    let (_dir, s) = service();
    let a = pair(&s, ClientKind::Desktop);
    let b = pair(&s, ClientKind::Agent);
    let refresh = a.refresh_token.as_ref().unwrap();
    let renewed = s.refresh(refresh).unwrap();
    assert_eq!(renewed.session_id, a.session_id);
    assert!(s.authenticate(&a.access_token, false, false).is_err());
    assert!(s.authenticate(&renewed.access_token, false, false).is_ok());
    assert!(s.refresh(refresh).is_err());
    assert!(s.authenticate(&renewed.access_token, false, false).is_err());
    assert!(s.authenticate(&b.access_token, false, false).is_ok());
}

#[test]
fn expiry_rejection_revoke_and_bad_credentials_do_not_mint_sessions() {
    let (_dir, s) = service();
    let (r, e) = pending(&s, ClientKind::Desktop);
    s.decide_request(&r.request_id, false).unwrap();
    assert!(matches!(
        s.collect(
            Collect {
                request_id: r.request_id,
                exchange_secret: e
            },
            "peer"
        ),
        Err(Error::PairingRejected)
    ));
    let (r, e) = pending(&s, ClientKind::Desktop);
    s.db()
        .unwrap()
        .execute(
            "UPDATE pairing_requests SET expires_at=0 WHERE id=?1",
            [&r.request_id],
        )
        .unwrap();
    assert!(matches!(
        s.collect(
            Collect {
                request_id: r.request_id,
                exchange_secret: e
            },
            "peer"
        ),
        Err(Error::PairingExpired)
    ));
    let invitation = s
        .create_invitation(InviteCreate {
            grant: Grant::viewer(),
        })
        .unwrap();
    s.revoke_invitation(&invitation.invite_id).unwrap();
    assert!(s
        .redeem(
            Redeem {
                invite_id: invitation.invite_id,
                secret: invitation.secret,
                client_label: "B".into(),
                client_kind: ClientKind::Desktop,
                exchange_secret_hash: hash_secret(&secret().unwrap())
            },
            "peer"
        )
        .is_err());
    assert!(s.sessions(ListQuery::default()).unwrap().items.is_empty());
}

#[test]
fn idle_absolute_and_access_expiries_are_enforced() {
    let (_dir, s) = service();
    let a = pair(&s, ClientKind::Desktop);
    s.db()
        .unwrap()
        .execute(
            "UPDATE sessions SET access_expires=0 WHERE id=?1",
            [&a.session_id],
        )
        .unwrap();
    assert!(s.authenticate(&a.access_token, false, false).is_err());
    assert!(s.refresh(a.refresh_token.as_ref().unwrap()).is_ok());
    let b = pair(&s, ClientKind::Browser);
    s.db()
        .unwrap()
        .execute(
            "UPDATE sessions SET last_activity=?2 WHERE id=?1",
            params![b.session_id, now().saturating_sub(IDLE_TTL + 1)],
        )
        .unwrap();
    assert!(s.authenticate(&b.access_token, true, true).is_err());
    let c = pair(&s, ClientKind::Agent);
    s.db()
        .unwrap()
        .execute(
            "UPDATE sessions SET absolute_expires=0 WHERE id=?1",
            [&c.session_id],
        )
        .unwrap();
    assert!(s.refresh(c.refresh_token.as_ref().unwrap()).is_err());
}

#[test]
fn grants_use_atomic_revision_checks_and_are_read_live() {
    let (_dir, s) = service();
    let a = pair(&s, ClientKind::Desktop);
    let input = GrantSet {
        principal_id: a.principal_id.clone(),
        expected_revision: "1".into(),
        grant: Grant::default(),
    };
    s.set_grant(input.clone()).unwrap();
    assert!(matches!(s.set_grant(input), Err(Error::RevisionConflict)));
    let identity = s.authenticate(&a.access_token, false, false).unwrap();
    assert_eq!(identity.grant_revision, "2");
    assert!(identity.grant.capabilities.is_empty());
    assert!(s
        .set_grant(GrantSet {
            principal_id: a.principal_id,
            expected_revision: "18446744073709551615".into(),
            grant: Grant::viewer()
        })
        .is_err());
}

#[test]
fn reusable_automation_policy_is_persisted_versioned_and_revocable() {
    let (dir, s) = service();
    let a = pair(&s, ClientKind::Desktop);
    let current = s.authenticate_live(&a.access_token, false, true).unwrap();
    let mut grant = Grant::default();
    grant.capabilities = [
        Capability::AutomationRead,
        Capability::AutomationEdit,
        Capability::AutomationToggle,
    ]
    .into();
    grant.automation_delegation = Some(AutomationDelegation {
        effects: [
            DelegatedAutomationEffect::Log,
            DelegatedAutomationEffect::Delay,
        ]
        .into(),
        allow_dynamic_targets: false,
    });
    s.set_grant(GrantSet {
        principal_id: a.principal_id.clone(),
        expected_revision: "1".into(),
        grant: grant.clone(),
    })
    .unwrap();
    let installed = current.snapshot().unwrap();
    assert_eq!(installed.grant_revision, "2");
    assert_eq!(
        installed.grant.automation_delegation,
        grant.automation_delegation
    );
    assert_eq!(
        s.authenticate(&a.access_token, false, false)
            .unwrap()
            .grant
            .automation_delegation,
        grant.automation_delegation
    );
    drop(current);
    drop(s);
    let s = CollaborationService::open(&dir.path().join("private")).unwrap();
    let current = s.authenticate_live(&a.access_token, false, true).unwrap();
    assert_eq!(
        current.snapshot().unwrap().grant.automation_delegation,
        grant.automation_delegation
    );
    s.set_grant(GrantSet {
        principal_id: a.principal_id,
        expected_revision: "2".into(),
        grant: Grant::viewer(),
    })
    .unwrap();
    let removed = current.snapshot().unwrap();
    assert_eq!(removed.grant_revision, "3");
    assert!(removed.grant.automation_delegation.is_none());
}

#[test]
fn connected_authorizations_observe_revocation_rotation_and_grants_without_disk_reads() {
    let (_dir, s) = service();
    let a = pair(&s, ClientKind::Desktop);
    let b = pair(&s, ClientKind::Agent);
    let view_a = s.authenticate_live(&a.access_token, false, true).unwrap();
    let view_b = s.authenticate_live(&b.access_token, false, true).unwrap();
    s.set_grant(GrantSet {
        principal_id: a.principal_id.clone(),
        expected_revision: "1".into(),
        grant: Grant::default(),
    })
    .unwrap();
    assert!(view_a.snapshot().unwrap().grant.capabilities.is_empty());
    assert_eq!(view_a.snapshot().unwrap().grant_revision, "2");
    assert!(view_b
        .snapshot()
        .unwrap()
        .grant
        .capabilities
        .contains(&Capability::LiveRead));
    let rotated = s.refresh(a.refresh_token.as_ref().unwrap()).unwrap();
    assert!(view_a.snapshot().is_err());
    let renewed = s
        .authenticate_live(&rotated.access_token, false, true)
        .unwrap();
    assert!(renewed.snapshot().is_ok());
    s.revoke_session(&rotated.session_id, "local-owner")
        .unwrap();
    assert!(renewed.snapshot().is_err());
    assert!(view_b.snapshot().is_ok());
    s.configure(Configuration::default()).unwrap();
    assert!(view_b.snapshot().is_err());
}

#[test]
fn audit_rollback_does_not_revoke_a_connected_authorization() {
    let (_dir, s) = service();
    let credentials = pair(&s, ClientKind::Desktop);
    let view = s
        .authenticate_live(&credentials.access_token, false, false)
        .unwrap();
    s.db().unwrap().execute_batch("CREATE TRIGGER audit_failure BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT,'audit unavailable'); END;").unwrap();
    assert!(s
        .revoke_session(&credentials.session_id, "local-owner")
        .is_err());
    assert!(view.snapshot().is_ok());
    assert!(s
        .authenticate(&credentials.access_token, false, false)
        .is_ok());
}

#[test]
fn restart_keeps_approved_sessions_but_invalidates_pending_exchanges() {
    let (dir, s) = service();
    let a = pair(&s, ClientKind::Desktop);
    let (r, e) = pending(&s, ClientKind::Agent);
    s.decide_request(&r.request_id, true).unwrap();
    drop(s);
    let s = CollaborationService::open(&dir.path().join("private")).unwrap();
    assert!(s.authenticate(&a.access_token, false, false).is_ok());
    assert!(matches!(
        s.collect(
            Collect {
                request_id: r.request_id,
                exchange_secret: e
            },
            "peer"
        ),
        Err(Error::PairingExpired)
    ));
}

#[test]
fn failed_attempts_consume_rate_budget_and_disabled_feature_revokes_access() {
    let _clock = FrozenNow::at(now());
    let (_dir, s) = service();
    let a = pair(&s, ClientKind::Desktop);
    for _ in 0..5 {
        assert!(matches!(
            s.redeem(
                Redeem {
                    invite_id: "unknown".into(),
                    secret: secret().unwrap(),
                    client_label: "B".into(),
                    client_kind: ClientKind::Desktop,
                    exchange_secret_hash: hash_secret(&secret().unwrap())
                },
                "bad-peer"
            ),
            Err(Error::PairingExpired)
        ));
    }
    assert!(matches!(
        s.redeem(
            Redeem {
                invite_id: "unknown".into(),
                secret: secret().unwrap(),
                client_label: "B".into(),
                client_kind: ClientKind::Desktop,
                exchange_secret_hash: hash_secret(&secret().unwrap())
            },
            "bad-peer"
        ),
        Err(Error::RateLimited)
    ));
    s.configure(Configuration::default()).unwrap();
    assert!(matches!(
        s.authenticate(&a.access_token, false, false),
        Err(Error::Disabled)
    ));
    s.configure(Configuration {
        enabled: true,
        advertised_origin: Some("http://127.0.0.1:8788".into()),
        ..Configuration::default()
    })
    .unwrap();
    assert!(s.authenticate(&a.access_token, false, false).is_err());
}

#[test]
fn failed_pairing_rate_budget_resets_at_the_next_wall_minute() {
    let minute = now() / MINUTE;
    let last_millisecond = (minute + 1) * MINUTE - 1;
    let clock = FrozenNow::at(last_millisecond);
    let (_dir, s) = service();
    let source = "minute-boundary-peer";
    let attempt = || {
        s.redeem(
            Redeem {
                invite_id: "unknown".into(),
                secret: "a".repeat(64),
                client_label: "Boundary fixture".into(),
                client_kind: ClientKind::Desktop,
                exchange_secret_hash: hash_secret("boundary exchange"),
            },
            source,
        )
    };
    let persisted = || {
        s.db()
            .unwrap()
            .query_row(
                "SELECT window,count FROM rate_limits WHERE category='redeem-source' AND identity=?1",
                [hash_secret(source)],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .unwrap()
    };
    for _ in 0..5 {
        assert!(matches!(attempt(), Err(Error::PairingExpired)));
    }
    assert_eq!(persisted(), (minute, 5));
    assert!(matches!(attempt(), Err(Error::RateLimited)));
    assert_eq!(persisted(), (minute, 6));

    // Advancing one millisecond crosses the real fixed-window boundary. The
    // next failure is admitted and consumes the first budget slot, without a
    // sleep, weakening the five-attempt limit or clearing the stored counter.
    clock.set(last_millisecond + 1);
    assert!(matches!(attempt(), Err(Error::PairingExpired)));
    assert_eq!(persisted(), (minute + 1, 1));
}

#[test]
fn audit_failure_rolls_back_admission_and_config_validation_fails_closed() {
    let (_dir, s) = service();
    for config in [
        Configuration {
            enabled: true,
            advertised_origin: Some("http://public.example".into()),
            ..Configuration::default()
        },
        Configuration {
            enabled: true,
            advertised_origin: Some("https://public.example".into()),
            ..Configuration::default()
        },
        Configuration {
            enabled: true,
            bind_address: "0.0.0.0:8788".into(),
            advertised_origin: Some("https://public.example".into()),
            tls_proxy: true,
            ..Configuration::default()
        },
    ] {
        assert!(s.configure(config).is_err());
    }
    s.db().unwrap().execute_batch("CREATE TRIGGER audit_failure BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT,'injected audit failure'); END;").unwrap();
    assert!(s
        .create_invitation(InviteCreate {
            grant: Grant::viewer()
        })
        .is_err());
    assert!(s
        .invitations(ListQuery::default())
        .unwrap()
        .items
        .is_empty());
}

#[cfg(unix)]
#[test]
fn storage_rejects_public_directories_and_symlinked_database() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(CollaborationService::open(dir.path()).is_err());
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    symlink(
        dir.path().join("outside"),
        dir.path().join("collaboration.db"),
    )
    .unwrap();
    assert!(CollaborationService::open(dir.path()).is_err());
}

#[test]
fn owner_pages_reach_all_historical_identities_sessions_and_invitations() {
    let (_dir, s) = service();
    // History can exceed the active-device admission limit. These are old,
    // revoked devices, as after months of independently pairing controllers.
    let mut db = s.db().unwrap();
    let tx = db.transaction().unwrap();
    for index in 1..=1050 {
        let id = Uuid::from_u128(index).to_string();
        tx.execute(
            "INSERT INTO invitations VALUES (?1,?1,'{}',0,0,'expired')",
            [&id],
        )
        .unwrap();
        tx.execute("INSERT INTO principals(id,label,kind,grant_json,revoked) VALUES (?1,'old device','desktop','{}',1)", [&id]).unwrap();
        tx.execute("INSERT INTO sessions(id,principal_id,kind,access_hash,access_expires,created_at,last_activity,absolute_expires,revoked) VALUES (?1,?1,'desktop',?1,0,0,0,0,1)", [&id]).unwrap();
        if index <= 7 {
            tx.execute("INSERT INTO pairing_requests(id,invite_id,exchange_hash,label,kind,grant_json,state,expires_at) VALUES (?1,?1,?1,'new device','desktop','{}','pending',?2)", params![id,now()+MINUTE]).unwrap();
        }
    }
    tx.commit().unwrap();
    drop(db);
    fn all<T>(mut load: impl FnMut(ListQuery) -> Result<ListPage<T>, Error>) -> Vec<T> {
        let mut after = None;
        let mut found = Vec::new();
        loop {
            let page = load(ListQuery {
                after: after.clone(),
                limit: Some(73),
            })
            .unwrap();
            assert!(page.items.len() <= 73);
            found.extend(page.items);
            match page.next_after {
                Some(cursor) => {
                    assert_ne!(Some(&cursor), after.as_ref());
                    after = Some(cursor);
                }
                None => return found,
            }
        }
    }
    let expected: Vec<_> = (1..=1050)
        .map(|id| Uuid::from_u128(id).to_string())
        .collect();
    assert_eq!(
        all(|q| s.invitations(q))
            .into_iter()
            .map(|item| item.invite_id)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        all(|q| s.principals(q))
            .into_iter()
            .map(|item| item.principal_id)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        all(|q| s.sessions(q))
            .into_iter()
            .map(|item| item.session_id)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        all(|q| s.requests(q))
            .into_iter()
            .map(|item| item.request_id)
            .collect::<Vec<_>>(),
        expected[..7]
    );
    assert!(s
        .invitations(ListQuery {
            after: Some(expected.last().unwrap().clone()),
            limit: None
        })
        .unwrap()
        .items
        .is_empty());
    for query in [
        ListQuery {
            after: Some("invalid".into()),
            limit: None,
        },
        ListQuery {
            after: None,
            limit: Some(0),
        },
        ListQuery {
            after: None,
            limit: Some(101),
        },
    ] {
        assert!(matches!(
            s.invitations(query.clone()),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            s.principals(query.clone()),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(s.sessions(query.clone()), Err(Error::Invalid(_))));
        assert!(matches!(s.requests(query), Err(Error::Invalid(_))));
    }
}

#[test]
fn large_grants_reduce_owner_page_size_without_losing_the_next_record() {
    let (_dir, s) = service();
    let actions: std::collections::BTreeSet<_> = (0..128)
        .map(|index| format!("{index:03}{}", "a".repeat(197)))
        .collect();
    let grant = Grant {
        actions: (0..16)
            .map(|index| (format!("plugin-{index}"), actions.clone()))
            .collect(),
        ..Grant::default()
    };
    grant.validate().unwrap();
    let encoded = serde_json::to_string(&grant).unwrap();
    let db = s.db().unwrap();
    for index in 1..=3 {
        let id = Uuid::from_u128(index).to_string();
        db.execute(
            "INSERT INTO principals(id,label,kind,grant_json) VALUES (?1,'moderator','desktop',?2)",
            params![id, encoded],
        )
        .unwrap();
    }
    drop(db);
    let mut after = None;
    for index in 1..=3 {
        let result = s
            .principals(ListQuery {
                after,
                limit: Some(100),
            })
            .unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(
            result.items[0].principal_id,
            Uuid::from_u128(index).to_string()
        );
        assert!(serde_json::to_vec(&result).unwrap().len() < 1024 * 1024);
        after = result.next_after;
        assert_eq!(after.is_some(), index != 3);
    }
}

#[test]
fn configuration_generation_is_durable_and_audit_failure_cannot_advance_it() {
    let (dir, s) = service();
    let (configuration, first_revision) = s.configuration_snapshot().unwrap();
    let second_revision = s
        .configure(configuration.clone())
        .unwrap()
        .configuration_revision;
    assert_ne!(first_revision, second_revision);
    assert_eq!(s.configuration_snapshot().unwrap().1, second_revision);
    s.db().unwrap().execute_batch("CREATE TRIGGER fail_configuration_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT,'test'); END").unwrap();
    assert!(s.configure(configuration.clone()).is_err());
    assert_eq!(s.configuration_snapshot().unwrap().1, second_revision);
    s.db()
        .unwrap()
        .execute_batch("DROP TRIGGER fail_configuration_audit")
        .unwrap();
    drop(s);
    let restored = CollaborationService::open(&dir.path().join("private")).unwrap();
    assert_eq!(
        restored.configuration_snapshot().unwrap(),
        (configuration, second_revision)
    );
}

fn imported_record(label: &str) -> AuditRecord {
    AuditRecord {
        event_id: hash_secret(label),
        timestamp: now(),
        principal_id: "principal-a".into(),
        session_id: "session-a".into(),
        method: "workflows.save".into(),
        resource: CommandResource {
            kind: ResourceKind::Workflow,
            id: "welcome".into(),
            scope: "host".into(),
        },
        command_id: Some("command-a".into()),
        before_revision: "0".into(),
        after_revision: "1".into(),
        result_code: "succeeded".into(),
        correlation_id: hash_secret(label),
    }
}
#[test]
fn durable_audit_import_is_idempotent_and_rejects_reused_event_payloads_atomically() {
    let (_dir, s) = service();
    let record = imported_record("once");
    assert_eq!(
        s.import_audit(&[record.clone(), record.clone()]).unwrap(),
        1
    );
    assert_eq!(s.import_audit(std::slice::from_ref(&record)).unwrap(), 0);
    let entries = s
        .audit(AuditQuery {
            after: 0,
            limit: None,
        })
        .unwrap();
    let imported: Vec<_> = entries
        .iter()
        .filter_map(|entry| entry.details.as_ref())
        .collect();
    assert_eq!(imported, vec![&record]);
    let mut conflict = record.clone();
    conflict.after_revision = "2".into();
    assert!(matches!(
        s.import_audit(&[imported_record("must-rollback"), conflict]),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        s.db()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_imports", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    s.configure(Configuration::default()).unwrap();
    assert_eq!(
        s.import_audit(&[imported_record("disabled-host")]).unwrap(),
        1
    );
}
#[test]
fn audit_detail_write_failure_rolls_back_receipts_and_metadata_fields_are_closed() {
    let (_dir, s) = service();
    let record = imported_record("failed-details");
    s.db().unwrap().execute_batch("CREATE TRIGGER fail_audit_details BEFORE INSERT ON audit_details BEGIN SELECT RAISE(ABORT,'test'); END").unwrap();
    assert!(s.import_audit(std::slice::from_ref(&record)).is_err());
    assert_eq!(
        s.db()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_imports", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(s
        .audit(AuditQuery {
            after: 0,
            limit: None
        })
        .unwrap()
        .iter()
        .all(|entry| entry.details.is_none()));
    s.db()
        .unwrap()
        .execute_batch("DROP TRIGGER fail_audit_details")
        .unwrap();
    assert_eq!(s.import_audit(std::slice::from_ref(&record)).unwrap(), 1);
    let mut untrusted = serde_json::to_value(&record).unwrap();
    untrusted["params"] = serde_json::json!({"password":"must not persist"});
    assert!(serde_json::from_value::<AuditRecord>(untrusted).is_err());
    let mut invalid = record.clone();
    invalid.method = "private script body".into();
    assert!(s.import_audit(&[invalid]).is_err());
    assert!(s.import_audit(&vec![record; 101]).is_err());
}
#[test]
fn late_audit_retry_cannot_resurrect_expired_history_after_retention() {
    let (_dir, s) = service();
    let mut record = imported_record("expired-import");
    record.timestamp = now() - 31 * IDLE_TTL;
    let json = serde_json::to_string(&record).unwrap();
    {
        let db = s.db().unwrap();
        db.execute("INSERT INTO audit(timestamp,principal_id,action,resource_id,outcome) VALUES (?1,'principal-a','workflows.save','welcome','succeeded')",[record.timestamp]).unwrap();
        let cursor = db.last_insert_rowid();
        db.execute(
            "INSERT INTO audit_details VALUES (?1,?2)",
            params![cursor, json],
        )
        .unwrap();
        db.execute(
            "INSERT INTO audit_imports VALUES (?1,?2,?3)",
            params![record.event_id, hash_secret(&json), record.timestamp],
        )
        .unwrap();
    }
    // Idle history expires when it is queried, without requiring another
    // mutation/import to trigger retention first.
    assert!(s
        .audit(AuditQuery {
            after: 0,
            limit: None
        })
        .unwrap()
        .iter()
        .all(|entry| entry.details.is_none()));
    assert_eq!(s.import_audit(&[record.clone(), record]).unwrap(), 0);
    assert!(s
        .audit(AuditQuery {
            after: 0,
            limit: None
        })
        .unwrap()
        .iter()
        .all(|entry| entry.details.is_none()));
    assert_eq!(
        s.db()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_details", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        s.db()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_imports", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
