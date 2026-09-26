// Shared hotkey state behind `event.data.key/modifiers/sequence`.
//
// TypeScript port of the matching logic from the retired Rust process
// plugin (`KeyState`, key normalization, chord model, bind parsing,
// backend reports). Pure module: no `require`, no napi-vm imports, so
// `bun test` exercises it directly. The published contract is unchanged:
// `key` is the normalized non-modifier key, `modifiers` is the canonical
// `ctrl+shift+alt+meta` chord, and `sequence` is the rolling history of
// the last MAX_SEQUENCE_KEYS non-modifier presses.

/// Rolling history behind `event.data.sequence`.
export const MAX_SEQUENCE_KEYS = 8;
/// Backpressure cap; the host additionally caps 16 events per poll tick.
export const MAX_PENDING_EVENTS = 64;
/// Protocol batch: at most this many events per poll response, in order.
export const POLL_MAX_EVENTS_PER_RESPONSE = 16;
/// Idle time after which held-key state is discarded. Releases can be lost
/// across sleep/resume, device hotplug, focus changes, or missed grabs; a
/// stuck modifier would otherwise poison every later chord.
export const STUCK_KEY_TIMEOUT_MS = 120_000;

export interface KeyPressRecord {
  key: string;
  modifiers: string;
  sequence: string;
}

export class KeyState {
  /// Currently held non-modifier keys (kills auto-repeat duplicates).
  private pressed = new Set<string>();
  private modifiers = new Set<string>();
  private sequence: string[] = [];
  /// Monotonic tick (ms) of the last press/release, for stuck-key expiry.
  private lastActivityMs = 0;

  /// Records one normalized key transition. Returns a press record for
  /// non-modifier presses, `null` for releases, modifier-only traffic, and
  /// auto-repeat duplicates. `nowMs` is monotonic milliseconds and may come
  /// from any steady clock.
  apply(name: string, pressed: boolean, nowMs: number): KeyPressRecord | null {
    this.expireStuckKeys(nowMs);
    this.lastActivityMs = nowMs;
    if (isModifier(name)) {
      if (pressed) {
        this.modifiers.add(name);
      } else {
        this.modifiers.delete(name);
      }
      return null;
    }
    if (pressed) {
      // Auto-repeat: the OS re-sends presses while a key is held.
      if (this.pressed.has(name)) {
        return null;
      }
      this.pressed.add(name);
      this.sequence.push(name);
      while (this.sequence.length > MAX_SEQUENCE_KEYS) {
        this.sequence.shift();
      }
      return {
        key: name,
        modifiers: canonicalModifiers(this.modifiers),
        sequence: this.sequence.join(" "),
      };
    }
    this.pressed.delete(name);
    return null;
  }

  /// Drops all in-flight held state. Called on listener (re)start so a
  /// missed release cannot leave a modifier or key permanently stuck.
  /// The rolling sequence is history, not held state, so it survives.
  reset(): void {
    this.pressed.clear();
    this.modifiers.clear();
  }

  private expireStuckKeys(nowMs: number): void {
    if (nowMs - this.lastActivityMs > STUCK_KEY_TIMEOUT_MS) {
      this.pressed.clear();
      this.modifiers.clear();
    }
  }
}

export function isModifier(name: string): boolean {
  return name === "shift" || name === "ctrl" || name === "alt" || name === "meta";
}

/// Conventional chord order (ctrl+shift+alt+meta) so recorded combos match
/// what users type in filters, independent of set iteration order.
export function canonicalModifiers(modifiers: Set<string> | string[]): string {
  const ordered = [...modifiers];
  ordered.sort((a, b) => modifierRank(a) - modifierRank(b));
  return ordered.join("+");
}

function modifierRank(name: string): number {
  switch (name) {
    case "ctrl":
      return 0;
    case "shift":
      return 1;
    case "alt":
      return 2;
    case "meta":
      return 3;
    default:
      return 4;
  }
}

