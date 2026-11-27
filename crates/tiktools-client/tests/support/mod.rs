//! Boots a native host over a throwaway home and serves it on a private IPC
//! name, so the SDK tests can call it directly and through the IPC client.
use std::{path::Path, sync::Arc, time::Duration};
use tiktools_core::{
    ipc::messages::HostMessage, paths::AppPaths, AppCore, HostEmitter, NoopMediaHost,
};

struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}

pub struct IpcHost {
    pub core: Arc<AppCore>,
    pub api: Arc<tiktools_client::ControlApi>,
    pub server: tokio::task::JoinHandle<std::io::Result<()>>,
}

/// Starts the host under `<temporary>/home`. `label` and the directory name
/// make the IPC name unique; the process env is pointed at the new host.
pub async fn start_ipc_host(temporary: &Path, label: &str) -> IpcHost {
    let home = temporary.join("home");
    std::env::set_var("TIKTOOLS_HOME", &home);
    std::env::set_var(
        "TIKTOOLS_IPC_NAME",
        format!(
            "tiktools-{label}-{}-{}",
            std::process::id(),
            temporary.file_name().unwrap().to_string_lossy()
        ),
    );
    let paths = AppPaths {
        data: home.join("data"),
        plugins: home.join("plugins"),
        plugin_data: home.join("plugin-data"),
        builtin_plugins: home.join("builtins"),
        development_plugins: None,
        logs: home.join("logs"),
        temp: home.join("temp"),
        root: home,
    };
    paths.ensure_directories().unwrap();
    for path in [
        paths.points_database(),
        paths.automation_database(),
        paths.analytics_database(),
    ] {
        std::fs::File::create(path).unwrap();
    }
    let core = Arc::new(AppCore::with_paths(
        Arc::new(Emitter),
        Arc::new(NoopMediaHost),
        paths,
    ));
    let api = Arc::new(tiktools_client::ControlApi::new(core.clone()));
    let (ready, started) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(tiktools_control_api::run_ipc_shared_with_ready(
        api.clone(),
        move || {
            let _ = ready.send(());
        },
    ));
    tokio::time::timeout(Duration::from_secs(5), started)
        .await
        .unwrap()
        .unwrap();
    IpcHost { core, api, server }
}
