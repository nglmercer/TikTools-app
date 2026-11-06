//! Application update policy, independent of the desktop window system.
//!
//! A validated manifest is not proof that an update is authentic. The updater
//! must verify the package signature before allowing an installation.

mod channel;
mod check;
mod download;
mod install;
mod manifest;
mod platform;
mod service;
mod state;

pub use channel::Channel;
pub use check::{authenticate_manifest, check_for_update, AuthenticatedRelease, CheckError};
pub use download::{
    download_verified, verify_signature, DownloadError, VerifiedPackage, MAX_UPDATE_BYTES,
};
pub use install::{InstallError, InstallPlan};
pub use manifest::{validate_manifest, ManifestError, ValidatedRelease};
pub use platform::{detect_installation, DetectedInstallation, InstallationType};
pub use service::{AppUpdater, ServiceError, UpdateError};
pub use state::{Phase, State, StateError, UpdateState};
