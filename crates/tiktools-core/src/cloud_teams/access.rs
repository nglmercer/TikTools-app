use super::*;

pub async fn grants(auth: &CloudAuth, host_id: &str) -> Result<TeamGrantList, OperationError> {
    valid_uuid(host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{host_id}/grants"),
            None,
        )
        .await?,
    )
}

pub async fn update_grant(
    auth: &CloudAuth,
    params: TeamGrantUpdateParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.grant_id)?;
    let mut body = json!({"expectedRevision": params.expected_revision});
    if let Some(role) = &params.role {
        one_of(role, PRESET_ROLES, "role")?;
        body["role"] = Value::String(role.clone());
    }
    if let Some(policy) = params.policy {
        body["policy"] = policy;
    }
    let (origin, token) = credentials(auth).await?;
    send(
        Method::PATCH,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/grants/{}",
            params.host_id, params.grant_id
        ),
        Some(body),
    )
    .await
}

pub async fn revoke_grant(
    auth: &CloudAuth,
    params: TeamGrantRevokeParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.grant_id)?;
    let (origin, token) = credentials(auth).await?;
    send(
        Method::POST,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/grants/{}/revoke",
            params.host_id, params.grant_id
        ),
        Some(json!({"expectedRevision": params.expected_revision})),
    )
    .await
}

pub async fn sessions(auth: &CloudAuth, host_id: &str) -> Result<TeamSessionList, OperationError> {
    valid_uuid(host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{host_id}/sessions"),
            None,
        )
        .await?,
    )
}

pub async fn revoke_session(
    auth: &CloudAuth,
    params: TeamSessionParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.session_id)?;
    let (origin, token) = credentials(auth).await?;
    send(
        Method::POST,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/sessions/{}/revoke",
            params.host_id, params.session_id
        ),
        None,
    )
    .await
}
