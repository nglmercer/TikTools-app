use super::*;

struct Clock;
impl Clock {
    fn new() -> Self {
        TEST_NOW.with(|clock| clock.set(Some(1_000_000)));
        crate::peer_clock::TEST_CLOCK.with(|clock| {
            *clock.borrow_mut() = Some(crate::peer_clock::Stamp {
                boot: "test-boot".into(),
                elapsed_ms: 1000,
            })
        });
        Self
    }
    fn advance(&self, duration: i64) {
        TEST_NOW.with(|clock| clock.set(Some(clock.get().unwrap() + duration)));
        crate::peer_clock::TEST_CLOCK
            .with(|clock| clock.borrow_mut().as_mut().unwrap().elapsed_ms += duration);
    }
}
impl Drop for Clock {
    fn drop(&mut self) {
        TEST_NOW.with(|clock| clock.set(None));
        crate::peer_clock::TEST_CLOCK.with(|clock| *clock.borrow_mut() = None);
    }
}

fn service(path: &Path) -> CollaborationService {
    let s = CollaborationService::open(path).unwrap();
    s.configure(Configuration {
        enabled: true,
        peer: Some(PeerConfiguration::default()),
        advertised_origin: Some("http://127.0.0.1:8788".into()),
        ..Configuration::default()
    })
    .unwrap();
    s
}

fn grant() -> CloudPeerGrant {
    CloudPeerGrant {
        origin: "https://cloud.example".into(),
        account_id: "owner".into(),
        host_id: Uuid::new_v4().to_string(),
        device_id: Uuid::new_v4().to_string(),
        grant_id: Uuid::new_v4().to_string(),
        cloud_revision: 1,
        endpoint_id: "ab".repeat(32),
        label: "Moderator".into(),
        client_kind: ClientKind::Desktop,
        grant: Grant::viewer(),
    }
}

#[test]
fn approved_peer_keeps_identity_after_restart_and_expires_without_cloud_renewal() {
    let clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let s = service(&home.path().join("private"));
    let g = grant();
    let identity = s.apply_cloud_peer_grant(g.clone()).unwrap();
    let live = s.authenticate_peer(&g.endpoint_id).unwrap();
    assert_eq!(live.snapshot().unwrap().principal_id, identity.principal_id);
    clock.advance(PEER_OFFLINE_TTL_MS - 1);
    assert!(s.authenticate_peer(&g.endpoint_id).is_ok());
    drop(s);
    let s = CollaborationService::open(&home.path().join("private")).unwrap();
    let reopened_view = s.authenticate_peer(&g.endpoint_id).unwrap();
    assert_eq!(
        reopened_view.snapshot().unwrap().session_id,
        identity.session_id
    );
    clock.advance(1);
    assert!(matches!(
        s.authenticate_peer(&g.endpoint_id),
        Err(Error::Unauthorized)
    ));
    assert!(live.snapshot().is_err());
    assert!(reopened_view.snapshot().is_err());
}

#[test]
fn calendar_changes_do_not_change_peer_rpc_or_execution_deadlines() {
    let clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("private");
    let s = Arc::new(service(&path));
    let g = grant();
    let identity = s.apply_cloud_peer_grant(g.clone()).unwrap();
    let view = s.authenticate_peer(&g.endpoint_id).unwrap();
    let execution = s
        .execution_authorization(&identity.principal_id, &identity.session_id)
        .unwrap();
    let saved = execution.snapshot().unwrap();
    // Calendar time can move forward past the old display timestamp, then back.
    for wall in [365 * IDLE_TTL, 1] {
        TEST_NOW.with(|now| now.set(Some(wall)));
        assert!(view.snapshot().is_ok());
        assert!(s.authenticate_peer(&g.endpoint_id).is_ok());
        assert!(execution.snapshot().is_ok());
        assert!(saved.check_deadline().is_ok());
    }
    clock.advance(PEER_OFFLINE_TTL_MS);
    TEST_NOW.with(|now| now.set(Some(1)));
    assert!(view.snapshot().is_err());
    assert!(saved.check_deadline().is_err());
    assert!(execution.snapshot().is_err());
    assert!(s.authenticate_peer(&g.endpoint_id).is_err());
    drop(execution);
    drop(s);
    let reopened = CollaborationService::open(&path).unwrap();
    assert!(reopened.authenticate_peer(&g.endpoint_id).is_err());
}

