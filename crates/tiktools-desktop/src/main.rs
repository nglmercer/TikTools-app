#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod event;
mod icon;
mod logging;
mod media;
mod platform;
mod plugin_webview;
mod single_instance;
mod tray;
mod webview;
mod window;

fn main() {
    // rustls 0.23 cannot choose a provider when multiple TLS stacks are unified
    // by the workspace. Install the provider before any async client is created.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    let log_path = match logging::init() {
        Ok(path) => path,
        Err(error) => {
            let log_path = tiktools_core::paths::AppPaths::from_environment()
                .logs
                .join("tiktools.log");
            show_startup_failure(
                &format!("Persistent host logging could not be initialized: {error}"),
                &log_path,
            );
            std::process::exit(1);
        }
    };

    if let Err(error) = app::run(log_path.clone()) {
        tracing::error!(%error, "TikTools Rust host terminated during startup");
        show_startup_failure(&error.to_string(), &log_path);
        std::process::exit(1);
    }
}

pub(crate) fn show_startup_failure(error: &str, log_path: &std::path::Path) {
    let message = format!(
        "TikTools could not start.\n\n{error}\n\nSee log:\n{}",
        log_path.display()
    );
    let _ = rfd::MessageDialog::new()
        .set_title("TikTools could not start")
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}
