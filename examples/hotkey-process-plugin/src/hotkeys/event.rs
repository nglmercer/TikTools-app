//! Shared key-state tracking behind `event.data.key/modifiers/sequence`.
//!
//! Every backend (rdev on Windows/macOS/X11, evdev on Linux, portal
//! activations on Wayland) feeds normalized `(name, pressed)` pairs into
//! [`KeyState`]. The contract published to behaviors is unchanged:
//! `key` is the normalized non-modifier key, `modifiers` is the canonical
//! `ctrl+shift+alt+meta` chord, and `sequence` is the rolling history of the
//! last [`MAX_SEQUENCE_KEYS`] non-modifier presses.

use std::collections::{BTreeSet, VecDeque};

/// Rolling history behind `event.data.sequence`.
pub const MAX_SEQUENCE_KEYS: usize = 8;
/// Backpressure cap; the host additionally caps 16 events per poll tick.
pub const MAX_PENDING_EVENTS: usize = 64;
/// Idle time after which held-key state is discarded. Releases can be lost
/// across sleep/resume, device hotplug, focus changes, or missed grabs; a
/// stuck modifier would otherwise poison every later chord.
pub const STUCK_KEY_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Default)]
pub struct KeyState {
    /// Currently held non-modifier keys (kills auto-repeat duplicates).
    pressed: BTreeSet<String>,
    modifiers: BTreeSet<String>,
    sequence: VecDeque<String>,
    /// Monotonic tick (ms) of the last press/release, for stuck-key expiry.
    last_activity_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPressRecord {
    pub key: String,
    pub modifiers: String,
    pub sequence: String,
}

impl KeyState {
    /// Records one normalized key transition. Returns a press record for
    /// non-modifier presses, `None` for releases, modifier-only traffic, and
    /// auto-repeat duplicates. `now_ms` is monotonic milliseconds and may come
    /// from any steady clock.
    pub fn apply(&mut self, name: &str, pressed: bool, now_ms: u64) -> Option<KeyPressRecord> {
        self.expire_stuck_keys(now_ms);
        self.last_activity_ms = now_ms;
        if is_modifier(name) {
            if pressed {
                self.modifiers.insert(name.to_owned());
            } else {
                self.modifiers.remove(name);
            }
            return None;
        }
        if pressed {
            // Auto-repeat: the OS re-sends presses while a key is held.
            if !self.pressed.insert(name.to_owned()) {
                return None;
            }
            self.sequence.push_back(name.to_owned());
            while self.sequence.len() > MAX_SEQUENCE_KEYS {
                self.sequence.pop_front();
            }
            Some(KeyPressRecord {
                key: name.to_owned(),
                modifiers: canonical_modifiers(&self.modifiers),
                sequence: self.sequence.iter().cloned().collect::<Vec<_>>().join(" "),
            })
        } else {
            self.pressed.remove(name);
            None
        }
    }

    /// Canonical snapshot of currently held modifiers (`ctrl+shift`, ...).
    /// Used by the evdev backend to decide whether a press belongs to a
    /// portal-registered chord without mutating shared state. Linux-only.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn modifiers_snapshot(&self) -> String {
        canonical_modifiers(&self.modifiers)
    }

    /// Drops all in-flight state. Called on backend (re)connect, device
    /// re-enumeration, and session resume so a missed release cannot leave a
    /// modifier or key permanently stuck.
    pub fn reset(&mut self) {
        self.pressed.clear();
        self.modifiers.clear();
        // The rolling sequence is history, not held state, so it survives.
    }

    fn expire_stuck_keys(&mut self, now_ms: u64) {
        if now_ms.saturating_sub(self.last_activity_ms)
            > STUCK_KEY_TIMEOUT_SECS.saturating_mul(1000)
        {
            self.pressed.clear();
            self.modifiers.clear();
        }
    }

    #[cfg(test)]
    pub fn held_for_tests(&self) -> (Vec<String>, Vec<String>) {
        (
            self.pressed.iter().cloned().collect(),
            self.modifiers.iter().cloned().collect(),
        )
    }
}

pub fn is_modifier(name: &str) -> bool {
    matches!(name, "shift" | "ctrl" | "alt" | "meta")
}

