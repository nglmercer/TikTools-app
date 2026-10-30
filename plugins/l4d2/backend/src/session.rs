//! One L4D2api IPC transaction per action. Game installation is managed externally.
use crate::command::ParsedAction;
use crate::config::Settings;
use crate::result;
use l4d2api_client::{Client, Error as IpcError, Health};
use std::time::Duration;
use tiktools_plugin_sdk::{ActionResult, PluginError, PluginResult};

fn map_ipc_error(error: IpcError) -> PluginError {
    match error {
        IpcError::UnsupportedCapability(name) => PluginError::capability_unavailable(format!("L4D2api addon does not advertise {name}")),
        IpcError::NoHeartbeat => PluginError::other("L4D2api heartbeat not found. Install the addon from https://github.com/nglmercer/L4D2api/releases/tag/v0.0.1 and start a local L4D2 campaign."),
        IpcError::Timeout(duration) => PluginError::other(format!("L4D2api reply timed out after {duration:?}; game outcome unconfirmed (do not retry automatically)")),
        other => PluginError::other(format!("L4D2api IPC: {other}")),
    }
}

fn ensure_gameplay_ready(health: Health) -> PluginResult<()> {
    match health {
        Health::GameplayReady => Ok(()),
        Health::Connected => Err(PluginError::other(
            "L4D2api is connected, but start an active local campaign before testing or spawning",
        )),
        Health::NotReady { reason } => {
            Err(PluginError::other(format!("L4D2api not ready: {reason}")))
        }
        Health::Disconnected => Err(PluginError::other(
            "L4D2api disconnected: no valid heartbeat. Check addon installation and start L4D2.",
        )),
        Health::Stale { reason } => Err(PluginError::other(format!(
            "L4D2api heartbeat stale: {reason}; restart the game and test again"
        ))),
    }
}

pub fn execute(settings: &Settings, parsed: ParsedAction) -> PluginResult<ActionResult> {
    let game_dir = settings.resolve_game_dir()?;
    let ipc_dir = Settings::ipc_dir(&game_dir);
    let mut client = Client::new(&ipc_dir, settings.timeout, Duration::from_millis(250));

    let health = client.status().map_err(map_ipc_error)?;
    ensure_gameplay_ready(health)?;
    client.connect().map_err(map_ipc_error)?;

    // Upstream owns generations, sequencing, locks, checksum, capabilities, and reply matching.
    // A timeout never retries or assumes the action did not execute.
    let reply = client.send_auto(parsed.action).map_err(map_ipc_error)?;
    result::from_game(parsed.label, reply)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_states_must_not_be_reported_as_gameplay_success() {
        for health in [
            Health::Disconnected,
            Health::Connected,
            Health::Stale { reason: "stale" },
            Health::NotReady { reason: "not_host" },
        ] {
            assert!(ensure_gameplay_ready(health).is_err());
        }
        assert!(ensure_gameplay_ready(Health::GameplayReady).is_ok());
    }
}