#[test]
fn another_boot_requires_fresh_cloud_validation_without_replacing_the_session() {
    let _clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("private");
    let s = Arc::new(service(&path));
    let g = grant();
    let first = s.apply_cloud_peer_grant(g.clone()).unwrap();
    let view = s.authenticate_peer(&g.endpoint_id).unwrap();
    let execution = s
        .execution_authorization(&first.principal_id, &first.session_id)
        .unwrap();
    crate::peer_clock::TEST_CLOCK.with(|clock| {
        *clock.borrow_mut() = Some(crate::peer_clock::Stamp {
            boot: "next-boot".into(),
            elapsed_ms: 500,
        });
    });
    assert!(view.snapshot().is_err());
    assert!(execution.snapshot().is_err());
    drop(execution);
    drop(s);
    let s = Arc::new(CollaborationService::open(&path).unwrap());
    assert!(s.authenticate_peer(&g.endpoint_id).is_err());
    assert!(s
        .execution_authorization(&first.principal_id, &first.session_id)
        .is_err());
    let renewed = s.apply_cloud_peer_grant(g.clone()).unwrap();
    assert_eq!(first.session_id, renewed.session_id);
    assert_eq!(first.principal_id, renewed.principal_id);
    assert!(s
        .authenticate_peer(&g.endpoint_id)
        .unwrap()
        .snapshot()
        .is_ok());
    assert!(s
        .execution_authorization(&first.principal_id, &first.session_id)
        .is_ok());
}

#[test]
fn old_peer_bindings_without_clock_evidence_need_fresh_validation() {
    let _clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let s = Arc::new(service(&home.path().join("private")));
    let g = grant();
    let first = s.apply_cloud_peer_grant(g.clone()).unwrap();
    s.db()
        .unwrap()
        .execute("DELETE FROM cloud_peer_leases", [])
        .unwrap();
    assert!(s.authenticate_peer(&g.endpoint_id).is_err());
    assert!(s
        .execution_authorization(&first.principal_id, &first.session_id)
        .is_err());
    let renewed = s.apply_cloud_peer_grant(g.clone()).unwrap();
    assert_eq!(first.session_id, renewed.session_id);
    assert!(s.authenticate_peer(&g.endpoint_id).is_ok());
    s.db()
        .unwrap()
        .execute("UPDATE cloud_peer_leases SET until_ms=issued_ms", [])
        .unwrap();
    assert!(s.authenticate_peer(&g.endpoint_id).is_err());
    assert!(s
        .execution_authorization(&first.principal_id, &first.session_id)
        .is_err());
}

#[test]
fn fresh_validation_renews_the_lease_without_extending_saved_execution_snapshots() {
    let clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let s = Arc::new(service(&home.path().join("private")));
    let g = grant();
    let first = s.apply_cloud_peer_grant(g.clone()).unwrap();
    let view = s.authenticate_peer(&g.endpoint_id).unwrap();
    let execution = s
        .execution_authorization(&first.principal_id, &first.session_id)
        .unwrap();
    let saved = execution.snapshot().unwrap();
    clock.advance(PEER_OFFLINE_TTL_MS - 1);
    s.apply_cloud_peer_grant(g.clone()).unwrap();
    clock.advance(1);
    assert!(view.snapshot().is_ok());
    assert!(execution.snapshot().is_ok());
    assert!(saved.check_deadline().is_err());
    clock.advance(PEER_OFFLINE_TTL_MS - 1);
    assert!(view.snapshot().is_err());
    assert!(execution.snapshot().is_err());
}

#[test]
fn grant_changes_revalidate_live_views_and_local_revocation_cannot_be_undone_by_cloud() {
    let _clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let s = service(&home.path().join("private"));
    let mut g = grant();
    let first = s.apply_cloud_peer_grant(g.clone()).unwrap();
    let view = s.authenticate_peer(&g.endpoint_id).unwrap();
    g.cloud_revision = 2;
    g.grant = Grant::default();
    let next = s.apply_cloud_peer_grant(g.clone()).unwrap();
    assert_eq!(first.principal_id, next.principal_id);
    assert_eq!(next.grant_revision, "2");
    assert!(view.snapshot().unwrap().grant.capabilities.is_empty());
    s.revoke_principal(&first.principal_id).unwrap();
    assert!(view.snapshot().is_err());
    assert!(matches!(
        s.authenticate_peer(&g.endpoint_id),
        Err(Error::Unauthorized)
    ));
    g.cloud_revision = 3;
    assert!(matches!(
        s.apply_cloud_peer_grant(g),
        Err(Error::Unauthorized)
    ));
}

#[test]
fn cloud_rewinds_endpoint_rotation_and_local_owner_edits_require_reconciliation() {
    let _clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let s = service(&home.path().join("private"));
    let mut g = grant();
    let initial = s.apply_cloud_peer_grant(g.clone()).unwrap();
    let mut other = g.clone();
    other.endpoint_id = "cd".repeat(32);
    assert!(matches!(
        s.apply_cloud_peer_grant(other),
        Err(Error::RevisionConflict)
    ));
    other = g.clone();
    other.account_id = "another-account".into();
    assert!(matches!(
        s.apply_cloud_peer_grant(other),
        Err(Error::Forbidden)
    ));
    other = g.clone();
    other.grant = Grant::default();
    assert!(matches!(
        s.apply_cloud_peer_grant(other),
        Err(Error::RevisionConflict)
    ));
    s.set_grant(GrantSet {
        principal_id: initial.principal_id,
        expected_revision: "1".into(),
        grant: Grant::default(),
    })
    .unwrap();
    g.cloud_revision = 2;
    assert!(matches!(
        s.apply_cloud_peer_grant(g),
        Err(Error::RevisionConflict)
    ));
}

