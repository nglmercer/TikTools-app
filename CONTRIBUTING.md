<span id="contributing-to-tiktools-"></span>

# Contributing to TikTools

This guide is for source contributors. TikTools uses Vue/Bun for the frontend
and a Rust Winit/Wry desktop host around AppCore. Keep changes focused and
preserve the shared typed control surface, runtime boundaries and existing
owners' work. User installation belongs in the
[user documentation](docs/content/en/user/installation.md).

<span id="supported-toolchain-"></span>

## Supported toolchain

- Bun 1.4.4, pinned by `package.json`, and Node.js compatible with the locked
  Vite toolchain.
- Rust 1.95.0 or newer. The workspace MSRV is 1.95; CI uses Rust 1.99.1.
- Windows: MSVC Build Tools, WebView2 and native Strawberry Perl for vendored
  OpenSSL. LLVM `lld-link` is optional; the Cargo wrapper discovers it when
  available. MSYS/Cygwin Perl is unsuitable for this MSVC build path.
- Linux desktop development: GTK 3, WebKitGTK 4.1, ALSA, appindicator, libxdo
  and `pkg-config` development packages. Use the current CI dependency list
  and [platform setup](docs/content/en/developer/setup.md).
- Android/Termux: use the native Android Rust toolchain and the
  [platform guide](docs/content/en/developer/platforms.md). Desktop WebView
  libraries are not prerequisites for the native CLI.

<span id="setup-and-checks-"></span>

## Setup and checks

Read applicable repository instructions, inspect `git status`, then install
the exact committed dependencies:

```sh
bun ci
```

Use `bun install` only for an intentional dependency change. Preserve both
lockfiles for unrelated work. The docs workspace keeps stable VitePress and
its Vite toolchain separate from the application toolchain.

Choose the appropriate edit loop:

```sh
bun run start                 # owned Vite + Wry desktop host
bun run dev                   # headless host + loopback browser adapter + Vite
bun scripts/task.ts serve     # frontend Vite server only
```

`start` can prepare plugins from a sibling `plugins-example-tiktools` checkout
or `TIKTOOLS_PLUGINS_REPO`. `TIKTOOLS_SKIP_DEV_PLUGINS=1` disables that
preparation; `dev` sets it automatically. Plugin sources, dependencies and
their tests belong to that separate repository. `build` compiles the app's
widgets, frontend and desktop binary without bundling external plugins.

Use an isolated `TIKTOOLS_HOME` for native tests or experiments. Stop a running
owner cleanly before starting a competing host, and use the same isolated home
from CLI terminals controlling that host.

Run focused checks during development:

```sh
bun run test web live-batch
bun run test bun automation
bun run test rust tiktools-core batched_likes --features native-tiktok
bun run test --changed
```

Before submitting application changes, run the relevant frontend and Rust
checks with committed lockfiles:

```sh
bun scripts/task.ts contracts --check
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked -- --test-threads=1
```

Use the narrow test runner or `bun run test --changed` while editing. The CI
scope selector omits native jobs for reviewed docs/frontend paths, retains all
gates for native/infrastructure/unknown changes, and fails closed when Git
comparison is unavailable. Reusable callers default to full validation; manual
CI accepts frontend/native/full. Keep SQLite/profile tests serial until their
shared resources are isolated. Site and release recovery run only manually;
recovery defaults to Linux verification without publication. Avoid dispatching
remote workflows for checks already established locally.

`frontend-lint` includes Vue type checking and package SDK/UI checks.
`bun run ci` also checks the signer revision, universal widget contracts,
native batched-like regression, optional local-browser adapter, sandbox API
verification and production application build. Browser tests use
`bun run test e2e`; follow the [testing guide](docs/content/en/developer/testing.md)
for built assets, Playwright browsers and fixture requirements.

Report what changed, which checks ran, their results and any remaining
environment limits. A filter that runs zero tests is not evidence. A fake-host
browser pass does not establish native Wry, real-account or physical two-PC
acceptance. Pull requests to `remake` should satisfy the applicable frontend,
Rust, desktop, MSRV, plugin runtime, CodeQL and dependency-review checks.

## Documentation and demo maintenance

Public articles live under `docs/content/en/` and `docs/content/es/`, separated
into user and developer audiences. Read current source before updating a
guide, especially when an old design contains future requirements. Update
both equivalent paths in the same change with complete localized prose.

Each public article needs exactly one H1, a clear goal and prerequisites,
concrete steps or examples, an expected result, failure recovery and an honest
feature status. Preserve stable `docId` and `translationKey` values. Metadata
includes localized `title`, `locale`, `audience`, `featureStatus`,
`contentStatus: reviewed`, `sourceLocale: en` and numeric `order`.
`featureStatus` is `available`, `experimental`, `planned` or `historical`.

