//! Read the operator's ICE discovery services without persisting them in profiles.
use super::*;
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RtcTransportConfig {
    pub protocol_version: u32,
    pub stun_urls: Vec<String>,
}
impl RtcTransportConfig {
    fn validate(&self) -> Result<(), OperationError> {
        if self.protocol_version != 1 || self.stun_urls.len() > 8 {
            return Err(OperationError::unavailable(
                "Cloud WebRTC configuration is unsupported",
            ));
        }
        for raw in &self.stun_urls {
            let host = raw
                .strip_prefix("stun:")
                .ok_or_else(|| OperationError::invalid("invalid STUN discovery configuration"))?;
            let url = url::Url::parse(&format!("udp://{host}"))
                .map_err(|_| OperationError::invalid("invalid STUN discovery configuration"))?;
            if raw.len() > 2048
                || raw.chars().any(char::is_control)
                || url.host().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || !url.path().is_empty()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.port() == Some(0)
            {
                return Err(OperationError::invalid(
                    "invalid STUN discovery configuration",
                ));
            }
        }
        Ok(())
    }
}
impl CloudTeams {
    pub async fn rtc_config(&self, auth: &CloudAuth) -> Result<RtcTransportConfig, OperationError> {
        let (origin, token) = credentials(auth).await?;
        let config: RtcTransportConfig = decode(
            send(
                Method::GET,
                &origin,
                &token,
                "/api/v3/remote/rtc-config",
                None,
            )
            .await
            .map_err(|error| {
                if error.code == "not_found" {
                    OperationError::unavailable(
                        "Cloud WebRTC configuration is unavailable; update the backend",
                    )
                } else {
                    error
                }
            })?,
        )?;
        config.validate()?;
        Ok(config)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ice_discovery_rejects_private_credentials_and_unsupported_protocols() {
        for raw in [
            "turn:example.test:3478",
            "stun:user:secret@example.test:3478",
            "stun:example.test?secret=bad",
            "stun:example.test/path",
            "stun:",
            "stun:example.test:0",
        ] {
            assert!(RtcTransportConfig {
                protocol_version: 1,
                stun_urls: vec![raw.into()]
            }
            .validate()
            .is_err());
        }
        assert!(RtcTransportConfig {
            protocol_version: 2,
            stun_urls: vec![]
        }
        .validate()
        .is_err());
        assert!(RtcTransportConfig {
            protocol_version: 1,
            stun_urls: vec!["stun:example.test:3478".into()]
        }
        .validate()
        .is_ok());
    }
}