/// Conventional chord order (ctrl+shift+alt+meta) so recorded combos match
/// what users type in filters, independent of set iteration order.
pub fn canonical_modifiers(modifiers: &BTreeSet<String>) -> String {
    let mut ordered: Vec<&String> = modifiers.iter().collect();
    ordered.sort_by_key(|name| modifier_rank(name));
    ordered.into_iter().cloned().collect::<Vec<_>>().join("+")
}

fn modifier_rank(name: &str) -> u8 {
    match name {
        "ctrl" => 0,
        "shift" => 1,
        "alt" => 2,
        "meta" => 3,
        _ => 4,
    }
}

/// Stable, layout-independent key names derived from the rdev debug label so
/// new `Key` variants degrade to a readable fallback instead of breaking the
/// build or the event stream.
pub fn key_name(key: &rdev::Key) -> String {
    match format!("{key:?}").as_str() {
        "ShiftLeft" | "ShiftRight" => "shift".to_owned(),
        "ControlLeft" | "ControlRight" => "ctrl".to_owned(),
        // AltGr produces the local third-level chooser; it behaves as Alt
        // for chord matching and must never stick as its own modifier.
        "Alt" | "AltGr" => "alt".to_owned(),
        "MetaLeft" | "MetaRight" => "meta".to_owned(),
        "Space" => "space".to_owned(),
        "Return" => "enter".to_owned(),
        "Tab" => "tab".to_owned(),
        "Escape" => "esc".to_owned(),
        "Backspace" => "backspace".to_owned(),
        "Delete" => "delete".to_owned(),
        "Insert" => "insert".to_owned(),
        "Home" => "home".to_owned(),
        "End" => "end".to_owned(),
        "PageUp" => "pageup".to_owned(),
        "PageDown" => "pagedown".to_owned(),
        "UpArrow" => "up".to_owned(),
        "DownArrow" => "down".to_owned(),
        "LeftArrow" => "left".to_owned(),
        "RightArrow" => "right".to_owned(),
        "CapsLock" => "capslock".to_owned(),
        "NumLock" => "numlock".to_owned(),
        "ScrollLock" => "scrolllock".to_owned(),
        "PrintScreen" => "printscreen".to_owned(),
        "Pause" => "pause".to_owned(),
        "Comma" => ",".to_owned(),
        "Dot" => ".".to_owned(),
        "Slash" => "/".to_owned(),
        "SemiColon" => ";".to_owned(),
        "Quote" => "'".to_owned(),
        "LeftBracket" => "[".to_owned(),
        "RightBracket" => "]".to_owned(),
        "BackSlash" => "\\".to_owned(),
        "Minus" => "-".to_owned(),
        "Equal" => "=".to_owned(),
        "BackQuote" => "`".to_owned(),
        "Multiply" => "*".to_owned(),
        "Add" => "+".to_owned(),
        "Subtract" => "-".to_owned(),
        "Decimal" => ".".to_owned(),
        "Divide" => "/".to_owned(),
        "KpReturn" => "enter".to_owned(),
        "KpMinus" => "-".to_owned(),
        "KpPlus" => "+".to_owned(),
        "KpMultiply" => "*".to_owned(),
        "KpDivide" => "/".to_owned(),
        "KpDecimal" => ".".to_owned(),
        "KpEqual" => "=".to_owned(),
        "KpComma" => ",".to_owned(),
        name if name.len() == 4 && name.starts_with("Key") => name[3..].to_lowercase(),
        name if name.len() == 4 && name.starts_with("Num") => name[3..].to_owned(),
        name if name.starts_with('F')
            && name.len() <= 3
            && name[1..].chars().all(|c| c.is_ascii_digit()) =>
        {
            name.to_lowercase()
        }
        name if name.starts_with("Numpad") && name.len() == 7 => name[6..].to_owned(),
        name if name.starts_with("Kp") && name.len() == 3 => name[2..].to_owned(),
        other => other.to_lowercase(),
    }
}

