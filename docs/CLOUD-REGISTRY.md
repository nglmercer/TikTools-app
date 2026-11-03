# Official Cloud registry integration

TikTools Desktop can install and update plugins from a RemakeBackend Cloud
registry without a Cloud login. Public downloads stay offline-verifiable: the
client pins its trust roots independently and checks every signature and
package digest itself.

## How it works

A Cloud origin publishes immutable snapshot generations instead of a live
registry tree:

- `GET {origin}/api/v1/registry/current` is the atomic pointer. It is never
  cached and answers `{"generation": N, ...}`, or 404 when nothing has been
  published yet.
- `GET {origin}/api/v1/registry/generations/{N}/files/{path}` serves one
  immutable snapshot: the standard v1 documents (`root.json`, `keys.json`,
  `config.json`, `catalog/page-*.json`, `index/*/*.json`, each with a `.sig`
  sidecar) plus the signed `.plugin` archives. Files are safe to cache
  forever; superseded generations stay readable.

On every refresh, search, install, or update check touching the official
source, `RegistryClient` first resolves the active generation and pins the
stored source URL to that generation's endpoint. The snapshot taken at the
start of an operation is used for the whole operation, so a generation change
mid-operation cannot mix snapshots. The next operation picks up the new
generation automatically.

Trust never comes from the server. The root keys pinned on the source are
provisioned independently (compile-time pins in Desktop releases); the served
`root.json` is verified like any other document and cannot introduce trust.
Without pinned roots the source stays unverified and installs require
explicit consent.

When the origin is unreachable but a generation was resolved before, the
client keeps that generation URL so cached signed metadata and installed
plugins keep working offline. An authoritative "no published generation"
(404 on `/current`) fails closed instead and is recorded on the registry
status, which the plugin sources UI already renders per registry along with
the verified badge, last refresh time, advisories, and revocations.

## Configuration

The official source is a builtin and cannot be removed, only disabled:

| Variable | Meaning |
| --- | --- |
| `TIKTOOLS_OFFICIAL_REGISTRY_URL` + `TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS` | Static v1 tree (previous behavior, kept for backwards compatibility). |
| `TIKTOOLS_CLOUD_ORIGIN` + `TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS` | Cloud origin with per-refresh generation discovery. |
| `TIKTOOLS_STAGING_CLOUD_ORIGIN` + `TIKTOOLS_STAGING_REGISTRY_ROOT_KEYS` | Disposable staging registry under the separate `staging` source. |

Root keys are JSON arrays of public keys, baked in at compile time. Staging
uses its own source name and keys so a disposable staging anchor can never be
mistaken for the official one; configuring only half of a staging pair leaves
staging disabled rather than running unverified.

Production trust pins must be distributed through an independent channel
(offline root ceremony); see the Cloud `REGISTRY-OPERATIONS.md` runbook. No
staging key may ever be presented as an official release.

## Verification performed

- `cargo test -p tiktools-plugin-registry` — 17 tests, including Cloud
  discovery/refresh/search, install-download verification, update across
  generations, offline cached metadata, wrong root, tampered index, digest
  mismatch, yank/revocation/advisory enforcement, interrupted download,
  generation-change pinning, third-party coexistence, and config round-trip.
- `cargo test -p tiktools-plugin-installer` — 20 tests, including a full
  Cloud install → update → yank-rejection loop through the real prepare,
  preview, consent, and commit pipeline.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`
  are clean.

## Real-binary end-to-end validation

With disposable keys from `tiktools-plugin keygen`, a declarative demo
plugin, and a static file server speaking the exact backend endpoint shape
(`GET /api/v1/registry/current` plus generation-scoped files), the real
`tiktools --standalone` CLI built with `TIKTOOLS_CLOUD_ORIGIN` and an
independently pinned `TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS` was verified to:

- list `official` as a verified builtin with the pinned fingerprint;
- refresh generation 1, discover, fetch, search, and install `acme.demo`
  0.1.0 with signature verification enforced;
- advance to generation 2, report the 0.2.0 update, and install it;
- keep working from cached signed metadata with the origin down
  (refresh/package/list all served);
- fail closed installing from a tampered index document
  (`failed verification: plugin signature does not verify`).

## Limitations

- The Cloud mocks in these tests speak the exact backend endpoint shape
  (`/api/v1/registry/current` plus generation-scoped files), but they are not
  the backend itself; backend compatibility is covered by the Cloud
  repository's own `registry_tests` against the pinned client.
- No full GUI boot is exercised here; the install loop above runs the real
  installer pipeline into temporary directories instead.
- Desktop releases still need independently provisioned official root pins,
  authorized operators, and reviewed deployment infrastructure before any
  production use.
