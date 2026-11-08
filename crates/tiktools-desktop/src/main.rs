#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
#[cfg(feature = "remote-client")]
mod controller;
mod event;
mod icon;
mod logging;
mod media;
mod platform;
mod plugin_webview;
mod single_instance;
mod startup;
mod tray;
mod webview;
mod window;

fn main() {
    let mode = match startup::Mode::parse(std::env::args().skip(1)) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    // Handle this before logging, the single-instance lock, or GUI initialization.
    // Release CI runs the actual binary and compares this value to its tag.
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--update-installation-type"))
    {
        let installation = tiktools_core::app_update::detect_installation().kind();
        println!(
            "{}",
            serde_json::to_string(&installation).expect("installation type serializes")
        );
        return;
    }
    // rustls 0.23 cannot choose a provider when multiple TLS stacks are unified
    // by the workspace. Install the provider before any async client is created.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    #[cfg(not(feature = "remote-client"))]
    if matches!(mode, startup::Mode::Controller(_)) {
        eprintln!("remote controller support is unavailable in this build");
        std::process::exit(2);
    }
    let log_path = match if matches!(mode, startup::Mode::Controller(_)) {
        logging::init_controller()
    } else {
        logging::init()
    } {
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

    let result = match mode {
        startup::Mode::Local => app::run(log_path.clone()),
        #[cfg(feature = "remote-client")]
        startup::Mode::Controller(profile) => controller::run(profile, log_path.clone()),
        #[cfg(not(feature = "remote-client"))]
        startup::Mode::Controller(_) => unreachable!(),
    };
    if let Err(error) = result {
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