/// Normalized names for evdev `KEY_*` codes. Returns `None` for non-keyboard
/// codes (mouse buttons, touch, unmapped). Modifier codes map to the same
/// canonical modifier names as [`key_name`] so both backends emit one
/// contract.
#[cfg(target_os = "linux")]
pub fn evdev_key_name(code: evdev::KeyCode) -> Option<String> {
    use evdev::KeyCode as K;
    let name: &str = match code {
        K::KEY_ESC => "esc",
        K::KEY_1 => "1",
        K::KEY_2 => "2",
        K::KEY_3 => "3",
        K::KEY_4 => "4",
        K::KEY_5 => "5",
        K::KEY_6 => "6",
        K::KEY_7 => "7",
        K::KEY_8 => "8",
        K::KEY_9 => "9",
        K::KEY_0 => "0",
        K::KEY_MINUS => "-",
        K::KEY_EQUAL => "=",
        K::KEY_BACKSPACE => "backspace",
        K::KEY_TAB => "tab",
        K::KEY_Q => "q",
        K::KEY_W => "w",
        K::KEY_E => "e",
        K::KEY_R => "r",
        K::KEY_T => "t",
        K::KEY_Y => "y",
        K::KEY_U => "u",
        K::KEY_I => "i",
        K::KEY_O => "o",
        K::KEY_P => "p",
        K::KEY_LEFTBRACE => "[",
        K::KEY_RIGHTBRACE => "]",
        K::KEY_ENTER => "enter",
        K::KEY_LEFTCTRL => "ctrl",
        K::KEY_A => "a",
        K::KEY_S => "s",
        K::KEY_D => "d",
        K::KEY_F => "f",
        K::KEY_G => "g",
        K::KEY_H => "h",
        K::KEY_J => "j",
        K::KEY_K => "k",
        K::KEY_L => "l",
        K::KEY_SEMICOLON => ";",
        K::KEY_APOSTROPHE => "'",
        K::KEY_GRAVE => "`",
        K::KEY_LEFTSHIFT => "shift",
        K::KEY_BACKSLASH => "\\",
        K::KEY_Z => "z",
        K::KEY_X => "x",
        K::KEY_C => "c",
        K::KEY_V => "v",
        K::KEY_B => "b",
        K::KEY_N => "n",
        K::KEY_M => "m",
        K::KEY_COMMA => ",",
        K::KEY_DOT => ".",
        K::KEY_SLASH => "/",
        K::KEY_RIGHTSHIFT => "shift",
        K::KEY_KPASTERISK => "*",
        K::KEY_LEFTALT => "alt",
        K::KEY_SPACE => "space",
        K::KEY_CAPSLOCK => "capslock",
        K::KEY_F1 => "f1",
        K::KEY_F2 => "f2",
        K::KEY_F3 => "f3",
        K::KEY_F4 => "f4",
        K::KEY_F5 => "f5",
        K::KEY_F6 => "f6",
        K::KEY_F7 => "f7",
        K::KEY_F8 => "f8",
        K::KEY_F9 => "f9",
        K::KEY_F10 => "f10",
        K::KEY_NUMLOCK => "numlock",
        K::KEY_SCROLLLOCK => "scrolllock",
        K::KEY_KP7 => "7",
        K::KEY_KP8 => "8",
        K::KEY_KP9 => "9",
        K::KEY_KPMINUS => "-",
        K::KEY_KP4 => "4",
        K::KEY_KP5 => "5",
        K::KEY_KP6 => "6",
        K::KEY_KPPLUS => "+",
        K::KEY_KP1 => "1",
        K::KEY_KP2 => "2",
        K::KEY_KP3 => "3",
        K::KEY_KP0 => "0",
        K::KEY_KPDOT => ".",
        K::KEY_F11 => "f11",
        K::KEY_F12 => "f12",
        K::KEY_KPENTER => "enter",
        K::KEY_RIGHTCTRL => "ctrl",
        K::KEY_KPSLASH => "/",
        K::KEY_RIGHTALT => "alt",
        K::KEY_HOME => "home",
        K::KEY_UP => "up",
        K::KEY_PAGEUP => "pageup",
        K::KEY_LEFT => "left",
        K::KEY_RIGHT => "right",
        K::KEY_END => "end",
        K::KEY_DOWN => "down",
        K::KEY_PAGEDOWN => "pagedown",
        K::KEY_INSERT => "insert",
        K::KEY_DELETE => "delete",
        K::KEY_LEFTMETA => "meta",
        K::KEY_RIGHTMETA => "meta",
        K::KEY_COMPOSE => "compose",
        K::KEY_PAUSE => "pause",
        K::KEY_PRINT => "printscreen",
        K::KEY_KPCOMMA => ",",
        K::KEY_KPEQUAL => "=",
        K::KEY_F13 => "f13",
        K::KEY_F14 => "f14",
        K::KEY_F15 => "f15",
        K::KEY_F16 => "f16",
        K::KEY_F17 => "f17",
        K::KEY_F18 => "f18",
        K::KEY_F19 => "f19",
        K::KEY_F20 => "f20",
        K::KEY_F21 => "f21",
        K::KEY_F22 => "f22",
        K::KEY_F23 => "f23",
        K::KEY_F24 => "f24",
        K::KEY_PLAYPAUSE => "mediaplaypause",
        K::KEY_STOPCD => "mediastop",
        K::KEY_NEXTSONG => "medianext",
        K::KEY_PREVIOUSSONG => "mediaprev",
        K::KEY_VOLUMEUP => "volumeup",
        K::KEY_VOLUMEDOWN => "volumedown",
        K::KEY_MUTE => "mute",
        _ => return None,
    };
    Some(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(state: &mut KeyState, name: &str, now_ms: u64) -> Option<KeyPressRecord> {
        state.apply(name, true, now_ms)
    }

    #[test]
    fn ignores_auto_repeat_and_tracks_modifiers() {
        let mut state = KeyState::default();
        assert!(state.apply("ctrl", true, 0).is_none());
        let first = press(&mut state, "k", 10).expect("first press records");
        assert_eq!(first.key, "k");
        assert_eq!(first.modifiers, "ctrl");
        assert_eq!(first.sequence, "k");
        assert!(press(&mut state, "k", 20).is_none(), "repeat is swallowed");
        assert!(state.apply("k", false, 30).is_none());
        let second = press(&mut state, "k", 40).expect("release re-arms");
        assert_eq!(second.sequence, "k k");
    }

    #[test]
    fn modifiers_never_enter_the_sequence() {
        let mut state = KeyState::default();
        for modifier in ["ctrl", "shift", "alt", "meta"] {
            assert!(state.apply(modifier, true, 0).is_none());
        }
        let record = press(&mut state, "g", 5).unwrap();
        assert_eq!(record.sequence, "g");
        assert_eq!(record.modifiers, "ctrl+shift+alt+meta");
    }

    #[test]
    fn sequence_rolls_with_an_eight_key_limit() {
        let mut state = KeyState::default();
        let mut last = String::new();
        for (index, key) in ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]
            .into_iter()
            .enumerate()
        {
            // Release each key so the next press is not treated as repeat.
            let record = press(&mut state, key, index as u64).unwrap();
            state.apply(key, false, index as u64);
            last = record.sequence;
        }
        assert_eq!(last, "c d e f g h i j");
    }

    #[test]
    fn reset_clears_stuck_modifiers_but_keeps_history() {
        let mut state = KeyState::default();
        state.apply("ctrl", true, 0);
        press(&mut state, "g", 1);
        state.reset();
        let record = press(&mut state, "o", 2).unwrap();
        assert_eq!(record.modifiers, "");
        assert_eq!(record.sequence, "g o");
        let (held, modifiers) = state.held_for_tests();
        assert_eq!(held, vec!["o".to_owned()]);
        assert!(modifiers.is_empty());
    }

    #[test]
    fn idle_expiry_unsticks_modifiers_after_sleep_or_hotplug() {
        let mut state = KeyState::default();
        state.apply("ctrl", true, 1_000);
        // Two minutes of silence: the next event starts from a clean slate.
        let record = press(&mut state, "k", 1_000 + STUCK_KEY_TIMEOUT_SECS * 1000 + 1).unwrap();
        assert_eq!(record.modifiers, "");
    }

    #[test]
    fn key_names_stay_stable_and_readable() {
        assert_eq!(key_name(&rdev::Key::KeyA), "a");
        assert_eq!(key_name(&rdev::Key::Space), "space");
        assert_eq!(key_name(&rdev::Key::F12), "f12");
        assert_eq!(key_name(&rdev::Key::MetaRight), "meta");
        assert!(is_modifier(&key_name(&rdev::Key::ShiftLeft)));
        assert!(is_modifier(&key_name(&rdev::Key::AltGr)));
        assert!(!is_modifier(&key_name(&rdev::Key::KeyG)));
        assert!(!is_modifier(&key_name(&rdev::Key::CapsLock)));
        assert!(!is_modifier(&key_name(&rdev::Key::NumLock)));
        assert_eq!(key_name(&rdev::Key::KpReturn), "enter");
    }
}
