// Native addon fixture for the napi-vm runtime integration test.
//
// Authored in TypeScript and compiled ahead of time with the repository's
// TypeScript compiler (no runtime transpilation in napi-vm):
//
//   node_modules/.bin/tsc -p crates/tiktools-plugin-loader/tests/fixtures/napi-vm-native/tsconfig.json
//
// The host loads `dist/index.js` as an ES module through
// `napi_vm::RustPluginHost`. Native calls go through the bundled
// `rdev-node` package exactly like a real device plugin would.

import { add, startListener, stopListener } from 'rdev-node';

declare const require: (specifier: string) => unknown;

export type PluginCall =
  | { type: "action"; action: Record<string, unknown>; event: unknown }
  | { type: "poll" }
  | { type: "enrich"; request: unknown }
  | { type: "event"; event: unknown };

export interface PluginContext {
  pluginId: string;
  name: string;
  version: string;
  [key: string]: unknown;
}

export interface PluginEvent {
  type: string;
  data: unknown;
}

export interface PluginCallResult {
  summary?: string;
  logs: string[];
  intents: unknown[];
  events: PluginEvent[];
}

export interface TikToolsPlugin {
  onLoad?(context: PluginContext): void | Promise<void>;
  call(
    request: PluginCall,
    context: PluginContext,
  ): PluginCallResult | Promise<PluginCallResult>;
  onUnload?(context: PluginContext): unknown | Promise<unknown>;
}

interface QueuedNativeEvent {
  event: string;
  at: number;
}

const queue: QueuedNativeEvent[] = [];

const plugin: TikToolsPlugin = {
  onLoad(context) {
    // NOTE: napi-vm invokes onLoad/onUnload with its own context shape
    // ({name, version} plus a reason on unload); only call() receives the
    // TikTools PluginContext below.
    if (typeof context.name !== "string" || context.name.length === 0) {
      throw new Error("napi-vm-native: missing name in context");
    }
  },

  async call(request, context) {
    if (typeof context.pluginId !== "string" || context.pluginId.length === 0) {
      throw new Error("napi-vm-native: missing pluginId in TikTools context");
    }
    if (request.type === "poll") {
      const events = queue
        .splice(0)
        .map((item) => ({ type: "native.key", data: item }));
      return { logs: [], intents: [], events };
    }
    if (request.type === "action") {
      const typeId = request.action["typeId"];
      if (typeId === "native.start") {
        const ok = startListener((event: string) => {
          queue.push({ event, at: Date.now() });
        });
        return { summary: `started:${String(ok)}`, logs: [], intents: [], events: [] };
      }
      if (typeId === "native.stop") {
        const stopped = stopListener();
        return { summary: `stopped:${String(stopped)}`, logs: [], intents: [], events: [] };
      }
      if (typeId === "native.sum") {
        return { summary: `sum:${add(19, 23)}`, logs: [], intents: [], events: [] };
      }
      if (typeId === "native.require") {
        // Same package through the VM's CommonJS `require()` instead of
        // the static ESM import above: both entries resolve one package.
        const binding = require("rdev-node") as {
          add: (left: number, right: number) => number;
        };
        return {
          summary: `require:${typeof binding.add}:${binding.add(20, 22)}`,
          logs: [],
          intents: [],
          events: [],
        };
      }
      if (typeId === "native.evil") {
        // Probes the allowlist: `dist/evil.node` exists on disk but is
        // never declared, so requiring it must fail closed.
        try {
          require("./evil.node");
          return { summary: "evil:loaded", logs: [], intents: [], events: [] };
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          return { summary: `evil:${message}`, logs: [], intents: [], events: [] };
        }
      }
    }
    return { summary: "native:noop", logs: [], intents: [], events: [] };
  },

  onUnload() {
    // Persistent addon resources stop through their own API; the host
    // never terminates threads the addon detached itself.
    try {
      stopListener();
    } catch {
      // Unloading must not fail when the listener never started.
    }
    return { unloaded: true };
  },
};

export default plugin;
