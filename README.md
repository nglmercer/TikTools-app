<span id="tiktools-"></span>

# TikTools

TikTools connects to TikTok LIVE, displays chat and engagement events, keeps
viewer points and runs local automation rules. Use it on Windows or Linux;
the optional native CLI also supports a Termux workflow on Android.

[Download published binaries and checksums](https://github.com/nglmercer/TikTools-app/releases)
· [Latest stable release](https://github.com/nglmercer/TikTools-app/releases/latest)
· [English user documentation](docs/content/en/user/index.md)
· [Documentación de usuario en español](docs/content/es/user/index.md)
· [Guias em português](docs/content/pt/user/index.md)
· [Guides en français](docs/content/fr/user/index.md)

<span id="quick-start-"></span>

## Quick start

1. Open the releases page and choose a published asset for your system:
   Windows x86_64 installer or portable ZIP, or Linux x86_64 AppImage, DEB or
   portable TAR.GZ. Compare its hash with that release's `SHA256SUMS.txt`.
2. Follow the [installation guide](docs/content/en/user/installation.md)
   ([español](docs/content/es/user/installation.md)). Portable packages need
   their `web/` directory beside the executable. Windows needs WebView2;
   Linux requirements depend on the package and distribution.
3. Open **Connect**, enter a creator who is currently LIVE, and start with
   guest access when the room allows it. Follow
   [your first LIVE](docs/content/en/user/first-live.md)
   ([primer LIVE](docs/content/es/user/first-live.md)) for connection recovery.

Released desktop binaries do not require Rust, Cargo or Bun. Optional plugins
are installed separately. Use the [updates guide](docs/content/en/user/updates.md)
for manual updates and the limits of experimental installer handoffs. Version
numbers in source and candidate release notes do not establish publication;
use the assets actually attached to a published release.

<span id="what-it-includes-"></span>

## What it includes

- LIVE connection and creator monitoring, a bounded event feed, viewer points
  and statistics.
- Saved automation events, reusable actions, workflows, templates and profiles.
- Runtime plugin packages, including native, process, declarative HTTP and
  QuickJS integrations. The compatible manifest name `napi-vm` uses QuickJS.
  WASM execution is currently unavailable.
- Local overlay previews and widgets for OBS; live gateway delivery requires
  the separately installed integration.
- English, Spanish, Portuguese and French UI translations (`en`, `es`, `pt`,
  `fr`), with complete documentation in the same four languages.
- Optional Cloud account and section-specific Changes controls. Remote
  collaboration and transports remain experimental; grants, host capabilities
  and the current guide determine the operations available to a client.

Rankings and competitions have a native engine and a direct route; their
management page is currently hidden from normal navigation. The
[user guide](docs/content/en/user/index.md) explains other feature limits.

<span id="documentation"></span>
<span id="documentation-"></span>

## Documentation and demo

Start with [English guides](docs/content/en/user/index.md) or
[guías en español](docs/content/es/user/index.md). Source contributors use the
separate [developer documentation](docs/content/en/developer/index.md)
([desarrollo en español](docs/content/es/developer/index.md)).

The repository includes a VitePress site and an interactive demo with fictional
data. After the development dependency setup below, build a local preview:

```sh
TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build
bun scripts/task.ts site preview
```

The default preview serves [English docs](http://127.0.0.1:4173/TikTools-app/en/user/),
[Spanish docs](http://127.0.0.1:4173/TikTools-app/es/user/) and the
[interactive demo](http://127.0.0.1:4173/TikTools-app/demo/). See
[docs and demo development](docs/content/en/developer/demo.md) for separate
development loops, build paths and configuration. Site publication is tracked
separately from this local preview.

Retired documents remain in the versioned historical archive. The
[migration map](docs/migration-map.json) records every original document,
heading and image; [archive provenance](docs/internal/archive/PROVENANCE.json)
identifies the unchanged historical sources and their hashes. The
[original README](docs/internal/archive/README.md) remains available verbatim.

<span id="development--quality-"></span>
<span id="development-quality"></span>

## Development / Quality

Use Bun 1.4.4, Rust 1.95 or newer, and Node.js compatible with the locked Vite
toolchain. Windows source builds need MSVC Build Tools and native Strawberry
Perl; Linux desktop builds need GTK/WebKitGTK and audio development packages.
Follow [development setup](docs/content/en/developer/setup.md) for your platform.

```sh
bun ci
bun run start
```

`start` allocates and owns a loopback Vite development server, waits until it
is ready, then runs the Winit/Wry desktop host against that URL. Stop an existing
desktop or control host cleanly before starting another development owner.

Example plugins live in the separate
[`plugins-example-tiktools`](https://github.com/nglmercer/plugins-example-tiktools)
repository. When that checkout is a sibling, or `TIKTOOLS_PLUGINS_REPO` points
to it, `start` can prepare its packages in the ignored `.dev-plugins` directory.
Use `TIKTOOLS_SKIP_DEV_PLUGINS=1 bun run start` to skip preparation.
`bun run dev` starts the headless host, local browser adapter and Vite without
automatic development-plugin preparation.

Use an isolated `TIKTOOLS_HOME` for native experiments. Follow
[CONTRIBUTING.md](CONTRIBUTING.md) for focused checks, full CI requirements,
documentation maintenance and release policy.

<span id="commands-"></span>

## Commands

| Command | Result |
| --- | --- |
| `bun run start` | Owned Vite server and Wry desktop development host |
| `bun run dev` | Headless host, loopback browser adapter and Vite |
| `bun run build` | Widgets, frontend assets and release desktop binary; external plugins install separately |
| `bun run lint` | Frontend/SDK lint and type checks plus Rust formatting and Clippy |
| `bun run test` | Fast Bun and Vitest unit suites |
| `bun run ci` | Contract checks, lint, tests, native verification and application build |
| `bun scripts/task.ts docs check` | Public article metadata, translations and links |
| `bun scripts/task.ts docs dev` | VitePress development server |
| `bun scripts/task.ts demo dev` | Fictional-data browser demo development server |
| `bun scripts/task.ts site build` | Build and assemble docs plus demo into `dist/site` |

`bun ci` installs from the committed lockfile. Run targeted tests through the
task runner, for example `bun run test web live-batch` or
`bun run test rust tiktools-core batched_likes --features native-tiktok`.
Native Termux instructions are in [English](docs/content/en/user/termux.md)
and [español](docs/content/es/user/termux.md).

<span id="project-layout-"></span>

## Project layout

```text
crates/tiktools-desktop/       Winit/Wry shell, tray and UI-thread lifecycle
crates/tiktools-core/          AppCore, services, SQLite, events and automations
crates/tiktools-control-api/   Typed JSON-RPC registry and local transports
crates/tiktools-client/        Typed SDK and optional remote transports
crates/tiktools-cli/           IPC-first native CLI and optional browser adapter
crates/tiktools-collaboration/ Grants, sessions, execution authority and receipts
crates/tiktools-remote/        Optional HTTPS/WSS host gateway
crates/tiktools-rtc/           Native WebRTC transport and signaling contracts
crates/tiktools-plugin-*/      Plugin contracts, runtimes, installer and tooling
src/web/shared/api/           Frontend control client and transport contracts
src/web/shared/ui/            Shared Vue components
src/web/shared/i18n/          Four UI locale catalogs
src/web/entities/             Domain state and reusable business models
src/web/features/             User operations
src/shared/messages.ts       Host-push message contract
src/automation/              Automation contracts and frontend helpers
src/widgets/                 TikTools widget renderers and shared SDK
docs/content/{en,es,pt,fr}/  Localized public user and developer guides
docs/internal/archive/       Unchanged historical sources and provenance
```

<span id="runtime-plugin-model-"></span>

## Runtime plugin model

Plugins are discovered from runtime directories. Adding an integration does
not require rebuilding the application, and the application build does not
bundle the external example plugins. Use the
[plugin installation guide](docs/content/en/user/plugins/index.md) or
[authoring guide](docs/content/en/developer/plugins/quickstart.md).

Native libraries and trusted `.node` addons run inside the host process.
Process isolation is not an OS sandbox. Manifest permissions and capabilities
govern the host APIs exposed to a package; they do not make arbitrary native
code safe. See [runtime boundaries](docs/content/en/developer/plugins/runtimes.md).

<span id="data-and-privacy-"></span>

## Data and privacy

Rust resolves writable state under platform app-data directories. Development
overrides include `TIKTOOLS_HOME`, `TIKTOOLS_DATA_DIR`, `TIKTOOLS_PLUGINS_DIR`,
`TIKTOOLS_PLUGIN_DATA_DIR`, `TIKTOOLS_LOG_DIR` and `TIKTOOLS_TEMP_DIR`.
See [storage](docs/content/en/developer/storage.md) and
[backup guidance](docs/content/en/user/preferences/backups.md) before moving
or restoring a profile.

Keep TikTok cookies, Cloud credentials, invitation tokens and secret Variables
out of logs, issues and screenshots. TikTok session cookies stay in memory.
Cloud login is separate from the TikTok LIVE connection. Review diagnostic
exports before sharing them.
