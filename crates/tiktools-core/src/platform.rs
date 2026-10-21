//! Platform constraints shared by every host; no second runtime or event bus.
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RuntimePlatform {
    Windows,
    Linux,
    MacOS,
    AndroidTermux,
    AndroidOther,
}

impl RuntimePlatform {
    pub fn detect() -> Self {
        if cfg!(target_os = "android") {
            let prefix = std::env::var_os("PREFIX");
            let signalled = std::env::var_os("TERMUX_VERSION").is_some()
                || std::env::var_os("TERMUX_APP_PID").is_some();
            if termux_signals(signalled, prefix.as_deref().map(Path::new)) {
                Self::AndroidTermux
            } else {
                Self::AndroidOther
            }
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOS
        } else {
            Self::Linux
        }
    }

    /// Platform/build support, not a claim that network, storage or a plugin
    /// has passed a runtime probe. Desktop facilities require a host backend.
    pub fn capabilities(self) -> serde_json::Value {
        let android = matches!(self, Self::AndroidTermux | Self::AndroidOther);
        serde_json::json!({
            "localIpc": cfg!(any(unix, windows)),
            "filesystemPersistence": cfg!(feature = "persistence"),
            "networkConnections": true,
            "nativePlugins": cfg!(feature = "native-plugins") && !android,
            "processPlugins": true,
            "javascriptVm": true,
            "nativeNodeAddons": cfg!(feature = "napi-vm-node-api") && !android,
            "audioOutput": if android { "unsupported" } else { "host-dependent" },
            "globalHotkeys": if android { "unsupported" } else { "plugin-dependent" },
            "nativeFilePicker": if android { "unsupported" } else { "host-dependent" },
            "webView": if android { "unsupported" } else { "host-dependent" },
            "desktopNotifications": if android { "unsupported" } else { "host-dependent" },
        })
    }
}

fn termux_signals(signalled: bool, prefix: Option<&Path>) -> bool {
    // An Android OS alone, or a forged PREFIX alone, is insufficient.
    signalled
        && prefix.is_some_and(|p| {
            p.is_absolute() && p.join("bin/pkg").is_file() && p.join("bin/termux-info").is_file()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn android_capabilities_fail_closed_for_unvalidated_native_backends() {
        for platform in [
            RuntimePlatform::AndroidTermux,
            RuntimePlatform::AndroidOther,
        ] {
            let caps = platform.capabilities();
            assert_eq!(caps["nativePlugins"], false);
            assert_eq!(caps["nativeNodeAddons"], false);
            assert_eq!(caps["webView"], "unsupported");
            assert_eq!(caps["processPlugins"], true);
        }
    }
    #[test]
    fn android_without_termux_environment_is_not_termux() {
        assert!(!termux_signals(false, Some(Path::new("/"))));
        assert!(!termux_signals(true, None));
        assert!(!termux_signals(true, Some(Path::new("relative"))));
    }
}
