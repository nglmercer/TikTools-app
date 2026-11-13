//! Unsupported grants must neither be confirmed nor block valid devices.
use std::path::Path;
use tiktools_cloud_auth::CloudAuth;
pub async fn unsupported_device(
    core: &tiktools_core::AppCore,
    auth: &CloudAuth,
    data: &Path,
    host: &str,
) -> String {
    use tiktools_core::cloud_teams::*;
    let teams = CloudTeams::from_data_directory(data);
    let device = teams
        .enroll(
            auth,
            TeamEnrollParams {
                kind: "controller".into(),
                label: "Future policy device".into(),
                host_id: None,
            },
        )
        .await
        .unwrap();
    let invitation = core
        .teams_invitation_create(TeamInvitationCreateParams {
            host_id: host.into(),
            requested_role: "moderator".into(),
            ttl_seconds: Some(300),
        })
        .await
        .unwrap();
    let request = redeem_invitation(
        auth,
        &device.id,
        TeamRedeemParams {
            token: invitation.token,
        },
    )
    .await
    .unwrap();
    core.teams_request_decide(TeamDecisionParams {
        host_id: host.into(),
        request_id: request.request_id,
        decision: "approve".into(),
        role: Some("moderator".into()),
    })
    .await
    .unwrap();
    let grants = core.teams_grants(host.into()).await.unwrap();
    let grant = grants
        .items
        .iter()
        .find(|g| g.device_id == device.id)
        .unwrap();
    core.teams_grant_update(TeamGrantUpdateParams {host_id:host.into(),grant_id:grant.id.clone(),role:Some("custom".into()),policy:Some(serde_json::json!({"nativeGrant":{"capabilities":["future.permission"]},"nativeClientKind":"desktop"})),expected_revision:grant.revision}).await.unwrap();
    grant.id.clone()
}
