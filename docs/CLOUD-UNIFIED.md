# Unified Cloud in the app

This page describes what the app does with the v3 Cloud surfaces, which host
methods back each one, and what is not yet implemented. The server contract is
in the backend repository at `docs/CLOUD-API-V3.md`.

## One origin, no manual infrastructure

The app uses one Cloud origin that the operator provisions. It never asks for
relay addresses, database details, integration keys or host names.

## Capabilities gate every surface

Each panel is shown only when the connected host announces its methods through
`rpc.discover`. An announced method is available; an unannounced one is
absent; before discovery the app keeps calling methods and does not treat
unknown as absent. Older hosts keep the legacy Cloud views.

| Surface | Methods | Where |
| --- | --- | --- |
| Catalog | `catalog.items`, `catalog.item`, `catalog.mine` | Plugins page, Explore tab |
| Connections | `integrations.connections`, `integrations.connect`, `integrations.events`, `integrations.disconnect` | Connections page |
| Teams | `teams.hosts.*`, `teams.devices.*`, `teams.invitations.*`, `teams.requests.*`, `teams.grants.*`, `teams.sessions.*`, `teams.host.status`, `teams.host.start`, `teams.host.stop` | Settings, Teams tab |
| Images and packages | `bundles.assets.upload`, `bundles.packages.preflight`, `bundles.packages.read`, `bundles.overlays.*`, `bundles.packages.create` | Overlays, Cloud library/editor |

### Lifecycle methods and the Cloud capability gate

A second gate sits in the host: besides being announced by `rpc.discover`, each method below
checks the Cloud instance's anonymous `GET /api/v2/capabilities` before it makes its own request. A
missing key or operation means the call fails with `capability_unavailable`
(`data = {reason: "cloud_feature_unsupported", capability, operation}`, or
`reason: "capabilities_missing"` when the instance has no capability document) and the feature
endpoint is never contacted. Input is validated before any network use.

| Method | Params | Result | Needs |
| --- | --- | --- | --- |
| `bundles.assets.list` | `{cursor?, limit?}` (limit 1-50) | `CloudAssetPage {items: CloudAsset[], nextCursor}` | `media.operations` `list` |
| `bundles.assets.setLabel` | `{id, label}` (`label` required; `null` clears; else trimmed 1-120 chars) | `CloudAsset` | `media.operations` `rename` |
| `bundles.assets.delete` | `{id}` | `CloudDeleted {id, deleted}` | `media.operations` `delete` |
| `bundles.overlays.list` | `{cursor?, limit?}` | `OverlayPage {items: OverlaySummary[], nextCursor}` | `overlays.operations` `list` |
| `bundles.overlays.revisions` | `{id, cursor?, limit?}` | `OverlayRevisionPage` | `overlays.operations` `revisions` |
| `bundles.overlays.revision` | `{id, revision}` | `OverlayRevisionRecord` (with `document`) | `overlays.operations` `revision` |
| `bundles.overlays.delete` | `{id}` | `CloudDeleted` | `overlays.operations` `delete` |
| `bundles.packages.delete` | `{id}` | `CloudDeleted` | `packages.operations` `delete` |
| `catalog.fork` | `{id, revision?}` | `CatalogForkResult {id, revision, status: "draft", forkOf: {documentId, revision}}` | `library.fork.available` |

`CloudAsset` gained optional `createdAt` and `label`. Typed failures:

- `conflict` with `data.reason` `assetReferenced` (an overlay revision still names the image) or
  `overlayPinned` (a package still pins the overlay; delete the package first, which releases it).
  `data.cloud` keeps the safe envelope fields (`code`, `requestId`).
- `quota_exceeded` (HTTP 507; assets, overlays, revisions, library): also returned by the existing
  create and revise calls now, where it used to be a generic `unavailable`.
