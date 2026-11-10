use super::*;

#[tokio::test]
async fn epoch_barrier_and_fenced_writes_preserve_identity_and_decimal_contract() {
    let (session, server) = native_session(vec![
        Reply::json(json!({"writeEpoch":"9223372036854775807", "epochRequired":true})),
        Reply::json(json!({"writeEpoch":"12"})),
        Reply::json(record(1, Some(preference("dark")))),
        Reply::json(record(2, None)),
        Reply::status(412),
        Reply::json(json!({"writeEpoch":"13", "epochRequired":true})),
        Reply::json(json!({"writeEpoch":"13", "epochRequired":false})),
    ])
    .await;
    let epoch = session.workspace_write_epoch().await.unwrap();
    assert_eq!(epoch.decimal(), "9223372036854775807");
    assert_eq!(epoch.epoch_required(), Some(true));
    let saved = serde_json::to_value(&epoch).unwrap();
    assert_eq!(saved["origin"], session.instance_origin());
    assert_eq!(saved["accountId"], "account-a");
    assert_eq!(saved["writeEpoch"], "9223372036854775807");
    assert_eq!(saved["epochRequired"], true);
    let restored: WorkspaceWriteEpoch = serde_json::from_value(saved.clone()).unwrap();
    assert_eq!(epoch, restored);
    let barrier = session.workspace_write_barrier().await.unwrap();
    assert_eq!(barrier.decimal(), "12");
    assert_eq!(barrier.epoch_required(), None);
    assert!(restored.is_after(&barrier).unwrap());
    assert!(!barrier.is_after(&restored).unwrap());
    assert!(!barrier.is_after(&barrier).unwrap());
    let put = session
        .workspace_put_fenced(
            "preferences",
            "desktop",
            preference("dark"),
            None,
            &restored,
        )
        .await
        .unwrap();
    assert_eq!(put.revision, 1);
    let deleted = session
        .workspace_delete_fenced("preferences", "desktop", 1, &barrier)
        .await
        .unwrap();
    assert_eq!(deleted.revision, 2);
    assert!(matches!(
        session
            .workspace_delete_fenced("preferences", "desktop", 2, &barrier)
            .await,
        Err(SyncError::Http(412))
    ));
    let enforced = session.workspace_require_write_epoch().await.unwrap();
    assert_eq!(enforced.epoch_required(), Some(true));
    assert!(enforced.is_after(&barrier).unwrap());
    assert!(matches!(
        session.workspace_require_write_epoch().await,
        Err(SyncError::Protocol)
    ));

    // An epoch restored for another account or instance must not send HTTP.
    let mut wrong_account = saved.clone();
    wrong_account["accountId"] = json!("account-b");
    let wrong_account: WorkspaceWriteEpoch = serde_json::from_value(wrong_account).unwrap();
    assert!(matches!(
        barrier.is_after(&wrong_account),
        Err(SyncError::WrongAccount)
    ));
    assert!(matches!(
        session
            .workspace_put_fenced(
                "preferences",
                "desktop",
                preference("dark"),
                None,
                &wrong_account
            )
            .await,
        Err(SyncError::WrongAccount)
    ));
    let mut wrong_origin = saved.clone();
    wrong_origin["origin"] = json!("https://wrong.invalid");
    let wrong_origin: WorkspaceWriteEpoch = serde_json::from_value(wrong_origin).unwrap();
    assert!(matches!(
        session
            .workspace_delete_fenced("preferences", "desktop", 1, &wrong_origin)
            .await,
        Err(SyncError::WrongAccount)
    ));
    for invalid in [
        json!(0),
        json!("-1"),
        json!("00"),
        json!("+1"),
        json!("9223372036854775808"),
    ] {
        let mut encoded = saved.clone();
        encoded["writeEpoch"] = invalid;
        assert!(serde_json::from_value::<WorkspaceWriteEpoch>(encoded).is_err());
    }
    let requests = server.await.unwrap();
    native(&requests);
    assert_eq!(requests[1].method, "GET");
    assert_eq!(requests[1].target, "/api/v2/sync/write-epoch");
    assert_eq!(requests[2].method, "POST");
    assert_eq!(requests[2].target, "/api/v2/sync/write-barrier");
    assert!(requests[2].body.is_empty());
    assert_eq!(requests[3].method, "PUT");
    assert_eq!(
        requests[3]
            .headers
            .get("x-workspace-write-epoch")
            .map(String::as_str),
        Some("9223372036854775807")
    );
    assert_eq!(
        requests[3].headers.get("if-none-match").map(String::as_str),
        Some("*")
    );
    assert_eq!(requests[4].method, "DELETE");
    assert_eq!(
        requests[4]
            .headers
            .get("x-workspace-write-epoch")
            .map(String::as_str),
        Some("12")
    );
    assert_eq!(
        requests[4].headers.get("if-match").map(String::as_str),
        Some("\"1\"")
    );
    assert_eq!(requests[6].method, "POST");
    assert_eq!(requests[6].target, "/api/v2/sync/require-write-epoch");
}

