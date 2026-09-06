//! Linux session detection.
//!
//! Backend selection must be runtime-based: `DISPLAY` is frequently present
//! inside Wayland sessions through XWayland, so it must never be treated as
//! proof of a native X11 session. `XDG_SESSION_TYPE` is authoritative when
//! set; otherwise the display sockets disambiguate.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxSession {
    Wayland,
    X11,
    Unknown,
}

impl LinuxSession {
    pub fn as_str(self) -> &'static str {
        match self {
            LinuxSession::Wayland => "wayland",
            LinuxSession::X11 => "x11",
            LinuxSession::Unknown => "unknown",
        }
    }
}

/// The subset of the environment that determines session topology.
#[derive(Debug, Clone, Default)]
pub struct EnvSnapshot {
    pub session_type: Option<String>,
    pub wayland_display: Option<String>,
    pub display: Option<String>,
    pub current_desktop: Option<String>,
    pub desktop_session: Option<String>,
}

impl EnvSnapshot {
    pub fn from_env() -> Self {
        Self {
            session_type: std::env::var("XDG_SESSION_TYPE").ok(),
            wayland_display: std::env::var("WAYLAND_DISPLAY").ok(),
            display: std::env::var("DISPLAY").ok(),
            current_desktop: std::env::var("XDG_CURRENT_DESKTOP").ok(),
            desktop_session: std::env::var("DESKTOP_SESSION").ok(),
        }
    }

    #[cfg(test)]
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        let mut snapshot = Self::default();
        for (key, value) in pairs {
            let value = value.to_string();
            match *key {
                "XDG_SESSION_TYPE" => snapshot.session_type = Some(value),
                "WAYLAND_DISPLAY" => snapshot.wayland_display = Some(value),
                "DISPLAY" => snapshot.display = Some(value),
                "XDG_CURRENT_DESKTOP" => snapshot.current_desktop = Some(value),
                "DESKTOP_SESSION" => snapshot.desktop_session = Some(value),
                _ => {}
            }
        }
        snapshot
    }
}

fn is_set(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|value| !value.trim().is_empty())
}

/// Classifies the Linux session. `XDG_SESSION_TYPE` wins when it names a
/// known session; otherwise a set `WAYLAND_DISPLAY` implies Wayland even
/// when `DISPLAY` is also present (the XWayland case), and a lone `DISPLAY`
/// implies X11.
pub fn detect_session(env: &EnvSnapshot) -> LinuxSession {
    if let Some(session_type) = env
        .session_type
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
    {
        match session_type.as_str() {
            "wayland" => return LinuxSession::Wayland,
            "x11" => return LinuxSession::X11,
            _ => {}
        }
    }
    if is_set(&env.wayland_display) {
        return LinuxSession::Wayland;
    }
    if is_set(&env.display) {
        return LinuxSession::X11;
    }
    LinuxSession::Unknown
}

/// True when `DISPLAY` exists only because the Wayland compositor runs
/// XWayland. The X11 listener would attach to the XWayland socket and miss
/// native Wayland windows, so the portal backend must be preferred.
pub fn is_xwayland_display(env: &EnvSnapshot) -> bool {
    detect_session(env) == LinuxSession::Wayland && is_set(&env.display)
}

/// Human desktop label for diagnostics (`KDE`, `GNOME`, ...), normalized to
/// uppercase without version suffixes. Never used for backend selection.
pub fn desktop_label(env: &EnvSnapshot) -> String {
    let raw = env
        .current_desktop
        .clone()
        .or_else(|| env.desktop_session.clone())
        .unwrap_or_default();
    let first = raw.split([':', ';']).next().unwrap_or("").trim();
    if first.is_empty() {
        return "unknown".to_owned();
    }
    first.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authoritative_session_type_wins() {
        let env = EnvSnapshot::from_pairs(&[
            ("XDG_SESSION_TYPE", "wayland"),
            ("DISPLAY", ":0"),
            ("WAYLAND_DISPLAY", "wayland-0"),
        ]);
        assert_eq!(detect_session(&env), LinuxSession::Wayland);

        let env = EnvSnapshot::from_pairs(&[("XDG_SESSION_TYPE", "x11"), ("DISPLAY", ":0")]);
        assert_eq!(detect_session(&env), LinuxSession::X11);
    }

    #[test]
    fn xwayland_display_is_not_native_x11() {
        // No session-type hint, but both sockets are present: Wayland with
        // XWayland, exactly what KDE Plasma Wayland and GNOME Wayland expose.
        let env = EnvSnapshot::from_pairs(&[("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")]);
        assert_eq!(detect_session(&env), LinuxSession::Wayland);
        assert!(is_xwayland_display(&env));
    }

    #[test]
    fn lone_display_means_x11() {
        let env = EnvSnapshot::from_pairs(&[("DISPLAY", ":0")]);
        assert_eq!(detect_session(&env), LinuxSession::X11);
        assert!(!is_xwayland_display(&env));
    }

    #[test]
    fn empty_environment_is_unknown() {
        assert_eq!(
            detect_session(&EnvSnapshot::default()),
            LinuxSession::Unknown
        );
        let env = EnvSnapshot::from_pairs(&[
            ("XDG_SESSION_TYPE", "tty"),
            ("DISPLAY", ""),
            ("WAYLAND_DISPLAY", "  "),
        ]);
        assert_eq!(detect_session(&env), LinuxSession::Unknown);
    }

    #[test]
    fn desktop_label_is_diagnostic_only() {
        let env = EnvSnapshot::from_pairs(&[("XDG_CURRENT_DESKTOP", "KDE")]);
        assert_eq!(desktop_label(&env), "KDE");
        let env = EnvSnapshot::from_pairs(&[("DESKTOP_SESSION", "gnome-wayland")]);
        assert_eq!(desktop_label(&env), "GNOME-WAYLAND");
        assert_eq!(desktop_label(&EnvSnapshot::default()), "unknown");
    }
}
