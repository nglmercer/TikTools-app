import { describe, expect, mock, test } from "bun:test";

let starts = 0;
let stops = 0;
let failStart = false;
mock.module("node:module", () => ({
  createRequire: () => () => ({
    startListener: () => {
      if (failStart) throw new Error("native hook cannot restart");
      starts += 1;
    },
    stopListener: () => { stops += 1; return true; },
  }),
}));
mock.module("tiktools:events", () => ({ emit: () => {} }));

describe("native listener lifecycle", () => {
  test("does not start and stop the X11 hook before the first binding", async () => {
    const { default: plugin } = await import("../src/index");
    const context = { name: "Hotkeys", version: "0.2.1", pluginId: "hotkeys" };
    await plugin.onLoad!(context);
    await plugin.call({ type: "action", action: { typeId: "hotkey.bind", config: {
      shortcuts: [], sequencesNeeded: false,
    } }, event: null }, context);
    expect(starts).toBe(0);
    expect(stops).toBe(0);
    await plugin.call({ type: "action", action: { typeId: "hotkey.bind", config: {
      shortcuts: [{ key: "k", modifiers: "ctrl" }], sequencesNeeded: true,
    } }, event: null }, context);
    expect(starts).toBe(1);
    await plugin.call({ type: "action", action: { typeId: "hotkey.bind", config: {
      shortcuts: [], sequencesNeeded: false,
    } }, event: null }, context);
    expect(stops).toBe(1);
    failStart = true;
    await expect(plugin.call({ type: "action", action: { typeId: "hotkey.bind", config: {
      shortcuts: [{ key: "k", modifiers: "ctrl" }], sequencesNeeded: true,
    } }, event: null }, context)).rejects.toThrow("native hook cannot restart");
    await plugin.onUnload!();
    expect(stops).toBe(1);
  });
});
