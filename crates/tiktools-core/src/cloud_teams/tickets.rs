use super::*;

impl CloudTeams {
    /// Obtains a connection ticket for this device by proving possession of its key.
    pub async fn issue_ticket(
        &self,
        auth: &CloudAuth,
        host_id: &str,
        device_id: &str,
    ) -> Result<String, OperationError> {
        self.issue_ticket_as(auth, "host", host_id, device_id).await
    }

    /// Obtains a ticket for `role` (`host` or `controller`) bound to this device and team.
    pub async fn issue_ticket_as(
        &self,
        auth: &CloudAuth,
        role: &str,
        host_id: &str,
        device_id: &str,
    ) -> Result<String, OperationError> {
        one_of(role, &["host", "controller"], "relay role")?;
        valid_uuid(host_id)?;
        valid_uuid(device_id)?;
        let binding = json!({"deviceId": device_id, "hostId": host_id, "role": role});
        let key = self.device_key()?;
        let (origin, token) = credentials(auth).await?;
        let challenge = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/challenges",
            Some(json!({"purpose": "ticket", "binding": binding})),
        )
        .await?;
        let challenge_id = str_field(&challenge, "id")?;
        let nonce = URL_SAFE_NO_PAD
            .decode(str_field(&challenge, "nonce")?)
            .map_err(|_| OperationError::internal("cloud sent an unexpected challenge"))?;
        let digest = str_field(&challenge, "bindingSha256")?;
        if digest != binding_digest(&binding) {
            return Err(OperationError::internal("cloud changed the ticket binding"));
        }
        let signature = key.sign(&proof_message("ticket", &nonce, &digest));
        let issued = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/tickets",
            Some(json!({
                "challengeId": challenge_id,
                "binding": binding,
                "signature": URL_SAFE_NO_PAD.encode(signature.as_ref()),
            })),
        )
        .await?;
        let ticket = str_field(&issued, "ticket")?;
        if ticket.len() < 32 {
            return Err(OperationError::internal("cloud sent an unexpected ticket"));
        }
        Ok(ticket)
    }
}
