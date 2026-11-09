# Diagnostics UI refinement

The floating desktop panel and mobile debug page now share a compact search/action bar, an expandable filter area, individually removable source/level/entity filters and a result count. Copy uses the current view; Copy all remains available in the filter area when the view is restricted. Export TXT, clear history, contextual selection, retained older selections, loading more rows and clipboard fallback remain available.

Rows show severity symbols with accessible level labels, time, source/context and a two-line preview. Expanding reveals the full message, code, timestamps and distinct details. Identical message/details are displayed once without changing the retained record or exported text. A source already indicated by the active filter is not repeated on every row. Explanatory text is shortened and empty history is distinguished from a search with no results.

The filter button exposes aria-expanded/controls. Escape closes the filter area and restores focus. Keyboard/pointer movement, minimize/restore and close focus return are preserved; minimizing no longer discards local search/level/filter state. Mobile has one compact back/title header. The diagnostic panel uses flat surfaces without shadow. English and Spanish UI text is included; technical log messages remain exactly as recorded by their source.

## Scope and safety

No log retention, redaction, deduplication, domain state, native API or automation feature was changed. Icon-only actions have accessible names/tooltips and proper disabled states. Copy/export continue to use formatDiagnostics on already-redacted records. Tests cover redacted fallback when clipboard permission is denied.

## Validation

- Full web suite during integration: 152 files / 869 tests passed.
- After final empty-state/minimization/UI adjustments: 4 affected files / 15 tests passed.
- TypeScript, ESLint and production build passed; final renders were rebuilt and browser-tested.
- Chromium 1440x1000 and 390x844 against a real isolated native host: filter/entry keyboard interaction, filter chips, search/reset, copied text, downloaded TXT, minimize/restore state retention, panel close focus and mobile return. No page errors or document horizontal overflow.
- Screenshots use actual initial disconnected/update-unavailable logs. No fake API responses or synthetic diagnostic entries were injected. Error-level formatting, long/older selections and credential redaction are unit-tested; those states are not fabricated in captures.

See [current diagnostics screenshots](ui/debug-current/README.md). No native desktop save dialog or OS-specific assistive technology certification is claimed.
