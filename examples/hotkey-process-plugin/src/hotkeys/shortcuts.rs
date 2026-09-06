//! Portal shortcut registration model.
//!
//! The rdev/evdev backends observe every key and let behaviors filter after
//! the fact. The XDG Desktop Portal works the other way around: each chord
//! must be registered up front with `BindShortcuts`, and the compositor only
//! wakes the plugin for shortcuts the user approved. This module converts
//! TikTools chord filters (`event.data.key eq k` +
//! `event.data.modifiers eq ctrl+shift`) into portal shortcut descriptors and
//! tracks which chords are bound, so the host (or the user, via the
//! `hotkey.bind` action) can synchronize configured behaviors into the
//! portal session.
//!
//! Chords that cannot be represented as compositor shortcuts — sequence
//! filters such as `event.data.sequence contains g o` — are rejected here
//! and stay routed to the raw input backend.

use serde_json::Value;

/// A normalized global chord, e.g. `Ctrl+Shift+K`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chord {
    /// Lowercase key name from the shared `hotkey.pressed` contract.
    pub key: String,
    /// Canonical modifier list in `ctrl+shift+alt+meta` order.
    pub modifiers: String,
}

impl Chord {
    pub fn new(key: impl Into<String>, modifiers: impl Into<String>) -> Option<Self> {
        let key = normalize_key(key.into())?;
        let modifiers = normalize_modifiers(&modifiers.into())?;
        Some(Self { key, modifiers })
    }

    /// Stable portal shortcut id, e.g. `ctrl-shift-k`.
    pub fn shortcut_id(&self) -> String {
        if self.modifiers.is_empty() {
            format!("key-{}", sanitize(&self.key))
        } else {
            format!(
                "{}-{}",
                self.modifiers.replace('+', "-"),
                sanitize(&self.key)
            )
        }
    }

    pub fn description(&self) -> String {
        if self.modifiers.is_empty() {
            format!("TikTools hotkey {}", self.key)
        } else {
            format!(
                "TikTools hotkey {}+{}",
                self.modifiers
                    .split('+')
                    .map(capitalize)
                    .collect::<Vec<_>>()
                    .join("+"),
                self.key
            )
        }
    }
}

fn sanitize(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
    }
}

fn normalize_key(key: String) -> Option<String> {
    let key = key.trim().to_ascii_lowercase();
    if key.is_empty() || key == "shift" || key == "ctrl" || key == "alt" || key == "meta" {
        return None;
    }
    // Keep the shared contract spellings; reject control-only pseudo keys.
    if key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_=[]\\;',./` ".contains(c))
        || key.starts_with('f')
        || [
            "space",
            "enter",
            "tab",
            "esc",
            "backspace",
            "delete",
            "insert",
            "home",
            "end",
            "pageup",
            "pagedown",
            "up",
            "down",
            "left",
            "right",
            "capslock",
            "numlock",
            "scrolllock",
            "printscreen",
            "pause",
            "compose",
        ]
        .contains(&key.as_str())
    {
        Some(key)
    } else {
        None
    }
}

fn normalize_modifiers(raw: &str) -> Option<String> {
    let mut seen = std::collections::BTreeSet::new();
    for part in raw
        .split('+')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty())
    {
        match part.as_str() {
            "ctrl" | "control" => {
                seen.insert("ctrl");
            }
            "shift" => {
                seen.insert("shift");
            }
            "alt" | "altgr" => {
                seen.insert("alt");
            }
            "meta" | "super" | "win" | "mod4" => {
                seen.insert("meta");
            }
            _ => return None,
        }
    }
    let ordered = ["ctrl", "shift", "alt", "meta"]
        .into_iter()
        .filter(|modifier| seen.contains(*modifier))
        .collect::<Vec<_>>()
        .join("+");
    Some(ordered)
}

/// Portal trigger string for a chord. Single letters/digits become
/// `CTRL+SHIFT+K`; named keys use their portal spelling. Returns `None` when
/// the key has no portal spelling (compose, media keys, keypad aliases):
/// those chords stay on the raw backend. Linux-only (portal lives there).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn portal_trigger_string(chord: &Chord) -> Option<String> {
    if chord.key.len() == 1 {
        let key = chord.key.to_ascii_uppercase();
        if chord.modifiers.is_empty() {
            return Some(key);
        }
        let modifiers = portal_modifier_prefixes(&chord.modifiers)?;
        return Some(format!("{}+{key}", modifiers.join("+")));
    }
    let key = portal_key_spelling(&chord.key)?;
    if chord.modifiers.is_empty() {
        return Some(key.to_owned());
    }
    let modifiers = portal_modifier_prefixes(&chord.modifiers)?;
    Some(format!("{}+{key}", modifiers.join("+")))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn portal_modifier_prefixes(modifiers: &str) -> Option<Vec<&'static str>> {
    modifiers
        .split('+')
        .filter(|modifier| !modifier.is_empty())
        .map(|modifier| match modifier {
            "ctrl" => Some("CTRL"),
            "shift" => Some("SHIFT"),
            "alt" => Some("ALT"),
            "meta" => Some("SUPER"),
            _ => None,
        })
        .collect()
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn portal_key_spelling(key: &str) -> Option<&'static str> {
    Some(match key {
        "space" => "SPACE",
        "enter" => "ENTER",
        "tab" => "TAB",
        "esc" => "ESCAPE",
        "backspace" => "BACKSPACE",
        "delete" => "DELETE",
        "insert" => "INSERT",
        "home" => "HOME",
        "end" => "END",
        "pageup" => "PAGE_UP",
        "pagedown" => "PAGE_DOWN",
        "up" => "UP",
        "down" => "DOWN",
        "left" => "LEFT",
        "right" => "RIGHT",
        "capslock" => "CAPS_LOCK",
        "numlock" => "NUM_LOCK",
        "printscreen" => "PRINT",
        "pause" => "PAUSE",
        "f1" => "F1",
        "f2" => "F2",
        "f3" => "F3",
        "f4" => "F4",
        "f5" => "F5",
        "f6" => "F6",
        "f7" => "F7",
        "f8" => "F8",
        "f9" => "F9",
        "f10" => "F10",
        "f11" => "F11",
        "f12" => "F12",
        _ => return None,
    })
}

