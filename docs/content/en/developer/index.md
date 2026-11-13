---
title: "Develop TikTools"
docId: "developer.overview"
translationKey: "developer.overview"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "ffb1cbe7579915541fc907b15934fad5850b4483acba65764a485f9d083142d0"
translationRevision: "ffb1cbe7579915541fc907b15934fad5850b4483acba65764a485f9d083142d0"
order: 0
---

# Develop TikTools

TikTools keeps one native AppCore behind typed control operations. Vue renders the desktop, local-browser and controller interfaces; Winit/Wry owns desktop windows. The CLI and SDK use the same operations. Start with the [development setup](setup.md), then choose the smallest loop that covers your change.

## Choose an entry point

| Work | Entry point | What it establishes |
| --- | --- | --- |
| Vue components and stores | Vite, Vitest and the injected ControlClient | Interface behavior; no native window validation. |
| Desktop integration | `bun run start` | Development WebView and IPC integration. |
| Native services | Targeted Cargo tests | Core contracts, persistence and service behavior. |
| Headless browser access | `bun run dev` | A native host, local adapter and Vite with pairing. |
| Documentation | `bun scripts/task.ts docs dev` | Markdown and isolated VitePress build. |
| Public demo | Separate demo entry and simulated transport | Fictional deterministic scenarios, not upstream services. |

## Source ownership

- `crates/tiktools-core`: services, operations and SQLite. It does not depend on a WebView.
- `crates/tiktools-control-api`: registry, JSON-RPC routing, IPC and discovery.
- `crates/tiktools-desktop`: Winit/Wry, UI-thread bridge, lifecycle and packaged assets.
- `src/web/app`: bootstrap, router and shell. `pages` compose `widgets` and `features`; `entities` own domain stores/contracts; `shared` holds reusable UI and infrastructure.
- `src/automation/contracts/generated`: versioned frontend event contracts generated from Rust. Do not retype these schemas in a demo or guide.
- `packages`: the existing TypeScript plugin SDK/UI packages. Plugin implementations live in the external plugins repository.

## Contribution boundary

Keep native, local-browser, remote-controller and demo entry points explicit. Absence of a native host is an error in a real entry point; it must not silently select a mock. Dispose listeners, subscriptions and timers with the owning scope. Keep Rust/Bun lockfiles and required CI gates intact.

User installation belongs in the [user guide](../user/installation.md). These development checks do not establish real TikTok connectivity, OBS rendering, native WebView compatibility or two-computer collaboration by themselves.