/// Stable, layout-independent key names from rdev-node `KeyCode` labels
/// (`KeyA`, `Space`, `F12`, ...). Unknown labels degrade to lowercase
/// instead of breaking the event stream.
export function keyName(keyCode: string): string {
  switch (keyCode) {
    case "ShiftLeft":
    case "ShiftRight":
      return "shift";
    case "ControlLeft":
    case "ControlRight":
      return "ctrl";
    // AltGr produces the local third-level chooser; it behaves as Alt
    // for chord matching and must never stick as its own modifier.
    case "Alt":
    case "AltGr":
      return "alt";
    case "MetaLeft":
    case "MetaRight":
      return "meta";
    case "Space":
      return "space";
    case "Return":
      return "enter";
    case "Tab":
      return "tab";
    case "Escape":
      return "esc";
    case "Backspace":
      return "backspace";
    case "Delete":
      return "delete";
    case "Insert":
      return "insert";
    case "Home":
      return "home";
    case "End":
      return "end";
    case "PageUp":
      return "pageup";
    case "PageDown":
      return "pagedown";
    case "UpArrow":
      return "up";
    case "DownArrow":
      return "down";
    case "LeftArrow":
      return "left";
    case "RightArrow":
      return "right";
    case "CapsLock":
      return "capslock";
    case "NumLock":
      return "numlock";
    case "ScrollLock":
      return "scrolllock";
    case "PrintScreen":
      return "printscreen";
    case "Pause":
      return "pause";
    case "Comma":
      return ",";
    case "Dot":
      return ".";
    case "Slash":
      return "/";
    case "SemiColon":
      return ";";
    case "Quote":
      return "'";
    case "LeftBracket":
      return "[";
    case "RightBracket":
      return "]";
    case "BackSlash":
      return "\\";
    case "IntlBackslash":
      return "\\";
    case "Minus":
      return "-";
    case "Equal":
      return "=";
    case "BackQuote":
      return "`";
    case "Multiply":
      return "*";
    case "Add":
      return "+";
    case "Subtract":
      return "-";
    case "Decimal":
      return ".";
    case "Divide":
      return "/";
    case "KpReturn":
      return "enter";
    case "KpMinus":
      return "-";
    case "KpPlus":
      return "+";
    case "KpMultiply":
      return "*";
    case "KpDivide":
      return "/";
    case "KpDecimal":
      return ".";
    case "KpEqual":
      return "=";
    case "KpComma":
      return ",";
    case "KpDelete":
      return "delete";
    default:
      break;
  }
  if (keyCode.length === 4 && keyCode.startsWith("Key")) {
    return keyCode.slice(3).toLowerCase();
  }
  if (keyCode.length === 4 && keyCode.startsWith("Num")) {
    return keyCode.slice(3);
  }
  if (
    keyCode.startsWith("F") &&
    keyCode.length <= 3 &&
    /^F\d+$/.test(keyCode)
  ) {
    return keyCode.toLowerCase();
  }
  if (keyCode.startsWith("Numpad") && keyCode.length === 7) {
    return keyCode.slice(6);
  }
  if (keyCode.startsWith("Kp") && keyCode.length === 3) {
    return keyCode.slice(2);
  }
  return keyCode.toLowerCase();
}

/// One normalized key press waiting for the host `poll` tick.
export interface PendingEvent {
  key: string;
  modifiers: string;
  sequence: string;
  backend: string;
}

export interface PressQueue {
  events: PendingEvent[];
  dropped: number;
}

export function createPressQueue(): PressQueue {
  return { events: [], dropped: 0 };
}

/// Records a press in shared state and queues the resulting event.
/// Returns true when an event was queued. Overflow drops the oldest
/// event and counts it, so a flood can never grow the queue unboundedly.
export function emitPress(
  state: KeyState,
  queue: PressQueue,
  name: string,
  pressed: boolean,
  backend: string,
  nowMs: number,
): boolean {
  const record = state.apply(name, pressed, nowMs);
  if (!pressed || record === null) {
    return false;
  }
  queue.events.push({
    key: record.key,
    modifiers: record.modifiers,
    sequence: record.sequence,
    backend,
  });
  while (queue.events.length > MAX_PENDING_EVENTS) {
    queue.events.shift();
    queue.dropped += 1;
  }
  return true;
}

/// Drains at most one protocol batch from the pending queue, in order.
/// Remainder stays queued for the next tick: events are never removed
/// when the consumer would discard them.
export function drainBatch(queue: PressQueue): PendingEvent[] {
  return queue.events.splice(
    0,
    Math.min(queue.events.length, POLL_MAX_EVENTS_PER_RESPONSE),
  );
}

