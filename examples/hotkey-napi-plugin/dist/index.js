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
// 8-key sequence, and answers the host `poll` call with everything
// observed since the previous tick. It never sends keystrokes anywhere;
// it only reports what was pressed as `hotkey.pressed` events.
import { createRequire } from "node:module";
import { chordDescription, createPressQueue, drainBatch, emitPress, keyName, KeyState, overallStatus, parseBindConfig, rdevCapabilities, shortcutId, wantsListening, } from "./hotkeys";
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
            const pressed = event.eventType === "KeyPress";
            const keyCode = event.eventType === "KeyPress"
                ? event.keyPress?.key
                : event.eventType === "KeyRelease"
                    ? event.keyRelease?.key
                    : undefined;
            if (keyCode === undefined) {
                return;
            }
            emitPress(state, pending, keyName(keyCode), pressed, "rdev", Date.now());
        }, (message) => {
            listening = false;
            setReport({ backend: "rdev", state: "failed", detail: message });
        });
        listening = true;
        setReport({ backend: "rdev", state: "running", detail: "" });
    }
    catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        setReport({ backend: "rdev", state: "failed", detail });
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
            const events = drainBatch(pending).map((item) => ({
                type: "hotkey.pressed",
                data: {
                    key: item.key,
                    modifiers: item.modifiers,
                    sequence: item.sequence,
                    backend: item.backend,
                },
            }));
            const logs = [];
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
