# Settings layout refinement

Local follow-up to PR #62 (`6b325b47bee58b34e70212aeea5fec57ae06fe03`).

- Desktop retains the settings category index; mobile uses a labelled native selector with full category names and existing query routes.
- General, account and update controls retain their existing contracts. Toggle rows stay aligned on small screens. Signed-out Cloud remains signed-out; no capabilities were invented.
- Collaboration separates overview, devices/requests and network/security, retaining mounted drafts. Capability/revision warnings and pending request navigation remain visible. Existing confirmation, invitation permissions and secret controls are preserved.
- Advanced groups sources/trust, runtime variables and session diagnostics with native disclosure controls. Legacy globals links open their section.
- Connections prioritizes servers. Live-room exploration is a secondary disclosure; opening it does not start exploration. Plugin schema and authorization controls retain their existing handlers.
- English and Spanish strings are included.

## Automation exclusion

AutomationsPage.vue and its test are restored exactly from origin/remake (b24ddbe at inspection), undoing their earlier PR #62 redesign. Route-scoped CSS preserves previous control density, panel wrapping and hint styling. Shared form accessibility associations and the pre-existing PR diagnostics launcher remain; this is not a claim that all global application chrome is pixel-identical to remake.

## Validation

- Complete Vitest suite: 150 files, 856 tests passed, no unhandled errors.
- vue-tsc --noEmit and ESLint src/web --max-warnings=0 passed.
- Frontend production build passed.
- Chromium at 1440x1000 and 390x844 against a real isolated native host: settings navigation, responsive toggles, collaboration draft retention without saving, warnings, requests, disclosure controls, no horizontal overflow/page errors. Restored mobile automation layout and chrome density checked.
- No authenticated Cloud synchronization, remote controller connections, update installation, real plugin credential writes or native desktop platform testing was performed. The local host lacks remote-controller support and screenshots show that actual warning.

Screenshots and browser evidence are in /tmp/settings-implementation. Library upload failed at hosted apps tools/list with HTTP 401; no library IDs were issued.

## Integration status

No push, PR update or merge performed. PR #62 is currently CONFLICTING/DIRTY. Read-only merge-tree inspection against remake identifies a content conflict in src/web/pages/plugins/ui/PluginsPage.vue from newer catalog work. No worktree merge was attempted and no remote work was overwritten. Resolve that separately before publication.
