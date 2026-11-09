use super::*;
use tiktools_collaboration::{Grant, GrantSet};

#[tokio::test]
async fn native_controller_reads_only_scoped_competition_status_and_recovers_after_grant_change() {
    let f = Fixture::new().await;
    f.core
        .ranking_create(
            serde_json::from_value(json!({"schemaVersion":1,"id":"private-ranking",
        "creatorId":"owner","roomId":"private-room","limit":10,"capacity":10,
        "sources":[{"source":"custom.score","metric":"value","weight":1,"enabled":true}]}))
            .unwrap(),
        )
        .unwrap();
    for id in ["alpha", "bravo", "charlie"] {
        f.core.competition_create(serde_json::from_value(json!({"schemaVersion":1,"id":id,"name":format!("Competition {id}"),
            "kind":"leaderboard_challenge","rankingId":"private-ranking","options":[],"selectionMode":"highest_score",
            "tiePolicy":"stable","durationMs":null,"voteLimit":1,"allowSwitch":false,"minimumParticipants":1,
            "onUserWin":{"actionId":"private-action","inputs":{"secret":"do-not-send"}}})).unwrap()).unwrap();
    }
    let mut grant = Grant {
        capabilities: [
            Capability::CompetitionsRead,
            Capability::CompetitionsManage,
            Capability::CommandsRead,
        ]
        .into(),
        resources: [(
            ResourceKind::Competition,
            ["alpha".into(), "charlie".into()].into(),
        )]
        .into(),
        ..Grant::default()
    };
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = manager
        .install(
            "moderator",
            &f.endpoint,
            f.pair(grant.clone()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    let controller = Controller::new(manager);
    controller.connect("moderator").await.unwrap();
    wait(&controller, |s| s.phase == "ready").await;
    let initial = controller.status();
    let hello = initial.hello.unwrap();
    let request = json!({"epoch":initial.epoch,"connectionId":hello.connection_id,"after":null});
    let page = controller
        .manage("controller.competitions", request.clone())
        .await
        .unwrap();
    let rows = page["competitions"].as_array().unwrap();
    assert!(rows.iter().all(|row| row["revision"] == "1"));
    assert_eq!(
        rows.iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["alpha", "charlie"]
    );
    for secret in [
        "bravo",
        "private-ranking",
        "private-room",
        "private-action",
        "do-not-send",
    ] {
        assert!(!page.to_string().contains(secret));
    }
    let previous_revision = controller.status().competition_revision;
    let mut updated = f.core.competition_get("charlie").unwrap();
    updated.name = "Updated by owner".into();
    f.core.competition_update(updated).unwrap();
    wait(&controller, |s| {
        s.phase == "ready" && s.competition_revision != previous_revision
    })
    .await;
    let refreshed = controller
        .manage("controller.competitions", request.clone())
        .await
        .unwrap();
    assert_eq!(refreshed["competitions"][1]["name"], "Updated by owner");
    let command = json!({"epoch":initial.epoch,"connectionId":hello.connection_id,"hostInstanceId":hello.host_instance_id,"grantRevision":hello.grant_revision,"id":"alpha","expectedRevision":"1","expectedGeneration":"0","operation":"cancel-competition"});
    let result = controller
        .manage("controller.competition.command", command)
        .await
        .unwrap();
    assert!(result["pending"].is_null());
    assert_eq!(result["recent"][0]["outcome"], "succeeded");
    assert_eq!(result["recent"][0]["intent"]["expectedGeneration"], "0");
    assert_eq!(
        f.core.competition_state("alpha").unwrap().lifecycle,
        tiktools_control_api::modules::competitions::Lifecycle::Cancelled
    );
    assert_eq!(
        controller
            .forward(
                &initial.epoch,
                "competitions.start",
                json!({"id":"alpha"}),
                None
            )
            .await
            .unwrap_err()
            .code,
        "forbidden"
    );
    grant
        .resources
        .insert(ResourceKind::Competition, ["charlie".into()].into());
    f.service
        .set_grant(GrantSet {
            principal_id: profile.principal_id,
            expected_revision: "1".into(),
            grant,
        })
        .unwrap();
    wait(&controller, |s| {
        s.phase == "ready" && s.hello.as_ref().is_some_and(|h| h.grant_revision == "2")
    })
    .await;
    assert!(controller
        .manage("controller.competitions", request)
        .await
        .is_err());
    let next = controller.status();
    let page = controller.manage("controller.competitions", json!({"epoch":next.epoch,"connectionId":next.hello.unwrap().connection_id,"after":null})).await.unwrap();
    assert_eq!(page["competitions"].as_array().unwrap().len(), 1);
    assert_eq!(page["competitions"][0]["id"], "charlie");
    controller.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn competition_only_controller_remains_usable_with_minimum_request_budget() {
    let f = Fixture::configured(
        |_| {},
        |configuration| {
            configuration.limits.requests_per_second = 1;
            configuration.limits.request_burst = 1;
        },
    )
    .await;
    f.core
        .ranking_create(
            serde_json::from_value(json!({"schemaVersion":1,"id":"rank",
        "creatorId":"owner","roomId":"room","limit":10,"capacity":10,
        "sources":[{"source":"custom.score","metric":"value","weight":1,"enabled":true}]}))
            .unwrap(),
        )
        .unwrap();
    f.core.competition_create(serde_json::from_value(json!({"schemaVersion":1,"id":"alpha","name":"Competition",
        "kind":"leaderboard_challenge","rankingId":"rank","options":[],"selectionMode":"highest_score",
        "tiePolicy":"stable","durationMs":null,"voteLimit":1,"allowSwitch":false,"minimumParticipants":1})).unwrap()).unwrap();
    let grant = Grant {
        capabilities: [Capability::CompetitionsRead].into(),
        resources: [(ResourceKind::Competition, ["alpha".into()].into())].into(),
        ..Grant::default()
    };
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = manager
        .install(
            "limited",
            &f.endpoint,
            f.pair(grant.clone()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    let controller = Controller::new(manager);
    controller.connect("limited").await.unwrap();
    wait(&controller, |s| s.phase == "ready").await;
    let first = controller.status();
    let connection = first.hello.as_ref().unwrap().connection_id.clone();
    let mut successes = 0;
    tokio::time::timeout(Duration::from_secs(16), async {
        while successes < 4 {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let status = controller.status();
            assert_eq!(
                status.phase, "ready",
                "rate limiting must not force reconnection"
            );
            assert_eq!(status.hello.as_ref().unwrap().connection_id, connection);
            let result = controller
                .manage(
                    "controller.competitions",
                    json!({"epoch":status.epoch,"connectionId":connection,"after":null}),
                )
                .await;
            match result {
                Ok(page) => {
                    assert_eq!(page["competitions"][0]["id"], "alpha");
                    successes += 1;
                }
                Err(error) if error.code == "rate_limited" => {}
                Err(error) => panic!("unexpected failure: {error:?}"),
            }
        }
    })
    .await
    .expect("low-rate controller must keep making progress");
    f.service
        .revoke_session(&profile.session_id, "local-owner")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while controller.status().phase == "ready" {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("revoked controller must leave ready");
    assert!(controller
        .manage(
            "controller.competitions",
            json!({"epoch":first.epoch,"connectionId":connection,"after":null})
        )
        .await
        .is_err());
    controller.disconnect().await;
    f.finish().await;
}

#[tokio::test]
async fn combined_grants_controller_recovers_with_minimum_request_budget() {
    let f = Fixture::configured(
        |_| {},
        |configuration| {
            configuration.limits.requests_per_second = 1;
            configuration.limits.request_burst = 1;
        },
    )
    .await;
    f.core
        .ranking_create(
            serde_json::from_value(json!({"schemaVersion":1,"id":"rank",
        "creatorId":"owner","roomId":"room","limit":10,"capacity":10,
        "sources":[{"source":"custom.score","metric":"value","weight":1,"enabled":true}]}))
            .unwrap(),
        )
        .unwrap();
    f.core.competition_create(serde_json::from_value(json!({"schemaVersion":1,"id":"alpha","name":"Competition",
        "kind":"leaderboard_challenge","rankingId":"rank","options":[],"selectionMode":"highest_score",
        "tiePolicy":"stable","durationMs":null,"voteLimit":1,"allowSwitch":false,"minimumParticipants":1})).unwrap()).unwrap();
    let grant = Grant {
        capabilities: [
            Capability::CompetitionsRead,
            Capability::WorkflowsRead,
            Capability::PointsRead,
            Capability::LiveRead,
        ]
        .into(),
        resources: [(ResourceKind::Competition, ["alpha".into()].into())].into(),
        ..Grant::default()
    };
    let home = tempfile::tempdir().unwrap();
    let manager =
        ProfileManager::new(home.path().join("profiles"), Arc::new(Store::default())).unwrap();
    let profile = manager
        .install(
            "limited",
            &f.endpoint,
            f.pair(grant.clone()).await,
            ClientKind::Agent,
        )
        .await
        .unwrap();
    let controller = Controller::new(manager);
    controller.connect("limited").await.unwrap();
    wait(&controller, |s| s.phase == "ready").await;
    let first = controller.status();
    let connection = first.hello.as_ref().unwrap().connection_id.clone();
    let mut successes = 0;
    tokio::time::timeout(Duration::from_secs(25), async {
        while successes < 4 {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            let status = controller.status();
            assert_eq!(
                status.phase, "ready",
                "rate limiting must not force reconnection"
            );
            assert_eq!(status.hello.as_ref().unwrap().connection_id, connection);
            let result = controller
                .manage(
                    "controller.competitions",
                    json!({"epoch":status.epoch,"connectionId":connection,"after":null}),
                )
                .await;
            match result {
                Ok(page) => {
                    assert_eq!(page["competitions"][0]["id"], "alpha");
                    successes += 1;
                }
                Err(error) if error.code == "rate_limited" => {}
                Err(error) => panic!("unexpected failure: {error:?}"),
            }
        }
    })
    .await
    .expect("low-rate controller must keep making progress");
    f.service
        .revoke_session(&profile.session_id, "local-owner")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while controller.status().phase == "ready" {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("revoked controller must leave ready");
    assert!(controller
        .manage(
            "controller.competitions",
            json!({"epoch":first.epoch,"connectionId":connection,"after":null})
        )
        .await
        .is_err());
    controller.disconnect().await;
    f.finish().await;
}
