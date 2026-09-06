//! Human-readable diagnostics for the `hotkey.diagnostics` action.
//!
//! Never prints captured keystrokes or key sequences: only backend states,
//! device counts, environment presence, and recommendations.

use super::detect::{desktop_label, detect_session, is_xwayland_display, EnvSnapshot};
use super::shortcuts::Chord;
use super::state::{overall_status, SharedStatus};

/// Renders the full diagnostics report. `bound` lists portal-registered
/// chords; `sequences_needed` records whether raw input is desired.
pub fn render_diagnostics(
    status: &SharedStatus,
    env: &EnvSnapshot,
    bound: &[Chord],
    sequences_needed: bool,
) -> String {
    let session = detect_session(env);
    let (_, overall) = overall_status(&status.backends);
    let mut out = String::from("TikTools Hotkey Diagnostics\n\n");
    out.push_str(&format!("OS:\n  {}\n\n", std::env::consts::OS));
    out.push_str("Session:\n");
    out.push_str(&format!("  {}\n", session.as_str()));
    out.push_str(&format!("  desktop: {}\n\n", desktop_label(env)));
    out.push_str("Environment:\n");
    for (key, value) in [
        ("XDG_SESSION_TYPE", &env.session_type),
        ("WAYLAND_DISPLAY", &env.wayland_display),
        ("DISPLAY", &env.display),
        ("XDG_CURRENT_DESKTOP", &env.current_desktop),
        ("DESKTOP_SESSION", &env.desktop_session),
    ] {
        match value {
            Some(value) if !value.trim().is_empty() => {
                out.push_str(&format!("  {key}={value}\n"));
            }
            _ => out.push_str(&format!("  {key}=(unset)\n")),
        }
    }
    if is_xwayland_display(env) {
        out.push_str("  note: DISPLAY is provided by XWayland; native session is Wayland\n");
    }
    out.push('\n');
    out.push_str("Backends:\n");
    if status.backends.is_empty() {
        out.push_str("  (no backend started yet)\n");
    }
    for report in &status.backends {
        out.push_str(&format!(
            "  {}:\n    status: {}\n",
            report.backend.display_name(),
            report.state.as_str()
        ));
        if !report.detail.is_empty() {
            out.push_str(&format!("    detail: {}\n", report.detail));
        }
    }
    if let Some(summary) = &status.evdev_summary {
        out.push_str(&format!("  evdev devices: {summary}\n"));
    }
    out.push('\n');
    out.push_str("Selected:\n");
    out.push_str(&format!("  {overall}\n\n"));
    out.push_str("Capabilities:\n");
    out.push_str(&format!("  registered shortcuts: {}\n", bound.len()));
    for chord in bound {
        out.push_str(&format!(
            "    - {} ({})\n",
            chord.description(),
            chord.shortcut_id()
        ));
    }
    out.push_str(&format!(
        "  arbitrary key sequences: {}\n\n",
        if sequences_needed {
            "requested (raw input backend)"
        } else {
            "not requested (portal chords only)"
        }
    ));
    out.push_str("Recommendation:\n");
    out.push_str(&format!("  {}\n", recommendation(status, sequences_needed)));
    if !status.notes.is_empty() {
        out.push_str("\nNotes:\n");
        for note in &status.notes {
            out.push_str(&format!("  - {note}\n"));
        }
    }
    out
}

fn recommendation(status: &SharedStatus, sequences_needed: bool) -> String {
    use super::state::{BackendRunState, HotkeyBackend};
    let permission = status.backends.iter().find(|report| {
        report.backend == HotkeyBackend::Evdev
            && report.state == BackendRunState::PermissionRequired
    });
    if let Some(report) = permission {
        if sequences_needed {
            return format!(
                "Grant raw-input access to enable sequence triggers: {} See docs/HOTKEYS_LINUX.md for the udev/input-group setup.",
                report.detail
            );
        }
        return "Raw-input permission is missing, but no sequence triggers are configured; chord shortcuts are unaffected.".to_owned();
    }
    if status.backends.iter().any(|report| {
        report.backend == HotkeyBackend::Portal && report.state == BackendRunState::Running
    }) {
        if sequences_needed
            && !status.backends.iter().any(|report| {
                report.backend == HotkeyBackend::Evdev && report.state == BackendRunState::Running
            })
        {
            return "Portal shortcuts are active. Enable raw-input permission only if sequence triggers are needed.".to_owned();
        }
        return "Hotkey listeners are active.".to_owned();
    }
    "No hotkey backend is active; check the backend details above.".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkeys::detect::EnvSnapshot;
    use crate::hotkeys::state::{BackendReport, BackendRunState, HotkeyBackend};

    fn wayland_env() -> EnvSnapshot {
        EnvSnapshot::from_pairs(&[
            ("XDG_SESSION_TYPE", "wayland"),
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("DISPLAY", ":0"),
            ("XDG_CURRENT_DESKTOP", "KDE"),
        ])
    }

    #[test]
    fn diagnostics_never_leak_keystrokes() {
        let mut status = SharedStatus::default();
        status.set_platform("linux", detect_session(&wayland_env()), "KDE");
        status.upsert_backend(BackendReport::new(
            HotkeyBackend::Portal,
            BackendRunState::Running,
            "3 shortcuts bound",
        ));
        status.set_evdev_summary("discovered 3, readable 0");
        let bound = vec![Chord::new("k", "ctrl+shift").unwrap()];
        let report = render_diagnostics(&status, &wayland_env(), &bound, true);
        assert!(report.contains("wayland"));
        assert!(report.contains("XWayland"));
        assert!(report.contains("discovered 3, readable 0"));
        assert!(report.contains("ctrl-shift-k"));
        // No sequence contents, no key contents beyond registered chords.
        assert!(!report.contains("g o"));
    }

    #[test]
    fn diagnostics_recommend_permission_only_for_sequences() {
        let mut status = SharedStatus::default();
        status.upsert_backend(BackendReport::new(
            HotkeyBackend::Evdev,
            BackendRunState::PermissionRequired,
            "no readable /dev/input/event* devices",
        ));
        let chords_only = render_diagnostics(&status, &EnvSnapshot::default(), &[], false);
        assert!(chords_only.contains("unaffected"), "{chords_only}");
        let sequences = render_diagnostics(&status, &EnvSnapshot::default(), &[], true);
        assert!(sequences.contains("Grant raw-input access"), "{sequences}");
    }
}
