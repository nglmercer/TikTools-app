use super::*;

impl CloudTeams {
    pub async fn enroll(
        &self,
        auth: &CloudAuth,
        params: TeamEnrollParams,
    ) -> Result<TeamDevice, OperationError> {
        one_of(&params.kind, &["controller", "host"], "device kind")?;
        let label = bounded_text(&params.label, MAX_LABEL_CHARS, "label")?;
        let key = self.device_key()?;
        let public_key = URL_SAFE_NO_PAD.encode(key.public_key().as_ref());
        let mut binding = json!({"kind": params.kind, "label": label, "publicKey": public_key});
        if params.kind == "host" {
            let host = params
                .host_id
                .as_deref()
                .ok_or_else(|| OperationError::invalid("a host device needs a team"))?;
            valid_uuid(host)?;
            binding["hostId"] = Value::String(host.to_owned());
        } else if params.host_id.is_some() {
            return Err(OperationError::invalid(
                "a controller device does not take a team",
            ));
        }

        let (origin, token) = credentials(auth).await?;
        let challenge = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/challenges",
            Some(json!({"purpose": "enroll", "binding": binding})),
        )
        .await?;
        let challenge_id = str_field(&challenge, "id")?;
        let nonce = URL_SAFE_NO_PAD
            .decode(str_field(&challenge, "nonce")?)
            .map_err(|_| OperationError::internal("cloud sent an unexpected challenge"))?;
        let digest = str_field(&challenge, "bindingSha256")?;
        if digest != binding_digest(&binding) {
            return Err(OperationError::internal(
                "cloud changed the enrollment binding",
            ));
        }
        let signature = key.sign(&proof_message("enroll", &nonce, &digest));
        let device = send(
            Method::POST,
            &origin,
            &token,
            "/api/v3/remote/devices",
            Some(json!({
                "challengeId": challenge_id,
                "binding": binding,
                "signature": URL_SAFE_NO_PAD.encode(signature.as_ref()),
            })),
        )
        .await?;
        let mut device: TeamDevice = decode(device)?;
        // The enrollment is already complete in Cloud. A local failure must not
        // be read as "not enrolled" and answered with a second enrollment.
        let saved = if params.kind == "host" {
            let host = params.host_id.clone().unwrap_or_default();
            self.save_host_binding(&host, &device.id)
        } else {
            self.save_controller_device(&device.id)
        };
        saved.map_err(|error| {
            OperationError::internal(format!(
                "this PC was enrolled in Cloud as device {} but its local record could not be saved ({}); do not enroll again, revoke that device first",
                device.id, error.message
            ))
        })?;
        device.is_this_device = device.kind == "host";
        Ok(device)
    }
}

pub async fn devices(auth: &CloudAuth) -> Result<TeamDeviceList, OperationError> {
    let (origin, token) = credentials(auth).await?;
    decode(send(Method::GET, &origin, &token, "/api/v3/remote/devices", None).await?)
}

pub async fn revoke_device(
    auth: &CloudAuth,
    params: TeamDeviceParams,
) -> Result<Value, OperationError> {
    valid_uuid(&params.device_id)?;
    let (origin, token) = credentials(auth).await?;
    send(
        Method::DELETE,
        &origin,
        &token,
        &format!("/api/v3/remote/devices/{}", params.device_id),
        None,
    )
    .await
}
