# UI diagnostics

`feat/diagnostics-log-panel` adds a session-only diagnostic history. The application shell installs one collector; status components only navigate and never append events during rendering.

- Desktop: contextual status buttons reveal “View logs” on hover or keyboard focus. A nonmodal panel supports dragging, arrow-key movement, minimizing, resetting and returning focus on close.
- Mobile/touch: actions remain visible; narrow layouts open `/debug` with source, entity and entry context plus a return location.
- Sources: LIVE errors/transitions, monitor, room discovery, plugin health results, cloud account/sync, updater and transport failures/gaps.
- The viewer offers source/level/text filters, copy all/filtered/selected, TXT export and clear. Clearing history does not clear connection errors. Export in the native shell uses `system.saveFile`; browsers download locally.
- Retention: 2,000 entries and 2 MiB of serialized UTF-8 data, oldest first eviction. Identical consecutive events for one source/entity within five seconds share an entry with a repetition count.
- Redaction happens before retention, including known session-cookie values, credential headers, named secrets, bearer/basic credentials, JWTs and URL userinfo/query/fragment. Fields are capped at 4,000 characters. Collectors intentionally do not retain full settings, RPC arguments, HTTP bodies or console output. Arbitrary unlabelled secrets in third-party messages are not guaranteed recognizable: avoid adding raw payloads to collectors.
- History is in memory, reset on reload/unmount. It does not import native rotating log files or synchronize to cloud. Rendering uses Vue text interpolation, never HTML from messages.

Inline validation, authorization/provisioning controls, guided recovery, sync conflict decisions and manual updater downloads remain in their original flows. Installation/registry errors outside these sources are not yet collected.

Verification: `bun x --no-install vitest run`, `bun x --no-install vue-tsc --noEmit`, `bun x --no-install eslint src/web --max-warnings=0`, `bun scripts/task.ts frontend-build`. Unit tests cover redaction, retention, deduplication, context routing and unsupported-host updater presentation. Real-browser smoke checks use a local headless host and verify hover/focus, drag/keyboard movement, focus return and mobile navigation. Native-shell download behavior still requires a desktop runtime check.
