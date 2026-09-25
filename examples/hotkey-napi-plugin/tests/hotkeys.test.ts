// Unit tests for the ported hotkey core: key state, normalization,
// chords, bind parsing, queueing, and status. Run with:
//
//   bun test examples/hotkey-napi-plugin/
//
// `hotkeys.ts` is dependency-free on purpose so these run under stock
// bun/node with no napi-vm and no native binding.

import { describe, expect, test } from "bun:test";
import {
  canonicalModifiers,
  chordDescription,
  createListenerStats,
  createPressQueue,
  diagnosticStatsLines,
  drainBatch,
  emitPress,
  isModifier,
  keyName,
  KeyState,
  MAX_PENDING_EVENTS,
  noteNativeCallback,
  overallStatus,
  parseBindConfig,
  POLL_MAX_EVENTS_PER_RESPONSE,
  shortcutId,
  STUCK_KEY_TIMEOUT_MS,
  wantsListening,
} from "../src/hotkeys";

describe("KeyState", () => {
  test("ignores auto-repeat and tracks modifiers", () => {
    const state = new KeyState();
    expect(state.apply("ctrl", true, 0)).toBeNull();
    const first = state.apply("k", true, 10);
    expect(first).toEqual({ key: "k", modifiers: "ctrl", sequence: "k" });
    expect(state.apply("k", true, 20)).toBeNull();
    expect(state.apply("k", false, 30)).toBeNull();
    const second = state.apply("k", true, 40);
    expect(second?.sequence).toBe("k k");
  });

  test("modifiers never enter the sequence", () => {
    const state = new KeyState();
    for (const modifier of ["ctrl", "shift", "alt", "meta"]) {
      expect(state.apply(modifier, true, 0)).toBeNull();
    }
    const record = state.apply("g", true, 5);
    expect(record?.sequence).toBe("g");
    expect(record?.modifiers).toBe("ctrl+shift+alt+meta");
  });

  test("sequence rolls with an eight-key limit", () => {
    const state = new KeyState();
    let last = "";
    ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"].forEach((key, index) => {
      const record = state.apply(key, true, index);
      state.apply(key, false, index);
      last = record?.sequence ?? "";
    });
    expect(last).toBe("c d e f g h i j");
  });

  test("reset clears stuck modifiers but keeps history", () => {
    const state = new KeyState();
    state.apply("ctrl", true, 0);
    state.apply("g", true, 1);
    state.reset();
    const record = state.apply("o", true, 2);
    expect(record?.modifiers).toBe("");
    expect(record?.sequence).toBe("g o");
  });

  test("idle expiry unsticks modifiers after the timeout", () => {
    const state = new KeyState();
    state.apply("ctrl", true, 1000);
    // Two minutes of silence: the next event starts from a clean slate.
    const record = state.apply("k", true, 1000 + STUCK_KEY_TIMEOUT_MS + 1);
    expect(record?.modifiers).toBe("");
  });

  test("modifiers normalize to canonical chord order", () => {
    const state = new KeyState();
    for (const modifier of ["meta", "alt", "shift", "ctrl"]) {
      expect(state.apply(modifier, true, 1)).toBeNull();
    }
    const record = state.apply("k", true, 2);
    expect(record?.modifiers).toBe("ctrl+shift+alt+meta");
  });
});

