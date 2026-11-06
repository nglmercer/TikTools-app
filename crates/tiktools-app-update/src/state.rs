use crate::{Channel, InstallationType, UpdateError};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Verifying,
    ReadyToInstall,
    Installing,
    RestartRequired,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub current_version: String,
    pub available_version: Option<String>,
    pub channel: Channel,
    pub state: Phase,
    pub installation_type: InstallationType,
    pub install_supported: bool,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub progress: Option<f64>,
    pub release_notes: Option<String>,
    pub last_checked: Option<u64>,
    pub error: Option<UpdateError>,
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("another update operation is active or the state transition is invalid")]
    InvalidTransition,
}

pub struct UpdateState(Mutex<State>);
impl UpdateState {
    pub fn new(
        version: impl Into<String>,
        channel: Channel,
        installation_type: InstallationType,
    ) -> Self {
        Self(Mutex::new(State {
            current_version: version.into(),
            available_version: None,
            channel,
            state: Phase::Idle,
            installation_type,
            // The desktop lifecycle has not yet enabled installation.
            install_supported: false,
            downloaded_bytes: 0,
            total_bytes: None,
            progress: None,
            release_notes: None,
            last_checked: None,
            error: None,
        }))
    }
    pub fn snapshot(&self) -> State {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub fn transition(&self, next: Phase) -> Result<(), StateError> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let permitted = matches!(
            (state.state, next),
            (
                Phase::Idle
                    | Phase::UpToDate
                    | Phase::Available
                    | Phase::ReadyToInstall
                    | Phase::Failed,
                Phase::Checking
            ) | (
                Phase::Checking,
                Phase::UpToDate | Phase::Available | Phase::Failed
            ) | (Phase::Available, Phase::Downloading)
                | (Phase::Downloading, Phase::Verifying | Phase::Failed)
                | (Phase::Verifying, Phase::ReadyToInstall | Phase::Failed)
                | (Phase::ReadyToInstall, Phase::Installing)
                | (
                    Phase::Installing,
                    Phase::RestartRequired | Phase::Completed | Phase::Failed
                )
        ) || (state.state == Phase::Failed
            && next == Phase::Downloading
            && state
                .error
                .as_ref()
                .is_some_and(|error| error.code == "download_failed"));
        if !permitted {
            return Err(StateError::InvalidTransition);
        }
        state.state = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_operations_and_invalid_transitions_are_rejected() {
        let state = UpdateState::new("4.0.0", Channel::Stable, InstallationType::Unknown);
        assert!(state.transition(Phase::Checking).is_ok());
        assert!(state.transition(Phase::Checking).is_err());
        assert!(state.transition(Phase::Installing).is_err());
        assert!(state.transition(Phase::Available).is_ok());
        assert!(!state.snapshot().install_supported);
    }

    #[test]
    fn only_one_concurrent_check_is_admitted() {
        let state = std::sync::Arc::new(UpdateState::new(
            "4.0.0",
            Channel::Stable,
            InstallationType::Unknown,
        ));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let state = state.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    state.transition(Phase::Checking).is_ok()
                })
            })
            .collect();
        let admitted = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|ok| *ok)
            .count();
        assert_eq!(admitted, 1);
    }
}
