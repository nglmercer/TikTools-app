# Cloud Account (Desktop native auth)

PR6 (`feat/desktop-cloud-auth`) integrates RemakeBackend native Cloud login into
TikTools Desktop by reusing the pinned `cloud-client` crate (vendored snapshot
of RemakeBackend at `6aead4e`). No second OAuth flow is implemented.

## Components

| Area | Location |
| --- | --- |
| Vendored Cloud client | `crates/cloud-client/` (pinned snapshot, do not hand-edit; re-vendor to update) |
| Auth state machine | `crates/tiktools-cloud-auth/` (`CloudAuth`, `TokenStore`, `Persistence`) |
| Core control surface | `crates/tiktools-core/src/cloud_account.rs`, `crates/tiktools-core/src/control/cloud_account.rs` |
| Control API (native routes) | `crates/tiktools-control-api/src/modules/cloud.rs` (`cloud.*`, feature `cloud-auth`) |
| Desktop wiring | `crates/tiktools-desktop` (feature `cloud-auth`, OS keychain via `os-keychain`) |
| CLI | `crates/tiktools-cli/src/commands/cloud.rs` (`cloud status/login/logout/restore`, feature `cloud-auth`) |
| Frontend | `src/web/entities/cloud-account/`, `src/web/features/cloud-account/ui/CloudAccountEditor.vue`, Settings section |

## Flow

1. `cloud.login.begin` starts the loopback receiver, then returns the
   `browserUrl` (`{origin}/desktop/login?...&state=...&codeChallenge=...`).
2. `cloud.login.open` opens the URL in the **system browser** (never a WebView).
3. `cloud.login.complete` waits for the loopback callback (validates `state`,
   single-use code), redeems the PKCE code at
   `POST {origin}/api/v1/native/handoff/redeem`, then verifies the session
   with `GET {origin}/api/v1/native/me`.
4. The session token is stored **only** in the OS credential store
   (`OsTokenStore`, via the pinned `cloud-client` keychain adapter) and in
   process memory. It never reaches the frontend, localStorage, SQLite, logs,
   or plaintext files; only nonsecret `AccountStatus` crosses to the UI.
5. Restart: `cloud.status` / `cloud.restore` reload the token from the
   keychain and revalidate it against `/me`. Expired sessions surface
   `signedIn: false` with an error instead of failing silently.
6. `cloud.logout` revokes server-side (`POST /native/logout`) and deletes the
   keychain entry.

## Configuration

- Release builds pin the Cloud instance at build time using
  `TIKTOOLS_CLOUD_ORIGIN`, or use the built-in official origin when it is absent.
  Runtime environment variables cannot replace or disable a release's instance.
  Operators can compile an empty value, `off` or `none` to disable Cloud;
  `cloud.*` then fails closed with `cloud-not-configured`.
- Debug builds additionally accept runtime `TIKTOOLS_CLOUD_ORIGIN` for isolated
  development/staging (including the same disable values). Their precedence is:
  1. the runtime development override,
  2. the operator's build-time value,
  3. the built-in official origin
     (`tiktools_cloud_auth::DEFAULT_CLOUD_ORIGIN`,
     `https://remakebackend-production-a872.up.railway.app`).
- Signing in stays optional: the app never contacts the origin until the user
  presses **Sign in** (or a persisted session is restored), and every local
  feature works offline.
- Plain HTTP is accepted only for loopback IP literals (local development).
- Without the `cloud-auth` (resp. `os-keychain`) cargo features, the backend
  has no `cloud.*` methods and the CLI rejects `cloud ...` with a rebuild hint.
- The OS keychain is mandatory: there is no file fallback. When the keychain
  is unavailable, login reports `keychain-unavailable` and keeps no token.

## Security properties

- System browser only; loopback receiver binds before the browser opens.
- `state` mismatch and unexpected callback origins are rejected; codes are
  single-use and callback replay fails.
- Bearer [REDACTED] are attached only to native Cloud API routes.
- Tokens are `SecretString`/`Zeroize` in memory (`LoginAttempt` wipes the
  verifier material); keychain entries are namespaced by origin + account id,
  isolating multiple accounts/origins.
- No automatic fallback to insecure storage; offline startup keeps local-first
  behavior unchanged.

## Validation (2026-11-03, local)

- `cargo test -p tiktools-cloud-auth --locked`: **10 passed, 0 failed**.
  Covers login/logout (incl. fresh-process logout revocation + cleanup),
  expired session, invalid state/PKCE, callback replay, browser cancellation,
  offline startup/restore, account isolation, restart persistence, keychain
  failure. Suite previously deadlocked (callback delivered with nobody
  listening) and the mock spoke snake_case PKCE; both fixed in this PR.
- `cargo test -p tiktools-core --locked`: **292 passed**; control-api
  suites green.
- `cargo fmt --all -- --check` clean; `cargo clippy --workspace
  --all-targets --all-features --locked -- -D warnings` clean.
- Builds with `--no-default-features` (no `cloud-auth`) still compile;
  CLI `cloud ...` without the feature prints a rebuild hint.
- Frontend: `vitest` **101 files / 550 tests green**, `eslint
  --max-warnings=0` clean, `vue-tsc --noEmit` clean.
- Real-binary login E2E (`tiktools --standalone cloud ...`) against a
  disposable loopback mock Cloud **plus the real OS keychain**
  (KDE ksecretd): status → login (callback injected, state+PKCE verified,
  redeem, `/me`) → fresh-process restore (keychain persistence) → logout
  (server revoke + keychain delete). ALL-E2E-PASS; keychain left clean.
- Not validated: real Google OAuth, real OS keychains on Windows/macOS,
  production trust material. Staging keys are not production anchors.
