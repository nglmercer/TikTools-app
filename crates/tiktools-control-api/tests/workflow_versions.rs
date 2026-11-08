#![cfg(feature = "persistence")]

use serde_json::json;
use std::sync::Arc;
use tiktools_control_api::{ControlApi, RemoteCapability, RemoteSession, RequestContext};
use tiktools_core::{ipc::messages::HostMessage, AppCore, HostEmitter};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

#[tokio::test]
async fn workflow_envelopes_preserve_shapes_enforce_preconditions_and_reject_unsupported_metadata()
{
    let home = std::env::temp_dir().join(format!("tt-version-api-{}", std::process::id()));
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var("TIKTOOLS_BUILTIN_PLUGINS_DIR", home.join("builtins"));
    std::env::set_var("TIKTOOLS_DEV_PLUGINS_DIR", home.join("development"));
    std::env::set_var("TIKTOOLS_PLUGIN_DATA_DIR", home.join("plugin-data"));
    let core = Arc::new(AppCore::new(Arc::new(Emitter)));
    let api = ControlApi::new(Arc::clone(&core));
    let graph = json!({"id":"welcome","name":"Welcome","enabled":false,"nodes":[],"edges":[]});
    let created = api.execute_value(&json!({"id":1,"method":"workflows.save","params":{"graph":graph},"meta":{"expectedRevision":"0"}})).await;
    assert_eq!(created.result.as_ref().unwrap()["id"], "welcome");
    assert!(created
        .result
        .as_ref()
        .unwrap()
        .get("resourceVersions")
        .is_none());
    assert_eq!(created.meta.unwrap().resource_versions[0].revision, "1");
    // Legacy local writers also invalidate a controller's previously read revision.
    core.workflow_set_enabled("welcome", true).unwrap();
    let stale = api.execute_value(&json!({"id":2,"method":"workflows.save","params":{"graph":graph},"meta":{"expectedRevision":"1"}})).await.error.unwrap();
    assert_eq!(stale.code, "revision_conflict");
    assert_eq!(stale.data.unwrap()["resourceVersions"][0]["revision"], "2");
    let listed = api
        .execute_value(&json!({"id":3,"method":"workflows.list"}))
        .await;
    assert_eq!(listed.result.unwrap()["workflows"][0]["enabled"], true);
    assert_eq!(listed.meta.unwrap().resource_versions[0].revision, "2");
    let deleted = api.execute_value(&json!({"id":4,"method":"workflows.delete","params":{"id":"welcome"},"meta":{"expectedRevision":"2"}})).await;
    assert_eq!(
        deleted.result.unwrap(),
        json!({"id":"welcome","deleted":true})
    );
    assert_eq!(deleted.meta.unwrap().resource_versions[0].revision, "3");
    let missing = api
        .execute_value(&json!({"id":5,"method":"workflows.get","params":{"id":"welcome"}}))
        .await
        .error
        .unwrap();
    assert_eq!(missing.code, "workflow_not_found");
    assert_eq!(
        missing.data.unwrap()["resourceVersions"][0]["revision"],
        "3"
    );
    assert_eq!(api.execute_value(&json!({"id":6,"method":"workflows.save","params":{"graph":graph},"meta":{"expectedRevision":"0"}})).await.error.unwrap().code,"revision_conflict");
    for meta in [
        json!({"commandId":"pending-ledger"}),
        json!({"hostInstanceId":"pending-fencing"}),
    ] {
        assert_eq!(
            api.execute_value(
                &json!({"id":7,"method":"workflows.save","params":{"graph":graph},"meta":meta})
            )
            .await
            .error
            .unwrap()
            .code,
            "invalid_params"
        );
    }
    assert!(core
        .workflow_get_versioned("welcome")
        .unwrap()
        .value
        .is_none());
    assert_eq!(
        api.execute_value(
            &json!({"id":8,"method":"system.shutdown","meta":{"expectedRevision":"0"}})
        )
        .await
        .error
        .unwrap()
        .code,
        "invalid_params"
    );
    assert!(!core.is_shutdown());
    assert_eq!(api.execute_value(&json!({"id":9,"method":"workflows.save","params":{"graph":graph},"meta":{"role":"owner"}})).await.error.unwrap().code,"invalid_params");
    let context = RequestContext::Remote(
        RemoteSession::new(
            "moderator",
            "session",
            [
                RemoteCapability::WorkflowsEdit,
                RemoteCapability::WorkflowsRead,
            ],
            std::time::Duration::from_secs(60),
        )
        .unwrap(),
    );
    for method in ["workflows.save", "nonexistent"] {
        assert_eq!(
            api.execute_value_with_context(
                &context,
                &json!({"id":10,"method":method,"meta":"malformed","params":{"graph":graph}})
            )
            .await
            .error
            .unwrap()
            .code,
            "forbidden"
        );
    }
    let discovery = api
        .execute_value(&json!({"id":11,"method":"rpc.schema","params":{"method":"workflows.save"}}))
        .await
        .result
        .unwrap();
    assert_eq!(discovery["concurrency"], "compare-and-set");
    core.shutdown().await;
    drop(api);
    drop(core);
    std::fs::remove_dir_all(home).unwrap();
}
