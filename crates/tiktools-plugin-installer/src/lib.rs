mod error;
mod installer;
mod jobs;
mod layout;
mod locks;
mod native_libs;
mod preview;
mod receipt;
mod sources;
mod spec;
mod updates;

#[cfg(test)]
mod tests;

pub use error::{InstallError, InstallErrorCode};
pub use installer::{
    ActivationHooks, CommitOptions, Consent, InstallOutcome, InstalledVersions, Installer,
    InstallerConfig, NoopHooks, PreparedPackage, SignaturePolicy, DEFAULT_RETAINED_VERSIONS,
};
pub use jobs::{
    JobHandle, JobKind, JobListener, JobProgress, JobRegistry, JobReporter, JobSnapshot, JobStage,
    JobStatus, NoopReporter,
};
pub use layout::{StoreLayout, STAGING_DIRECTORY};
pub use preview::{ChangeKind, PackagePreview, PermissionDiff, SecurityModel};
pub use receipt::{
    InstallReceipt, PackageOrigin, ReceiptStore, RetainedVersion, SignatureState, SignatureStatus,
    UpdatePolicy,
};
pub use sources::{ExpectedRelease, PackageSources, ResolvedSource};
pub use spec::{PackageSpec, PackageSpecInput};
pub use updates::{check_updates, UpdateInfo, UpdateKind, UpdateReport};
