//! Native-only package journal orchestration, not wired into product.
//!
//! Every transition is admitted separately. Filesystem work runs only after
//! the durable `mutating` transition and outside both admission and CloudAuth.
//! A result, including an error, retains the journal for native reconciliation.
//! Product activation also requires a confined native purge path, enforced
//! invocation/package lock ownership, a verified artifact/runtime review and
//! a fresh trusted preview to determine whether a commit is install or update.
use std::path::Path;

use crate::db::{
    DatabaseManager, PackageMutationIntent, PackageMutationKind, PackageMutationPhase,
};
use tiktools_cloud_sync::SyncCoordinator;
use tiktools_plugin_installer::{
    ActivationHooks, CommitOptions, Consent, InstallError, InstallErrorCode, InstallOutcome,
    Installer, JobReporter, PreparedPackage,
};
use tiktools_plugin_package::Sha256Hasher;

trait PackageAdmission {
    fn admit<R>(&self, operation: impl FnOnce() -> R) -> Result<R, InstallError>;
}

impl PackageAdmission for SyncCoordinator {
    fn admit<R>(&self, operation: impl FnOnce() -> R) -> Result<R, InstallError> {
        self.with_package_journal_admission(operation)
            .map_err(InstallError::io)
    }
}

/// A completed call is not a resolved journal. The caller must inspect the
/// actual artifact and runtime through a future native recovery flow.
struct Unreconciled<T> {
    intent: PackageMutationIntent,
    result: Result<T, InstallError>,
}

struct PackageJournal<'a, A> {
    db: &'a DatabaseManager,
    admission: &'a A,
}

fn field(hash: &mut Sha256Hasher, value: &[u8]) {
    hash.update(&(value.len() as u64).to_be_bytes());
    hash.update(value);
}

fn operation_digest(id: &str, kind: PackageMutationKind, parts: &[&[u8]]) -> String {
    let mut hash = Sha256Hasher::new();
    field(&mut hash, b"tiktools-package-mutation-v1");
    field(&mut hash, id.as_bytes());
    field(
        &mut hash,
        match kind {
            PackageMutationKind::Install => b"install",
            PackageMutationKind::Update => b"update",
            PackageMutationKind::Rollback => b"rollback",
            PackageMutationKind::Uninstall => b"uninstall",
        },
    );
    for part in parts {
        field(&mut hash, part);
    }
    hash.finish().to_string()
}