/// Parses `hotkey.bind` action configs of the form
/// `{"shortcuts": [{"key": "k", "modifiers": "ctrl+shift"}], "sequencesNeeded": true}`.
/// Invalid entries are skipped (reported via the returned warnings) so one
/// typo cannot break the whole binding set.
pub fn parse_bind_config(config: &Value) -> (Vec<Chord>, Vec<String>) {
    let mut chords = Vec::new();
    let mut warnings = Vec::new();
    let shortcuts = config
        .get("shortcuts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (index, entry) in shortcuts.iter().enumerate() {
        let key = entry.get("key").and_then(Value::as_str).unwrap_or_default();
        let modifiers = entry
            .get("modifiers")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match Chord::new(key, modifiers) {
            Some(chord) => {
                if !chords.contains(&chord) {
                    chords.push(chord);
                }
            }
            None => warnings.push(format!(
                "shortcut #{index} is not a portal chord (key={key:?}, modifiers={modifiers:?}); sequences stay on raw input"
            )),
        }
    }
    (chords, warnings)
}

/// Reads the `TIKTOOLS_HOTKEY_SHORTCUTS` environment fallback, a JSON array
/// like `[{"key":"k","modifiers":"ctrl+shift"}]`. Empty when unset/invalid.
pub fn shortcuts_from_env() -> Vec<Chord> {
    let raw = std::env::var("TIKTOOLS_HOTKEY_SHORTCUTS").unwrap_or_default();
    if raw.trim().is_empty() {
        return Vec::new();
    }
    let value: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let (chords, _) = parse_bind_config(&serde_json::json!({ "shortcuts": value }));
    chords
}

/// Whether the host asked for arbitrary key sequences (portal cannot serve
/// them). Defaults to true so existing `sequence contains` behaviors keep
/// working until the host opts chord-only users out of raw input.
pub fn sequences_needed_from_config(config: &Value) -> bool {
    config
        .get("sequencesNeeded")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_normalizes_modifiers_and_builds_portal_trigger() {
        let chord = Chord::new("K", "Shift+Ctrl").expect("valid chord");
        assert_eq!(chord.key, "k");
        assert_eq!(chord.modifiers, "ctrl+shift");
        assert_eq!(chord.shortcut_id(), "ctrl-shift-k");
        assert_eq!(
            portal_trigger_string(&chord).as_deref(),
            Some("CTRL+SHIFT+K")
        );
    }

    #[test]
    fn modifier_aliases_fold_to_canonical_names() {
        let chord = Chord::new("t", "control+win").expect("aliases resolve");
        assert_eq!(chord.modifiers, "ctrl+meta");
        assert_eq!(
            portal_trigger_string(&chord).as_deref(),
            Some("CTRL+SUPER+T")
        );
    }

    #[test]
    fn bare_modifiers_and_sequences_are_not_chords() {
        assert!(Chord::new("ctrl", "").is_none());
        assert!(Chord::new("", "ctrl").is_none());
        // A `sequence contains g o` behavior has no key/modifier pair.
        let (chords, warnings) = parse_bind_config(&serde_json::json!({
            "shortcuts": [{"key": "ctrl", "modifiers": ""}]
        }));
        assert!(chords.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn unportalizable_keys_stay_on_raw_input() {
        // Single printable characters map 1:1.
        let chord = Chord::new(",", "ctrl").expect("contract key");
        assert_eq!(portal_trigger_string(&chord).as_deref(), Some("CTRL+,"));
        // No portal spelling: raw input owns it.
        let chord = Chord::new("compose", "").expect("contract key");
        assert_eq!(portal_trigger_string(&chord), None);
        let chord = Chord::new("f8", "alt").expect("function key");
        assert_eq!(portal_trigger_string(&chord).as_deref(), Some("ALT+F8"));
        let chord = Chord::new("enter", "").expect("named key");
        assert_eq!(portal_trigger_string(&chord).as_deref(), Some("ENTER"));
    }

    #[test]
    fn bind_config_dedupes_and_warns_per_entry() {
        let (chords, warnings) = parse_bind_config(&serde_json::json!({
            "shortcuts": [
                {"key": "k", "modifiers": "ctrl+shift"},
                {"key": "K", "modifiers": "shift+ctrl"},
                {"key": "nope!", "modifiers": "hyper"}
            ],
            "sequencesNeeded": false
        }));
        assert_eq!(chords.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert!(!sequences_needed_from_config(
            &serde_json::json!({"sequencesNeeded": false})
        ));
        assert!(sequences_needed_from_config(&serde_json::json!({})));
    }
}
