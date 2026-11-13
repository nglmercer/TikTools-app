# Documentation migration handoff

Source commit: `80c5cb1fbfa45eaeb1c086c71320a15e565d8541`. Branch: `feat/docs-site-demo`. No workspace commits or pushes were made.

## Completed scope

- 71 original documents covered: 69 bilingual legacy stubs, the user-first README and developer-only CONTRIBUTING.
- 71 document archives and 11 image archives match the inventory SHA-256 and byte length exactly. All 11 original images remain unchanged.
- 695 inventoried heading IDs have explicit spans. The map retains 1,548 heading IDs and aliases, including incoming fragments and VitePress/GitHub variants.
- All 939 inventoried links have an explicit treatment: 870 document links retain compatibility paths/fragments and archive relationships; 11 image links retain original/archive copies; 26 external links stay unchanged; 32 source-code references have exact-commit permalinks in the map.
- Each of the 71 entries has a topic-specific source audit and existing code evidence paths. 41 entries migrate instructions; 30 retain long designs, specifications or dated evidence internally.
- The 118 distinct EN/ES guide destinations separate user, developer and release audiences. No compatibility stub enters the public VitePress srcDir or local search.
- README documents binary downloads, current Vite/Wry startup, optional external plugin preparation, Cargo-free docs/demo commands, four UI locales and the shared/api bridge. CONTRIBUTING covers setup, validation, metadata/translation maintenance and verified immutable release/feed publication.

## Verification actually run

- Python provenance/coverage audit passed: 71 document hashes, 11 image hashes, 11 unchanged originals, 695 explicit inventory anchors, one H1 per rewritten document, 939 link treatments, all original document/image archive targets present.
- `git diff --check -- README.md CONTRIBUTING.md docs` passed.
- `bun scripts/task.ts docs check` passed against the then-present 117 public articles, 909 internal links/images and 73 external links. The final primary check passed with 159 articles and 1,255 internal links/images, verified 17 component families and 291 RPC methods, and passed documentation types.
- No Cargo build, native GUI, production request or real account was used. External URL availability was not probed.

## Parent reconciliation

- All eight per-version release pages are now complete in EN/ES under the newly assigned scope. All 118 migration destinations exist. RC3 is identified as a published prerelease; 4.2.2 remains unpublished after Linux packaging failure, with compilation/CI green and recovery ongoing.
- Archived source-code links remain verbatim and can point outside the archive tree. Their 32 exact-commit permalinks are recorded in `linkTreatment.originalLinks[].historicalTarget`; code snapshots were not copied or substituted for the required document/image archive.
- Publication of the website, update feeds and 4.2.2 artifacts was not verified or claimed. The old 4.2.2 source prose is retained only as historical evidence.
- The parent owns any published website aliases, release status, site task runner, assembly, preview, VitePress configuration, demo and public article corrections.



## Site and release-feed publication handoff

The second assigned scope adds `scripts/publish-site.ts`, its focused tests and `.github/workflows/site.yml`, and updates only the **Publish update feed** step in `.github/workflows/release.yml`. It does not alter the release environment, packaging, signing, immutable uploads or the existing frontend CI workflow. These four files are intended for the parent's publication block, separately from the 164 primary documentation paths below. Total assigned changed paths: **168**.

Both writers use one bounded CAS/reapply driver and normal fast-forward pushes. A conflict reconstructs only the writer's authorized overlay from the current branch, rather than applying an old checkout or snapshot. Site writes/removals are limited to exact-hash manifest ownership; protected update paths, CNAME, `.nojekyll`, foreign blobs and their modes remain inherited. Dirty artifacts, wrong base/source revision, unlisted files, symlinks and implicit adoption of legacy paths are rejected. A reviewed ownership seed is supported for the first legacy migration.

The feed writer authenticates candidate artifacts and current channel heads through the existing `stage-update-channels.ts` on every retry. Its overlay can touch only stable/beta JSON plus their signatures. It rejects downgrade, same-version byte rewrites, incomplete pairs and foreign paths, and preserves unselected channels byte for byte. Site publication has no permanent disabled guard: it requires workflow_dispatch, the explicit publish input, the remake ref, a verified build artifact and authorized credentials for RELEASE_REPO. No workflow was dispatched and no production Git remote was read or written.