Set both `sourceRevision` and `translationRevision`, in both languages, to the
SHA-256 of the English Markdown body after removing frontmatter and trimming
outer whitespace. Recompute after an English body change and review the
translation; a copied English article is not a Spanish translation. Keep UI
labels and API identifiers exact when they name real controls or methods.

```sh
bun scripts/task.ts docs check
bun scripts/task.ts docs test
bun scripts/task.ts docs build
bun scripts/task.ts demo build
TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build
bun scripts/task.ts site preview
```

For separate development loops use `bun scripts/task.ts docs dev` and
`bun scripts/task.ts demo dev`. The default combined preview uses
`http://127.0.0.1:4173/TikTools-app/`; the demo is under `demo/`. The dirty-build
flag permits a local review artifact and records its dirty status. It is not
a source revision or publication claim. Keep `TIKTOOLS_SITE_BASE` consistent
across docs, demo and assembly; see
[the demo development guide](docs/content/en/developer/demo.md).

The demo must use fictional fixtures and remain usable without native IPC,
real accounts, production requests or effectful plugin execution. Keep demo
behavior behind its explicit transport/bootstrap boundary. Do not introduce
demo-only business rules into the production core.

`docs/inventory.json` is the migration baseline. Every original path and
heading must retain a meaningful route through `docs/migration-map.json` and
the bilingual legacy stubs. Preserve the exact originals and images in
`docs/internal/archive/<original-repository-path>`; update migration notes
rather than historical bytes. Normative plans and dated evidence belong in
the internal archive. Legacy stubs remain outside public search.

The site and release-feed workflows share the exported `reapplyWithCas`
contract in `scripts/publish-site.ts`: fetch the current head on every attempt,
reconstruct only authorized changes, and use a normal fast-forward push. Site
publication requires the manual publish input on `remake` and credentials for
the selected target. The feed writer reauthenticates and selects channels from
current signed heads on every retry; it never restores an old snapshot. GitHub
concurrency groups do not serialize writers running in different repositories.
`RELEASE_REPO` selects the actual target, including when a fork builds for the
canonical repository.

The site publisher verifies the assembled manifest's hashes, source commit and
central base, and rejects dirty production artifacts. Its default mode computes
a plan; `--publish` is the explicit write mode. It preserves `updates/`, CNAME,
`.nojekyll` and all foreign paths. First publication over a legacy site requires
an explicit reviewed ownership seed with hashes matching the current branch;
the publisher refuses to adopt existing files implicitly. A conflict reapplies
the site to the new branch head without restoring any earlier feed snapshot.
The local `TIKTOOLS_SITE_ALLOW_DIRTY` flag never enables production publication.

<span id="security-and-privacy-"></span>

## Security and privacy

Never commit TikTok cookies, session tokens, API keys, private keys, `.env`
files, credentials or generated local databases. Keep session cookies in
memory and omit them from logs, issues, tests and pull requests. Use fake
credentials and isolated profile directories in fixtures.

Preserve caller identity, host capability discovery, scoped grants and native
ownership boundaries. Unknown outcomes require the existing durable receipt
recovery; a new command ID must not silently repeat an effect. Native plugins
and trusted `.node` addons execute in process, and a process boundary is not
an OS sandbox. For frontend changes use `src/web/shared/api/` and shared UI
primitives rather than recreating the retired `src/web/platform` layer.

<span id="releases-and-branch-policy-"></span>

## Releases and branch policy

Release tags are maintainer-controlled. Keep application and packager version
files synchronized through the version task, then verify them against the
staged executable:

```sh
bun scripts/task.ts version check
```

An intentional version change uses `bun scripts/task.ts version bump <version>`;
use the actual reviewed version. Packaging stages the executable, `web/`,
license and README, with separately installed external plugins. Extraction
checks complement real install, upgrade, shutdown and restart tests.

Follow `.github/workflows/release.yml` and the
[release engineering guide](docs/content/en/developer/releases.md):

- Keep historical tags and published assets immutable. Produce and verify
  checksums before exposing a release.
- Sign installer bytes and exact manifest bytes using protected signing
  secrets. Embed only the trusted public key; plugin publisher keys are
  separate from application updater keys.
- Upload to a draft release, download the uploaded assets and verify their
  exact names and hashes, then publish the verified draft.
- Promote signed update channels only after their referenced assets are
  published and verified. Authenticate existing channel heads, enforce SemVer
  ordering and serialize publishers.
- Use bootstrap only for genuinely absent initial channel heads. Preserve a
  valid feed if signing, upload, verification or publication fails.
- Keep experimental installer gates dependent on their platform acceptance.
  Source versions, release notes and queued or failed jobs are not proof that
  a release or website is published.

Repository administrators should protect `remake` with pull requests, required
applicable status checks and an up-to-date branch; keep force pushes and branch
deletion disabled. Preserve other owners' changes and document any checks
that the local environment cannot run.

The [exact original contributor guide](docs/internal/archive/CONTRIBUTING.md)
is retained with its hashes in the [migration map](docs/migration-map.json).
