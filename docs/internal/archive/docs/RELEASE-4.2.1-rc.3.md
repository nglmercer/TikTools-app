# TikTools 4.2.1-rc.3

Linux candidate for testing the desktop changes after RC2. AppImage, DEB and
portable archive are built locally; GitHub Actions remains disabled. RC1 and
RC2 assets remain unchanged. This is a prerelease, not the stable update.

## Changes

- Retains RC2's correction for concurrent Changes/conflict reads and its local
  retry with the native failure reason.
- Variables now persist explicit text, number, boolean, list and local-secret
  types. Saves are atomic and reject a changed host or stale editor revision.
  Existing text rendering in automation templates remains compatible.
- A secret stays masked in the editor and never falls back to a legacy write
  after switching hosts. Native classification also blocks publication of
  values queued before the variable became secret; Changes projections omit
  those values. Saving Variables remains local.
- The native Variables bridge provides local cached comparison, explicit
  one-variable adoption and cache-only download. Application rejects changed
  session, consent, local revision or target cache entry. Variables-scoped
  publication excludes other section queues and plugin application.

The native bridge is not yet exposed as a Variables Changes button. Reviewed
send/withdraw and pending-conflict handling still need the section adapter.
An already published value is not erased by marking its local copy secret.

## Verification

- 28 native Variables/read tests, 170 sync-engine tests, 17 Cloud-enabled typed
  client parity tests and the Owner/remote RPC authorization test pass.
- The control API compiles with Cloud support and without persistence; the
  unavailable implementation does not perform a network action.
- Frontend ESLint/TypeScript and Rust formatting checks pass.
- The optimized desktop build reports `4.2.1-rc.3`. AppImage/DEB content checks
  and portable archive verification pass. The actual AppImage reaches
  `frontend-ready` with a fresh isolated native profile and XDG directories.
  This check does not exercise signed-in production sync or media playback.
- Three browser tests against the production frontend pass: typed Variables,
  Settings sections and the legacy globals link. They use a fake native host.
- The rebuilt CLI's `api verify --sandbox` passes its typed-method validation
  and all 23 read-only smoke calls without an existing account.
- AppImage SHA256:
  `4b11260836457350debf2680b230c52cea8408ddf17ccdc729d0025a62149b7e`.
  All three asset hashes are included in `SHA256SUMS.txt`.

## Testing and remaining scope

Compare this candidate with RC2 using a separate native profile and separate
XDG data/cache/config directories. Check Changes loading and retry, typed
Variables save/reload, Settings/Teams layout and optional Driver.js tours.
Keep local data backups when testing a candidate with an existing profile.

Nickname search and recipient-bound invitations require backend commit
`6a69c99` and additive migration
`20261113000001_remote_invitation_recipients.sql`. This candidate does not
deploy that backend. On 2026-11-13 production capabilities returned HTTP 200;
the new `/api/v3/remote/people` route still returned HTTP 404.

Automations still uses a local reference comparison. Full section adapters,
Variables reviewed send/withdraw, Overlay revisions and the remaining
Collaboration presentation audit are unfinished. The historical LIVE SQLite
lock holder remains unidentified; the diagnostic reason codes and retry
regression are present.

RC1's copied-profile migration/account-policy event is documented in
[the debugging plan](SYNC-CHANGES-PLAN.md#421-rc1-reproduction).
The RC3 startup check uses a fresh isolated profile without saved credentials.