### Publication verification actually run

- `bun test ./scripts/publish-site.test.ts`: **37 passed**, including real temporary Git conflicts in both directions, exact JSON/signature preservation, foreign file modes, no-op replay, lost push acknowledgement, bounded conflicts and rejection cases. All test remotes are temporary local bare repositories.
- Targeted TypeScript passed with strict mode, unchecked-index checking and unused-symbol checking. Publisher ESLint passed with zero warnings.
- Both workflow files parsed as YAML. `git diff --check` passed for the assigned scope.
- `bun scripts/task.ts docs test` passed its then-current six Bun metadata regressions. The parent subsequently expanded that task to demo/help units and focused Playwright; that expanded suite and the assembled browser artifact remain parent integration checks.
- `bun scripts/task.ts docs check` passed the final 159-article content/migration check, generated-reference check (17 families/291 methods) and docs TypeScript.
- One global `vue-tsc --noEmit` run failed in the parent-owned `src/web/demo/runtime/host.ts:149`: `queued` was used where the transport exposed `queuedResponses`. This report records that observed result; the file was not edited here.
- No Cargo build, production signature verification, real account, native GUI, GitHub Actions execution or live publication was performed. Feed cryptographic verification remains in the unchanged existing signing/staging pipeline; new Git-conflict tests inject authenticated fixture pairs.

### Publication changed paths

- [scripts/publish-site.ts](../../../scripts/publish-site.ts)
- [scripts/publish-site.test.ts](../../../scripts/publish-site.test.ts)
- [.github/workflows/site.yml](../../../.github/workflows/site.yml)
- [.github/workflows/release.yml](../../../.github/workflows/release.yml)

## Every changed path in the primary scope

164 primary paths: 71 rewritten files and 93 additions, including eight complete localized release notes. Original image paths are preserved, not changed.