describe("keyName", () => {
  // Parity vectors against the retired Rust `key_name`: rdev-node
  // `KeyCode` labels must normalize to the exact contract spellings.
  const cases: Array<[string, string]> = [
    ["KeyA", "a"],
    ["KeyZ", "z"],
    ["Num1", "1"],
    ["Num0", "0"],
    ["Space", "space"],
    ["Return", "enter"],
    ["KpReturn", "enter"],
    ["Tab", "tab"],
    ["Escape", "esc"],
    ["Backspace", "backspace"],
    ["Delete", "delete"],
    ["Insert", "insert"],
    ["Home", "home"],
    ["End", "end"],
    ["PageUp", "pageup"],
    ["PageDown", "pagedown"],
    ["UpArrow", "up"],
    ["DownArrow", "down"],
    ["LeftArrow", "left"],
    ["RightArrow", "right"],
    ["CapsLock", "capslock"],
    ["F1", "f1"],
    ["F12", "f12"],
    ["ShiftLeft", "shift"],
    ["ControlRight", "ctrl"],
    ["Alt", "alt"],
    ["AltGr", "alt"],
    ["MetaLeft", "meta"],
    ["Comma", ","],
    ["Dot", "."],
    ["Slash", "/"],
    ["SemiColon", ";"],
    ["Quote", "'"],
    ["LeftBracket", "["],
    ["RightBracket", "]"],
    ["BackSlash", "\\"],
    ["Minus", "-"],
    ["Equal", "="],
    ["BackQuote", "`"],
    ["KpMinus", "-"],
    ["KpPlus", "+"],
    ["KpMultiply", "*"],
    ["KpDivide", "/"],
    ["KpDecimal", "."],
    ["Kp0", "0"],
    ["KpDelete", "delete"],
    ["PrintScreen", "printscreen"],
    ["ScrollLock", "scrolllock"],
    ["Pause", "pause"],
  ];
  for (const [label, expected] of cases) {
    test(`${label} -> ${expected}`, () => {
      expect(keyName(label)).toBe(expected);
    });
  }

  test("unknown labels degrade to lowercase", () => {
    expect(keyName("SomeFutureKey")).toBe("somefuturekey");
  });

  test("modifier detection matches the contract", () => {
    for (const label of ["ShiftLeft", "ControlRight", "AltGr", "MetaLeft"]) {
      expect(isModifier(keyName(label))).toBe(true);
    }
    for (const label of ["KeyG", "CapsLock", "NumLock", "Space"]) {
      expect(isModifier(keyName(label))).toBe(false);
    }
  });

  test("canonical modifiers sort ctrl+shift+alt+meta", () => {
    expect(canonicalModifiers(new Set(["meta", "ctrl", "alt", "shift"]))).toBe(
      "ctrl+shift+alt+meta",
    );
    expect(canonicalModifiers([])).toBe("");
  });
});

describe("chords", () => {
  test("shortcut ids and descriptions are stable", () => {
    expect(shortcutId({ key: "k", modifiers: "ctrl+shift" })).toBe("ctrl-shift-k");
    expect(shortcutId({ key: "a", modifiers: "" })).toBe("key-a");
    expect(chordDescription({ key: "k", modifiers: "ctrl+shift" })).toBe(
      "TikTools hotkey Ctrl+Shift+k",
    );
    expect(chordDescription({ key: "a", modifiers: "" })).toBe("TikTools hotkey a");
  });
});

describe("parseBindConfig", () => {
  test("parses chords, dedupes, defaults sequences to true", () => {
    const parsed = parseBindConfig({
      shortcuts: [
        { key: "k", modifiers: "ctrl" },
        { key: "K", modifiers: "control" },
        { key: "g", modifiers: "" },
      ],
    });
    expect(parsed.chords).toEqual([
      { key: "k", modifiers: "ctrl" },
      { key: "g", modifiers: "" },
    ]);
    expect(parsed.warnings).toEqual([]);
    expect(parsed.sequencesNeeded).toBe(true);
  });

  test("malformed shortcuts warn instead of failing the bind", () => {
    const parsed = parseBindConfig({
      shortcuts: [{ key: "ctrl", modifiers: "" }, { key: "k", modifiers: "bogus" }],
      sequencesNeeded: false,
    });
    expect(parsed.chords).toEqual([]);
    expect(parsed.warnings).toHaveLength(2);
    expect(parsed.sequencesNeeded).toBe(false);
  });

  test("modifier aliases normalize", () => {
    const parsed = parseBindConfig({
      shortcuts: [{ key: "k", modifiers: "win+control" }],
    });
    expect(parsed.chords).toEqual([{ key: "k", modifiers: "ctrl+meta" }]);
  });
});

