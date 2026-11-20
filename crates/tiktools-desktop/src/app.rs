use std::{env, sync::Arc};

use tiktools_core::{ipc::IpcRouter, AppCore};
use tokio::runtime::Builder;
use winit::event_loop::EventLoop;

use crate::window::DesktopApp;
use crate::{event::DesktopEvent, platform, single_instance, webview::FrontendSource};

pub fn run(log_path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(archive) = install_plugin_argument()? {
        return install_plugin(archive, replace_plugin_argument());
    }
    let mut instance = match single_instance::acquire()? {
        Ok(instance) => instance,
        Err(single_instance::InstanceRole::Secondary) => return Ok(()),
    };
    // Validate the executable-relative packaged frontend before opening the
    // database, scanning plugins, or constructing the live client.
    let source = FrontendSource::from_environment().map_err(std::io::Error::other)?;
    platform::initialize()?;
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    let mut event_loop_builder = EventLoop::<DesktopEvent>::with_user_event();
    platform::configure_event_loop(&mut event_loop_builder);
    let event_loop = event_loop_builder.build()?;
    let proxy = event_loop.create_proxy();
    let emitter = crate::event::shared_emitter(proxy.clone());
    let media = Arc::new(crate::media::DesktopMediaHost);
    let core = Arc::new(AppCore::with_media_host(emitter.clone(), media));
    core.set_sync_origin("ui");
    core.enable_desktop_app_updater();
    let router = Arc::new(IpcRouter::new(core.clone()));
    instance.start_listener(proxy.clone());
    let mut app = DesktopApp::new(
        core,
        router,
        source,
        runtime.handle().clone(),
        proxy,
        log_path,
        emitter,
    );
    event_loop.run_app(&mut app)?;
    let update = app.take_update_plan();
    let restart = app.take_restart_request();
    drop(app);
    // Release WebViews, native plugins, the Tokio runtime, and the instance
    // lock before the updater transfers control to an installer.
    drop(instance);
    drop(runtime);
    let update = match update {
        Ok(update) => update,
        Err(spanish) => {
            let (title, message) = if spanish {
                ("Actualización cancelada", "TikTools no pudo cerrar todos los procesos o guardar todos los datos de forma segura. No se instaló la actualización; abre TikTools de nuevo y revisa el registro.")
            } else {
                ("Update canceled", "TikTools could not safely stop all processes or save all data. The update was not installed; reopen TikTools and check the log.")
            };
            let _ = rfd::MessageDialog::new()
                .set_title(title)
                .set_description(message)
                .set_level(rfd::MessageLevel::Error)
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
            return Ok(());
        }
    };
    let has_update = update.is_some();
    if let Some((update, spanish)) = update {
        let outcome = crate::logging::flush()
            .map_err(|error| error.to_string())
            .and_then(|()| update.apply().map_err(|error| error.to_string()));
        if let Err(error) = outcome {
            tracing::error!(%error, "application update failed after shutdown");
            let (title, message) = if spanish {
                ("Error al actualizar TikTools", "No se pudo instalar o reiniciar la aplicación. Descarga e instala TikTools desde https://github.com/nglmercer/TikTools-app/releases. Los datos de la aplicación se guardan por separado.")
            } else {
                ("TikTools update failed", "The application could not be installed or restarted. Download and reinstall TikTools from https://github.com/nglmercer/TikTools-app/releases. Your application data is stored separately.")
            };
            let _ = rfd::MessageDialog::new()
                .set_title(title)
                .set_description(message)
                .set_level(rfd::MessageLevel::Error)
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        }
    }
    // A plugin restart never races an update handoff: the installer owns the
    // relaunch in that case. By here core, WebViews and the instance lock are
    // all released, so the fresh process can acquire the single-instance slot.
    if !has_update {
        if let Some(locale) = restart {
            if let Err(error) = relaunch_application() {
                tracing::error!(%error, "application restart failed after shutdown");
                let _ = rfd::MessageDialog::new()
                    .set_title(crate::native_messages::restart_failed_title(&locale))
                    .set_description(crate::native_messages::restart_failed_description(&locale))
                    .set_level(rfd::MessageLevel::Error)
                    .set_buttons(rfd::MessageButtons::Ok)
                    .show();
            }
        }
    }
    Ok(())
}

