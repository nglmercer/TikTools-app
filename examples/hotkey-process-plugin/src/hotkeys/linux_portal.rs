//! XDG Desktop Portal GlobalShortcuts backend (preferred on Wayland).
//!
//! Wayland forbids arbitrary global key observation, so compositor-authorized
//! chords are registered up front with `BindShortcuts` and the compositor
//! wakes the plugin through `Activated`. This covers ordinary shortcuts such
//! as `Ctrl+Shift+K`; arbitrary keys and `sequence contains` filters stay on
//! the evdev backend. Implemented with the maintained `ashpd` crate over
//! D-Bus — never shell-outs to `gdbus`/`dbus-send`.

use std::time::Duration;

use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use ashpd::desktop::CreateSessionOptions;
use futures_util::StreamExt;

use super::shortcuts::{portal_trigger_string, Chord};
use super::state::{BackendReport, BackendRunState, HotkeyBackend};
use super::{emit_press, now_ms, BackendHandles};

const REBIND_POLL_SECS: u64 = 5;

pub fn spawn_portal_listener(handles: BackendHandles) {
    set_status(
        &handles,
        BackendRunState::Starting,
        "connecting to the desktop portal",
    );
    std::thread::Builder::new()
        .name("tiktools-hotkey-portal".to_owned())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    set_status(
                        &handles,
                        BackendRunState::Failed,
                        format!("could not start portal runtime: {error}"),
                    );
                    return;
                }
            };
            runtime.block_on(portal_main(handles));
        })
        .ok();
}

async fn portal_main(handles: BackendHandles) {
    let proxy = match GlobalShortcuts::new().await {
        Ok(proxy) => proxy,
        Err(error) => {
            set_status(
                &handles,
                BackendRunState::Unsupported,
                format!(
                    "desktop portal unavailable ({error}); is xdg-desktop-portal running with DBUS_SESSION_BUS_ADDRESS set?"
                ),
            );
            return;
        }
    };
    if proxy.version() == 0 {
        set_status(
            &handles,
            BackendRunState::Unsupported,
            "portal GlobalShortcuts interface not exposed by this compositor/back-end",
        );
        return;
    }
    let session = match proxy.create_session(CreateSessionOptions::default()).await {
        Ok(session) => session,
        Err(error) => {
            let message = error.to_string();
            set_status(&handles, classify_portal_error(&message), message);
            return;
        }
    };
    let mut activations = match proxy.receive_activated().await {
        Ok(stream) => stream,
        Err(error) => {
            let message = error.to_string();
            set_status(&handles, classify_portal_error(&message), message);
            return;
        }
    };
    // The session must outlive the loop; dropping it unbinds everything.
    let _session_guard = &session;
    let mut ticker = tokio::time::interval(Duration::from_secs(REBIND_POLL_SECS));
    let mut last_bound_generation: Option<u64> = None;
    loop {
        tokio::select! {
            activation = activations.next() => {
                let Some(activation) = activation else {
                    set_status(
                        &handles,
                        BackendRunState::Failed,
                        "portal activation stream ended; restart TikTools to reattach",
                    );
                    return;
                };
                on_portal_activation(&handles, activation.shortcut_id());
            }
            _ = ticker.tick() => {
                let (chords, generation) = handles
                    .config
                    .lock()
                    .map(|config| (config.bindings.clone(), config.generation))
                    .unwrap_or_default();
                if last_bound_generation == Some(generation) {
                    continue;
                }
                match bind_current(&proxy, &session, &chords).await {
                    Ok(bound) => {
                        last_bound_generation = Some(generation);
                        set_status(
                            &handles,
                            BackendRunState::Running,
                            format!("{bound} shortcuts bound"),
                        );
                    }
                    Err(message) => {
                        set_status(&handles, classify_portal_error(&message), message);
                    }
                }
            }
        }
    }
}

async fn bind_current(
    proxy: &GlobalShortcuts,
    session: &ashpd::desktop::Session<GlobalShortcuts>,
    chords: &[Chord],
) -> Result<usize, String> {
    if chords.is_empty() {
        return Ok(0);
    }
    let mut shortcuts = Vec::with_capacity(chords.len());
    let mut triggers = Vec::with_capacity(chords.len());
    for chord in chords {
        // `preferred_trigger` copies the trigger; the owned string must
        // outlive the async bind call, hence the side table.
        triggers.push(chord_trigger(chord));
        let descriptor = NewShortcut::new(chord.shortcut_id(), chord.description());
        shortcuts.push(match &triggers[triggers.len() - 1] {
            Some(trigger) => descriptor.preferred_trigger(Some(trigger.as_str())),
            None => descriptor,
        });
    }
    let request = proxy
        .bind_shortcuts(session, &shortcuts, None, BindShortcutsOptions::default())
        .await
        .map_err(|error| error.to_string())?;
    request.response().map_err(|error| error.to_string())?;
    Ok(shortcuts.len())
}

/// Preferred trigger for one chord, or `None` when the key has no portal
/// spelling (punctuation, media keys): the shortcut is still registered and
/// the user assigns the trigger in system settings.
fn chord_trigger(chord: &Chord) -> Option<String> {
    portal_trigger_string(chord)
}

fn on_portal_activation(handles: &BackendHandles, shortcut_id: &str) {
    let chord = handles.config.lock().ok().and_then(|config| {
        config
            .bindings
            .iter()
            .find(|entry| entry.shortcut_id() == shortcut_id)
            .cloned()
    });
    let Some(chord) = chord else {
        // Stale activation for a chord that was unbound; ignore it.
        return;
    };
    // A portal activation has no release event. Synthesize press+release so
    // the rolling sequence advances exactly like a raw press.
    emit_press(
        handles,
        &chord.key,
        true,
        HotkeyBackend::Portal.name(),
        Some(shortcut_id.to_owned()),
        Some(&chord.modifiers),
    );
    handles
        .key_state
        .lock()
        .ok()
        .map(|mut state| state.apply(&chord.key, false, now_ms()));
}

fn classify_portal_error(message: &str) -> BackendRunState {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("denied")
        || lowered.contains("not allowed")
        || lowered.contains("permission")
        || lowered.contains("access")
    {
        BackendRunState::PermissionRequired
    } else if lowered.contains("not supported")
        || lowered.contains("unknown method")
        || lowered.contains("unknown interface")
        || lowered.contains("no such")
        || lowered.contains("not implemented")
    {
        BackendRunState::Unsupported
    } else {
        BackendRunState::Failed
    }
}

fn set_status(handles: &BackendHandles, state: BackendRunState, detail: impl Into<String>) {
    handles
        .shared
        .lock()
        .map(|mut shared| {
            shared.upsert_backend(BackendReport::new(HotkeyBackend::Portal, state, detail));
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_errors_map_to_actionable_states() {
        assert_eq!(
            classify_portal_error(
                "GDBus.Error:org.freedesktop.portal.Error.NotAllowed: access denied"
            ),
            BackendRunState::PermissionRequired
        );
        assert_eq!(
            classify_portal_error(
                "GDBus.Error:org.freedesktop.DBus.Error.UnknownMethod: no such method"
            ),
            BackendRunState::Unsupported
        );
        assert_eq!(
            classify_portal_error("connection reset by peer"),
            BackendRunState::Failed
        );
    }
}