describe("press queue", () => {
  function press(
    state: KeyState,
    queue: ReturnType<typeof createPressQueue>,
    key: string,
  ): void {
    emitPress(state, queue, key, true, "rdev", Date.now());
    state.apply(key, false, Date.now());
  }

  test("poll drains at most one protocol batch in order", () => {
    const state = new KeyState();
    const queue = createPressQueue();
    for (let index = 0; index < 40; index += 1) {
      press(state, queue, `k${index}`);
    }
    expect(queue.events).toHaveLength(40);
    // 40 queued -> three polls of 16/16/8, order preserved, none lost.
    const first = drainBatch(queue);
    expect(first).toHaveLength(POLL_MAX_EVENTS_PER_RESPONSE);
    expect(first[0]?.key).toBe("k0");
    expect(first[15]?.key).toBe("k15");
    expect(queue.events).toHaveLength(24);
    const second = drainBatch(queue);
    expect(second).toHaveLength(POLL_MAX_EVENTS_PER_RESPONSE);
    expect(second[0]?.key).toBe("k16");
    expect(queue.events).toHaveLength(8);
    const third = drainBatch(queue);
    expect(third).toHaveLength(8);
    expect(third[7]?.key).toBe("k39");
    expect(queue.events).toHaveLength(0);
  });

  test("queue overflow drops oldest and counts drops", () => {
    const state = new KeyState();
    const queue = createPressQueue();
    for (let index = 0; index < MAX_PENDING_EVENTS + 6; index += 1) {
      press(state, queue, `k${index}`);
    }
    expect(queue.events).toHaveLength(MAX_PENDING_EVENTS);
    expect(queue.dropped).toBe(6);
    expect(queue.events[0]?.key).toBe("k6");
    expect(drainBatch(queue)).toHaveLength(POLL_MAX_EVENTS_PER_RESPONSE);
  });

  test("releases and repeats queue nothing", () => {
    const state = new KeyState();
    const queue = createPressQueue();
    expect(emitPress(state, queue, "k", false, "rdev", 1)).toBe(false);
    expect(emitPress(state, queue, "k", true, "rdev", 2)).toBe(true);
    expect(emitPress(state, queue, "k", true, "rdev", 3)).toBe(false);
    expect(emitPress(state, queue, "ctrl", true, "rdev", 4)).toBe(false);
    expect(queue.events).toHaveLength(1);
  });
});

describe("overallStatus", () => {
  test("prefers running, then most actionable state", () => {
    expect(
      overallStatus([
        { backend: "rdev", state: "failed", detail: "no display" },
        { backend: "rdev", state: "running", detail: "" },
      ]).summary,
    ).toBe("Global Hotkeys: running via native listener");
    expect(
      overallStatus([{ backend: "rdev", state: "failed", detail: "no display" }])
        .summary,
    ).toBe("Global Hotkeys: failed via native listener (no display)");
    expect(overallStatus([]).summary).toBe("Global Hotkeys: starting");
  });
});

describe("listener stats", () => {
  test("noteNativeCallback reports the first callback exactly once", () => {
    const stats = createListenerStats();
    expect(stats.nativeCallbacks).toBe(0);
    expect(noteNativeCallback(stats, 100)).toBe(true);
    expect(noteNativeCallback(stats, 200)).toBe(false);
    expect(stats.nativeCallbacks).toBe(2);
    expect(stats.firstCallbackAtMs).toBe(100);
    expect(stats.lastCallbackAtMs).toBe(200);
  });

  test("diagnosticStatsLines renders counters and last-callback age", () => {
    expect(diagnosticStatsLines(createListenerStats(), 1000)).toEqual([
      "  native callbacks: 0 (last never)",
      "  presses queued: 0",
      "  polls served: 0",
      "  failures: 0",
    ]);
    const stats = {
      ...createListenerStats(),
      nativeCallbacks: 3,
      pressesQueued: 2,
      pollsServed: 9,
      failures: 1,
      firstCallbackAtMs: 500,
      lastCallbackAtMs: 900,
    };
    expect(diagnosticStatsLines(stats, 1000)).toEqual([
      "  native callbacks: 3 (last 100ms ago)",
      "  presses queued: 2",
      "  polls served: 9",
      "  failures: 1",
    ]);
  });
});

describe("wantsListening", () => {
  test("chord-only binds keep the listener running (no portal backend)", () => {
    expect(wantsListening([{ key: "k", modifiers: "ctrl+shift" }], false)).toBe(true);
  });

  test("sequences always need the listener", () => {
    expect(wantsListening([], true)).toBe(true);
  });

  test("fully unbound with sequences off stops the listener", () => {
    expect(wantsListening([], false)).toBe(false);
  });
});
