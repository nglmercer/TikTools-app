> Historical planning record: package-script names below (lint/typecheck/test/build:web, check:web, start:rust) predate the five-script consolidation. Current commands are bun run start|dev|build|lint|ci plus bun scripts/task.ts internal steps; see README.md and CONTRIBUTING.md.

# TikTools — CI, Security, Release & Code Hardening Task

Repository:

```text
https://github.com/nglmercer/TikTools-app
```

Target branch:

```text
remake
```

## Objective

Prepare the current Rust + Vue/Bun remake of TikTools for reliable CI and future GitHub Releases.

Apply all required changes to:

* fix known unsafe Rust behavior
* improve frontend linting
* make dependency installs reproducible
* add CI
* add security scanning
* add dependency automation
* establish consistent version validation
* build release artifacts correctly
* prevent broken releases from being published
* keep the current architecture and behavior intact

Do not perform unnecessary architectural rewrites.

---

# 1. Fix native plugin FFI zero-length buffer handling

File:

```text
crates/tiktools-plugin-loader/src/native.rs
```

There is currently a potentially invalid use of:

```rust
std::slice::from_raw_parts(response.ptr, response.len)
```

when:

```text
response.ptr == null
response.len == 0
```

`PluginBuffer::empty()` intentionally starts with a null pointer and zero length.

Change the native plugin response handling so that:

```rust
response.len == 0
```

returns:

```rust
Vec::new()
```

without calling:

```rust
std::slice::from_raw_parts
```

The logic should still reject:

```text
response.len > MAX_FRAME_BYTES
```

and:

```text
response.len > 0 && response.ptr.is_null()
```

Do not weaken any existing validation.

Expected shape:

```rust
let bytes = if status == PluginStatus::Ok {
    if response.len > tiktools_plugin_api::MAX_FRAME_BYTES
        || (response.len > 0 && response.ptr.is_null())
    {
        unsafe { free_buffer(&mut response) };
        return Err(PluginLoaderError::Runtime(
            "plugin returned an invalid buffer".to_owned(),
        ));
    }

    if response.len == 0 {
        Vec::new()
    } else {
        unsafe {
            std::slice::from_raw_parts(response.ptr, response.len).to_vec()
        }
    }
} else {
    Vec::new()
};
```

Preserve the existing `free_buffer` behavior.

## Add tests

Add a regression test covering a native plugin response where:

```text
status = PluginStatus::Ok
ptr = null
len = 0
```

The result must be an empty `Vec<u8>` and must not cause undefined behavior or panic.

If direct dynamic-library testing is impractical, extract the buffer-copy validation into a small internal helper that can be unit tested independently.

Also test:

```text
len > 0 + null ptr => error
len > MAX_FRAME_BYTES => error
len == 0 => empty Vec
```

Keep unsafe code isolated.

---

# 2. Standardize Bun as the frontend package manager

The repository currently contains both:

```text
bun.lock
package-lock.json
```

TikTools uses Bun for development and scripts.

Make Bun the canonical frontend package manager.

## Required changes

Delete:

```text
package-lock.json
```

Do not delete:

```text
bun.lock
```

Add a pinned package manager declaration to `package.json`.

Use the Bun version available in the current development environment if it can be determined reliably.

Example:

```json
{
  "packageManager": "bun@1.x.x"
}
```

Do not use `"latest"`.

If the exact currently intended Bun version cannot be determined from project history or tooling, choose a stable current Bun version and document the choice.

CI must use the same pinned version.

---

# 3. Add frontend ESLint

The project currently has TypeScript checking but no dedicated lint command.

Add ESLint suitable for:

* TypeScript
* Vue 3
* Vue SFC files
* TS/TSX/JSX currently used by the project

Prefer modern flat ESLint configuration.

Install appropriate development dependencies, likely including:

```text
eslint
typescript-eslint
eslint-plugin-vue
vue-eslint-parser
```

Add an ESLint config at the repository root.

Suggested file:

```text
eslint.config.js
```

or:

```text
eslint.config.mjs
```

The lint configuration should be strict enough to catch real defects but should not force a large unrelated rewrite of existing source code.

Do not duplicate TypeScript's type checker.

Avoid initially enabling noisy stylistic rules.

## package.json

Add:

```json
"lint": "eslint src scripts vite.config.ts --max-warnings=0"
```

Adjust paths if necessary.

The following commands must exist and work:

```bash
bun run lint
bun run typecheck
bun run test
bun run build:web
```

If legitimate existing code violates lint rules, either fix the code or configure the rule appropriately.

