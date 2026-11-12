//! Secret-free widget readiness derived from gateway state and bounded probes.
use super::*;

impl AppCore {
    /// Pure state resolution over already-gathered inputs, so every
    /// Widgets tab state is unit-testable without processes or sockets.
    pub(super) fn resolve_widgets_state(
        discovered: bool,
        installed: bool,
        enabled: bool,
        available: bool,
        running: bool,
        credential_present: bool,
        probe: Option<ProbeOutcome>,
    ) -> WidgetsState {
        if !discovered || !installed {
            return WidgetsState::Missing;
        }
        if !enabled || !available {
            return WidgetsState::Disabled;
        }
        if !running {
            return WidgetsState::Stopped;
        }
        if !credential_present {
            // The gateway generates credentials on its first start; a
            // running gateway without one needs a restart (or its settings
            // were just reset).
            return WidgetsState::CredentialUnavailable;
        }
        match probe {
            None | Some(ProbeOutcome::NotListening) => WidgetsState::Starting,
            Some(ProbeOutcome::BadGateway) => WidgetsState::Unreachable,
            Some(ProbeOutcome::AssetsMissing) => WidgetsState::AssetsMissing,
            Some(ProbeOutcome::Ready) => WidgetsState::Ready,
        }
    }

    pub async fn widgets_status(&self) -> Result<WidgetsStatusResult, OperationError> {
        let plugin = self.plugins.get(GATEWAY_PLUGIN_ID);
        let Some(plugin) = plugin else {
            return Ok(WidgetsStatusResult {
                state: WidgetsState::Missing,
                port: GATEWAY_DEFAULT_PORT,
                error: Some("the Event Gateway plugin is not installed".to_owned()),
            });
        };
        let values = self
            .capabilities
            .load_plugin_settings_raw(&plugin.manifest)
            .map_err(|error| OperationError::internal(error.to_string()))?;
        let port = self.gateway_port(&values);
        let bind = self.gateway_bind(&values);
        let (installed, enabled) = self.gateway_activation();
        let running = self.plugins.is_running(GATEWAY_PLUGIN_ID);
        let credential_present = self.gateway_widget_credential(&values).is_some();
        let probe = if installed && enabled && plugin.available && running && credential_present {
            Some(probe_gateway(bind, port).await)
        } else {
            None
        };
        let state = Self::resolve_widgets_state(
            true,
            installed,
            enabled,
            plugin.available,
            running,
            credential_present,
            probe,
        );
        let error = match state {
            WidgetsState::Ready => None,
            WidgetsState::Missing => Some("the Event Gateway plugin is not installed".to_owned()),
            WidgetsState::Disabled => Some("the Event Gateway plugin is disabled".to_owned()),
            WidgetsState::Stopped => Some("the Event Gateway is stopped".to_owned()),
            WidgetsState::Starting => {
                Some("the Event Gateway is starting; retry in a moment".to_owned())
            }
            WidgetsState::CredentialUnavailable => {
                Some("no widget credential is stored; restart the Event Gateway once".to_owned())
            }
            WidgetsState::AssetsMissing => {
                Some("the gateway does not serve widget assets".to_owned())
            }
            WidgetsState::Unreachable => {
                Some("the Event Gateway is not answering on its loopback port".to_owned())
            }
        };
        Ok(WidgetsStatusResult { state, port, error })
    }
}
