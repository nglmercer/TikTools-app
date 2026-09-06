//! Raw Linux input backend (`/dev/input/event*`) for arbitrary keys and
//! key sequences on Wayland, and as an X11 fallback.
//!
//! The portal backend intentionally cannot observe arbitrary input, so
//! `sequence contains` behaviors need this path. It opens each keyboard
//! device read-only through the maintained `evdev` crate and merges all
//! keyboards into the shared [`KeyState`].
//!
//! # Permissions (no root, no 777)
//!
//! Device nodes are typically `root:input` `660`. The supported setups, in
//! order of preference, are:
//!
//! 1. Logind seat ACLs (nothing to do on most desktop distros when the user
//!    is on the active local seat);
//! 2. adding the user to the `input` group (`sudo usermod -aG input $USER`
//!    then re-login) — grants read access to *all* input devices, so it is
//!    still a sensitive grant, but it never touches TikTools itself;
//! 3. a narrow udev rule granting one keyboard device to one group.
//!
//! Never run TikTools as root and never `chmod 777 /dev/input/event*`. The
//! backend only forwards normalized `(key, pressed)` pairs; it cannot read
//! arbitrary files or devices.

use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use evdev::{Device, EventSummary};

use super::event::{evdev_key_name, is_modifier};
use super::state::{BackendReport, BackendRunState, HotkeyBackend};
use super::{emit_press, BackendHandles};

const RESCAN_SECS: u64 = 10;

/// Automatic per-device permission hint shown when no device is readable.
pub const EVDEV_PERMISSION_HINT: &str =
    "no readable /dev/input/event* devices; TikTools will request a per-device ACL automatically (see docs/HOTKEYS_LINUX.md) — never run TikTools as root";

pub fn spawn_evdev_listener(handles: BackendHandles) {
    set_status(
        &handles,
        BackendRunState::Starting,
        "scanning /dev/input/event* keyboards",
    );
    std::thread::Builder::new()
        .name("tiktools-hotkey-evdev".to_owned())
        .spawn(move || supervise_keyboards(handles))
        .ok();
}

fn supervise_keyboards(handles: BackendHandles) {
    let mut managed: HashSet<PathBuf> = HashSet::new();
    let mut permission_requests: HashSet<PathBuf> = HashSet::new();
    loop {
        if !sequences_enabled(&handles) {
            set_status(
                &handles,
                BackendRunState::Unsupported,
                "sequence triggers disabled in configuration (hotkey.bind sequencesNeeded=false)",
            );
            std::thread::sleep(Duration::from_secs(RESCAN_SECS));
            continue;
        }
        let survey = survey_devices();
        handles
            .shared
            .lock()
            .map(|mut shared| {
                shared.set_evdev_summary(format!(
                    "discovered {}, readable {}",
                    survey.discovered,
                    survey.readable_keyboards.len()
                ));
            })
            .ok();
        if survey.readable_keyboards.is_empty() {
            if survey.permission_denied > 0 {
                let pending = survey
                    .permission_denied_paths
                    .iter()
                    .filter(|path| !permission_requests.contains(*path))
                    .cloned()
                    .collect::<Vec<_>>();
                if !pending.is_empty() {
                    set_status(
                        &handles,
                        BackendRunState::Starting,
                        "requesting raw-input permission from the system",
                    );
                    let request_result = request_device_access(&pending);
                    permission_requests.extend(pending);
                    if let Err(error) = request_result {
                        set_status(
                            &handles,
                            BackendRunState::PermissionRequired,
                            format!("{EVDEV_PERMISSION_HINT}; automatic request failed: {error}"),
                        );
                    }
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }
                set_status(&handles, BackendRunState::PermissionRequired, EVDEV_PERMISSION_HINT);
            } else {
                set_status(
                    &handles,
                    BackendRunState::Unsupported,
                    "no keyboard devices found under /dev/input",
                );
            }
            std::thread::sleep(Duration::from_secs(RESCAN_SECS));
            continue;
        }
        let current: HashSet<PathBuf> = survey.readable_keyboards.iter().cloned().collect();
        if current != managed {
            // Hotplug/reconnect: drop possibly-stuck holds from vanished
            // devices before serving the new set.
            if let Ok(mut state) = handles.key_state.lock() {
                state.reset();
            }
            for path in current.difference(&managed) {
                spawn_device_reader(handles.clone(), path.clone());
            }
            managed = current;
            let count = survey.readable_keyboards.len();
            set_status(
                &handles,
                BackendRunState::Running,
                format!("{count} keyboard device(s)"),
            );
        }
        std::thread::sleep(Duration::from_secs(RESCAN_SECS));
    }
}

fn sequences_enabled(handles: &BackendHandles) -> bool {
    handles
        .config
        .lock()
        .map(|config| config.sequences_needed)
        .unwrap_or(true)
}

fn portal_running(handles: &BackendHandles) -> bool {
    handles
        .shared
        .lock()
        .map(|shared| {
            shared.backends.iter().any(|report| {
                report.backend == HotkeyBackend::Portal && report.state == BackendRunState::Running
            })
        })
        .unwrap_or(false)
}

fn spawn_device_reader(handles: BackendHandles, path: PathBuf) {
    std::thread::Builder::new()
        .name(format!(
            "tiktools-hotkey-evdev-{}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("input")
        ))
        .spawn(move || read_device_loop(handles, path))
        .ok();
}

