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
| Catalog | `catalog.items`, `catalog.item`, `catalog.mine` | Plugins page, Cloud view |
| Connections | `integrations.connections`, `integrations.connect`, `integrations.events`, `integrations.disconnect` | Settings, account section |
| Teams | `teams.hosts.*`, `teams.devices.*`, `teams.invitations.*`, `teams.requests.*`, `teams.grants.*`, `teams.sessions.*`, `teams.host.status`, `teams.host.start`, `teams.host.stop` | Settings, account section |
| Images and packages | `bundles.assets.upload`, `bundles.packages.preflight`, `bundles.packages.read`, `bundles.overlays.*`, `bundles.packages.create` | Settings, account section |

## Catalog

- **All** pages through the public catalog. Replies are applied only for the
  newest query; a slow older reply is dropped.
- **Installed** resolves each installed plugin with one bounded `catalog.item`
  lookup, four at a time, at most 50 plugins. A plugin the catalog does not
  publish is shown as local and labelled as not published. Installed plugins
  beyond the budget are not resolved, and the view says so.
- **Míos** pages through the signed-in owner's templates and profiles with
  `catalog.mine`. It needs a session; signing in is offered in the toolbar. It
  never asks for plugins.
- **Install** a plugin through the signed package review, the same consent step
  as any registry install. Yanked and unpublished plugins cannot be installed
  from the catalog.
- **Import a template** submits the published revision through the local library
  journal, with the Cloud origin, account and revision as provenance. The
  imported template is inert until the user enables it.
- **Profiles** import the same way as templates: the profile is parsed, its
  default parameters are applied, and it is submitted to the library journal.
  Imported automations stay disabled.

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
  and confirms its approved grants. It executes only `live.status`; every other
  operation is answered as unsupported. The link reconnects with backoff.

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
- **Overlay editor** creates and revises overlays of schema version 1. The editor
  checks each change against the Cloud's rules and refuses a save while a problem
  remains. The preview is drawn by the editor; the live stream does not render
  overlays yet.

## Retirement and compatibility

- The Settings "Biblioteca Cloud" panel is shown only on hosts that do not
  announce `catalog.items`. Its services remain in use by the catalog.
- The legacy Cloud plugin view stays for hosts without the unified catalog.
  Removing it needs a migration note and is not done.
- Saved view state for the plugins page keeps its names; no preference needed a
  rewrite.

## Not implemented

- Overlay rendering in the live stream. The preview in the editor is not the
  stream renderer.
- Overlays as package items: they are skipped during an apply.
- Overlay assets: the owner-only asset endpoint cannot be used as an image source
  in the webview, so images are shown as placeholders in the preview.
- End-to-end verification of the app's bearer-token path against a running
  backend. The backend's own suite passes; the app's bearer path depends on a
  browser handoff that cannot be scripted here.