/// One host-registered chord: lowercase key plus canonical modifiers.
export interface Chord {
  key: string;
  modifiers: string;
}

export function chordEquals(left: Chord, right: Chord): boolean {
  return left.key === right.key && left.modifiers === right.modifiers;
}

/// Stable shortcut id, e.g. `ctrl-shift-k`.
export function shortcutId(chord: Chord): string {
  if (chord.modifiers === "") {
    return `key-${sanitize(chord.key)}`;
  }
  return `${chord.modifiers.split("+").join("-")}-${sanitize(chord.key)}`;
}

export function chordDescription(chord: Chord): string {
  if (chord.modifiers === "") {
    return `TikTools hotkey ${chord.key}`;
  }
  const modifiers = chord.modifiers
    .split("+")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join("+");
  return `TikTools hotkey ${modifiers}+${chord.key}`;
}

function sanitize(key: string): string {
  return [...key]
    .map((c) => (/[A-Za-z0-9]/.test(c) ? c.toLowerCase() : "-"))
    .join("");
}

function normalizeKey(raw: string): string | null {
  const key = raw.trim().toLowerCase();
  if (
    key === "" ||
    key === "shift" ||
    key === "ctrl" ||
    key === "alt" ||
    key === "meta"
  ) {
    return null;
  }
  const NAMED = new Set([
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
  ]);
  // Keep the shared contract spellings; reject control-only pseudo keys.
  if (
    [...key].every((c) => /[A-Za-z0-9]/.test(c) || "-_=[]\\;',./` ".includes(c)) ||
    key.startsWith("f") ||
    NAMED.has(key)
  ) {
    return key;
  }
  return null;
}

function normalizeModifiers(raw: string): string | null {
  const seen = new Set<string>();
  for (const part of raw.split("+")) {
    const name = part.trim().toLowerCase();
    if (name === "") {
      continue;
    }
    switch (name) {
      case "ctrl":
      case "control":
        seen.add("ctrl");
        break;
      case "shift":
        seen.add("shift");
        break;
      case "alt":
      case "altgr":
        seen.add("alt");
        break;
      case "meta":
      case "super":
      case "win":
      case "mod4":
        seen.add("meta");
        break;
      default:
        return null;
    }
  }
  return ["ctrl", "shift", "alt", "meta"]
    .filter((modifier) => seen.has(modifier))
    .join("+");
}

export interface BindConfig {
  chords: Chord[];
  warnings: string[];
  sequencesNeeded: boolean;
}

/// Parses a `hotkey.bind` action config: the host's persisted Behavior
/// projection (`shortcuts` chords plus the `sequencesNeeded` raw-input
/// opt-in). Malformed shortcuts warn instead of failing the whole bind.
export function parseBindConfig(config: unknown): BindConfig {
  const shortcuts =
    typeof config === "object" &&
    config !== null &&
    Array.isArray((config as Record<string, unknown>)["shortcuts"])
      ? ((config as Record<string, unknown>)["shortcuts"] as unknown[])
      : [];
  const chords: Chord[] = [];
  const warnings: string[] = [];
  shortcuts.forEach((entry, index) => {
    const record = (typeof entry === "object" && entry !== null
      ? entry
      : {}) as Record<string, unknown>;
    const key = typeof record["key"] === "string" ? (record["key"] as string) : "";
    const modifiers =
      typeof record["modifiers"] === "string" ? (record["modifiers"] as string) : "";
    const normalizedKey = normalizeKey(key);
    const normalizedModifiers = normalizeModifiers(modifiers);
    if (normalizedKey !== null && normalizedModifiers !== null) {
      const chord = { key: normalizedKey, modifiers: normalizedModifiers };
      if (!chords.some((known) => chordEquals(known, chord))) {
        chords.push(chord);
      }
    } else {
      warnings.push(
        `shortcut #${index} is not a chord (key=${JSON.stringify(key)}, modifiers=${JSON.stringify(modifiers)}); sequences stay on raw input`,
      );
    }
  });
  const sequencesNeeded =
    typeof config === "object" &&
    config !== null &&
    typeof (config as Record<string, unknown>)["sequencesNeeded"] === "boolean"
      ? ((config as Record<string, unknown>)["sequencesNeeded"] as boolean)
      : true;
  return { chords, warnings, sequencesNeeded };
}