impl<A: PackageAdmission> PackageJournal<'_, A> {
    /// Caller holds the local invocation gate. The callback must be the exact
    /// native operation described by `digest`; it runs once and never under
    /// admission. Do not expose this helper or its opaque intent to a UI.
    fn run<T>(
        &self,
        id: &str,
        kind: PackageMutationKind,
        digest: &str,
        effect: impl FnOnce() -> Result<T, InstallError>,
    ) -> Result<Unreconciled<T>, InstallError> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(InstallError::io)?;
        let intent_id: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        let intent = self.admission.admit(|| {
            let intent = self
                .db
                .prepare_package_mutation(id, &intent_id, kind, digest)
                .map_err(InstallError::io)?
                .ok_or_else(|| InstallError::busy(id))?;
            if !self
                .db
                .begin_package_mutation(&intent)
                .map_err(InstallError::io)?
            {
                return Err(InstallError::new(
                    InstallErrorCode::Busy,
                    "package source changed before mutation",
                ));
            }
            let started = self
                .db
                .package_mutation(id)
                .map_err(InstallError::io)?
                .ok_or_else(|| InstallError::io("package mutation disappeared"))?;
            if started.intent_id != intent.intent_id
                || started.plugin_id != intent.plugin_id
                || started.operation != intent.operation
                || started.operation_digest != intent.operation_digest
                || started.source_generation != intent.source_generation
                || started.phase != PackageMutationPhase::Mutating
            {
                return Err(InstallError::io("package mutation changed after admission"));
            }
            Ok(started)
        })??;
        let result = effect();
        Ok(Unreconciled { intent, result })
    }

    fn commit(
        &self,
        kind: PackageMutationKind,
        installer: &Installer,
        prepared: &PreparedPackage,
        options: &CommitOptions,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<Unreconciled<InstallOutcome>, InstallError> {
        if !matches!(
            kind,
            PackageMutationKind::Install | PackageMutationKind::Update
        ) {
            return Err(InstallError::io("invalid package operation"));
        }
        let origin = serde_json::to_vec(&prepared.origin).map_err(InstallError::io)?;
        let signature = serde_json::to_vec(&prepared.signature).map_err(InstallError::io)?;
        let policy = serde_json::to_vec(&options.policy).map_err(InstallError::io)?;
        let artifact = prepared.digest.to_string();
        let (consent_kind, consent) = match &options.consent {
            Consent::Token(token) => (b"token".as_slice(), token.as_bytes()),
            Consent::Implicit {
                replace_existing: true,
            } => (b"implicit".as_slice(), b"replace".as_slice()),
            Consent::Implicit {
                replace_existing: false,
            } => (b"implicit".as_slice(), b"new".as_slice()),
        };
        let digest = operation_digest(
            &prepared.manifest.id,
            kind,
            &[
                artifact.as_bytes(),
                prepared.manifest.version.as_bytes(),
                &origin,
                &signature,
                &policy,
                consent_kind,
                consent,
            ],
        );
        self.run(&prepared.manifest.id, kind, &digest, || {
            installer.commit(prepared, options, hooks, reporter)
        })
    }

    fn rollback(
        &self,
        installer: &Installer,
        id: &str,
        version: Option<&str>,
        hooks: &dyn ActivationHooks,
        reporter: &dyn JobReporter,
    ) -> Result<Unreconciled<InstallOutcome>, InstallError> {
        let (present, version_bytes) = version.map_or((b"none".as_slice(), b"".as_slice()), |v| {
            (b"some".as_slice(), v.as_bytes())
        });
        let digest = operation_digest(id, PackageMutationKind::Rollback, &[present, version_bytes]);
        self.run(id, PackageMutationKind::Rollback, &digest, || {
            installer.rollback(id, version, hooks, reporter)
        })
    }

    fn uninstall(
        &self,
        installer: &Installer,
        id: &str,
        purge_data: Option<&Path>,
        hooks: &dyn ActivationHooks,
    ) -> Result<Unreconciled<()>, InstallError> {
        let (present, path) = purge_data.map_or((b"none".as_slice(), b"".as_slice()), |path| {
            (b"some".as_slice(), path.as_os_str().as_encoded_bytes())
        });
        let digest = operation_digest(id, PackageMutationKind::Uninstall, &[present, path]);
        self.run(id, PackageMutationKind::Uninstall, &digest, || {
            installer.uninstall(id, purge_data, hooks)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;
    use std::cell::Cell;

    struct Admission {
        held: Cell<bool>,
    }
    impl PackageAdmission for Admission {
        fn admit<R>(&self, operation: impl FnOnce() -> R) -> Result<R, InstallError> {
            if self.held.replace(true) {
                return Err(InstallError::io("admission busy"));
            }
            struct Release<'a>(&'a Cell<bool>);
            impl Drop for Release<'_> {
                fn drop(&mut self) {
                    self.0.set(false);
                }
            }
            let _release = Release(&self.held);
            Ok(operation())
        }
    }
    struct Fixture {
        db: DatabaseManager,
        root: std::path::PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "package-journal-orchestration-{}-{}",
                std::process::id(),
                fastrand::u64(..)
            ));
            let paths = AppPaths {
                data: root.join("data"),
                plugins: root.join("plugins"),
                plugin_data: root.join("plugin-data"),
                builtin_plugins: root.join("absent"),
                development_plugins: None,
                logs: root.join("logs"),
                temp: root.join("temp"),
                root: root.clone(),
            };
            paths.ensure_directories().unwrap();
            Self {
                db: DatabaseManager::new(paths),
                root,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn operation_executes_once_outside_admission_and_failure_stays_durable() {
        let fixture = Fixture::new();
        let admission = Admission {
            held: Cell::new(false),
        };
        let journal = PackageJournal {
            db: &fixture.db,
            admission: &admission,
        };
        let calls = Cell::new(0);
        let digest = operation_digest(
            "acme.plugin",
            PackageMutationKind::Rollback,
            &[b"some", b"1.0.0"],
        );
        let result = journal
            .run::<()>(
                "acme.plugin",
                PackageMutationKind::Rollback,
                &digest,
                || {
                    assert!(!admission.held.get());
                    calls.set(calls.get() + 1);
                    Err(InstallError::io("fixture failure"))
                },
            )
            .unwrap();
        assert!(result.result.is_err());
        assert_eq!(calls.get(), 1);
        let reopened = DatabaseManager::new(fixture.db.paths().clone());
        let saved = reopened.package_mutation("acme.plugin").unwrap().unwrap();
        assert_eq!(saved.intent_id, result.intent.intent_id);
        assert_eq!(saved.operation_digest, digest);
        assert_eq!(result.intent.phase, PackageMutationPhase::Mutating);
        assert_eq!(saved, result.intent);
        assert!(!reopened.begin_package_mutation(&result.intent).unwrap());
        assert!(journal
            .run(
                "acme.plugin",
                PackageMutationKind::Rollback,
                &digest,
                || {
                    calls.set(calls.get() + 1);
                    Ok(())
                }
            )
            .is_err());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn framed_digest_binds_kind_id_and_each_exact_argument() {
        let id = "acme.plugin";
        let digest = operation_digest(id, PackageMutationKind::Install, &[b"ab", b"c"]);
        assert_ne!(
            digest,
            operation_digest(id, PackageMutationKind::Install, &[b"a", b"bc"])
        );
        assert_ne!(
            digest,
            operation_digest(id, PackageMutationKind::Update, &[b"ab", b"c"])
        );
        assert_ne!(
            digest,
            operation_digest("other.plugin", PackageMutationKind::Install, &[b"ab", b"c"])
        );
        assert_eq!(digest.len(), 64);
    }
}
