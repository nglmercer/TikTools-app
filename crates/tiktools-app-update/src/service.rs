use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    check_for_update, download_verified, AuthenticatedRelease, Channel, InstallPlan,
    InstallationType, Phase, State, VerifiedPackage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("application updates are not configured in this build")]
    Unavailable,
    #[error("an update operation is already running or the state is invalid")]
    InvalidState,
}

type Notify = dyn Fn(State) + Send + Sync;

struct Inner {
    state: State,
    release: Option<Arc<AuthenticatedRelease>>,
    package: Option<VerifiedPackage>,
}

/// Owns update jobs and verified bytes; neither URLs nor paths are accepted
/// from clients. Installation is intentionally not exposed by this service.
pub struct AppUpdater {
    inner: Mutex<Inner>,
    key: Option<String>,
    target: Option<&'static str>,
    install_target: Option<std::path::PathBuf>,
    notify: Arc<Notify>,
}

impl AppUpdater {
    pub fn new(
        version: &str,
        key: Option<&str>,
        installation_type: InstallationType,
        notify: impl Fn(State) + Send + Sync + 'static,
    ) -> Arc<Self> {
        let target = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Some("linux-x86_64"),
            ("windows", "x86_64") => Some("windows-x86_64"),
            _ => None,
        };
        Arc::new(Self {
            inner: Mutex::new(Inner {
                state: State {
                    current_version: version.to_owned(),
                    available_version: None,
                    channel: Channel::Stable,
                    state: Phase::Idle,
                    installation_type,
                    install_supported: false,
                    downloaded_bytes: 0,
                    total_bytes: None,
                    progress: None,
                    release_notes: None,
                    last_checked: None,
                    error: None,
                },
                release: None,
                package: None,
            }),
            key: key
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned),
            target,
            install_target: None,
            notify: Arc::new(notify),
        })
    }

    /// Desktop-only construction detects the source locally; callers do not
    /// provide a path. Installation remains off unless explicitly enabled in
    /// the trusted build after platform testing.
    pub fn new_desktop(
        version: &str,
        key: Option<&str>,
        install_enabled: bool,
        nsis_enabled: bool,
        notify: impl Fn(State) + Send + Sync + 'static,
    ) -> Arc<Self> {
        let detected = crate::detect_installation();
        let mut updater = Self::new(version, key, detected.kind, notify);
        let service = Arc::get_mut(&mut updater).expect("new updater is uniquely owned");
        service.install_target = detected.target;
        let supported = install_enabled
            && service.configured()
            && service.install_target.is_some()
            && (detected.kind != InstallationType::Nsis || nsis_enabled);
        service
            .inner
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .state
            .install_supported = supported;
        updater
    }

    pub fn status(&self) -> State {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .state
            .clone()
    }
    pub fn configured(&self) -> bool {
        self.key.is_some() && self.target.is_some()
    }

    /// Displayed only in the native confirmation, never supplied by RPC.
    pub fn installation_location(&self) -> Option<&std::path::Path> {
        self.install_target.as_deref()
    }

    fn update(&self, f: impl FnOnce(&mut Inner)) {
        let state = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut inner);
            inner.state.clone()
        };
        (self.notify)(state);
    }

    pub fn check(self: &Arc<Self>, channel: Channel) -> Result<State, ServiceError> {
        let key = self.key.clone().ok_or(ServiceError::Unavailable)?;
        let target = self.target.ok_or(ServiceError::Unavailable)?;
        let current;
        let state = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if !matches!(
                inner.state.state,
                Phase::Idle
                    | Phase::UpToDate
                    | Phase::Available
                    | Phase::ReadyToInstall
                    | Phase::Failed
            ) {
                return Err(ServiceError::InvalidState);
            }
            current = Version::parse(&inner.state.current_version)
                .map_err(|_| ServiceError::Unavailable)?;
            inner.release = None;
            inner.package = None;
            inner.state.channel = channel;
            inner.state.state = Phase::Checking;
            inner.state.available_version = None;
            inner.state.release_notes = None;
            inner.state.error = None;
            inner.state.downloaded_bytes = 0;
            inner.state.total_bytes = None;
            inner.state.progress = None;
            inner.state.clone()
        };
        (self.notify)(state.clone());
        let service = Arc::clone(self);
        tokio::spawn(async move {
            match check_for_update(channel, target, &current, &key).await {
                Ok(release) => service.update(|inner| {
                    inner.state.last_checked = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|t| t.as_secs());
                    if release.release().available() {
                        inner.state.available_version =
                            Some(release.release().release().version.to_string());
                        inner.state.release_notes = release.release().release().notes.clone();
                        inner.release = Some(Arc::new(release));
                        inner.state.state = Phase::Available;
                    } else {
                        inner.state.state = Phase::UpToDate;
                    }
                }),
                Err(error) => service.fail("check_failed", error.to_string()),
            }
        });
        Ok(state)
    }

    pub fn download(self: &Arc<Self>) -> Result<State, ServiceError> {
        if !self.configured() {
            return Err(ServiceError::Unavailable);
        }
        let (release, state) = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let retry = inner.state.state == Phase::Failed
                && inner
                    .state
                    .error
                    .as_ref()
                    .is_some_and(|error| error.code == "download_failed");
            if inner.state.state != Phase::Available && !retry {
                return Err(ServiceError::InvalidState);
            }
            let release = inner.release.clone().ok_or(ServiceError::InvalidState)?;
            inner.package = None;
            inner.state.state = Phase::Downloading;
            inner.state.error = None;
            inner.state.downloaded_bytes = 0;
            inner.state.total_bytes = None;
            inner.state.progress = None;
            (release, inner.state.clone())
        };
        (self.notify)(state.clone());
        let service = Arc::clone(self);
        tokio::spawn(async move {
            let progress_service = Arc::clone(&service);
            let verifying_service = Arc::clone(&service);
            let mut last = Instant::now()
                .checked_sub(Duration::from_secs(1))
                .unwrap_or_else(Instant::now);
            let result = download_verified(
                &release,
                move |downloaded, total| {
                    if total.is_some_and(|n| n == downloaded)
                        || last.elapsed() >= Duration::from_millis(250)
                    {
                        last = Instant::now();
                        progress_service.update(|inner| {
                            inner.state.downloaded_bytes = downloaded;
                            inner.state.total_bytes = total;
                            inner.state.progress = total
                                .filter(|n| *n > 0)
                                .map(|n| (downloaded as f64 * 100.0 / n as f64).min(100.0));
                        });
                    }
                },
                move || verifying_service.update(|inner| inner.state.state = Phase::Verifying),
            )
            .await;
            match result {
                Ok(package) => service.update(|inner| {
                    inner.package = Some(package);
                    inner.state.state = Phase::ReadyToInstall;
                }),
                Err(error) => service.fail("download_failed", error.to_string()),
            }
        });
        Ok(state)
    }

    /// Called only by the desktop after an affirmative native confirmation.
    /// This reserves the verified package and prevents conflicting jobs.
    pub fn prepare_install(&self) -> Result<InstallPlan, ServiceError> {
        let (release, package, current, target, kind) = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if !inner.state.install_supported {
                return Err(ServiceError::Unavailable);
            }
            if inner.state.state != Phase::ReadyToInstall {
                return Err(ServiceError::InvalidState);
            }
            if inner.release.is_none() || inner.package.is_none() {
                return Err(ServiceError::InvalidState);
            }
            let target = self
                .install_target
                .clone()
                .ok_or(ServiceError::Unavailable)?;
            let release = inner.release.take().ok_or(ServiceError::InvalidState)?;
            let package = inner.package.take().ok_or(ServiceError::InvalidState)?;
            inner.state.state = Phase::Installing;
            (
                release,
                package,
                inner.state.current_version.clone(),
                target,
                inner.state.installation_type,
            )
        };
        match InstallPlan::new(release, package, target, current, kind) {
            Ok(plan) => {
                (self.notify)(self.status());
                Ok(plan)
            }
            Err(error) => {
                self.fail("install_unavailable", error.to_string());
                Err(ServiceError::Unavailable)
            }
        }
    }

    fn fail(&self, code: &str, message: String) {
        self.update(|inner| {
            inner.state.state = Phase::Failed;
            inner.state.error = Some(UpdateError {
                code: code.to_owned(),
                message,
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unconfigured_updater_fails_closed() {
        let updater = AppUpdater::new("4.0.0", None, InstallationType::Unknown, |_| {});
        assert!(!updater.configured());
        assert!(matches!(
            updater.check(Channel::Stable),
            Err(ServiceError::Unavailable)
        ));
        assert!(matches!(updater.download(), Err(ServiceError::Unavailable)));
        assert!(matches!(
            updater.prepare_install(),
            Err(ServiceError::Unavailable)
        ));
        assert!(!updater.status().install_supported);
    }

    #[tokio::test]
    async fn a_new_channel_check_discards_ready_update_state() {
        let updater = AppUpdater::new(
            "4.0.0",
            Some("fixture-key"),
            InstallationType::AppImage,
            |_| {},
        );
        {
            let mut inner = updater.inner.lock().unwrap();
            inner.state.state = Phase::ReadyToInstall;
            inner.state.channel = Channel::Beta;
            inner.state.available_version = Some("4.1.0-beta.1".into());
        }
        let state = updater.check(Channel::Stable).unwrap();
        assert_eq!(state.state, Phase::Checking);
        assert_eq!(state.channel, Channel::Stable);
        assert!(state.available_version.is_none());
        let inner = updater.inner.lock().unwrap();
        assert!(inner.release.is_none());
        assert!(inner.package.is_none());
    }

    #[tokio::test]
    async fn retry_requires_a_failed_download_and_resets_progress() {
        let updater = AppUpdater::new(
            "4.0.0",
            Some("fixture-key"),
            InstallationType::AppImage,
            |_| {},
        );
        let metadata = br#"{"version":"4.1.0","platforms":{"linux-x86_64":{"url":"https://github.com/nglmercer/TikTools-app/releases/download/v4.1.0/TikTools-4.1.0-linux-x86_64-installer.AppImage","signature":"signed","format":"appimage"}}}"#;
        let release = crate::validate_manifest(
            metadata,
            "linux-x86_64",
            Channel::Stable,
            &Version::parse("4.0.0").unwrap(),
        )
        .unwrap();
        {
            let mut inner = updater.inner.lock().unwrap();
            inner.release = Some(Arc::new(AuthenticatedRelease {
                release,
                public_key: "fixture-key".into(),
            }));
            inner.state.state = Phase::Failed;
            inner.state.error = Some(UpdateError {
                code: "check_failed".into(),
                message: "failed".into(),
            });
            inner.state.downloaded_bytes = 50;
            inner.state.total_bytes = Some(100);
            inner.state.progress = Some(50.0);
        }
        assert!(matches!(
            updater.download(),
            Err(ServiceError::InvalidState)
        ));
        updater
            .inner
            .lock()
            .unwrap()
            .state
            .error
            .as_mut()
            .unwrap()
            .code = "download_failed".into();
        let state = updater.download().unwrap();
        assert_eq!(state.state, Phase::Downloading);
        assert_eq!(state.downloaded_bytes, 0);
        assert_eq!(state.total_bytes, None);
        assert_eq!(state.progress, None);
        assert!(matches!(
            updater.download(),
            Err(ServiceError::InvalidState)
        ));
    }

    #[test]
    fn installation_cannot_start_without_the_verified_package_and_release() {
        let mut updater = AppUpdater::new(
            "4.0.0",
            Some("fixture-key"),
            InstallationType::AppImage,
            |_| {},
        );
        Arc::get_mut(&mut updater).unwrap().install_target = Some("/tmp/fixture.AppImage".into());
        {
            let mut inner = updater.inner.lock().unwrap();
            inner.state.install_supported = true;
            inner.state.state = Phase::ReadyToInstall;
        }
        assert!(matches!(
            updater.prepare_install(),
            Err(ServiceError::InvalidState)
        ));
        assert_eq!(updater.status().state, Phase::ReadyToInstall);
    }
}
