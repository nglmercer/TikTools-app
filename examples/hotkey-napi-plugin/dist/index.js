// Global hotkeys napi-vm example.
//
// Authored in TypeScript and compiled ahead of time with the repository's
// TypeScript compiler (no runtime transpilation in napi-vm):
//
//   node_modules/.bin/tsc -p examples/hotkey-napi-plugin/tsconfig.json
//
// The host loads `dist/index.js` as an ES module through
// `napi_vm::RustPluginHost`. TikTools selects the exact host `.node`
// from the declared `rdev-node` package root and exposes it as a native
// `require()` alias, so the guest loads it through `node:module`
// without executing any package loader:
//
//   import { createRequire } from 'node:module';
//   const require = createRequire(import.meta.url);
//   const { startListener, stopListener } = require('rdev-node');
//
// The guest watches the OS keyboard, tracks modifiers plus a rolling
// 8-key sequence, and pushes every press through `tiktools:events` the
// moment the native callback runs — no 1-second poll wait. The `poll`
// call stays as the fallback: anything still queued (an emit that threw
// on an old host, a status transition, logs) drains there. It never
// sends keystrokes anywhere; it only reports what was pressed as
// `hotkey.pressed` events.
import { createRequire } from "node:module";
import { emit } from "tiktools:events";
import { chordDescription, createListenerStats, createPressQueue, diagnosticStatsLines, drainBatch, emitPress, keyName, KeyState, noteNativeCallback, overallStatus, parseBindConfig, rdevCapabilities, shortcutId, wantsListening, } from "./hotkeys";
const require = createRequire(import.meta.url);
const binding = require("rdev-node");
const { startListener } = binding;
// Older rdev-node binaries (v1.0.1 has none) predate stopListener: such a
// binary can start but never stop the native listener. Detect it once and
// report loudly instead of pretending the stop worked.
const stopListener = typeof binding.stopListener === "function" ? binding.stopListener : undefined;
const state = new KeyState();
const pending = createPressQueue();
// Host-registered chords plus the raw-input opt-in. The listener starts on
// load and each `hotkey.bind` projection re-decides via wantsListening.
let bindings = [];
let sequencesNeeded = false;
let listening = false;
let lastReportedDropped = 0;
// Backend health. `statusDirty` flips on every transition so `poll` emits
// exactly one `hotkey.status` event per change.
let report = { backend: "rdev", state: "starting", detail: "" };
let statusDirty = true;
// Debug counters (see `hotkey.diagnostics`) plus one-shot transition logs.
// Poll ticks stay silent unless something changed: the host logs every poll
// log line as a warning, so only transitions are queued here.
const stats = createListenerStats();
const pendingLogs = [];
const MAX_PENDING_LOGS = 8;
function pushLog(line) {
    pendingLogs.push(line);
    while (pendingLogs.length > MAX_PENDING_LOGS) {
        pendingLogs.shift();
    }
}
function setReport(next) {
    if (next.backend !== report.backend ||
        next.state !== report.state ||
        next.detail !== report.detail) {
        report = next;
        statusDirty = true;
    }
}
function startListening() {
    if (listening) {
        return;
    }
    state.reset();
    try {
        startListener((event) => {
            // First native callback ever: the native -> guest path is alive.
            // Counts only, never key contents.
            if (noteNativeCallback(stats, Date.now())) {
                pushLog("hotkey: first native key event observed (backend rdev)");
            }
            const pressed = event.eventType === "KeyPress";
            const keyCode = event.eventType === "KeyPress"
                ? event.keyPress?.key
                : event.eventType === "KeyRelease"
                    ? event.keyRelease?.key
                    : undefined;
            if (keyCode === undefined) {
                return;
            }
            if (emitPress(state, pending, keyName(keyCode), pressed, "rdev", Date.now())) {
                stats.pressesQueued += 1;
                pushPendingPresses();
            }
        }, (message) => {
            listening = false;
            stats.failures += 1;
            pushLog(`hotkey: rdev listener failed (${message})`);
            setReport({ backend: "rdev", state: "failed", detail: message });
        });
        listening = true;
        pushLog("hotkey: rdev listener started");
        setReport({ backend: "rdev", state: "running", detail: "" });
    }
    catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        stats.failures += 1;
        pushLog(`hotkey: rdev listener failed to start (${detail})`);
        setReport({ backend: "rdev", state: "failed", detail });
    }
}
// Pushes every queued press through `tiktools:events` immediately. A
// throwing emit (old host without push, rejected payload) puts this and
// the rest back in order, so the `poll` fallback still delivers them.
function pushPendingPresses() {
    const batch = drainBatch(pending);
    for (let index = 0; index < batch.length; index++) {
        const item = batch[index];
        try {
            emit({
                type: "hotkey.pressed",
                data: {
                    key: item.key,
                    modifiers: item.modifiers,
                    sequence: item.sequence,
                    backend: item.backend,
                },
            });
        }
        catch {
            pending.events.unshift(...batch.slice(index));
            break;
        }
    }
}
function stopListening() {
    state.reset();
    if (stopListener === undefined) {
        // The native listener is still running: keep `listening` true so
        // status and diagnostics describe reality.
        return false;
    }
    let stopped = false;
    try {
        stopped = stopListener();
    }
    catch {
        stopped = false;
    }
    listening = false;
    if (stopped) {
        pushLog("hotkey: listener stopped");
    }
    return stopped;
}
function statusEvent(droppedEvents, pendingEvents) {
    const caps = rdevCapabilities();
    return {
        type: "hotkey.status",
        data: {
            // A napi-vm guest observes no platform or session facts by design.
            platform: "unknown",
            session: "unknown",
            droppedEvents,
            pendingEvents,
            backends: [
                {
                    backend: report.backend,
                    state: report.state,
                    detail: report.detail,
                    summary: overallStatus([report]).summary,
                    capabilities: {
                        globalChords: caps.globalChords,
                        arbitraryKeys: caps.arbitraryKeys,
                        sequences: caps.sequences,
                        keyRelease: caps.keyRelease,
                    },
                },
            ],
        },
    };
}
const plugin = {
    onLoad(context) {
        // NOTE: napi-vm invokes onLoad/onUnload with its own context shape
        // ({name, version} plus a reason on unload); only call() receives the
        // TikTools PluginContext below.
        if (typeof context.name !== "string" || context.name.length === 0) {
            throw new Error("hotkeys: missing name in context");
        }
        startListening();
    },
    async call(request, context) {
        if (typeof context.pluginId !== "string" || context.pluginId.length === 0) {
            throw new Error("hotkeys: missing pluginId in TikTools context");
        }
        if (request.type === "poll") {
            stats.pollsServed += 1;
            const events = drainBatch(pending).map((item) => ({
                type: "hotkey.pressed",
                data: {
                    key: item.key,
                    modifiers: item.modifiers,
                    sequence: item.sequence,
                    backend: item.backend,
                },
            }));
            const logs = pendingLogs.splice(0, pendingLogs.length);
            // Fresh overflow this tick: one log line plus a status re-emit with
            // the new counters. Counts only — never key contents.
            const freshOverflow = pending.dropped !== lastReportedDropped;
            if (freshOverflow) {
                logs.push(`hotkey event queue overflowed: ${pending.dropped} event(s) dropped in total, ${pending.events.length} pending`);
                lastReportedDropped = pending.dropped;
            }
            if (statusDirty || freshOverflow) {
                statusDirty = false;
                events.push(statusEvent(pending.dropped, pending.events.length));
            }
            return { logs, intents: [], events };
        }
        if (request.type === "action") {
            const typeId = request.action["typeId"];
            if (typeId === "hotkey.bind" || typeId === "hotkey_bind" || typeId === "bind") {
                const parsed = parseBindConfig(request.action["config"] ?? {});
                bindings = parsed.chords;
                sequencesNeeded = parsed.sequencesNeeded;
                const logs = [...parsed.warnings];
                if (wantsListening(bindings, sequencesNeeded)) {
                    startListening();
                }
                else if (!stopListening() && stopListener === undefined) {
                    logs.push("staged rdev-node has no stopListener export; the native listener keeps running until unload");
                }
                if (bindings.length > 0) {
                    logs.push("portal shortcuts unavailable in the napi-vm build (compositor chords unsupported); raw input covers all keys");
                }
                for (const chord of bindings) {
                    logs.push(`bound ${chordDescription(chord)} (${shortcutId(chord)})`);
                }
                return {
                    summary: `hotkey bindings updated: ${bindings.length} chord(s) watched, sequences ${sequencesNeeded ? "enabled" : "disabled"}`,
                    logs,
                    intents: [],
                    events: [],
                };
            }
            if (typeId === "hotkey.status" || typeId === "hotkey_status" || typeId === "status") {
                return { summary: overallStatus([report]).summary, logs: [], intents: [], events: [] };
            }
            if (typeId === "hotkey.diagnostics" ||
                typeId === "hotkey_diagnostics" ||
                typeId === "diagnostics") {
                const lines = [
                    "Hotkey diagnostics (napi-vm):",
                    `  runtime: napi-vm (TypeScript guest, rdev-node ${listening ? "listening" : "stopped"})`,
                    `  backend: ${report.backend} state=${report.state}${report.detail === "" ? "" : ` detail=${report.detail}`}`,
                    `  stop: ${stopListener === undefined ? "unsupported (staged rdev-node predates stopListener)" : "available"}`,
                    `  bindings: ${bindings.length} chord(s) watched, sequences ${sequencesNeeded ? "enabled" : "disabled"}`,
                    "  portal: unsupported (compositor chords need the retired process plugin)",
                    "  session: unknown (guests observe no platform facts)",
                    ...diagnosticStatsLines(stats, Date.now()),
                    "Queue:",
                    `  dropped events (overflow): ${pending.dropped}`,
                    `  pending events: ${pending.events.length}`,
                ];
                return {
                    summary: "hotkey diagnostics ready (1 backend entry); see logs",
                    logs: lines,
                    intents: [],
                    events: [],
                };
            }
            if (typeId === "" || typeId === undefined) {
                return {
                    summary: "hotkey listener has no default action; configure hotkey.pressed events instead",
                    logs: [],
                    intents: [],
                    events: [],
                };
            }
            throw new Error(`unknown hotkey action ${JSON.stringify(typeId)}; use hotkey.bind, hotkey.status, or hotkey.diagnostics`);
        }
        return { summary: "hotkey:noop", logs: [], intents: [], events: [] };
    },
    onUnload() {
        // Persistent addon resources stop through their own API; the host
        // never terminates threads the addon detached itself.
        try {
            stopListening();
        }
        catch {
            // Unloading must not fail when the listener never started.
        }
        return { unloaded: true };
    },
};
export default plugin;