- [CONTRIBUTING.md](../../../CONTRIBUTING.md)
- [README.md](../../../README.md)
- [docs/AGENT_API.md](../../AGENT_API.md)
- [docs/APP-PACKAGING.md](../../APP-PACKAGING.md)
- [docs/APP-UPDATER.md](../../APP-UPDATER.md)
- [docs/ARCHITECTURE.md](../../ARCHITECTURE.md)
- [docs/AUTOMATIONS.md](../../AUTOMATIONS.md)
- [docs/BACKEND-LIKE-GOALS.md](../../BACKEND-LIKE-GOALS.md)
- [docs/CLOUD-ACCOUNT.md](../../CLOUD-ACCOUNT.md)
- [docs/CLOUD-REGISTRY.md](../../CLOUD-REGISTRY.md)
- [docs/CLOUD-SYNC.md](../../CLOUD-SYNC.md)
- [docs/CLOUD-UNIFIED.md](../../CLOUD-UNIFIED.md)
- [docs/CLOUD-V2-DESKTOP.md](../../CLOUD-V2-DESKTOP.md)
- [docs/CONTROL_API.md](../../CONTROL_API.md)
- [docs/DECLARATIVE_PLUGINS.md](../../DECLARATIVE_PLUGINS.md)
- [docs/DEPENDENCY-REFRESH.md](../../DEPENDENCY-REFRESH.md)
- [docs/DEVELOPMENT.md](../../DEVELOPMENT.md)
- [docs/EMOTES.md](../../EMOTES.md)
- [docs/GETTING_STARTED.md](../../GETTING_STARTED.md)
- [docs/HOTKEYS_LINUX.md](../../HOTKEYS_LINUX.md)
- [docs/LIVE-COLLABORATION-APPROVAL.md](../../LIVE-COLLABORATION-APPROVAL.md)
- [docs/LIVE-COLLABORATION-AUTOMATION.md](../../LIVE-COLLABORATION-AUTOMATION.md)
- [docs/LIVE-COLLABORATION-CLIENT-COMMANDS.md](../../LIVE-COLLABORATION-CLIENT-COMMANDS.md)
- [docs/LIVE-COLLABORATION-CLIENT.md](../../LIVE-COLLABORATION-CLIENT.md)
- [docs/LIVE-COLLABORATION-OWNER.md](../../LIVE-COLLABORATION-OWNER.md)
- [docs/LIVE-COLLABORATION-POINTS.md](../../LIVE-COLLABORATION-POINTS.md)
- [docs/LIVE-COLLABORATION-SPEC.md](../../LIVE-COLLABORATION-SPEC.md)
- [docs/LIVE-COLLABORATION-TRANSPORT.md](../../LIVE-COLLABORATION-TRANSPORT.md)
- [docs/LIVE-COLLABORATION-VALIDATION.md](../../LIVE-COLLABORATION-VALIDATION.md)
- [docs/LIVE-COLLABORATION.md](../../LIVE-COLLABORATION.md)
- [docs/LIVE-EVENT-RELIABILITY.md](../../LIVE-EVENT-RELIABILITY.md)
- [docs/LIVE-EVENTS.md](../../LIVE-EVENTS.md)
- [docs/LIVE_PERFORMANCE.md](../../LIVE_PERFORMANCE.md)
- [docs/LIVE_WEBVIEW_PROJECTION.md](../../LIVE_WEBVIEW_PROJECTION.md)
- [docs/LOCAL_BROWSER_ADAPTER_PLAN.md](../../LOCAL_BROWSER_ADAPTER_PLAN.md)
- [docs/LOCAL_BROWSER_ADAPTER_VALIDATION.md](../../LOCAL_BROWSER_ADAPTER_VALIDATION.md)
- [docs/MEDIA.md](../../MEDIA.md)
- [docs/NAPI-VM-MIGRATION-PLAN.md](../../NAPI-VM-MIGRATION-PLAN.md)
- [docs/NAPI_VM_PLUGINS.md](../../NAPI_VM_PLUGINS.md)
- [docs/P2P-COLLABORATION-PLAN.md](../../P2P-COLLABORATION-PLAN.md)
- [docs/PERFORMANCE_REFACTOR_PROGRESS.md](../../PERFORMANCE_REFACTOR_PROGRESS.md)
- [docs/PLAYWRIGHT.md](../../PLAYWRIGHT.md)
- [docs/PLUGINS.md](../../PLUGINS.md)
- [docs/PLUGIN_AUTHORING.md](../../PLUGIN_AUTHORING.md)
- [docs/PLUGIN_PACKAGE_MANAGER_PLAN.md](../../PLUGIN_PACKAGE_MANAGER_PLAN.md)
- [docs/PLUGIN_REGISTRY_PROTOCOL.md](../../PLUGIN_REGISTRY_PROTOCOL.md)
- [docs/PLUGIN_UI_ARCHITECTURE.md](../../PLUGIN_UI_ARCHITECTURE.md)
- [docs/RELEASE-4.2.1-rc.1.md](../../RELEASE-4.2.1-rc.1.md)
- [docs/RELEASE-4.2.1-rc.2.md](../../RELEASE-4.2.1-rc.2.md)
- [docs/RELEASE-4.2.1-rc.3.md](../../RELEASE-4.2.1-rc.3.md)
- [docs/RELEASE-4.2.2.md](../../RELEASE-4.2.2.md)
- [docs/SONICBOOM-LIVE-TTS-VALIDATION.md](../../SONICBOOM-LIVE-TTS-VALIDATION.md)
- [docs/SYNC-CHANGES-PLAN.md](../../SYNC-CHANGES-PLAN.md)
- [docs/TEMPLATES.md](../../TEMPLATES.md)
- [docs/TERMUX.md](../../TERMUX.md)
- [docs/TERMUX_AGENT_WORKFLOW.md](../../TERMUX_AGENT_WORKFLOW.md)
- [docs/TERMUX_VALIDATION.md](../../TERMUX_VALIDATION.md)
- [docs/TROUBLESHOOTING.md](../../TROUBLESHOOTING.md)
- [docs/TWO-PC-ACCEPTANCE.md](../../TWO-PC-ACCEPTANCE.md)
- [docs/UI_DIAGNOSTICS.md](../../UI_DIAGNOSTICS.md)
- [docs/UI_KIT_USAGE.md](../../UI_KIT_USAGE.md)
- [docs/USER_GUIDE.md](../../USER_GUIDE.md)
- [docs/V2_PRODUCT_OUTBOX_STAGING.md](../../V2_PRODUCT_OUTBOX_STAGING.md)
- [docs/WIDGET_SDK.md](../../WIDGET_SDK.md)
- [docs/aggregation/README.md](../../aggregation/README.md)
- [docs/internal/archive/CONTRIBUTING.md](CONTRIBUTING.md)
- [docs/internal/archive/MIGRATION-REPORT.md](MIGRATION-REPORT.md)
- [docs/internal/archive/PROVENANCE.json](PROVENANCE.json)
- [docs/internal/archive/README.md](README.md)
- [docs/internal/archive/docs/AGENT_API.md](docs/AGENT_API.md)
- [docs/internal/archive/docs/APP-PACKAGING.md](docs/APP-PACKAGING.md)
- [docs/internal/archive/docs/APP-UPDATER.md](docs/APP-UPDATER.md)
- [docs/internal/archive/docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/internal/archive/docs/AUTOMATIONS.md](docs/AUTOMATIONS.md)
- [docs/internal/archive/docs/BACKEND-LIKE-GOALS.md](docs/BACKEND-LIKE-GOALS.md)
- [docs/internal/archive/docs/CLOUD-ACCOUNT.md](docs/CLOUD-ACCOUNT.md)
- [docs/internal/archive/docs/CLOUD-REGISTRY.md](docs/CLOUD-REGISTRY.md)
- [docs/internal/archive/docs/CLOUD-SYNC.md](docs/CLOUD-SYNC.md)
- [docs/internal/archive/docs/CLOUD-UNIFIED.md](docs/CLOUD-UNIFIED.md)
- [docs/internal/archive/docs/CLOUD-V2-DESKTOP.md](docs/CLOUD-V2-DESKTOP.md)
- [docs/internal/archive/docs/CONTROL_API.md](docs/CONTROL_API.md)
- [docs/internal/archive/docs/DECLARATIVE_PLUGINS.md](docs/DECLARATIVE_PLUGINS.md)
- [docs/internal/archive/docs/DEPENDENCY-REFRESH.md](docs/DEPENDENCY-REFRESH.md)
- [docs/internal/archive/docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)
- [docs/internal/archive/docs/EMOTES.md](docs/EMOTES.md)
- [docs/internal/archive/docs/GETTING_STARTED.md](docs/GETTING_STARTED.md)
- [docs/internal/archive/docs/HOTKEYS_LINUX.md](docs/HOTKEYS_LINUX.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-APPROVAL.md](docs/LIVE-COLLABORATION-APPROVAL.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-AUTOMATION.md](docs/LIVE-COLLABORATION-AUTOMATION.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-CLIENT-COMMANDS.md](docs/LIVE-COLLABORATION-CLIENT-COMMANDS.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-CLIENT.md](docs/LIVE-COLLABORATION-CLIENT.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-OWNER.md](docs/LIVE-COLLABORATION-OWNER.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-POINTS.md](docs/LIVE-COLLABORATION-POINTS.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-SPEC.md](docs/LIVE-COLLABORATION-SPEC.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-TRANSPORT.md](docs/LIVE-COLLABORATION-TRANSPORT.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION-VALIDATION.md](docs/LIVE-COLLABORATION-VALIDATION.md)
- [docs/internal/archive/docs/LIVE-COLLABORATION.md](docs/LIVE-COLLABORATION.md)
- [docs/internal/archive/docs/LIVE-EVENT-RELIABILITY.md](docs/LIVE-EVENT-RELIABILITY.md)
- [docs/internal/archive/docs/LIVE-EVENTS.md](docs/LIVE-EVENTS.md)
- [docs/internal/archive/docs/LIVE_PERFORMANCE.md](docs/LIVE_PERFORMANCE.md)
- [docs/internal/archive/docs/LIVE_WEBVIEW_PROJECTION.md](docs/LIVE_WEBVIEW_PROJECTION.md)
- [docs/internal/archive/docs/LOCAL_BROWSER_ADAPTER_PLAN.md](docs/LOCAL_BROWSER_ADAPTER_PLAN.md)
- [docs/internal/archive/docs/LOCAL_BROWSER_ADAPTER_VALIDATION.md](docs/LOCAL_BROWSER_ADAPTER_VALIDATION.md)
- [docs/internal/archive/docs/MEDIA.md](docs/MEDIA.md)
- [docs/internal/archive/docs/NAPI-VM-MIGRATION-PLAN.md](docs/NAPI-VM-MIGRATION-PLAN.md)
- [docs/internal/archive/docs/NAPI_VM_PLUGINS.md](docs/NAPI_VM_PLUGINS.md)
- [docs/internal/archive/docs/P2P-COLLABORATION-PLAN.md](docs/P2P-COLLABORATION-PLAN.md)
- [docs/internal/archive/docs/PERFORMANCE_REFACTOR_PROGRESS.md](docs/PERFORMANCE_REFACTOR_PROGRESS.md)
- [docs/internal/archive/docs/PLAYWRIGHT.md](docs/PLAYWRIGHT.md)
- [docs/internal/archive/docs/PLUGINS.md](docs/PLUGINS.md)
- [docs/internal/archive/docs/PLUGIN_AUTHORING.md](docs/PLUGIN_AUTHORING.md)
- [docs/internal/archive/docs/PLUGIN_PACKAGE_MANAGER_PLAN.md](docs/PLUGIN_PACKAGE_MANAGER_PLAN.md)
- [docs/internal/archive/docs/PLUGIN_REGISTRY_PROTOCOL.md](docs/PLUGIN_REGISTRY_PROTOCOL.md)
- [docs/internal/archive/docs/PLUGIN_UI_ARCHITECTURE.md](docs/PLUGIN_UI_ARCHITECTURE.md)
- [docs/internal/archive/docs/RELEASE-4.2.1-rc.1.md](docs/RELEASE-4.2.1-rc.1.md)
- [docs/internal/archive/docs/RELEASE-4.2.1-rc.2.md](docs/RELEASE-4.2.1-rc.2.md)
- [docs/internal/archive/docs/RELEASE-4.2.1-rc.3.md](docs/RELEASE-4.2.1-rc.3.md)
- [docs/internal/archive/docs/RELEASE-4.2.2.md](docs/RELEASE-4.2.2.md)
- [docs/internal/archive/docs/SONICBOOM-LIVE-TTS-VALIDATION.md](docs/SONICBOOM-LIVE-TTS-VALIDATION.md)
- [docs/internal/archive/docs/SYNC-CHANGES-PLAN.md](docs/SYNC-CHANGES-PLAN.md)
- [docs/internal/archive/docs/TEMPLATES.md](docs/TEMPLATES.md)
- [docs/internal/archive/docs/TERMUX.md](docs/TERMUX.md)
- [docs/internal/archive/docs/TERMUX_AGENT_WORKFLOW.md](docs/TERMUX_AGENT_WORKFLOW.md)
- [docs/internal/archive/docs/TERMUX_VALIDATION.md](docs/TERMUX_VALIDATION.md)
- [docs/internal/archive/docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)
- [docs/internal/archive/docs/TWO-PC-ACCEPTANCE.md](docs/TWO-PC-ACCEPTANCE.md)
- [docs/internal/archive/docs/UI_DIAGNOSTICS.md](docs/UI_DIAGNOSTICS.md)
- [docs/internal/archive/docs/UI_KIT_USAGE.md](docs/UI_KIT_USAGE.md)
- [docs/internal/archive/docs/USER_GUIDE.md](docs/USER_GUIDE.md)
- [docs/internal/archive/docs/V2_PRODUCT_OUTBOX_STAGING.md](docs/V2_PRODUCT_OUTBOX_STAGING.md)
- [docs/internal/archive/docs/WIDGET_SDK.md](docs/WIDGET_SDK.md)
- [docs/internal/archive/docs/aggregation/README.md](docs/aggregation/README.md)
- [docs/internal/archive/docs/perf-phase4.md](docs/perf-phase4.md)
- [docs/internal/archive/docs/release_plan.md](docs/release_plan.md)
- [docs/internal/archive/docs/ui/tts-minimal/README.md](docs/ui/tts-minimal/README.md)
- [docs/internal/archive/docs/ui/tts-minimal/after-desktop.png](docs/ui/tts-minimal/after-desktop.png)
- [docs/internal/archive/docs/ui/tts-minimal/after-mobile.png](docs/ui/tts-minimal/after-mobile.png)
- [docs/internal/archive/docs/ui/tts-minimal/audio-bar.png](docs/ui/tts-minimal/audio-bar.png)
- [docs/internal/archive/docs/ui/tts-minimal/before-desktop.png](docs/ui/tts-minimal/before-desktop.png)
- [docs/internal/archive/docs/ui/tts-playback/README.md](docs/ui/tts-playback/README.md)
- [docs/internal/archive/docs/ui/tts-playback/after-unknown-duration.png](docs/ui/tts-playback/after-unknown-duration.png)
- [docs/internal/archive/docs/ui/tts-playback/after.png](docs/ui/tts-playback/after.png)
- [docs/internal/archive/docs/ui/tts-playback/before.png](docs/ui/tts-playback/before.png)
- [docs/internal/archive/docs/ui/tts-redesign/README.md](docs/ui/tts-redesign/README.md)
- [docs/internal/archive/docs/ui/tts-redesign/after-desktop.png](docs/ui/tts-redesign/after-desktop.png)
- [docs/internal/archive/docs/ui/tts-redesign/after-fields-tooltip.png](docs/ui/tts-redesign/after-fields-tooltip.png)
- [docs/internal/archive/docs/ui/tts-redesign/after-mobile.png](docs/ui/tts-redesign/after-mobile.png)
- [docs/internal/archive/docs/ui/tts-redesign/before-desktop.png](docs/ui/tts-redesign/before-desktop.png)
- [docs/internal/archive/docs/universal-widgets.md](docs/universal-widgets.md)
- [docs/migration-map.json](../../migration-map.json)
- [docs/perf-phase4.md](../../perf-phase4.md)
- [docs/release_plan.md](../../release_plan.md)
- [docs/ui/tts-minimal/README.md](../../ui/tts-minimal/README.md)
- [docs/ui/tts-playback/README.md](../../ui/tts-playback/README.md)
- [docs/ui/tts-redesign/README.md](../../ui/tts-redesign/README.md)
- [docs/universal-widgets.md](../../universal-widgets.md)
- [docs/content/en/releases/4.2.1-rc.1.md](../../content/en/releases/4.2.1-rc.1.md)
- [docs/content/en/releases/4.2.1-rc.2.md](../../content/en/releases/4.2.1-rc.2.md)
- [docs/content/en/releases/4.2.1-rc.3.md](../../content/en/releases/4.2.1-rc.3.md)
- [docs/content/en/releases/4.2.2.md](../../content/en/releases/4.2.2.md)
- [docs/content/es/releases/4.2.1-rc.1.md](../../content/es/releases/4.2.1-rc.1.md)
- [docs/content/es/releases/4.2.1-rc.2.md](../../content/es/releases/4.2.1-rc.2.md)
- [docs/content/es/releases/4.2.1-rc.3.md](../../content/es/releases/4.2.1-rc.3.md)
- [docs/content/es/releases/4.2.2.md](../../content/es/releases/4.2.2.md)
