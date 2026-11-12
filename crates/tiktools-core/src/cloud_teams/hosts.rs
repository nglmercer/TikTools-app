use super::*;

pub async fn hosts(auth: &CloudAuth) -> Result<TeamHostList, OperationError> {
    let (origin, token) = credentials(auth).await?;
    decode(send(Method::GET, &origin, &token, "/api/v3/remote/hosts", None).await?)
}

pub async fn create_host(
    auth: &CloudAuth,
    params: TeamCreateParams,
) -> Result<TeamHost, OperationError> {
    let name = bounded_text(&params.name, MAX_NAME_CHARS, "team name")?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/hosts",
            Some(json!({"name": name, "protocolVersion": PROTOCOL_VERSION})),
        )
        .await?,
    )
}

pub async fn get_host(
    auth: &CloudAuth,
    params: TeamHostParams,
) -> Result<TeamHost, OperationError> {
    valid_uuid(&params.host_id)?;
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::GET,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{}", params.host_id),
            None,
        )
        .await?,
    )
}

pub async fn update_host(
    auth: &CloudAuth,
    params: TeamUpdateParams,
) -> Result<TeamHost, OperationError> {
    valid_uuid(&params.host_id)?;
    let mut body = json!({"expectedRevision": params.expected_revision});
    if let Some(name) = &params.name {
        body["name"] = Value::String(bounded_text(name, MAX_NAME_CHARS, "team name")?);
    }
    if let Some(status) = &params.status {
        one_of(status, &["active", "disabled"], "team status")?;
        body["status"] = Value::String(status.clone());
    }
    let (origin, token) = credentials(auth).await?;
    decode(
        send(
            Method::PATCH,
            &origin,
            &token,
            &format!("/api/v3/remote/hosts/{}", params.host_id),
            Some(body),
        )
        .await?,
    )
}