fn source(g: &CloudPeerGrant) -> CloudPeerSource {
    CloudPeerSource {
        origin: g.origin.clone(),
        account_id: g.account_id.clone(),
        host_id: g.host_id.clone(),
    }
}
fn observed(g: &CloudPeerGrant, approved: bool, revoked: bool) -> CloudPeerObservation {
    CloudPeerObservation {
        device_id: g.device_id.clone(),
        grant_id: g.grant_id.clone(),
        revision: g.cloud_revision,
        approved,
        revoked,
    }
}

#[test]
fn changed_cloud_revision_fences_old_views_and_fresh_approval_keeps_logical_identity() {
    let _clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let s = service(&home.path().join("private"));
    let mut g = grant();
    let first = s.apply_cloud_peer_grant(g.clone()).unwrap();
    let old = s.authenticate_peer(&g.endpoint_id).unwrap();
    g.cloud_revision = 2;
    s.reconcile_cloud_peers(&source(&g), &[observed(&g, false, false)])
        .unwrap();
    assert!(old.snapshot().is_err());
    assert!(s.authenticate_peer(&g.endpoint_id).is_err());
    let next = s.apply_cloud_peer_grant(g.clone()).unwrap();
    assert_eq!(next.principal_id, first.principal_id);
    assert_eq!(next.session_id, first.session_id);
    assert!(old.snapshot().is_err());
    let fresh = s.authenticate_peer(&g.endpoint_id).unwrap();
    assert_eq!(fresh.snapshot().unwrap().grant_revision, "2");
    let mut stale = g.clone();
    stale.cloud_revision = 1;
    let mut history = stale.clone();
    history.grant_id = Uuid::new_v4().to_string();
    s.reconcile_cloud_peers(
        &source(&g),
        &[
            observed(&stale, false, false),
            observed(&history, false, true),
        ],
    )
    .unwrap();
    assert!(fresh.snapshot().is_ok());
    g.cloud_revision = 3;
    s.reconcile_cloud_peers(&source(&g), &[observed(&g, false, true)])
        .unwrap();
    assert!(fresh.snapshot().is_err());
    g.cloud_revision = 4;
    assert!(matches!(
        s.apply_cloud_peer_grant(g),
        Err(Error::Unauthorized)
    ));
}

#[test]
fn account_switch_suspends_only_cloud_peers_and_revocation_survives_suspension() {
    let _clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let s = service(&home.path().join("private"));
    let mut first = grant();
    s.apply_cloud_peer_grant(first.clone()).unwrap();
    let old = s.authenticate_peer(&first.endpoint_id).unwrap();
    let mut other = grant();
    other.account_id = "other".into();
    other.endpoint_id = "cd".repeat(32);
    s.apply_cloud_peer_grant(other.clone()).unwrap();
    let retained = s.authenticate_peer(&other.endpoint_id).unwrap();
    s.retain_cloud_peer_source(Some(&source(&other))).unwrap();
    assert!(old.snapshot().is_err());
    assert!(retained.snapshot().is_ok());
    assert!(s.authenticate_peer(&first.endpoint_id).is_err());
    first.cloud_revision = 2;
    s.reconcile_cloud_peers(&source(&first), &[observed(&first, false, true)])
        .unwrap();
    first.cloud_revision = 3;
    assert!(matches!(
        s.apply_cloud_peer_grant(first),
        Err(Error::Unauthorized)
    ));
    s.retain_cloud_peer_source(None).unwrap();
    assert!(retained.snapshot().is_err());
    assert!(s.authenticate_peer(&other.endpoint_id).is_err());
}

#[test]
fn rtc_binding_requires_exact_cloud_source_device_and_revision() {
    let _clock = Clock::new();
    let home = tempfile::tempdir().unwrap();
    let service = service(&home.path().join("private"));
    let binding = grant();
    service.apply_cloud_peer_grant(binding.clone()).unwrap();
    let source = CloudPeerSource {
        origin: binding.origin.clone(),
        account_id: binding.account_id.clone(),
        host_id: binding.host_id.clone(),
    };
    assert!(service
        .authenticate_rtc_peer(&binding.endpoint_id, &source, &binding.device_id, 1)
        .is_ok());
    assert!(service
        .authenticate_rtc_peer(&binding.endpoint_id, &source, &binding.device_id, 2)
        .is_err());
    assert!(service
        .authenticate_rtc_peer(
            &binding.endpoint_id,
            &source,
            &Uuid::new_v4().to_string(),
            1
        )
        .is_err());
    for wrong in [
        CloudPeerSource {
            origin: "https://other.example".into(),
            ..source.clone()
        },
        CloudPeerSource {
            account_id: "other".into(),
            ..source.clone()
        },
        CloudPeerSource {
            host_id: Uuid::new_v4().to_string(),
            ..source.clone()
        },
    ] {
        assert!(service
            .authenticate_rtc_peer(&binding.endpoint_id, &wrong, &binding.device_id, 1)
            .is_err());
    }
}
