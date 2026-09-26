// Echo fixture for the napi-vm runtime integration test.
//
// Authored in TypeScript and compiled ahead of time with the repository's
// TypeScript compiler (no runtime transpilation in napi-vm):
//
//   node_modules/.bin/tsc -p crates/tiktools-plugin-loader/tests/fixtures/napi-vm-echo/tsconfig.json
//
// The host loads `dist/index.js` as an ES module through
// `napi_vm::RustPluginHost` and drives the unchanged PluginCall /
// PluginCallResult JSON protocol through `call(request, context)`.
const plugin = {
    onLoad(context) {
        // NOTE: napi-vm invokes onLoad/onUnload with its own context shape
        // ({name, version} plus a reason on unload); only call() receives the
        // TikTools PluginContext below. `name` exists in both shapes, so this
        // assertion survives a future context unification upstream.
        if (typeof context.name !== "string" || context.name.length === 0) {
            throw new Error("napi-vm-echo: missing name in context");
        }
    },
    // Deliberately async: the host must await Promise results.
    async call(request, context) {
        if (typeof context.pluginId !== "string" || context.pluginId.length === 0) {
            throw new Error("napi-vm-echo: missing pluginId in TikTools context");
        }
        const kind = await Promise.resolve(request.type);
        if (kind === "poll") {
            return {
                logs: [],
                intents: [],
                events: [{ type: "echo.tick", data: { plugin: context.pluginId } }],
            };
        }
        if (kind === "action" && request.type === "action") {
            const typeId = request.action["typeId"];
            return {
                summary: `echo:${String(typeId ?? "unknown")}@${context.pluginId}`,
                logs: [`saw action for ${context.pluginId}`],
                intents: [],
                events: [],
            };
        }
        return {
            summary: `echo:${kind}`,
            logs: [],
            intents: [],
            events: [],
        };
    },
    onUnload() {
        return { unloaded: true };
    },
};
export default plugin;
