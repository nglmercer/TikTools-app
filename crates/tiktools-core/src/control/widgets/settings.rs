//! Host-internal gateway activation, loopback settings and raw credentials.
use super::*;

impl AppCore {
    pub(super) fn gateway_activation(&self) -> (bool, bool) {
        // Same defaults as the readiness hot path: plugins without a
        // persisted row count as installed and enabled.
        recover_rwlock_read(&self.plugin_state.activation, "plugin activation")
            .get(GATEWAY_PLUGIN_ID)
            .map(|state| (state.installed, state.enabled))
            .unwrap_or((true, true))
    }

    pub(super) fn gateway_port(&self, values: &Value) -> u16 {
        values
            .get("port")
            .and_then(Value::as_u64)
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port != 0)
            .unwrap_or(GATEWAY_DEFAULT_PORT)
    }

    pub(super) fn gateway_bind(&self, values: &Value) -> std::net::IpAddr {
        values
            .get("bind")
            .and_then(Value::as_str)
            .and_then(|bind| bind.parse::<std::net::IpAddr>().ok())
            .filter(|bind| bind.is_loopback())
            .unwrap_or_else(|| {
                GATEWAY_LOOPBACK_V4
                    .parse()
                    .expect("loopback literal is valid")
            })
    }

    /// Raw (unredacted) widget credential, host-internal only. The
    /// redaction placeholder is never a valid credential, even if it
    /// somehow ends up stored.
    pub(super) fn gateway_widget_credential(&self, values: &Value) -> Option<String> {
        let token = values
            .get("widgetToken")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|token| !token.is_empty())?;
        if token == SECRET_SETTING_PLACEHOLDER || token.len() > 4096 {
            return None;
        }
        Some(token.to_owned())
    }
}
