//! Configure matching GStreamer plugins before GTK, workers or WebKit start.
use std::path::{Path, PathBuf};

fn bundle_paths(executable: &Path, appdir: &Path) -> Option<(PathBuf, PathBuf)> {
    (executable.parent()? == appdir.join("usr/bin")).then(|| {
        (
            appdir.join("usr/lib/gstreamer-1.0"),
            appdir.join("usr/libexec/gstreamer-1.0/gst-plugin-scanner"),
        )
    })
}

pub fn configure() -> Result<(), String> {
    let Some(appdir) = std::env::var_os("APPDIR") else {
        return Ok(());
    };
    // An inherited APPDIR from another application must not affect a native run.
    let Ok(appdir) = std::fs::canonicalize(appdir) else {
        return Ok(());
    };
    let executable = std::env::current_exe().map_err(|_| "application path is unavailable")?;
    let Some((plugins, scanner)) = bundle_paths(&executable, &appdir) else {
        return Ok(());
    };
    if !plugins.join("libgstapp.so").is_file() || !scanner.is_file() {
        return Err(
            "AppImage is missing its GStreamer plugins or scanner; rebuild the package".into(),
        );
    }
    // Inherit into WebKit's child processes. A newer host plugin must never be
    // loaded by an older bundled GStreamer core through its default search path.
    std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", &plugins);
    std::env::set_var("GST_PLUGIN_PATH_1_0", &plugins);
    std::env::set_var("GST_PLUGIN_SCANNER_1_0", scanner);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configures_only_the_appimage_executable_and_keeps_helpers_inside_bundle() {
        let root = Path::new("/tmp/tiktools.AppDir");
        let (plugins, scanner) =
            bundle_paths(&root.join("usr/bin/tiktools-desktop"), root).unwrap();
        assert_eq!(plugins, root.join("usr/lib/gstreamer-1.0"));
        assert_eq!(
            scanner,
            root.join("usr/libexec/gstreamer-1.0/gst-plugin-scanner")
        );
        assert!(bundle_paths(Path::new("/usr/bin/tiktools-desktop"), root).is_none());
        assert!(bundle_paths(
            Path::new("/tmp/tiktools.AppDir-other/usr/bin/tiktools-desktop"),
            root
        )
        .is_none());
    }
}
