# Dependency refresh (2026-10-26)

The `chore/dependency-refresh` PR branch applies the dependency refresh to
`remake` at `05780e4`. It preserves the newer application changes already merged
there and contains no separate local-browser feature commits.

## Versions

- Bun package manager and CI: 1.4.4; CI Rust toolchain: 1.99.1.
- Frontend packages and transitive lock entries refreshed with `bun update --latest`.
  TypeScript 7.0.2 is installed as `@typescript/native`, alongside TypeScript 6.0.3
  for the current Vue/ESLint compiler API consumers. Other direct packages have no
  outstanding version updates in `bun outdated` at audit time.
- Cargo direct registry requirements raised to the current stable releases,
  including zip 9.1.2, rodio 0.23.0, rusqlite 0.41.0, rfd 0.17.2,
  rand 0.10.3 and getrandom 0.4.3. Refresh includes the
  independent example and native-fixture manifests and lockfiles.
- tiktok-signer Git crates pinned together to
  `141d39ea795536190fcf26b9dcab60c50637417b`; the revision verifier uses the same pin.
- textintel example pinned to `ab4377212a3a295bda099d4d5c4900f01acdf730`.
- GitHub Actions updated to CodeQL 4.39.1, upload-artifact 7.0.2 and
  download-artifact 8.0.2, retaining immutable commit pins. Other configured
  actions already match their latest releases.
- Dependabot uses the `bun` ecosystem instead of the failing npm configuration.
- napi-vm stays pinned until the separate [VM migration](NAPI-VM-MIGRATION-PLAN.md).

## Compatibility changes and constraints

ZIP 9 makes filename decoding fallible. The plugin installer propagates decoding
errors before path validation/extraction, and the packaging tests unwrap valid
fixture names. A malformed UTF-8 archive regression covers inspection and
installation without extracting files.

The upstream base already contains the event-history type alias required by
the newer Clippy; that implementation is retained. Lifecycle test
mocks retain native coverage on desktop and use the other in-process kind on
Android; they never load native code. The provider backfill test verifies clean
rejection of Android's unsupported target instead of expecting a desktop addon.
These fixture corrections add no ignored tests and do not bypass production
ABI admission rules. Native-addon integration cases now use the same Linux,
macOS and Windows cfg as the production backend. An Android integration case
checks rejection at both discovery and direct load; Android does not claim to
validate desktop TSFN delivery or native-addon reload behavior.

`rand` 0.10 moves extension methods to `RngExt`; the local-web pairing/token code
imports that trait. `rfd` 0.17 removes its `tokio` feature; the desktop uses its
synchronous file dialog interface and retains `xdg-portal`.

GTK remains 0.18.2. The latest Wry 0.57.1 and WebKitGTK 2.0.2 require GTK 0.18;
adding GTK 0.19 produces conflicting `gdk-3` native-library links and cannot be
resolved alongside those current releases. Revisit when those upstreams migrate.

The Android QuickJS feature dependency stays on rquickjs 0.12.2 to match the
latest tiktok-signer revision. It exists to enable bindgen on the signer's
engine. Updating this declaration to 0.14 would compile a second engine and
leave the actual signer without Android bindings. Upgrade it together with
ttl-sign-embedded when upstream migrates.

TypeScript 7.0 does not expose the compiler API required by typescript-eslint
and vue-tsc. Keep the latest TypeScript 6 compiler API alongside the native 7
compiler until these tools support the new API. This follows the upstream
[side-by-side guidance](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/#running-side-by-side-with-typescript-6.0).
Use `bun node_modules/@typescript/native/bin/tsc --version` to inspect the native
compiler on a supported platform. Its native package does not support Android;
Vue/ESLint checks use TypeScript 6 on Android too.

`cargo update --dry-run --verbose` resolves zero further updates. Some transitive
versions remain constrained by upstream requirements: generic-array 0.14.7,
matchit 0.8.4, toml 0.8.2, toml_datetime 0.6.3 and toml_edit 0.20.2. Do not
force incompatible transitive overrides into those upstream dependencies.

## Validation environment

Local validation runs on Android/Termux with Rust 1.99.1 and Bun 1.4.3.
The preinstalled Node executable fails dynamic linking. A Node 26.12.1 runtime
and its libc++ package were downloaded and extracted under the ignored
`.codex-tmp` directory; validation uses its wrapper via PATH without installing
system packages. Vue typechecking must use Node: Bun bypasses the compiler
patch and reports spurious unresolved `.vue` modules.

The following full validation was performed on the original implementation
snapshot `705e6f4`, based on application commit `39a0836`, before porting the
refresh onto current `remake`. These counts are historical evidence, not a
claim that the final PR head has already passed its desktop CI gates.

Completed checks on that original snapshot:

- ESLint and Vue typechecking for the app and SonicBoom UI.
- 287 Bun tests; 102 plugin tests; 470 browser unit tests across 87 files.
- Browser tests use one worker and a 30-second timeout on this loaded device:
  `bun run test:web --maxWorkers=1 --testTimeout=30000`.
- Web production build, all five widget builds and SonicBoom UI production build.
- Universal SDK integrity check, generated-contract check and signer-pin check.
- Native napi-rs fixture compilation; gateway example: 30 tests passed.
- Textintel example: 33 tests passed; audio example compiled and ran its empty test harness.
- Sandbox control API verification: `cargo run -p tiktools-cli --features local-web --locked -- api verify --sandbox`.
- Rust formatting and whitespace checks.
- `cargo check --workspace --exclude tiktools-desktop --all-targets --all-features --locked`.

The first browser run exceeded default timeouts under concurrent compilation;
the complete single-worker rerun passed. The first Rust/contract runs failed on
the system protoc library loader; the local library override resolves that.

Final headless Rust verification:

- `cargo test --workspace --exclude tiktools-desktop --all-features --locked`:
  704 passed, 0 failed, 0 ignored (including doctests).
- `cargo clippy --workspace --exclude tiktools-desktop --all-targets --all-features --locked -- -D warnings`: passed.
- The invalid UTF-8 archive regression and Android native-addon rejection
  integration test passed. Desktop native-addon integration is OS-gated to match
  the production backend and was not executed here.

Desktop Linux/Windows builds, audio/window/tray smoke tests and supported-desktop
native-addon integration remain necessary before merging or releasing. Android
results are headless validation, not proof of desktop execution.

## PR integration validation

The refresh was ported onto `remake` at `05780e4`; signer pins and lockfiles were
resolved against that base, preserving the upstream-decoder guard. The latest
SonicBoom UI was rebuilt from that base rather than copying the old compiled
asset. A frozen Bun install and that UI production build passed. CI on the PR
must validate the final combined application, including desktop builds and
native-addon execution. Final-head local checks are noted in the PR description.