- `catalog.fork`: `not_found` / `notPublished` (404), `conflict` / `alreadyInLibrary` (409, the copy keeps
  the source's portable id), `quota_exceeded` / `libraryQuota` (507), `unavailable` / `rateLimited` (429).

## Catalog

- **Plugins, Explore.** On a host that announces `catalog.items` (or before discovery has
  answered), the *Explore* tab of the Plugins page requests plugins only and lists them as rows in
  the same list style as *Installed*. Templates and profiles belong to Automations. A host
  that announces methods without `catalog.items` keeps the registry-based Explore. The old
  standalone catalog pages and their toolbar are gone; there is no registry or source to add.
- **Replies apply only for the newest query**; a slow older reply is dropped. Paging is by
  cursor (*Load more*), and *My library* (`catalog.mine`) never shows plugins.
- **Backend not available** (offline, unauthorized, error) is a neutral state with a retry icon
  and a sign-in icon when the session is the problem. `dataState: noActiveGeneration` is an empty
  plugin registry, not an outage: Explore says nothing is published yet for plugins while
  templates and profiles still render. The host forwards `dataState` (`ready` or `noActiveGeneration`) in the
  `catalog.items` reply; any other value is dropped and the field is absent for a Cloud that does not send it.
- **A plugin row** opens the existing remote details (an installed one opens its own details);
  the install icon starts the signed package review, the same consent step as any registry install.
  Yanked plugins cannot be installed. Bundled plugins that are not installed yet stay above the
  catalog rows. The "plugin id or https address" strip was removed: the product rejects remote
  package URLs, and the catalog rows are the install path.
- **A template or profile row** is imported through the single library import and the one owner
  review dialog; imported automations stay disabled. *Rule templates* (Automations) holds the same
  library in its *Cloud* tab, and a saved `?view=cloud` link opens it there.
- **Plugin preferences** keep only the update policy (channel, automatic updates, interval, local
  package installation).

## Connections

A webhook connection shows its secret once, in memory, when it is created. The
secret is cleared when the panel closes or the user dismisses it. Revoking a
connection asks for a second click. Events are bounded to the reply budget; the
view says when older events were not shown.

## Teams

- Create a team, rename it, and enable or disable it. Each change carries the
  current revision; a conflict reloads the team instead of overwriting it.
- Enroll this PC as a viewer, or as a host for a team. Enrollment proves
  possession of a device key created on this PC and stored owner-only.
- Invitations: creating one shows its token once. Inviting grants nothing; the
  owner approves each join request.
- Grants: a role change returns the grant to the host for confirmation, and the
  change closes the grant's sessions. Revoking a grant or a session asks for no
  second step but is idempotent on the server.
- **Serving** starts this PC's outbound link for a team. The PC dials the relay
  and confirms its approved grants. It executes the read operations `live.status`,
  `workflow.list`, `automation.list`, `competition.list` and `points.read`, and
  re-checks the grant when it runs. List operations return bounded summaries (id,
  name, enabled, trigger, status; at most 50 items and 200 characters per field),
  never the definition itself, so URLs, headers and credentials stay on the PC.
  The moderation operations (`workflow.disable`, `workflow.delete`, `points.adjust`,
  `automation.toggle`, `automation.delete`, `competition.control`) and
  `automation.draft.submit` are answered as unsupported. Running them needs a stable
  mapping from a Cloud grant to a local collaboration principal, session and journal
  (the host command engine keys its receipts to local principals), which is not built
  yet. The link reconnects with backoff.

## Images and packages

- **Images** upload PNG, JPEG or WebP up to 512 KiB. The Cloud reads the format
  from the bytes; the app does not decode them.
- **Package check** reads the preflight of a package by id. It applies nothing.
  The report lists each item's status and whether the package could be applied.
- **Package apply** runs only after a check that allows it. Each item goes through
  its own flow: plugins through the install review, templates and profiles
  through the library journal. Every step is written to the host's app state
  before and after it runs. A step that was running when the app stopped is
  marked interrupted and is never retried automatically. Overlays are recorded as
  skipped, because they are not applied from a package yet.
- **Local and cloud overlays are different things.** The app already had local
  overlays: widget designs for the seven kinds, served by the local gateway as
  stacked cards. A cloud overlay is an absolute canvas with positioned elements.
  The cloud editor and its files are named `cloud-overlay-*` to keep them apart.
- **Overlay in OBS (TODO: not verified in OBS).** The editor copies an OBS Browser Source URL for the draft.
  The document travels in the URL fragment as base64url JSON, at most 6000
  characters; a larger overlay is refused with a message. The widget
  `cloud-overlay` renders the canvas, scaled to the source, and fills live text
  fields from the gateway: viewer name and handle, gift name, count and diamonds,
  and chat message.
- **Overlay editor** creates and revises cloud overlays of schema version 1. The editor
  checks each change against the Cloud's rules and refuses a save while a problem
  remains. The preview is drawn by the editor; the live stream does not render
  overlays yet.
- **Overlay Changes** compares the draft with its saved and downloaded revisions.
  Download never applies the document. Send uses the reviewed original revision;
  concurrent changes require another review. Use Cloud restores one reviewed
  cached document locally. These revision APIs do not activate account sync v2.
  See [the comparison and recovery contract](SYNC-CHANGES-PLAN.md#overlay-revision-comparison-source-after-the-422-candidate).

## Errors and diagnostics

- The Cloud answers errors with `{code, requestId, messageKey, retryable, retryAfterMs}`. The
  host keeps only `code`, `requestId`, `retryable` and `retryAfterMs`, each validated and
  bounded, in the error `data` (`data.cloud`); everything else in the body is dropped. The
  teams, bundles, integrations and catalog stores expose it as `errorDetail`.
- Panels show a short localized message for the known codes (`unsupported`, `offline`,
  `unauthorized`, `forbidden`, `misconfigured`, `conflict`, `expired`, `rateLimited`,
  `temporaryFailure`) and the host's message for anything else. A retryable code does not
  make a mutation safe to repeat.
- Diagnostics has the sources catalog, install and registry, collaboration, relay and
  integrations. Each entry can carry the request id (redacted before it is stored, searched
  and exported). The history belongs to one account: leaving an account for another one, or
  for none, drops it.
- The catalog service errors are not yet mapped to the envelope: the catalog reports its
  host message.

## Retirement and compatibility

- The Settings "Biblioteca Cloud" panel is shown only on hosts that do not
  announce `catalog.items`. Its services remain in use by the catalog.
- The legacy Cloud plugin view stays for hosts without the unified catalog.
  Removing it needs a migration note and is not done.
- Saved view state for the plugins page keeps its names; a saved `catalog` or `cloud` view
  opens on the installed plugins. In Automations the *Cloud* library tab moved into *Rule
  templates*; `?view=cloud` still works and opens that dialog on its Cloud tab.
- The direct collaboration transport is unchanged. Its listen address, public HTTPS address,
  TLS files, reverse-proxy switch and allowed origins moved into a closed *Direct connection
  (advanced)* section of the overview.

## Not implemented

- **Moderation and draft operations by relay.** `workflow.disable`, `workflow.delete`,
  `points.adjust`, `automation.toggle`, `automation.delete`, `competition.control` and
  `automation.draft.submit` are answered as unsupported. The host command engine keys its
  journal and receipts to local collaboration principals and sessions, and `Authorization`
  can only be created by that service, so a Cloud grant needs a stable mapping to a local
  principal (with its own persistence and tests) before these can run. Until then the
  Moderator and Automation editor roles only add what the Observer role can already read.
- **Integrations beyond the signed webhook.** The backend publishes one provider. OBS, RCON
  and audio would run on the PC under relay authorization; they are out of scope here.
- **Cloud overlays in a browser source.** Overlay art, the live `points.total` source,
  applying an overlay from a package, and checking the `cloud-overlay` widget in OBS Browser
  Source (today it is checked in headless Chromium only) are all part of the OBS work and
  are not done.
- **Catalog errors in the envelope.** The catalog host methods do not map the Cloud error
  envelope yet, so they show the host message.
- **Per-host diagnostics scope.** History is scoped to the account; a host or connection
  switch does not drop it.
- **The Cloud Library fallback.** Settings still shows *Cloud Library* on hosts that do not
  announce `catalog.items`. Removing it needs a migration note.
- End-to-end verification of the app's bearer-token path against a running
  backend. The backend's own suite passes; the app's bearer path depends on a
  browser handoff that cannot be scripted here.
