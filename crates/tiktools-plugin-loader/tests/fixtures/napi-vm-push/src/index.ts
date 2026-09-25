// Push fixture for the tiktools:events capability integration test.
//
// Authored in TypeScript and compiled ahead of time with the repository's
// TypeScript compiler (no runtime transpilation in napi-vm):
//
//   node_modules/.bin/tsc -p crates/tiktools-plugin-loader/tests/fixtures/napi-vm-push/tsconfig.json
//
// Actions drive `emit`/`emitMany`; the test asserts delivery on the
// manager push bus without any poll.

import { emit, emitMany } from "tiktools:events";

export interface PluginContext {
  pluginId: string;
  name: string;
  version: string;
}

const plugin = {
  onLoad(context: { name: string }) {
    if (typeof context.name !== "string" || context.name.length === 0) {
      throw new Error("napi-vm-push: missing name in context");
    }
  },
  async call(request: any, context: PluginContext) {
    if (typeof context.pluginId !== "string" || context.pluginId.length === 0) {
      throw new Error("napi-vm-push: missing pluginId in TikTools context");
    }
    const kind = await Promise.resolve(request.type);
    if (kind === "poll") {
      return { logs: [], intents: [], events: [] };
    }
    if (kind === "action") {
      const typeId = request.action && request.action.typeId;
      if (typeId === "push.fire") {
        emit({ type: "push.tick", data: { n: 1 } });
        return { logs: [], intents: [], events: [], summary: "emitted" };
      }
      if (typeId === "push.fire-many") {
        const count = emitMany([{ type: "push.tick" }, { type: "push.tick", data: { n: 2 } }]);
        return { logs: [], intents: [], events: [], summary: `emitted:${count}` };
      }
      if (typeId === "push.undeclared") {
        emit({ type: "nope.undeclared" });
        return { logs: [], intents: [], events: [] };
      }
      throw new Error(`napi-vm-push: unknown action ${typeId}`);
    }
    throw new Error(`napi-vm-push: unknown request ${kind}`);
  },
  onUnload() {},
};

export default plugin;