Do not blanket-disable linting.

---

# 4. Preserve TypeScript strictness

Keep the existing TypeScript strict configuration.

Do not disable:

```text
strict
noUncheckedIndexedAccess
noImplicitOverride
noFallthroughCasesInSwitch
```

Do not reduce compiler safety merely to make CI pass.

Existing:

```text
noUnusedLocals = false
noUnusedParameters = false
```

may remain disabled initially.

ESLint may enforce unused values where reasonable, but avoid large unrelated cleanup.

---

# 5. Add Rust formatting and lint validation

CI must run:

```bash
cargo fmt --all -- --check
```

and:

```bash
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

The workspace currently requires:

```text
rust-version = 1.90
```

The pinned `tiktok-signer` revision currently pulls `rquickjs 0.12.2` (Rust 1.87), and `tray-icon` 0.26 (`muda` 0.21) requires Rust 1.90. These are
existing dependency requirements, so use Rust `1.90.0` explicitly in CI and document the MSRV change
rather than allowing the lockfile to fail on 1.88.

Do not raise the MSRV unless necessary and justified.

Fix all clippy errors introduced or currently exposed by the chosen configuration.

Do not solve warnings through broad:

```rust
#[allow(...)]
```

unless there is a concrete reason documented in code.

---

# 6. Add main CI workflow

Create:

```text
.github/workflows/ci.yml
```

CI should run for:

```yaml
pull_request:
  branches:
    - remake

push:
  branches:
    - remake
```

Use minimal permissions:

```yaml
permissions:
  contents: read
```

Add concurrency cancellation:

```yaml
concurrency:
  group: ci-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

## Frontend job

Use Ubuntu.

Steps should include:

```text
checkout
setup pinned Bun
bun ci
bun run lint
bun run typecheck
bun run test
bun run build:web
```

Use dependency caching where practical.

Use:

```bash
bun ci
```

rather than:

```bash
bun install
```

in CI so lockfile drift fails the build.

## Rust job

Use Ubuntu.

Install Linux dependencies necessary for Wry/Winit/WebKitGTK and audio support.

Likely packages include:

```text
libgtk-3-dev
libwebkit2gtk-4.1-dev
libasound2-dev
pkg-config
```

Adjust based on the actual Ubuntu runner package names.

Install:

```text
Rust 1.90.0
rustfmt
clippy
```

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

Add Cargo caching.

Do not use floating Rust `stable` for the main verification job.

---

# 7. Add desktop cross-platform compilation checks

Add a separate job in CI or a separate workflow.

Preferred CI matrix:

```text
ubuntu-24.04
windows-latest
macos-latest
```

Run at minimum:

```bash
cargo check -p tiktools-desktop --locked
```

Install platform-specific dependencies as required.

Linux needs WebKitGTK/GTK/audio packages.

Windows and macOS should use their normal Wry platform dependencies.

This matrix is intended to catch platform-specific compilation failures.

If `--all-features` causes a platform-specific unsupported configuration, document why and use the correct platform-compatible feature set.

Do not silently skip a platform.

---

# 8. Add GitHub CodeQL

Create:

```text
.github/workflows/codeql.yml
```

Analyze:

```text
Rust
JavaScript / TypeScript
```

Run on:

```text
pull requests to remake
pushes to remake
weekly schedule
```

Use minimal necessary GitHub permissions.

Typical permissions:

```yaml
security-events: write
packages: read
contents: read
```

Use the officially supported CodeQL actions.

Do not include languages not present in the repository.

---

# 9. Add dependency review

Create a workflow for dependency review on pull requests.

Suggested file:

```text
.github/workflows/dependency-review.yml
```

Trigger:

```yaml
pull_request:
  branches:
    - remake
```

Use GitHub's official Dependency Review Action.

Fail pull requests that introduce known vulnerable dependencies at a sensible severity threshold.

Suggested threshold:

```text
high
```

If repository visibility or GitHub plan prevents this feature from running, retain the workflow if appropriate and document the limitation.

---

# 10. Add Dependabot

Create:

```text
.github/dependabot.yml
```

Configure dependency updates for:

```text
cargo
npm
github-actions
```

Even though Bun is used, use the npm ecosystem for `package.json` dependencies.

Suggested cadence:

```text
weekly
```

Group reasonable development dependency updates when useful.

Avoid excessive PR noise.

Do not automatically merge updates.

---

# 11. Pin GitHub Actions safely

For newly created workflows, prefer trusted official Actions.

Examples:

