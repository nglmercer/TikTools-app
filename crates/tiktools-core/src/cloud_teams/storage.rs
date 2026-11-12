use super::*;

impl CloudTeams {
    /// Loads the device key, creating it once if this PC has none. Creation is
    /// exclusive: a racing creator that loses loads the winner's key, and an
    /// existing key that cannot be parsed is an error, never replaced.
    pub(super) fn device_key(&self) -> Result<Ed25519KeyPair, OperationError> {
        match fs::read(&self.key_path) {
            Ok(bytes) => return parse_device_key(&bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(OperationError::internal("device key is unreadable")),
        }
        let generated = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
            .map_err(|_| OperationError::internal("could not create a device key"))?;
        match private_file::create_exclusive(&self.key_path, generated.as_ref()) {
            Ok(()) => parse_device_key(generated.as_ref()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => self.winning_key(),
            Err(_) => Err(OperationError::internal("could not store the device key")),
        }
    }

    /// Another creator published the key first. Its file may still be mid-write
    /// for a moment, so a short, bounded re-read precedes an "invalid" verdict.
    pub(super) fn winning_key(&self) -> Result<Ed25519KeyPair, OperationError> {
        let mut last = OperationError::internal("device key is invalid");
        for attempt in 0..5 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(10));
            }
            match fs::read(&self.key_path) {
                Ok(bytes) => match parse_device_key(&bytes) {
                    Ok(key) => return Ok(key),
                    Err(error) => last = error,
                },
                Err(_) => return Err(OperationError::internal("device key is unreadable")),
            }
        }
        Err(last)
    }

    /// The team and device this PC serves as, recorded when the host device was enrolled.
    pub fn host_binding(&self) -> Result<Option<(String, String)>, OperationError> {
        let bytes = match fs::read(&self.binding_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(OperationError::internal("host binding is unreadable")),
        };
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| OperationError::internal("host binding is invalid"))?;
        let host = str_field(&value, "hostId")?;
        let device = str_field(&value, "deviceId")?;
        valid_uuid(&host)?;
        valid_uuid(&device)?;
        Ok(Some((host, device)))
    }

    /// The controller device this PC enrolled as, used to redeem an invitation.
    pub fn controller_device(&self) -> Result<String, OperationError> {
        let bytes = match fs::read(&self.controller_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(OperationError::invalid(
                    "enroll this PC as a viewer before joining a team",
                ))
            }
            Err(_) => return Err(OperationError::internal("controller record is unreadable")),
        };
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| OperationError::internal("controller record is invalid"))?;
        let device = str_field(&value, "deviceId")?;
        valid_uuid(&device)?;
        Ok(device)
    }

    /// Replaces the host binding record. The device key is a separate file and
    /// is never touched, so saving another binding keeps this PC's identity.
    pub(super) fn save_host_binding(&self, host: &str, device: &str) -> Result<(), OperationError> {
        valid_uuid(host)?;
        valid_uuid(device)?;
        save_record(
            &self.binding_path,
            &json!({"hostId": host, "deviceId": device}),
            "host binding",
        )
    }

    /// Replaces the controller record with the enrolled viewer device.
    pub(super) fn save_controller_device(&self, device: &str) -> Result<(), OperationError> {
        valid_uuid(device)?;
        save_record(
            &self.controller_path,
            &json!({"deviceId": device}),
            "controller record",
        )
    }
}

fn parse_device_key(pkcs8: &[u8]) -> Result<Ed25519KeyPair, OperationError> {
    Ed25519KeyPair::from_pkcs8(pkcs8).map_err(|_| OperationError::internal("device key is invalid"))
}

/// Serialises first, then atomically replaces the record: the previous file is
/// untouched until the new one is complete on disk.
fn save_record(path: &std::path::Path, record: &Value, what: &str) -> Result<(), OperationError> {
    let bytes = serde_json::to_vec(record)
        .map_err(|_| OperationError::internal(format!("could not encode the {what}")))?;
    private_file::replace_atomic(path, &bytes).map_err(|ReplaceError { destination_may_have_changed, .. }| {
        if destination_may_have_changed {
            OperationError::internal(format!(
                "the {what} was written but could not be confirmed on disk; read it again before retrying"
            ))
        } else {
            OperationError::internal(format!("could not store the {what}"))
        }
    })
}
