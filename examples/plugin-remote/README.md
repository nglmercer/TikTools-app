# Example: installable remote-control plugin

This is an **example** of the plugin that owns remote control. It is not
discovered or packaged by `scripts/build-plugin.ts` (which only scans
`<plugins-repo>/plugins/*/plugin.json`). It will be published through the
registry/discovery in the future.

The production package currently lives in `plugins-example-tiktools/plugins/remote-control/`
and is built with `bun scripts/task.ts plugins --plugin remote-control`.

The plugin never receives a bearer token, `ControlApi` or raw settings. It owns
its listener and executes host operations through `host.call`.