```text
actions/checkout
oven-sh/setup-bun
actions/cache
github/codeql-action
actions/dependency-review-action
```

Prefer pinning third-party and security-sensitive actions to full commit SHAs.

At minimum, avoid arbitrary untrusted actions.

If full SHA pinning is used, add a short comment indicating the corresponding release tag for readability.

Example pattern:

```yaml
uses: actions/checkout@<FULL_COMMIT_SHA> # v4.x.x
```

Do not invent SHAs.

Resolve them from trusted upstream releases.

---

# 12. Add CI-friendly project scripts

Improve `package.json` so developers can run the same checks locally.

Keep existing scripts.

Add useful aggregation scripts such as:

```json
"check:web": "bun run lint && bun run typecheck && bun run test && bun run build:web",
"check:rust": "cargo check --workspace --locked",
"lint:rust": "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
"fmt:rust": "cargo fmt --all -- --check",
"ci": "bun run check:web"
```

Do not overload one script with platform-specific system package installation.

Preserve:

```text
start
prepare:dev-plugins
start:rust
dev
build:web
serve:web
typecheck
test
test:rust
build:desktop
```

Update existing Rust scripts to include `--locked` where appropriate.

Example:

```json
"check:rust": "cargo check --workspace --locked",
"test:rust": "cargo test --workspace --locked",
"build:desktop": "cargo build -p tiktools-desktop --release --locked"
```

---

# 13. Establish version consistency

Current versions are inconsistent:

```text
package.json: 1.0.0
Cargo workspace: 0.1.0
historical GitHub releases: v3.8.x
```

For the remake, treat:

```toml
[workspace.package]
version = "..."
```

in the root `Cargo.toml` as the canonical application version.

Do not automatically change the current version number without considering compatibility and release history.

Add a version validation script.

Suggested:

```text
scripts/check-version.ts
```

It should:

1. read root `Cargo.toml`
2. extract `[workspace.package].version`
3. optionally read `package.json`
4. validate package.json version matches Cargo version, if package.json is intended to expose the app version
5. if supplied a Git tag, validate:

```text
tag == "v" + cargo_version
```

Example:

```bash
bun run scripts/check-version.ts v0.1.0
```

Add a package script:

```json
"check:version": "bun run scripts/check-version.ts"
```

For release CI, pass:

```text
github.ref_name
```

Do not silently rewrite versions during CI.

---

# 14. Decide how package.json version is handled

Preferred approach:

Keep:

```text
package.json
Cargo.toml
```

on the same version.

Cargo remains the source of truth.

If package.json version is not semantically meaningful because the package is private, it should still match Cargo to avoid confusion.

Do not keep:

```text
Cargo 0.1.0
package.json 1.0.0
```

without explicit documentation.

If changing the current version would create an inappropriate release tag, document the discrepancy and add a follow-up issue instead of guessing.

---

# 15. Add release workflow

Create:

```text
.github/workflows/release.yml
```

Do not release on every merge.

Trigger releases only via:

```yaml
push:
  tags:
    - "v*"
```

Optionally support:

```text
workflow_dispatch
```

for release testing.

A tagged release must first pass the same checks as CI.

Required sequence:

```text
checkout
validate tag/version
install pinned Bun
bun ci
lint
typecheck
frontend tests
build frontend
Rust fmt/clippy/tests where practical
build release binary
assemble complete application package
calculate checksums
upload GitHub Release assets
```

---

# 16. Package frontend assets with the desktop executable

The release binary is not sufficient by itself.

The Rust desktop host expects packaged frontend assets relative to the executable.

Current frontend output:

```text
dist/web/
```

Final release packages must contain something equivalent to:

```text
TikTools/
├── tiktools-desktop.exe
└── web/
    ├── index.html
    └── assets/
```

On Unix-like targets:

```text
TikTools/
├── tiktools-desktop
└── web/
```

Copy:

```text
dist/web
```

to:

```text
web
```

inside the release bundle.

Do not rely on a development localhost server.

Do not use:

```text
TIKTOOLS_DEV_URL
```

in release artifacts.

Add a validation step that confirms:

```text
web/index.html
```

exists in each final package.

---

# 17. Release build matrix

Create release artifacts for supported desktop platforms.

Recommended matrix:

```text
Windows x86_64
Linux x86_64
macOS arm64
macOS x86_64
```

If one of these targets cannot yet be supported reliably, document it and initially produce only validated targets.

Do not pretend a build succeeded if packaging is incomplete.

Preferred first-stage artifacts:

```text
TikTools-<version>-windows-x86_64.zip
TikTools-<version>-linux-x86_64.tar.gz
TikTools-<version>-macos-arm64.tar.gz
TikTools-<version>-macos-x86_64.tar.gz
```

Do not attempt a complex installer system until portable packages are reliable.

---

# 18. Generate SHA-256 checksums

For all release archives generate SHA-256 hashes.

Create:

```text
SHA256SUMS.txt
```

Example:

```text
<hash>  TikTools-0.1.0-windows-x86_64.zip
<hash>  TikTools-0.1.0-linux-x86_64.tar.gz
```

Upload it as a GitHub Release asset.

---

# 19. Use GitHub Release generation

The release workflow may use:

```text
gh release create
```

or an official/trusted release action.

Prefer GitHub CLI if it keeps permissions and behavior simple.

Use minimal permissions:

```yaml
permissions:
  contents: write
```

only in the release job that requires it.

Do not grant write permissions to CI jobs.

Generate release notes automatically where appropriate.

For prerelease tags such as:

```text
v4.0.0-alpha.1
```

mark them as prereleases.

---

# 20. Do not reuse legacy Electron/Squirrel release files

Historical releases contain artifacts such as:

```text
RELEASES
*.nupkg
latest.yml
old setup.exe formats
```

The current remake is a Rust Winit/Wry application.

Do not recreate legacy Electron/Squirrel release artifacts unless an updater implementation explicitly requires them.

The new release process should be designed around the Rust application.

---

# 21. Add release packaging script

Prefer implementing packaging in a script rather than embedding excessive shell logic in YAML.

Suggested:

```text
scripts/package-release.ts
```

or platform-specific scripts under:

```text
scripts/release/
```

Responsibilities:

```text
locate compiled binary
verify dist/web/index.html
create staging directory
copy binary
copy dist/web -> web
copy optional README/LICENSE
create platform archive
produce predictable artifact name
```

Keep GitHub workflow YAML readable.

Do not introduce a large build framework solely for packaging.

---

# 22. Add license file if missing

The Cargo workspace declares:

```text
MIT
```

Check whether the repository contains a root:

```text
LICENSE
```

If missing, add the standard MIT License with the correct copyright owner/year.

Do not claim an MIT license in Cargo without including the license text in release distributions.

Include `LICENSE` in release archives.

---

# 23. Audit secrets and generated files

Verify `.gitignore` continues to exclude:

```text
node_modules
dist
target
.dev-plugins
.plugin-staging
local SQLite files
logs
```

Check the tracked repository for obvious secrets, especially:

```text
TikTok cookies
session tokens
API keys
private keys
.env files
credentials
```

Do not print secret values into logs.

If no secrets are found, make no unrelated changes.

Add patterns such as:

```text
.env
.env.*
!.env.example
```

only if appropriate for the project.

---

# 24. Preserve WebView security behavior

Do not weaken existing security controls in:

```text
crates/tiktools-desktop/src/webview.rs
```

Preserve:

```text
same-origin navigation restrictions
loopback-only development URLs
production packaged assets
asset path canonicalization
path traversal rejection
Content Security Policy
credential rejection in development URLs
```

Do not add:

```text
script-src 'unsafe-eval'
```

Do not broaden navigation to arbitrary remote sites.

Do not enable production devtools by default.

---

# 25. Preserve plugin installer security

Do not weaken existing plugin validation.

Preserve protections for:

```text
path traversal
unsafe symlinks
checksum validation
ABI/protocol validation
package-root containment
entry validation
permissions/capabilities
```

Native plugins should continue to be documented as trusted native code.

Process boundaries must not be described as OS sandboxes.

---

# 26. Review process plugin timeout behavior

Current process plugin calls use a timeout.

Preserve this behavior.

Verify tests exist for:

```text
successful response
wrong request id
protocol mismatch
invalid JSON
timeout / terminated child
```

If missing, add focused tests where feasible.

Do not make plugin calls unbounded.

---

# 27. Add README CI documentation

Update the root `README.md` with a short Development / Quality section.

Document commands such as:

