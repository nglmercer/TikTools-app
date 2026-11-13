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
Without pinned roots the product does not register the source or permit a
Cloud installation. The explicit local-package sideload policy remains
separate and still requires its normal review.

The product fails closed if it cannot recheck the current generation: it does
not use a cached generation as authority for another network installation or
update. Installed plugins remain available locally. The standalone generic
registry library retains its separate offline-cache behavior.

## Configuration

The product uses one fixed Cloud instance for account, catalog and signed
registry discovery. Root pins are provisioned independently at build time.

| Variable | Meaning |
| --- | --- |
| `TIKTOOLS_OFFICIAL_REGISTRY_ROOT_KEYS` (origin from the account resolver: `TIKTOOLS_CLOUD_ORIGIN` or the built-in default) | Cloud origin with per-refresh generation discovery. |

Root keys are JSON arrays of public keys, baked in at compile time. Without
pinned official root keys the official source is not registered at all (it is
never trusted from keys served by the origin itself), and installs from the
registry cannot be reviewed. Plugins, Explore does not depend on it: it lists the Cloud catalog
(`catalog.items`); only hosts that do not announce that method fall back to the registry search,
which reports the marketplace as unavailable. Legacy static-tree and staging
environment variables no longer configure product sources; build the product
for the intended Cloud origin and provision its independent pins instead.
Previously configured extra sources remain in the local configuration file but
are neither trusted nor queried by the product. The product rejects source
changes and arbitrary remote package URLs; explicit local package import
continues to follow the sideload and consent policy.
The sideload settings admission fence is process-local to one AppCore;
separate processes or direct configuration edits are not synchronized and
must not be treated as cross-process enforcement.

For Cloud installation the product binds metadata, the archive and trust to
one immutable registry generation. It resolves the active generation again
after downloading and preparing the package, checks the exact version,
target, digest and size against the live v2 catalog, then rechecks the active
generation before the local commit. Yanked releases, revoked keys, disallowed
publishers, missing status and active advisories fail closed. Advisory-aware
approval and presentation remain pending, so a release with an active
advisory cannot currently be installed through this path. A server-side
change after the final check can still race the local commit; eliminating
that distributed window requires backend coordination.

The signed registry version list, release details and update summaries show
the last observed live status for the exact host artifact. A bounded catalog
query checks its generation, target, digest and size, and rejects duplicate or
incomplete results. Missing or stale-generation data is shown as unknown and
does not enable installation or automatic update. Existing receipts without
provenance keep the installed artifact's status unknown. New verified Cloud
installs record the original target and generation as part of the local
receipt; later checks bind that historical target to the current signed
artifact, publisher, signing key and live status rather than inferring them
from a digest. Update checks admit
only one run at a time and share a 30-second batch deadline; remaining items
become unknown. A shown status is an observation, not a lease: the server may
change it immediately afterwards, so installation repeats the check.

The product rollback action currently restores only local-file packages with
local installation enabled. The exact retained tree must match an integrity
proof recorded from the reviewed archive before activation. This proof is
checked under the same cooperative package-store lease as the swap. Packages
without that proof, including older retained versions and Cloud releases,
must instead be installed again through the normal review and Cloud status
checks. The proof is not authenticated against an independent process that
can rewrite the package store and its proof files, and an interrupted write
or loss of the proof can make a local rollback unavailable.
Store directories and metadata are checked against symbolic links, and
metadata is published through exclusive temporary files. These checks share
the cooperative store lease; a separate process racing to replace path
components is outside that lease's guarantees.
Migration recovery is tested for process interruption after visible writes;
the marker and directory are not fsynced, so power loss can leave metadata
missing and require manual review.

Production trust pins must be distributed through an independent channel
(offline root ceremony); see the Cloud `REGISTRY-OPERATIONS.md` runbook. No
staging key may ever be presented as an official release.

## Earlier verification

The checks below describe the earlier generic multi-registry implementation;
they are not evidence of acceptance for the fixed-instance product policy.

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

This is historical validation of the earlier generic registry policy, not a
current fixed-instance product acceptance test.

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