fn read_device_loop(handles: BackendHandles, path: PathBuf) {
    let mut device = match Device::open(&path) {
        Ok(device) => device,
        Err(_) => return,
    };
    loop {
        if !sequences_enabled(&handles) {
            // Disabled while running: stop consuming so chord-only users
            // leave no raw-input footprint; the supervisor reports it.
            std::thread::sleep(Duration::from_secs(1));
            continue;
        }
        let batch = match device.fetch_events() {
            Ok(batch) => batch,
            Err(_) => break, // Device vanished; the supervisor re-enumerates.
        };
        for event in batch {
            handle_input_event(&handles, event);
        }
    }
}

fn handle_input_event(handles: &BackendHandles, event: evdev::InputEvent) {
    let EventSummary::Key(_, code, value) = event.destructure() else {
        return;
    };
    let Some(name) = evdev_key_name(code) else {
        return;
    };
    // value: 0 = release, 1 = press, 2 = auto-repeat (deduped by KeyState).
    let pressed = value != 0;
    if is_modifier(&name) {
        handles
            .key_state
            .lock()
            .ok()
            .map(|mut state| state.apply(&name, pressed, super::now_ms()));
        return;
    }
    if pressed && portal_running(handles) && portal_owns_press(handles, &name) {
        // A portal-registered chord: the portal backend owns the event and
        // the sequence slot. Emitting here too would double-trigger.
        return;
    }
    emit_press(
        handles,
        &name,
        pressed,
        HotkeyBackend::Evdev.name(),
        None,
        None,
    );
}

/// True when `(name + currently held modifiers)` is a portal-bound chord.
fn portal_owns_press(handles: &BackendHandles, name: &str) -> bool {
    let modifiers = handles
        .key_state
        .lock()
        .map(|state| state.modifiers_snapshot())
        .unwrap_or_default();
    handles
        .config
        .lock()
        .map(|config| config.is_portal_chord(name, &modifiers))
        .unwrap_or(false)
}

/// Read-only view of a device-survey round.
struct DeviceSurvey {
    discovered: usize,
    readable_keyboards: Vec<PathBuf>,
    permission_denied: usize,
    permission_denied_paths: Vec<PathBuf>,
}

/// Lists `/dev/input/event*` nodes, counts them, and keeps the readable
/// keyboards. Pure I/O; never touches shared state.
fn survey_devices() -> DeviceSurvey {
    survey_devices_in(std::path::Path::new("/dev/input"))
}

fn survey_devices_in(root: &std::path::Path) -> DeviceSurvey {
    let mut survey = DeviceSurvey {
        discovered: 0,
        readable_keyboards: Vec::new(),
        permission_denied: 0,
        permission_denied_paths: Vec::new(),
    };
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return survey,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("event") {
            continue;
        }
        survey.discovered += 1;
        let path = entry.path();
        match Device::open(&path) {
            Ok(device) => {
                if is_keyboard(&device) {
                    survey.readable_keyboards.push(path);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                survey.permission_denied += 1;
                survey.permission_denied_paths.push(path);
            }
            Err(_) => {}
        }
    }
    survey.readable_keyboards.sort();
    survey.permission_denied_paths.sort();
    survey
}

/// Requests a narrow, immediate read ACL for the denied event nodes. Polkit
/// displays the authentication prompt; the TikTools process itself never
/// becomes root and no broad `input` group membership is changed.
fn request_device_access(paths: &[PathBuf]) -> Result<(), String> {
    let uid = current_uid()?;
    let rule = format!("u:{uid}:r");
    let pkexec = system_program("pkexec")
        .ok_or_else(|| "pkexec is not installed".to_owned())?;
    let setfacl = system_program("setfacl")
        .ok_or_else(|| "setfacl is not installed (install the acl package)".to_owned())?;
    let output = Command::new(pkexec)
        .arg(setfacl)
        .arg("-m")
        .arg(rule)
        .args(paths)
        .output()
        .map_err(|error| format!("could not start the system permission helper: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if stderr.is_empty() {
        Err(format!("system permission helper exited with {}", output.status))
    } else {
        Err(stderr.chars().take(240).collect())
    }
}

fn current_uid() -> Result<String, String> {
    let id = system_program("id").ok_or_else(|| "id is not installed".to_owned())?;
    let output = Command::new(id)
        .arg("-u")
        .output()
        .map_err(|error| format!("could not determine the current user id: {error}"))?;
    if !output.status.success() {
        return Err("could not determine the current user id".to_owned());
    }
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if uid.is_empty() || !uid.chars().all(|character| character.is_ascii_digit()) {
        return Err("the current user id was invalid".to_owned());
    }
    Ok(uid)
}

fn system_program(name: &str) -> Option<PathBuf> {
    ["/usr/bin", "/usr/sbin", "/bin", "/sbin"]
        .into_iter()
        .map(|directory| PathBuf::from(directory).join(name))
        .find(|path| path.is_file())
}

/// Keyboard heuristic: the device exposes the core alphanumeric block.
fn is_keyboard(device: &Device) -> bool {
    use evdev::KeyCode as K;
    device.supported_keys().is_some_and(|keys| {
        keys.contains(K::KEY_A) && (keys.contains(K::KEY_ENTER) || keys.contains(K::KEY_SPACE))
    })
}

fn set_status(handles: &BackendHandles, state: BackendRunState, detail: impl Into<String>) {
    handles
        .shared
        .lock()
        .map(|mut shared| {
            shared.upsert_backend(BackendReport::new(HotkeyBackend::Evdev, state, detail));
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_hint_points_at_docs_not_root() {
        assert!(EVDEV_PERMISSION_HINT.contains("docs/HOTKEYS_LINUX.md"));
        assert!(EVDEV_PERMISSION_HINT.contains("never run TikTools as root"));
    }

    #[test]
    fn current_uid_is_numeric() {
        assert!(current_uid().is_ok_and(|uid| !uid.is_empty()));
    }
}
