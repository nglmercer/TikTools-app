use super::*;

/// Redeems an invitation into a join request. Redeeming approves nothing; the owner decides.
pub async fn redeem_invitation(
    auth: &CloudAuth,
    controller_device: &str,
    params: TeamRedeemParams,
) -> Result<TeamRedeemResult, OperationError> {
    let token = params.token.trim();
    if token.len() < 32
        || token.len() > 128
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return Err(OperationError::invalid("the invitation token is not valid"));
    }
    let (origin, bearer) = credentials(auth).await?;
    decode(
        send(
            Method::POST,
            &origin,
            &bearer,
            "/api/v3/remote/invitations/redeem",
            Some(json!({"token": token, "deviceId": controller_device})),
        )
        .await?,
    )
}

pub async fn invitations(
    auth: &CloudAuth,
    host_id: &str,
) -> Result<TeamInvitationList, OperationError> {
    valid_uuid(host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{host_id}/invitations"),
            None,
        )
        .await?,
    )
}

pub async fn create_invitation(
    auth: &CloudAuth,
    params: TeamInvitationCreateParams,
) -> Result<TeamInvitationCreated, OperationError> {
    valid_uuid(&params.host_id)?;
    one_of(&params.requested_role, PRESET_ROLES, "role")?;
    let mut body = json!({"requestedRole": params.requested_role});
    if let Some(ttl) = params.ttl_seconds {
        if !(60..=900).contains(&ttl) {
            return Err(OperationError::invalid(
                "invitations last between 1 and 15 minutes",
            ));
        }
        body["ttlSeconds"] = json!(ttl);
    }
    let (origin, token) = credentials(auth).await?;
    let created: TeamInvitationCreated = decode(
        send(
            Method::POST,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{}/invitations", params.host_id),
            Some(body),
        )
        .await?,
    )?;
    if created.token.len() < 32 {
        return Err(OperationError::internal(
            "cloud sent an unexpected invitation",
        ));
    }
    Ok(created)
}

pub async fn revoke_invitation(
    auth: &CloudAuth,
    params: TeamInvitationParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.invitation_id)?;
    let (origin, token) = credentials(auth).await?;
    send(
        Method::POST,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/invitations/{}/revoke",
            params.host_id, params.invitation_id
        ),
        None,
    )
    .await
}

pub async fn requests(auth: &CloudAuth, host_id: &str) -> Result<TeamRequestList, OperationError> {
    valid_uuid(host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{host_id}/requests"),
            None,
        )
        .await?,
    )
}

pub async fn decide_request(
    auth: &CloudAuth,
    params: TeamDecisionParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.host_id)?;
    valid_uuid(&params.request_id)?;
    one_of(&params.decision, &["approve", "deny"], "decision")?;
    let mut body = json!({"decision": params.decision});
    if let Some(role) = &params.role {
        one_of(role, PRESET_ROLES, "role")?;
        body["role"] = Value::String(role.clone());
    }
    let (origin, token) = credentials(auth).await?;
    send(
        Method::POST,
        &origin,
        &token,
        &format!(
            "/api/v3/remote/hosts/{}/requests/{}/decision",
            params.host_id, params.request_id
        ),
        Some(body),
    )
    .await
}