#[tokio::test]
async fn epoch_responses_reject_malformed_or_ambiguous_wire_values() {
    let (session, server) = native_session(vec![
        Reply::json(json!({"writeEpoch": 0})),
        Reply::json(json!({"writeEpoch":"-1"})),
        Reply::json(json!({"writeEpoch":"00"})),
        Reply::json(json!({"writeEpoch":"9223372036854775808"})),
        Reply::json(json!({"writeEpoch":"0","other":true})),
        Reply::json(json!({"writeEpoch":"0","epochRequired":"false"})),
        Reply::status(201),
        Reply::status(503),
        Reply::json(json!({"writeEpoch":"0"})),
    ])
    .await;
    for _ in 0..7 {
        assert!(matches!(
            session.workspace_write_epoch().await,
            Err(SyncError::Protocol)
        ));
    }
    assert!(matches!(
        session.workspace_write_barrier().await,
        Err(SyncError::Http(503))
    ));
    assert_eq!(
        session.workspace_write_epoch().await.unwrap().decimal(),
        "0"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn epoch_capability_is_optional_and_validates_fixed_contract() {
    let old = workspace_capability_document();
    let mut supported = old.clone();
    supported["sync"]["writeEpochBarrier"] = json!({
        "epochPath":"/api/v2/sync/write-epoch", "barrierPath":"/api/v2/sync/write-barrier",
        "header":"X-Workspace-Write-Epoch", "staleStatus":412
    });
    let mut enforcement = supported.clone();
    enforcement["sync"]["writeEpochBarrier"]["requireEpochPath"] =
        json!("/api/v2/sync/require-write-epoch");
    enforcement["sync"]["writeEpochBarrier"]["missingEpochStatus"] = json!(428);
    let mut partial = supported.clone();
    partial["sync"]["writeEpochBarrier"]["requireEpochPath"] =
        json!("/api/v2/sync/require-write-epoch");
    let mut wrong = supported.clone();
    wrong["sync"]["writeEpochBarrier"]["barrierPath"] = json!("https://wrong.invalid/barrier");
    let (origin, server) = mock(vec![
        Reply::json(old),
        Reply::json(supported),
        Reply::json(enforcement),
        Reply::json(partial),
        Reply::json(wrong),
    ])
    .await;
    let catalog = CloudCatalog::new(&origin).unwrap();
    assert!(
        !catalog
            .workspace_sync_capabilities()
            .await
            .unwrap()
            .write_epoch_barrier
    );
    let capability = catalog.workspace_sync_capabilities().await.unwrap();
    assert!(capability.write_epoch_barrier);
    assert!(!capability.write_epoch_enforcement);
    let capability = catalog.workspace_sync_capabilities().await.unwrap();
    assert!(capability.write_epoch_barrier && capability.write_epoch_enforcement);
    assert!(matches!(
        catalog.workspace_sync_capabilities().await,
        Err(SyncError::Protocol)
    ));
    assert!(matches!(
        catalog.workspace_sync_capabilities().await,
        Err(SyncError::Protocol)
    ));
    for request in server.await.unwrap() {
        assert!(!request.headers.contains_key("authorization"));
        assert_eq!(request.target, "/api/v2/capabilities");
    }
}
