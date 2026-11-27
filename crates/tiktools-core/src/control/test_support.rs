//! Helpers shared by the owner journal unit tests: a silent host emitter, the
//! app paths under one root and a core opened over them.
use crate::{paths::AppPaths, AppCore, HostEmitter, NoopMediaHost};
use std::{path::Path, sync::Arc};

pub(crate) struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: crate::ipc::messages::HostMessage) {}
}

/// App paths for a throwaway root, with an absent built-in plugin directory.
pub(crate) fn paths(root: &Path) -> AppPaths {
    AppPaths {
        data: root.join("data"),
        plugins: root.join("plugins"),
        plugin_data: root.join("plugin-data"),
        builtin_plugins: root.join("absent"),
        development_plugins: None,
        logs: root.join("logs"),
        temp: root.join("temp"),
        root: root.into(),
    }
}

/// A core opened over `paths`; reopening over the same paths simulates a restart.
pub(crate) fn open_core(paths: &AppPaths) -> Arc<AppCore> {
    Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths.clone(),
    ))
}
