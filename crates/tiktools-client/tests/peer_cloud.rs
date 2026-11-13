#![cfg(feature = "cloud-peer")]
//! Cross-repository acceptance against RemakeBackend's real ephemeral server.
#[path = "peer_cloud/rtc.rs"]
mod rtc;
#[path = "peer_cloud/support.rs"]
mod support;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use support::{Fixture, NoBearer};
use tiktools_client::{
    remote::{cloud_peer::CloudPeerController, profiles::ProfileManager, ClientKind},
    RequestMetadata,
};
use tiktools_collaboration::{Capability, Grant, ResourceKind};
use tiktools_core::cloud_teams::*;

#[tokio::test]
#[ignore = "run scripts/collaboration-cloud-lab.py with the sibling backend checkout"]
async fn cloud_onboarding_reaches_the_real_native_host() {
    tokio::time::timeout(Duration::from_secs(110), run(false))
        .await
        .expect("Cloud/native acceptance must finish");
}

#[tokio::test]
#[ignore = "run scripts/collaboration-cloud-lab.py with the sibling backend checkout"]
async fn cloud_onboarding_uses_relay_when_webrtc_unavailable() {
    tokio::time::timeout(Duration::from_secs(110), run(true))
        .await
        .expect("Cloud/native relay acceptance must finish");
}

