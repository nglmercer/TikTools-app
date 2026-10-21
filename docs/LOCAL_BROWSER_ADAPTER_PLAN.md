# Local browser adapter implementation plan

Status: implementation in progress; see LOCAL_BROWSER_ADAPTER_VALIDATION.md. Baseline: `remake` after Termux PR #29,
merge commit `18840b501a3de207a4e0ab4d6ff3cad65c892dc6`.

## Outcome and scope

Open the existing Vue application in an Android browser at a loopback HTTP URL.
Use an authenticated WebSocket adapter to manage the same headless host as the
CLI. This milestone supports local access only; no LAN listener, public tunnel,
remote access, APK or WebView is included. Android capability limitations remain
visible and do not prevent supported screens from loading.

```text
Android browser: existing Vue UI
        | same-origin HTTP + authenticated WebSocket
Local adapter: static assets, authentication, protocol translation
        | existing private local IPC
Existing headless host: Control API -> one AppCore
        | existing services, SQLite, plugins, LIVE runtime
Termux CLI connects to this same host independently
```

## Source findings

- `src/web/shared/api/control-client.ts` already implements request correlation,
  timeouts, bounded pending calls, event handlers and gap handlers. Its transport
  currently depends on `window.ipc.postMessage` and WebView receive callbacks.
- The same file includes presentation-generation/revision and frontend-ready
  behavior. These are desktop host concerns that require an explicit audit;
  browser operation cannot be achieved by replacing postMessage alone.
- `crates/tiktools-client` provides typed operations over the existing Control
  API; `crates/tiktools-control-api/src/client.rs` provides local IPC framing and
  event subscriptions. Reuse these rather than inventing another method registry.
- `vite.config.ts` already contains development plugin-asset serving. Audit and
  reuse its asset resolution rules where applicable, with a production Rust
  server implementation and without assuming its development CORS policy is safe.
- `package.json` has `build:web`, `typecheck`, `test:web` and lint commands.
  Building assets is separate from running the Rust host/adapter. A prebuilt
  asset bundle must permit runtime use without Bun or Node on the phone.

## 1. Verify contracts before introducing a server

Inventory frontend calls and pushed messages against `api discover` and schemas.
Identify desktop-only handshake/projection calls, file/media URL schemes, plugin
iframes, file pickers, external links and any direct native bridge usage outside
control-client.ts. Record unsupported operations explicitly.

Define a small transport interface around send, receive, connect and disconnect.
Keep the current WebView transport and desktop behavior. Add a browser WebSocket
implementation behind the same ControlClient interface. Preserve existing method
names, request IDs, structured errors and event contracts. Adapt envelopes at the
boundary when IPC notifications differ from frontend HostMessage envelopes;
cover those mappings with contract fixtures.

Decide how browser views obtain initial snapshots and subscribe to updates using
existing Control API operations. Do not emulate successful desktop presentation
calls if the host cannot execute them. If a necessary contract is missing, add an
additive shared operation with parity tests rather than a browser-only business API.

Exit: documented message mappings and frontend dependency inventory; existing
ControlClient tests pass with the desktop transport unchanged.

## 2. Add the optional local adapter to the existing CLI

Proposed command (not currently supported):

```bash
tiktools host --ipc
# A second Termux session; assets built beforehand or provided by a release:
tiktools web --assets /path/to/web-dist --port 8787
# Open the printed http://127.0.0.1:8787 URL in the Android browser.
```

Add an optional positive Cargo feature for the HTTP/static/WebSocket adapter so
CLI-only users need not compile a web server. Prefer a module in tiktools-cli;
create another crate only if dependency boundaries justify it. Select a minimal
Rust server implementation after checking its actual Android dependency graph.

The adapter connects to the existing IPC host and never constructs AppCore or
acquires the host ownership lock. If the host is absent, return host_unavailable
with the startup command. Do not silently start another runtime. Exiting the
adapter must leave the host running; shutting down the host must invalidate
adapter connections cleanly.

Forward registered Control API operations through the existing client, without
shelling out per request. Use per-browser IPC connections or an explicit mapping
for subscriptions and cancellation. Audit cancellation semantics before claiming
that client disconnect cancels server work. Maintain bounded messages, clients,
pending calls and queues. Preserve gap reporting; reject overload explicitly.