```bash
bun ci
bun run lint
bun run typecheck
bun run test
bun run build:web

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

Mention that PRs are expected to pass CI.

Do not make the README excessively long.

---

# 28. Update development documentation

Update:

```text
docs/GETTING_STARTED.md
docs/DEVELOPMENT.md
```

where necessary to reflect:

```text
pinned Bun
bun ci for clean validation
lint command
Rust formatting/clippy requirements
release packaging layout
```

Keep docs consistent with actual scripts.

---

# 29. Add CONTRIBUTING.md if missing

If no contribution guide exists, add a concise:

```text
CONTRIBUTING.md
```

Include:

```text
supported Rust version
required Bun version
setup
frontend checks
Rust checks
PR expectations
do not commit secrets/cookies
release tags are maintainer-controlled
```

Avoid unnecessary bureaucracy.

---

# 30. GitHub branch protection documentation

GitHub branch protection itself may require repository settings rather than code changes.

Add a section to documentation recommending that `remake` require these checks before merge:

```text
frontend
rust
desktop cross-platform check
CodeQL where appropriate
dependency review where available
```

Recommend:

```text
require pull request
require status checks
require branch up to date
prevent force pushes
prevent branch deletion
```

Do not attempt to modify repository settings unless the environment explicitly has permission.

---

# 31. Validate lockfiles

After dependency changes run:

```bash
bun install
```

to update:

```text
bun.lock
```

Then validate:

```bash
bun ci
```

Rust dependency resolution must retain:

```text
Cargo.lock
```

Run:

```bash
cargo check --workspace --locked
```

Do not remove `Cargo.lock`.

This is an application repository, so the Cargo lockfile should stay committed.

---

# 32. Required local validation

Before considering the task complete, run all feasible checks.

Frontend:

```bash
bun ci
bun run lint
bun run typecheck
bun run test
bun run build:web
```

Rust:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build -p tiktools-desktop --release --locked
```

Version:

```bash
bun run check:version
```

Packaging:

```text
verify release staging contains binary
verify web/index.html exists
verify archive can be opened
verify SHA256 file matches generated artifacts
```

If the environment cannot build the desktop application due to missing OS libraries, install the documented dependencies if possible.

Do not claim tests passed unless they were actually run successfully.

---

# 33. Inspect generated desktop package

For a release staging directory verify it looks like:

```text
TikTools/
├── LICENSE
├── README.md
├── tiktools-desktop[.exe]
└── web/
    ├── index.html
    └── assets/
```

Optional documentation files are fine.

The binary must be able to locate the frontend using the application's existing packaged asset resolution.

---

# 34. Avoid unrelated refactors

Do not:

* migrate away from Vue
* migrate away from Bun
* migrate from Winit/Wry to Tauri
* rewrite Rust architecture
* replace SQLite
* rewrite IPC
* redesign plugin APIs unless required to fix a demonstrated bug
* reformat the entire frontend unnecessarily
* change UI behavior unrelated to this task

Keep the patch focused.

---

# 35. Commit structure

If creating commits, use logically separated commits.

Suggested sequence:

```text
fix(plugin-loader): handle empty native plugin buffers safely

chore(frontend): add eslint and standardize bun tooling

ci: add frontend rust and desktop validation

ci(security): add codeql dependency review and dependabot

build(release): add version validation and portable packaging

docs: document CI and release workflow
```

Do not combine unrelated code changes in one large commit when avoidable.

---

# 36. Final report required

At completion provide:

## Changed files

List every created, modified, and deleted file.

## Bug fixes

Explain the FFI buffer bug and how it was fixed.

## CI

List all CI jobs and their purpose.

## Security

List CodeQL, dependency review, Dependabot, and any repository security findings.

## Release

Explain:

```text
trigger
version source
build targets
package layout
artifact naming
checksums
```

## Verification

Provide exact results for:

```text
bun ci
bun run lint
bun run typecheck
bun run test
bun run build:web
cargo fmt --all -- --check
cargo clippy ...
cargo test ...
cargo build ...
```

Use:

```text
PASS
FAIL
NOT RUN
```

for each.

If anything fails, include the exact blocker.

Do not report completion while known required checks are failing.

---

# Definition of Done

This task is complete when:

* native plugin zero-length buffer handling is safe
* regression tests exist
* only Bun's lockfile is used for frontend dependencies
* Bun version is pinned
* ESLint exists and passes
* TypeScript typecheck passes
* frontend tests pass
* frontend production build passes
* `cargo fmt` passes
* `cargo clippy -D warnings` passes
* Rust workspace tests pass
* release desktop binary builds
* CI exists for PRs and pushes
* desktop compilation is checked across supported platforms
* CodeQL exists for Rust + JS/TS
* dependency review exists
* Dependabot exists
* version validation exists
* release workflow exists
* release artifacts include `web/`
* release archives include license
* SHA-256 checksums are generated
* documentation matches actual commands
* no security protections were weakened
* no unrelated architecture changes were introduced

The result should leave the `remake` branch in a state where GitHub Actions can act as the release gate.
