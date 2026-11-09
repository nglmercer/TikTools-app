# Flat UI and shared component review

These screenshots accompany the `feat/flat-ui-components` draft. The application
was built from `remake` base `0dc1b795f690c9020a9766532de0278b11d40873` plus this
change. The headless host was rebuilt from that same base with
`cargo build -p tiktools-cli --features local-web --locked`.

## Actual application views

Captured in Chromium against a disposable local host, without mocked API replies.
The automation name is a deliberately created, unsaved demonstration draft; no
live event was fired. The updater diagnostic is the real unsupported-headless-host
response. No personal account, credentials or production data is shown.

| Desktop, 1440 × 1000 | Mobile, 390 × 844 |
| --- | --- |
| [Automation editor, EN](automations-desktop-en.png) | [Same retained draft, EN](automations-mobile-en.png) |
| [Flat diagnostics panel, EN](diagnostics-desktop-en.png) | [Diagnostic page, ES](diagnostics-mobile-es.png) |
| | [Settings rows and touch density, ES](settings-mobile-es.png) |

## Development-only component catalogue

These are actual renders of `/ui-kit`, with explicitly labelled sample state.
They demonstrate the shared components, not a live account/plugin operation.
The mobile catalogue capture uses 390 × 1400 to show the whole example section.

- [Desktop examples, EN](component-catalog-desktop-en.png)
- [Mobile examples, ES](component-catalog-mobile-es.png)

## Verification

- Full web suite: 150 files / 854 tests passed during integration.
- After final layout/form adjustments: 12 targeted files / 52 tests passed.
- TypeScript, ESLint (zero warnings), production frontend build and diff checks.
- Real browser: unsaved automation name retained across desktop → mobile →
  desktop; Back keeps discard confirmation; contextual diagnostic selection;
  desktop hover/focus and touch-visible action; keyboard/pointer panel movement;
  focus returned on close; mobile route/return; 44px touch action; no horizontal
  page overflow in the tested flows; EN/ES; no page errors.

Existing autosave hook tests emit Vue lifecycle warnings when invoked outside a
component; assertions pass. No native GUI packaging, OS-specific assistive
technology certification or authenticated two-device cloud synchronization is
claimed by these screenshots. Cloud components use the current identifiers-only
conflict contract; they do not fabricate a base/local/cloud content comparison.