export type BackendRunState =
  | "starting"
  | "running"
  | "permission-required"
  | "failed"
  | "unsupported";

export interface BackendReport {
  backend: string;
  state: BackendRunState;
  detail: string;
}

export function statusLine(report: BackendReport): string {
  if (report.detail === "") {
    return `Global Hotkeys: ${report.state} via native listener`;
  }
  return `Global Hotkeys: ${report.state} via native listener (${report.detail})`;
}

/// Overall status prefers a running backend, then the most actionable
/// non-running state. Mirrors the retired Rust plugin's ordering.
export function overallStatus(reports: BackendReport[]): {
  state: BackendRunState;
  summary: string;
} {
  const running = reports.find((report) => report.state === "running");
  if (running !== undefined) {
    return { state: "running", summary: statusLine(running) };
  }
  for (const state of [
    "permission-required",
    "failed",
    "unsupported",
    "starting",
  ] as const) {
    const report = reports.find((candidate) => candidate.state === state);
    if (report !== undefined) {
      return { state, summary: statusLine(report) };
    }
  }
  return { state: "starting", summary: "Global Hotkeys: starting" };
}

/// What the rdev backend can observe: every key, releases, and sequences.
/// Whether the rdev listener must run for this bind state. The napi-vm
/// build has no portal backend, so watched chords need raw input too:
/// only a fully unbound plugin with sequences off may stop listening.
export function wantsListening(chords: Chord[], sequencesNeeded: boolean): boolean {
  return sequencesNeeded || chords.length > 0;
}

/// Cumulative listener counters behind `hotkey.diagnostics` and the
/// one-shot poll logs. Poll ticks stay silent by design (the host logs
/// every poll log line as a warning), so transitions and totals are the
/// only thing emitted: enough to tell "native never fired" apart from
/// "host drops everything" without spamming the log.
export interface ListenerStats {
  /// Raw native callbacks observed (presses and releases, all keys).
  nativeCallbacks: number;
  /// Non-modifier presses queued for the host.
  pressesQueued: number;
  /// Poll ticks answered.
  pollsServed: number;
  /// Start attempts that threw or reported failure.
  failures: number;
  /// Monotonic ms of the first native callback, 0 when none arrived yet.
  firstCallbackAtMs: number;
  /// Monotonic ms of the latest native callback, 0 when none arrived yet.
  lastCallbackAtMs: number;
}

export function createListenerStats(): ListenerStats {
  return {
    nativeCallbacks: 0,
    pressesQueued: 0,
    pollsServed: 0,
    failures: 0,
    firstCallbackAtMs: 0,
    lastCallbackAtMs: 0,
  };
}

/// Records one native callback. Returns true exactly once, on the first
/// callback ever, so the guest can log the native path coming alive.
export function noteNativeCallback(stats: ListenerStats, nowMs: number): boolean {
  stats.nativeCallbacks += 1;
  stats.lastCallbackAtMs = nowMs;
  if (stats.firstCallbackAtMs === 0) {
    stats.firstCallbackAtMs = nowMs;
    return true;
  }
  return false;
}

/// Counter lines for the `hotkey.diagnostics` report. `nowMs` uses the
/// same clock as `noteNativeCallback` (any monotonic ms).
export function diagnosticStatsLines(stats: ListenerStats, nowMs: number): string[] {
  const lastAge =
    stats.lastCallbackAtMs === 0 ? "never" : `${Math.max(0, nowMs - stats.lastCallbackAtMs)}ms ago`;
  return [
    `  native callbacks: ${stats.nativeCallbacks} (last ${lastAge})`,
    `  presses queued: ${stats.pressesQueued}`,
    `  polls served: ${stats.pollsServed}`,
    `  failures: ${stats.failures}`,
  ];
}

export function rdevCapabilities(): {
  globalChords: boolean;
  arbitraryKeys: boolean;
  sequences: boolean;
  keyRelease: boolean;
} {
  return {
    globalChords: true,
    arbitraryKeys: true,
    sequences: true,
    keyRelease: true,
  };
}