/// Spawn a fresh TikTools process after the current one has fully shut down.
/// AppImages must be relaunched from the mounted image path, not from the
/// `current_exe()` inside the old mount, so prefer `APPIMAGE` when present.
fn relaunch_application() -> Result<(), String> {
    let appimage = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from);
    let executable = appimage
        .clone()
        .or_else(|| std::env::current_exe().ok())
        .ok_or_else(|| "could not locate the TikTools executable".to_owned())?;
    let mut command = std::process::Command::new(executable);
    if appimage.is_some() {
        // Mount-relative paths from the old image would leak into the new
        // process and break its plugin/library loading (same cleanup the
        // updater applies in `tiktools-app-update::install`).
        for name in [
            "APPDIR",
            "APPIMAGE",
            "ARGV0",
            "OWD",
            "LD_LIBRARY_PATH",
            "LD_PRELOAD",
            "XDG_DATA_DIRS",
            "GIO_EXTRA_MODULES",
            "GTK_PATH",
            "QT_PLUGIN_PATH",
        ] {
            command.env_remove(name);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

fn install_plugin_argument() -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        if argument == "--install-plugin" {
            let archive = args.next().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "--install-plugin requires a .plugin path, plugin id, or https URL",
                )
            })?;
            return Ok(Some(archive.into()));
        }
    }
    Ok(None)
}

fn replace_plugin_argument() -> bool {
    env::args().any(|argument| argument == "--replace")
}

#[cfg(feature = "plugin-install")]
fn install_plugin(
    spec: std::path::PathBuf,
    replace_existing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    struct NullEmitter;
    impl tiktools_core::HostEmitter for NullEmitter {
        fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
    }

    let core = Arc::new(AppCore::new(Arc::new(NullEmitter)));
    let spec = spec.to_string_lossy().into_owned();
    let lower = spec.to_ascii_lowercase();
    let remote = lower.starts_with("https://")
        || lower.starts_with("http://")
        || (!lower.ends_with(".plugin") && !spec.contains(['/', '\\']));
    if !remote {
        let outcome = core
            .install_plugin(spec.trim_start_matches("file:"), replace_existing)
            .map_err(|error| std::io::Error::other(error.message))?;
        println!(
            "Installed {}@{} in {}",
            outcome.receipt.id, outcome.receipt.version, outcome.directory
        );
        return Ok(());
    }
    let allow_unsigned = env::args().any(|argument| argument == "--allow-unsigned");
    let runtime = Builder::new_current_thread().enable_all().build()?;
    let job = runtime.block_on(async {
        use tiktools_core::plugin_installer::PackageSpecInput;
        let preview = core
            .plugin_package_inspect(PackageSpecInput::Spec { spec: spec.clone() })
            .await?;
        if let Some(incompatibility) = &preview.incompatibility {
            return Err(tiktools_core::control::OperationError::invalid(
                incompatibility.to_string(),
            ));
        }
        if preview.requires_sideload_consent && !allow_unsigned {
            return Err(tiktools_core::control::OperationError::invalid(format!(
                "{} is not signed by a trusted publisher; pass --allow-unsigned to install it anyway",
                preview.id
            )));
        }
        let mut job = core.plugin_package_install(
            PackageSpecInput::Spec { spec },
            preview.consent_token,
            None,
        )?;
        while !job.status.is_finished() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            job = core.plugin_job(&job.id)?;
        }
        Ok(job)
    })
    .map_err(|error| std::io::Error::other(error.message))?;
    match job.error {
        Some(error) => Err(std::io::Error::other(error.message).into()),
        None => {
            println!(
                "Installed {}@{}",
                job.plugin_id.unwrap_or_default(),
                job.to_version.unwrap_or_default()
            );
            Ok(())
        }
    }
}

#[cfg(not(feature = "plugin-install"))]
fn install_plugin(
    _archive: std::path::PathBuf,
    _replace_existing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "plugin installation was disabled in this build",
    )
    .into())
}