Exit: browser and CLI observe the same runtime, data and plugin states.

## 3. Authenticate local access and serve assets safely

Bind only to IPv4 loopback `127.0.0.1`; reject non-loopback listen addresses in
this milestone. Validate the exact allowed Host and WebSocket Origin against the
active endpoint, preventing DNS rebinding and cross-site socket requests. Do not
enable wildcard CORS. Missing or unexpected browser origins fail closed.

Generate a cryptographically random one-use pairing code, shown only in the
foreground terminal. Exchange it through a same-origin POST for a short-lived,
HttpOnly, SameSite=Strict session cookie; rotate/invalidate sessions on restart.
Use same-origin validation and a session-bound CSRF mechanism for state-changing
HTTP endpoints. Authenticate before accepting a WebSocket upgrade. Keep codes,
cookies and TikTok credentials out of URLs, access logs and persistent browser
storage. Local HTTP is limited to loopback; remote TLS is outside this milestone.

Serve only the configured build root and explicitly allowed plugin assets.
Protect against encoded traversal, symlink escapes and arbitrary filesystem reads.
Set suitable CSP and iframe isolation, then test required existing plugin UIs.
Define explicit authenticated media routes only where existing UI behavior needs
them. Do not expose application data directories as a static file tree.

Exit: unauthorized HTTP/WS, forged origin/Host, expired pairing and traversal
negative tests pass on the native adapter.

## 4. Connect and stabilize the existing Vue UI

Select browser transport explicitly when running from the adapter origin; retain
the native transport for desktop builds. Display pairing, connecting, connected,
reconnecting, disconnected and host-unavailable states. Never blindly replay
mutating requests after disconnect. Reconnect with bounded exponential backoff,
resubscribe and reload authoritative snapshots; show detected event gaps.

Validate system health, discovery, LIVE status/connect/disconnect, plugin
list/health/enable/disable, automations, workflows and event inspection. Use
platform capabilities to explain unavailable actions without crashing pages.
Inventory plugin iframe RPC and assets separately; do not claim all plugin UIs
work merely because the main application connects. Preserve Unicode/gift metadata.
Make navigation and primary actions usable at Android phone viewport sizes;
limit layout work to changes required for those existing flows.

Exit: supported flows work in a real Android browser, with unsupported capabilities
reported accurately. Record any page or plugin still unavailable.

## 5. Validate and document

Use temporary isolated application homes and only test-owned processes. Extend
Termux smoke coverage for the adapter without touching the user's running host.

Required automated checks:

- Locked CLI compilation with adapter enabled and disabled on native Android.
- Focused Rust adapter, CLI, client and Control API contract tests.
- Browser transport tests: request correlation, errors, timeouts, overload,
  disconnect, subscriptions, gap handling and reconnect without mutation replay.
- HTTP/WS authentication, Origin/Host, traversal and resource-bound tests.
- Existing WebView ControlClient regression tests, typecheck, lint and web build
  in a compatible frontend environment. Report unavailable tools as skipped.
- CLI sandbox verification, formatting and changed-file whitespace checks.

Required device checks:

- Chrome or another Android browser opens the local UI and pairs successfully.
- CLI and browser share one host PID; SQLite state survives host restart.
- Synthetic events arrive with Unicode intact; browser suspension/resume and
  host/adapter restart recover without busy loops or unbounded buffering.
- Supported plugin UI assets and actions work; unsupported integrations are clear.
- Listener is loopback only; unauthorized browser origins cannot operate the host.
- Stopping the adapter leaves the host usable from the CLI.

Authenticated LIVE remains a separate credentialed check. Desktop regression
checks run in a compatible environment. Record passed, failed and skipped results
separately; do not label browser support complete before real Android browser tests.

Document asset preparation, pairing, launch/shutdown, logs, offline assets,
background limitations and exact verified commands in TERMUX.md. No remote
listener flag or deployment instructions are part of this implementation.

## Delivery order

1. Transport extraction, inventory and contract fixtures.
2. Optional Rust loopback server, IPC forwarding and authentication.
3. Browser transport, existing Vue startup and asset/plugin compatibility.
4. Isolated automated tests and real Android browser validation.
5. Update setup and validation records; review the local-only milestone.

Remote access is a separate later proposal and must not block this plan.