async fn run(relay_only: bool) {
    let fixture = Fixture::load();
    let home = tempfile::tempdir().unwrap();
    let owner_auth = fixture.account(&home.path().join("owner-account"));
    let core = support::host(&home.path().join("host"), (*owner_auth).clone());
    let begin = core.cloud_begin_login().await.unwrap();
    let callback = fixture
        .browser_callback(&begin.browser_url, &fixture.owner_cookie)
        .await;
    let (login, _) = tokio::join!(
        core.cloud_complete_login(),
        support::deliver_callback(&callback)
    );
    login.unwrap();
    let host = core
        .teams_host_create(TeamCreateParams {
            name: "Native acceptance".into(),
        })
        .await
        .unwrap();
    core.teams_device_enroll(TeamEnrollParams {
        kind: "host".into(),
        label: "PC A".into(),
        host_id: Some(host.id.clone()),
    })
    .await
    .unwrap();
    let api = Arc::new(tiktools_control_api::ControlApi::new(core.clone()));
    let supervisor = tokio::spawn(tiktools_remote::supervise(api.clone()));
    let service = core.collaboration().unwrap();
    support::until(|| service.status().unwrap().peer_gateway.gateway.available).await;
    core.teams_host_start().await.unwrap();
    support::until(|| service.status().unwrap().peer_gateway.gateway.running).await;

    let moderator_data = home.path().join("moderator-account");
    let moderator_auth = fixture.account(&moderator_data);
    fixture
        .login(&moderator_auth, &fixture.moderator_cookie)
        .await;
    let profiles = ProfileManager::new(home.path().join("profiles"), Arc::new(NoBearer)).unwrap();
    let controller = CloudPeerController::new(
        moderator_auth,
        moderator_data,
        profiles.clone(),
        ClientKind::Desktop,
    )
    .unwrap();
    let device = controller.enroll("PC B".into()).await.unwrap();
    let invitation = core
        .teams_invitation_create(TeamInvitationCreateParams {
            host_id: host.id.clone(),
            requested_role: "moderator".into(),
            ttl_seconds: Some(300),
            recipient_id: None,
        })
        .await
        .unwrap();
    let request = controller.redeem(invitation.token).await.unwrap();
    assert_eq!(request.host_id, host.id);
    assert!(
        controller.connect("moderator", &host.id).await.is_err(),
        "redeeming must not approve a device"
    );
    core.teams_request_decide(TeamDecisionParams {
        host_id: host.id.clone(),
        request_id: request.request_id,
        decision: "approve".into(),
        role: Some("moderator".into()),
    })
    .await
    .unwrap();
    let grants = core.teams_grants(host.id.clone()).await.unwrap();
    let grant = grants
        .items
        .iter()
        .find(|g| g.device_id == device.id)
        .unwrap();
    let native = Grant {
        capabilities: [
            Capability::LiveRead,
            Capability::PointsRead,
            Capability::PointsAdjust,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(ResourceKind::Viewer, ["alice".into()].into())].into(),
        max_points_delta: Some(10.0),
        ..Grant::default()
    };
    core.teams_grant_update(TeamGrantUpdateParams {
        host_id: host.id.clone(),
        grant_id: grant.id.clone(),
        role: Some("custom".into()),
        policy: Some(json!({"nativeGrant":native,"nativeClientKind":"desktop"})),
        expected_revision: grant.revision,
    })
    .await
    .unwrap();
    let profile = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            match controller.connect("moderator", &host.id).await {
                Ok(profile) => break profile,
                Err(error)
                    if matches!(
                        error.code.as_str(),
                        "approval_pending"
                            | "unauthorized"
                            | "transport"
                            | "unavailable"
                            | "rate_limited"
                    ) =>
                {
                    eprintln!("native admission retry: {}", error.code);
                    tokio::time::sleep(Duration::from_secs(2)).await
                }
                Err(error) => panic!("native connection failed: {}", error.code),
            }
        }
    })
    .await
    .expect("owner-approved grant must be confirmed by the native host");
    assert_eq!(profile.peer.as_ref().unwrap().cloud_host_id, host.id);
    assert_eq!(profile.host_id, service.host_id());
    assert_eq!(controller.teams().await.unwrap().len(), 1);
    let remote = profiles.open("moderator").await.unwrap();
    assert_eq!(remote.hello().principal_id, profile.principal_id);
    remote
        .call_value_with_meta("live.status", json!({}), None)
        .await
        .unwrap();
    let meta = RequestMetadata {
        command_id: Some("cloud-once".into()),
        host_instance_id: Some(remote.hello().host_instance_id.clone()),
        ..RequestMetadata::default()
    };
    let first = remote
        .call_value_with_meta(
            "points.adjust",
            json!({"uniqueId":"alice","delta":3}),
            Some(meta.clone()),
        )
        .await
        .unwrap();
    let retry = remote
        .call_value_with_meta(
            "points.adjust",
            json!({"uniqueId":"alice","delta":3}),
            Some(meta),
        )
        .await
        .unwrap();
    assert_eq!(first, retry);
    assert_eq!(core.points_viewer("alice").unwrap()["points"], 3.0);
    let receipt = remote
        .call_value_with_meta(
            "collaboration.commands.get",
            json!({"commandId":"cloud-once"}),
            None,
        )
        .await
        .unwrap();
    assert_eq!(receipt.result["result"]["totalPoints"], 3.0);
    let denied = remote
        .call_value_with_meta("points.viewer.get", json!({"uniqueId":"bob"}), None)
        .await
        .unwrap_err();
    assert_eq!(denied.code, "forbidden");
    assert_eq!(profile.version, 3);
    assert!(remote.uses_rtc());
    remote.close();
    // A version-two fixture has no stored IP or SDP dependency. Upgrade only
    // after admission to the same pinned key and stable RPC identity.
    let path = profiles.directory().join("moderator.json");
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    legacy["version"] = json!(2);
    legacy["peer"]
        .as_object_mut()
        .unwrap()
        .remove("hostDeviceId");
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let mut remote = profiles.open("moderator").await.unwrap();
    assert!(remote.uses_rtc());
    let migrated = profiles.get("moderator").await.unwrap();
    assert_eq!(migrated.version, 3);
    assert_eq!(migrated.profile_id, profile.profile_id);
    assert_eq!(migrated.principal_id, profile.principal_id);
    assert_eq!(migrated.session_id, profile.session_id);
    assert_eq!(
        migrated.peer.as_ref().unwrap().host_device_id,
        profile.peer.as_ref().unwrap().host_device_id
    );
    let old_receipt = remote
        .call_value_with_meta(
            "collaboration.commands.get",
            json!({"commandId":"cloud-once"}),
            None,
        )
        .await
        .unwrap();
    assert_eq!(old_receipt.result["result"]["totalPoints"], 3.0);
    let unsupported_grant = rtc::unsupported_device(
        &core,
        controller.account(),
        &home.path().join("future-policy"),
        &host.id,
    )
    .await;
    let meta = RequestMetadata {
        command_id: Some("rtc-managed-once".into()),
        host_instance_id: Some(remote.hello().host_instance_id.clone()),
        ..RequestMetadata::default()
    };
    let first = remote
        .call_value_with_meta(
            "points.adjust",
            json!({"uniqueId":"alice","delta":2}),
            Some(meta.clone()),
        )
        .await
        .unwrap();
    let retry = remote
        .call_value_with_meta(
            "points.adjust",
            json!({"uniqueId":"alice","delta":2}),
            Some(meta),
        )
        .await
        .unwrap();
    assert_eq!(first, retry);
    assert_eq!(core.points_viewer("alice").unwrap()["points"], 5.0);
    assert_eq!(
        remote
            .call_value_with_meta("points.viewer.get", json!({"uniqueId":"bob"}), None)
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    assert_eq!(
        core.teams_grants(host.id.clone())
            .await
            .unwrap()
            .items
            .iter()
            .find(|g| g.id == unsupported_grant)
            .unwrap()
            .status,
        "pendingHost"
    );
    if !relay_only {
        fixture.set_cloud_online(false).await;
        support::cloud_available(&fixture.origin, false).await;
        assert!(
            reqwest::Client::new()
                .get(format!("{}/api/v2/cloud", fixture.origin))
                .timeout(Duration::from_secs(1))
                .send()
                .await
                .is_err(),
            "Cloud must actually be unreachable"
        );
    }
    let offline_meta = RequestMetadata {
        command_id: Some("rtc-offline-once".into()),
        host_instance_id: Some(remote.hello().host_instance_id.clone()),
        ..RequestMetadata::default()
    };
    let first = remote
        .call_value_with_meta(
            "points.adjust",
            json!({"uniqueId":"alice","delta":1}),
            Some(offline_meta.clone()),
        )
        .await
        .unwrap();
    let retry = remote
        .call_value_with_meta(
            "points.adjust",
            json!({"uniqueId":"alice","delta":1}),
            Some(offline_meta),
        )
        .await
        .unwrap();
    assert_eq!(first, retry);
    if relay_only {
        let coordinator = tiktools_client::remote::controller::Controller::new(profiles.clone());
        coordinator.connect("moderator").await.unwrap();
        let before = coordinator.status();
        assert_eq!(before.phase, "ready");
        assert_eq!(before.transport_route.as_deref(), Some("relay"));
        let before_hello = before.hello.unwrap();
        assert_eq!(remote.peer_route(), Some(tiktools_peer::Route::Relay));
        let currency = coordinator
            .points(&before.epoch, &before.points_revision)
            .unwrap()["currencyName"]
            .as_str()
            .unwrap()
            .to_owned();
        fixture.hold_controller_responses().await;
        let change =
            coordinator.points_command(tiktools_client::remote::controller::PointsCommand {
                connection_id: Some(before_hello.connection_id.clone()),
                epoch: before.epoch.clone(),
                unique_id: "alice".into(),
                delta: 1.0,
                currency_name: currency,
            });
        let lose_reply = async {
            // The owner observes a durable success, while the native caller
            // cannot receive any controller-directed WebSocket bytes.
            support::until(|| {
                let Some(intent) = coordinator.status().command_journal.and_then(|j| j.pending)
                else {
                    return false;
                };
                core.collaboration_command_get(
                    tiktools_core::control::CommandCaller::LocalOwner,
                    tiktools_collaboration::CommandGet {
                        command_id: intent.command_id,
                        principal_id: Some(before_hello.principal_id.clone()),
                    },
                )
                .is_ok_and(|outcome| {
                    outcome.command.state == tiktools_collaboration::CommandState::Succeeded
                })
            })
            .await;
            let pending = coordinator
                .status()
                .command_journal
                .unwrap()
                .pending
                .unwrap();
            assert_eq!(core.points_viewer("alice").unwrap()["points"], 7.0);
            fixture.set_cloud_online(false).await;
            pending.command_id
        };
        let (answer, lost_id) = tokio::join!(change, lose_reply);
        assert!(answer.is_err(), "the committed reply must actually be lost");
        assert_eq!(
            coordinator
                .status()
                .command_journal
                .unwrap()
                .pending
                .unwrap()
                .command_id,
            lost_id
        );
        support::cloud_available(&fixture.origin, false).await;
        support::until(|| remote.is_closed()).await;
        support::until(|| coordinator.status().phase != "ready").await;
        assert!(remote
            .call_value_with_meta("live.status", json!({}), None)
            .await
            .is_err());
        fixture.set_cloud_online(true).await;
        support::cloud_available(&fixture.origin, true).await;
        support::until(|| matches!(coordinator.status().phase.as_str(), "ready" | "failed")).await;
        let after = coordinator.status();
        assert_eq!(
            after.phase, "ready",
            "native Desktop coordinator must recover: {:?}",
            after.error_code
        );
        assert_eq!(after.transport_route.as_deref(), Some("relay"));
        let after_hello = after.hello.unwrap();
        assert_ne!(after_hello.connection_id, before_hello.connection_id);
        assert_eq!(after_hello.host_id, before_hello.host_id);
        assert_eq!(after_hello.principal_id, before_hello.principal_id);
        assert_eq!(after_hello.session_id, before_hello.session_id);
        support::until(|| {
            coordinator.status().command_journal.is_some_and(|journal| {
                journal.pending.is_none()
                    && journal.recent.iter().any(|record| {
                        record.intent.command_id == lost_id
                            && record.outcome
                            == tiktools_client::remote::controller::LocalCommandOutcome::Succeeded
                            && record
                                .points_result
                                .as_ref()
                                .is_some_and(|points| points.total_points == 7.0)
                    })
            })
        })
        .await;
        assert_eq!(core.points_viewer("alice").unwrap()["points"], 7.0);
        coordinator.disconnect().await;
        remote = support::reopen(&profiles, "moderator").await;
        assert_eq!(remote.peer_route(), Some(tiktools_peer::Route::Relay));
    } else {
        for _ in 0..11 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let read = remote
                .call_value_with_meta("points.viewer.get", json!({"uniqueId":"alice"}), None)
                .await
                .unwrap();
            assert_eq!(read.result["viewer"]["points"], 6.0);
        }
        assert!(!remote.is_closed());
        assert_eq!(remote.peer_route(), Some(tiktools_peer::Route::Direct));
    }
    let receipt = remote
        .call_value_with_meta(
            "collaboration.commands.get",
            json!({"commandId":"rtc-offline-once"}),
            None,
        )
        .await
        .unwrap();
    assert_eq!(receipt.result["result"]["totalPoints"], 6.0);
    fixture.set_cloud_online(true).await;
    support::cloud_available(&fixture.origin, true).await;
    let reopened = support::reopen(&profiles, "moderator").await;
    assert!(reopened.uses_rtc());
    reopened.close();
    let current = core.teams_grants(host.id.clone()).await.unwrap();
    let grant = current
        .items
        .iter()
        .find(|g| g.device_id == device.id)
        .unwrap();
    core.teams_grant_revoke(TeamGrantRevokeParams {
        host_id: host.id,
        grant_id: grant.id.clone(),
        expected_revision: grant.revision,
    })
    .await
    .unwrap();
    support::until(|| remote.is_closed()).await;
    assert!(profiles.open("moderator").await.is_err());
    remote.close();
    owner_auth.logout().await.unwrap();
    support::until(|| !service.peer_gateway_status().unwrap().gateway.running).await;
    core.shutdown().await;
    tokio::time::timeout(Duration::from_secs(5), supervisor)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !home.path().join("profiles/data").exists(),
        "controller must not construct another host"
    );
}
