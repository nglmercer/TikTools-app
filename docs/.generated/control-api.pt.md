| Método | Efeitos | Destrutivo | Desktop | Concorrência | Comandos duráveis |
| --- | --- | --- | --- | --- | --- |
| [`analytics.summary`](#rpc-analytics-summary) | false | false | false | — | false |
| [`app.state.get`](#rpc-app-state-get) | false | false | false | — | false |
| [`app.state.set`](#rpc-app-state-set) | true | false | false | — | false |
| [`app.update.check`](#rpc-app-update-check) | false | false | false | — | false |
| [`app.update.download`](#rpc-app-update-download) | true | false | false | — | false |
| [`app.update.install`](#rpc-app-update-install) | true | false | false | — | false |
| [`app.update.status`](#rpc-app-update-status) | false | false | false | — | false |
| [`automation.adopt`](#rpc-automation-adopt) | true | false | false | compare-and-set | true |
| [`automation.adoption.get`](#rpc-automation-adoption-get) | false | false | false | — | false |
| [`automation.adoption.resolve`](#rpc-automation-adoption-resolve) | true | false | false | exclusive-transition | false |
| [`automation.adoption.review_unknown`](#rpc-automation-adoption-review_unknown) | true | false | false | exclusive-transition | false |
| [`automation.adoption.submit`](#rpc-automation-adoption-submit) | true | false | false | exclusive-transition | false |
| [`automation.approval.activate`](#rpc-automation-approval-activate) | true | false | false | compare-and-set | true |
| [`automation.approval.get`](#rpc-automation-approval-get) | false | false | false | — | false |
| [`automation.approval.resolve`](#rpc-automation-approval-resolve) | true | false | false | exclusive-transition | false |
| [`automation.approval.review`](#rpc-automation-approval-review) | false | false | false | — | false |
| [`automation.approval.review_unknown`](#rpc-automation-approval-review_unknown) | true | false | false | exclusive-transition | false |
| [`automation.approval.revoke`](#rpc-automation-approval-revoke) | true | false | false | compare-and-set | true |
| [`automation.approval.submit`](#rpc-automation-approval-submit) | true | false | false | exclusive-transition | false |
| [`automation.authority.get`](#rpc-automation-authority-get) | false | false | false | — | false |
| [`automation.context`](#rpc-automation-context) | false | false | false | — | false |
| [`automation.create`](#rpc-automation-create) | true | false | false | compare-and-set | true |
| [`automation.definition.get`](#rpc-automation-definition-get) | false | false | false | — | false |
| [`automation.definition.resolve`](#rpc-automation-definition-resolve) | true | false | false | exclusive-transition | false |
| [`automation.definition.review_unknown`](#rpc-automation-definition-review_unknown) | true | false | false | exclusive-transition | false |
| [`automation.definition.submit`](#rpc-automation-definition-submit) | true | true | false | exclusive-transition | false |
| [`automation.delegation.preflight`](#rpc-automation-delegation-preflight) | false | false | false | — | false |
| [`automation.delete`](#rpc-automation-delete) | true | true | false | compare-and-set | true |
| [`automation.disable`](#rpc-automation-disable) | true | false | false | compare-and-set | true |
| [`automation.effects.get`](#rpc-automation-effects-get) | false | false | false | — | false |
| [`automation.effects.history`](#rpc-automation-effects-history) | false | false | false | — | false |
| [`automation.enable`](#rpc-automation-enable) | true | false | false | compare-and-set | true |
| [`automation.fire`](#rpc-automation-fire) | true | false | false | — | false |
| [`automation.get`](#rpc-automation-get) | false | false | false | — | false |
| [`automation.library.get`](#rpc-automation-library-get) | false | false | false | — | false |
| [`automation.library.resolve`](#rpc-automation-library-resolve) | true | false | false | exclusive-transition | false |
| [`automation.library.review_unknown`](#rpc-automation-library-review_unknown) | true | false | false | exclusive-transition | false |
| [`automation.library.snapshot`](#rpc-automation-library-snapshot) | false | false | false | — | false |
| [`automation.library.submit`](#rpc-automation-library-submit) | true | true | false | exclusive-transition | false |
| [`automation.list`](#rpc-automation-list) | false | false | false | — | false |
| [`automation.manifest.get`](#rpc-automation-manifest-get) | false | false | false | — | false |
| [`automation.nodes.list`](#rpc-automation-nodes-list) | false | false | false | — | false |
| [`automation.review`](#rpc-automation-review) | false | false | false | — | false |
| [`automation.runs`](#rpc-automation-runs) | false | false | false | — | false |
| [`automation.script.analyze`](#rpc-automation-script-analyze) | false | false | false | — | false |
| [`automation.set_enablement`](#rpc-automation-set_enablement) | true | false | false | exclusive-transition | false |
| [`automation.snapshot`](#rpc-automation-snapshot) | false | false | false | — | false |
| [`automation.source.get`](#rpc-automation-source-get) | false | false | false | — | false |
| [`automation.test`](#rpc-automation-test) | false | false | false | — | false |
| [`automation.update`](#rpc-automation-update) | true | false | false | compare-and-set | true |
| [`bundles.assets.delete`](#rpc-bundles-assets-delete) | true | false | false | — | false |
| [`bundles.assets.list`](#rpc-bundles-assets-list) | false | false | false | — | false |
| [`bundles.assets.setLabel`](#rpc-bundles-assets-setLabel) | true | false | false | — | false |
| [`bundles.assets.upload`](#rpc-bundles-assets-upload) | true | false | false | — | false |
| [`bundles.overlays.create`](#rpc-bundles-overlays-create) | true | false | false | — | false |
| [`bundles.overlays.delete`](#rpc-bundles-overlays-delete) | true | false | false | — | false |
| [`bundles.overlays.list`](#rpc-bundles-overlays-list) | false | false | false | — | false |
| [`bundles.overlays.read`](#rpc-bundles-overlays-read) | false | false | false | — | false |
| [`bundles.overlays.revise`](#rpc-bundles-overlays-revise) | true | false | false | — | false |
| [`bundles.overlays.revision`](#rpc-bundles-overlays-revision) | false | false | false | — | false |
| [`bundles.overlays.revisions`](#rpc-bundles-overlays-revisions) | false | false | false | — | false |
| [`bundles.packages.create`](#rpc-bundles-packages-create) | true | false | false | — | false |
| [`bundles.packages.delete`](#rpc-bundles-packages-delete) | true | false | false | — | false |
| [`bundles.packages.preflight`](#rpc-bundles-packages-preflight) | false | false | false | — | false |
| [`bundles.packages.read`](#rpc-bundles-packages-read) | false | false | false | — | false |
| [`catalog.fork`](#rpc-catalog-fork) | true | false | false | — | false |
| [`catalog.item`](#rpc-catalog-item) | false | false | false | — | false |
| [`catalog.items`](#rpc-catalog-items) | false | false | false | — | false |
| [`catalog.mine`](#rpc-catalog-mine) | false | false | false | — | false |
| [`cloud.login.begin`](#rpc-cloud-login-begin) | true | false | false | — | false |
| [`cloud.login.cancel`](#rpc-cloud-login-cancel) | true | false | false | — | false |
| [`cloud.login.complete`](#rpc-cloud-login-complete) | true | false | false | — | false |
| [`cloud.login.open`](#rpc-cloud-login-open) | true | false | false | — | false |
| [`cloud.login.reserve`](#rpc-cloud-login-reserve) | true | false | false | — | false |
| [`cloud.logout`](#rpc-cloud-logout) | true | false | false | — | false |
| [`cloud.restore`](#rpc-cloud-restore) | false | false | false | — | false |
| [`cloud.status`](#rpc-cloud-status) | false | false | false | — | false |
| [`cloudLibrary.list`](#rpc-cloudLibrary-list) | false | false | false | — | false |
| [`cloudLibrary.read`](#rpc-cloudLibrary-read) | false | false | false | — | false |
| [`collaboration.audit.list`](#rpc-collaboration-audit-list) | false | false | false | — | false |
| [`collaboration.commands.get`](#rpc-collaboration-commands-get) | false | false | false | — | false |
| [`collaboration.configure`](#rpc-collaboration-configure) | true | false | false | — | false |
| [`collaboration.grants.set`](#rpc-collaboration-grants-set) | true | false | false | — | false |
| [`collaboration.hello`](#rpc-collaboration-hello) | false | false | false | — | false |
| [`collaboration.invites.create`](#rpc-collaboration-invites-create) | true | false | false | — | false |
| [`collaboration.invites.list`](#rpc-collaboration-invites-list) | false | false | false | — | false |
| [`collaboration.invites.revoke`](#rpc-collaboration-invites-revoke) | true | false | false | — | false |
| [`collaboration.me`](#rpc-collaboration-me) | false | false | false | — | false |
| [`collaboration.principals.get`](#rpc-collaboration-principals-get) | false | false | false | — | false |
| [`collaboration.principals.list`](#rpc-collaboration-principals-list) | false | false | false | — | false |
| [`collaboration.principals.revoke`](#rpc-collaboration-principals-revoke) | true | false | false | — | false |
| [`collaboration.requests.approve`](#rpc-collaboration-requests-approve) | true | false | false | — | false |
| [`collaboration.requests.list`](#rpc-collaboration-requests-list) | false | false | false | — | false |
| [`collaboration.requests.reject`](#rpc-collaboration-requests-reject) | true | false | false | — | false |
| [`collaboration.resume`](#rpc-collaboration-resume) | false | false | false | — | false |
| [`collaboration.sessions.list`](#rpc-collaboration-sessions-list) | false | false | false | — | false |
| [`collaboration.sessions.revoke`](#rpc-collaboration-sessions-revoke) | true | false | false | — | false |
| [`collaboration.snapshot`](#rpc-collaboration-snapshot) | false | false | false | — | false |
| [`collaboration.status`](#rpc-collaboration-status) | false | false | false | — | false |
| [`collaboration.subscribe`](#rpc-collaboration-subscribe) | false | false | false | — | false |
| [`competitions.action`](#rpc-competitions-action) | false | false | false | — | false |
| [`competitions.cancel`](#rpc-competitions-cancel) | true | true | false | compare-and-set | true |
| [`competitions.candidates`](#rpc-competitions-candidates) | false | false | false | — | false |
| [`competitions.create`](#rpc-competitions-create) | true | false | false | — | false |
| [`competitions.delete`](#rpc-competitions-delete) | true | true | false | — | false |
| [`competitions.finish`](#rpc-competitions-finish) | true | false | false | — | false |
| [`competitions.get`](#rpc-competitions-get) | false | false | false | — | false |
| [`competitions.list`](#rpc-competitions-list) | false | false | false | — | false |
| [`competitions.markActionUncertain`](#rpc-competitions-markActionUncertain) | true | false | false | — | false |
| [`competitions.pause`](#rpc-competitions-pause) | true | false | false | compare-and-set | true |
| [`competitions.resolveAction`](#rpc-competitions-resolveAction) | true | false | false | — | false |
| [`competitions.results`](#rpc-competitions-results) | false | false | false | — | false |
| [`competitions.resume`](#rpc-competitions-resume) | true | false | false | compare-and-set | true |
| [`competitions.selectWinner`](#rpc-competitions-selectWinner) | true | false | false | — | false |
| [`competitions.start`](#rpc-competitions-start) | true | false | false | compare-and-set | true |
| [`competitions.state`](#rpc-competitions-state) | false | false | false | — | false |
| [`competitions.summary`](#rpc-competitions-summary) | false | false | false | — | false |
| [`competitions.update`](#rpc-competitions-update) | true | false | false | — | false |
| [`creators.get`](#rpc-creators-get) | false | false | false | — | false |
| [`creators.history.clear`](#rpc-creators-history-clear) | true | true | false | — | false |
| [`creators.recent`](#rpc-creators-recent) | false | false | false | — | false |
| [`gifts.debug`](#rpc-gifts-debug) | false | false | false | — | false |
| [`gifts.list`](#rpc-gifts-list) | false | false | false | — | false |
| [`globals.delete`](#rpc-globals-delete) | true | true | false | — | false |
| [`globals.get`](#rpc-globals-get) | false | false | false | — | false |
| [`globals.list`](#rpc-globals-list) | false | false | false | — | false |
| [`globals.set`](#rpc-globals-set) | true | false | false | — | false |
| [`globals.typed.list`](#rpc-globals-typed-list) | false | false | false | — | false |
| [`globals.typed.replace`](#rpc-globals-typed-replace) | true | true | false | — | false |
| [`integrations.connect`](#rpc-integrations-connect) | true | false | false | — | false |
| [`integrations.connections`](#rpc-integrations-connections) | false | false | false | — | false |
| [`integrations.disconnect`](#rpc-integrations-disconnect) | true | false | false | — | false |
| [`integrations.events`](#rpc-integrations-events) | false | false | false | — | false |
| [`live.channels`](#rpc-live-channels) | false | false | false | — | false |
| [`live.connect`](#rpc-live-connect) | true | false | false | — | false |
| [`live.disconnect`](#rpc-live-disconnect) | true | false | false | — | false |
| [`live.monitor.start`](#rpc-live-monitor-start) | true | false | false | — | false |
| [`live.monitor.status`](#rpc-live-monitor-status) | false | false | false | — | false |
| [`live.monitor.stop`](#rpc-live-monitor-stop) | true | false | false | — | false |
| [`live.pick`](#rpc-live-pick) | true | false | false | — | false |
| [`live.status`](#rpc-live-status) | false | false | false | — | false |
| [`media.pick`](#rpc-media-pick) | true | false | true | — | false |
| [`media.play`](#rpc-media-play) | true | false | false | — | false |
| [`media.validate`](#rpc-media-validate) | false | false | false | — | false |
| [`plugins.action.execute`](#rpc-plugins-action-execute) | true | false | false | — | false |
| [`plugins.diagnostics`](#rpc-plugins-diagnostics) | false | false | false | — | false |
| [`plugins.disable`](#rpc-plugins-disable) | true | false | false | — | false |
| [`plugins.enable`](#rpc-plugins-enable) | true | false | false | — | false |
| [`plugins.get`](#rpc-plugins-get) | false | false | false | — | false |
| [`plugins.health`](#rpc-plugins-health) | false | false | false | — | false |
| [`plugins.install`](#rpc-plugins-install) | true | false | false | — | false |
| [`plugins.install.set`](#rpc-plugins-install-set) | true | false | false | — | false |
| [`plugins.jobs.cancel`](#rpc-plugins-jobs-cancel) | true | false | false | — | false |
| [`plugins.jobs.get`](#rpc-plugins-jobs-get) | false | false | false | — | false |
| [`plugins.jobs.list`](#rpc-plugins-jobs-list) | false | false | false | — | false |
| [`plugins.list`](#rpc-plugins-list) | false | false | false | — | false |
| [`plugins.options`](#rpc-plugins-options) | false | false | false | — | false |
| [`plugins.packages.inspect`](#rpc-plugins-packages-inspect) | false | false | false | — | false |
| [`plugins.packages.install`](#rpc-plugins-packages-install) | true | false | false | — | false |
| [`plugins.packages.policy`](#rpc-plugins-packages-policy) | true | false | false | — | false |
| [`plugins.packages.receipts`](#rpc-plugins-packages-receipts) | false | false | false | — | false |
| [`plugins.packages.rollback`](#rpc-plugins-packages-rollback) | true | false | false | — | false |
| [`plugins.packages.update`](#rpc-plugins-packages-update) | true | false | false | — | false |
| [`plugins.packages.versions`](#rpc-plugins-packages-versions) | false | false | false | — | false |
| [`plugins.settings.get`](#rpc-plugins-settings-get) | false | false | false | — | false |
| [`plugins.settings.reset`](#rpc-plugins-settings-reset) | true | true | false | — | false |
| [`plugins.settings.set`](#rpc-plugins-settings-set) | true | false | false | — | false |
| [`plugins.start`](#rpc-plugins-start) | true | false | false | — | false |
| [`plugins.stop`](#rpc-plugins-stop) | true | false | false | — | false |
| [`plugins.token.import`](#rpc-plugins-token-import) | true | false | false | — | false |
| [`plugins.token.provision`](#rpc-plugins-token-provision) | true | false | false | — | false |
| [`plugins.ui.assets`](#rpc-plugins-ui-assets) | false | false | false | — | false |
| [`plugins.uninstall`](#rpc-plugins-uninstall) | true | true | false | — | false |
| [`plugins.updates.check`](#rpc-plugins-updates-check) | false | false | false | — | false |
| [`plugins.updates.list`](#rpc-plugins-updates-list) | false | false | false | — | false |
| [`points.adjust`](#rpc-points-adjust) | true | false | false | additive | true |
| [`points.config.get`](#rpc-points-config-get) | false | false | false | — | false |
| [`points.config.set`](#rpc-points-config-set) | true | false | false | — | false |
| [`points.info`](#rpc-points-info) | false | false | false | — | false |
| [`points.leaderboard`](#rpc-points-leaderboard) | false | false | false | — | false |
| [`points.reset`](#rpc-points-reset) | true | true | false | — | false |
| [`points.view`](#rpc-points-view) | false | false | false | — | false |
| [`points.viewer.get`](#rpc-points-viewer-get) | false | false | false | — | false |
| [`processors.list`](#rpc-processors-list) | false | false | false | — | false |
| [`processors.status`](#rpc-processors-status) | false | false | false | — | false |
| [`processors.test`](#rpc-processors-test) | false | false | false | — | false |
| [`publisher.create`](#rpc-publisher-create) | true | false | false | — | false |
| [`publisher.delete`](#rpc-publisher-delete) | true | false | false | — | false |
| [`publisher.keys`](#rpc-publisher-keys) | false | false | false | — | false |
| [`publisher.keys.enroll`](#rpc-publisher-keys-enroll) | true | false | false | — | false |
| [`publisher.list`](#rpc-publisher-list) | false | false | false | — | false |
| [`publisher.publication.request`](#rpc-publisher-publication-request) | true | false | false | — | false |
| [`publisher.releases.get`](#rpc-publisher-releases-get) | false | false | false | — | false |
| [`publisher.releases.submit`](#rpc-publisher-releases-submit) | true | false | false | — | false |
| [`publisher.uploads.send`](#rpc-publisher-uploads-send) | true | false | false | — | false |
| [`rankings.contribute`](#rpc-rankings-contribute) | true | false | false | — | false |
| [`rankings.create`](#rpc-rankings-create) | true | false | false | — | false |
| [`rankings.delete`](#rpc-rankings-delete) | true | true | false | — | false |
| [`rankings.get`](#rpc-rankings-get) | false | false | false | — | false |
| [`rankings.list`](#rpc-rankings-list) | false | false | false | — | false |
| [`rankings.metrics`](#rpc-rankings-metrics) | false | false | false | — | false |
| [`rankings.pause`](#rpc-rankings-pause) | true | false | false | — | false |
| [`rankings.reset`](#rpc-rankings-reset) | true | true | false | — | false |
| [`rankings.resume`](#rpc-rankings-resume) | true | false | false | — | false |
| [`rankings.snapshot`](#rpc-rankings-snapshot) | false | false | false | — | false |
| [`rankings.update`](#rpc-rankings-update) | true | false | false | — | false |
| [`registry.cloudCatalog`](#rpc-registry-cloudCatalog) | false | false | false | — | false |
| [`registry.cloudCatalogPaginationCapabilities`](#rpc-registry-cloudCatalogPaginationCapabilities) | false | false | false | — | false |
| [`registry.cloudCatalogSummaries`](#rpc-registry-cloudCatalogSummaries) | false | false | false | — | false |
| [`registry.cloudCatalogVersions`](#rpc-registry-cloudCatalogVersions) | false | false | false | — | false |
| [`registry.defaults.get`](#rpc-registry-defaults-get) | false | false | false | — | false |
| [`registry.defaults.set`](#rpc-registry-defaults-set) | true | false | false | — | false |
| [`registry.list`](#rpc-registry-list) | false | false | false | — | false |
| [`registry.package`](#rpc-registry-package) | false | false | false | — | false |
| [`registry.refresh`](#rpc-registry-refresh) | false | false | false | — | false |
| [`registry.search`](#rpc-registry-search) | false | false | false | — | false |
| [`rpc.discover`](#rpc-rpc-discover) | false | false | false | — | false |
| [`rpc.schema`](#rpc-rpc-schema) | false | false | false | — | false |
| [`sync.disable`](#rpc-sync-disable) | true | false | false | — | false |
| [`sync.enable`](#rpc-sync-enable) | true | false | false | — | false |
| [`sync.now`](#rpc-sync-now) | true | false | false | — | false |
| [`sync.resolve`](#rpc-sync-resolve) | true | false | false | — | false |
| [`sync.status`](#rpc-sync-status) | false | false | false | — | false |
| [`sync.uncertain.list`](#rpc-sync-uncertain-list) | false | false | false | — | false |
| [`sync.uncertain.resolve`](#rpc-sync-uncertain-resolve) | true | false | false | — | false |
| [`sync.uncertain.review`](#rpc-sync-uncertain-review) | false | false | false | — | false |
| [`sync.v2.changes.list`](#rpc-sync-v2-changes-list) | false | false | false | — | false |
| [`sync.v2.conflict.list`](#rpc-sync-v2-conflict-list) | false | false | false | — | false |
| [`sync.v2.conflict.resolve`](#rpc-sync-v2-conflict-resolve) | true | false | false | — | false |
| [`sync.v2.conflict.review`](#rpc-sync-v2-conflict-review) | false | false | false | — | false |
| [`sync.v2.discover`](#rpc-sync-v2-discover) | false | false | false | — | false |
| [`sync.v2.inventory.repair`](#rpc-sync-v2-inventory-repair) | true | false | false | — | false |
| [`sync.v2.inventory.review`](#rpc-sync-v2-inventory-review) | false | false | false | — | false |
| [`sync.v2.migrate`](#rpc-sync-v2-migrate) | true | false | false | — | false |
| [`sync.v2.now`](#rpc-sync-v2-now) | true | false | false | — | false |
| [`sync.v2.preview`](#rpc-sync-v2-preview) | false | false | false | — | false |
| [`sync.v2.rebuild`](#rpc-sync-v2-rebuild) | true | false | false | — | false |
| [`sync.v2.section`](#rpc-sync-v2-section) | true | false | false | — | false |
| [`sync.v2.selected`](#rpc-sync-v2-selected) | false | false | false | — | false |
| [`sync.v2.stagePluginSettings`](#rpc-sync-v2-stagePluginSettings) | true | false | false | — | false |
| [`sync.v2.stagePreferences`](#rpc-sync-v2-stagePreferences) | true | false | false | — | false |
| [`sync.v2.stageVariables`](#rpc-sync-v2-stageVariables) | true | false | false | — | false |
| [`sync.v2.variables.apply`](#rpc-sync-v2-variables-apply) | true | true | false | — | false |
| [`sync.v2.variables.fetch`](#rpc-sync-v2-variables-fetch) | false | false | false | — | false |
| [`sync.v2.variables.review`](#rpc-sync-v2-variables-review) | false | false | false | — | false |
| [`sync.v2.variables.send`](#rpc-sync-v2-variables-send) | true | true | false | — | false |
| [`sync.v2.variables.withdraw`](#rpc-sync-v2-variables-withdraw) | true | true | false | — | false |
| [`system.doctor`](#rpc-system-doctor) | false | false | false | — | false |
| [`system.health`](#rpc-system-health) | false | false | false | — | false |
| [`system.info`](#rpc-system-info) | false | false | false | — | false |
| [`system.ping`](#rpc-system-ping) | false | false | false | — | false |
| [`system.requestInputAccess`](#rpc-system-requestInputAccess) | true | false | false | — | false |
| [`system.saveFile`](#rpc-system-saveFile) | true | false | false | — | false |
| [`system.shutdown`](#rpc-system-shutdown) | true | true | false | — | false |
| [`system.snapshot`](#rpc-system-snapshot) | false | false | false | — | false |
| [`teams.accessible.list`](#rpc-teams-accessible-list) | false | false | false | — | false |
| [`teams.commands.run`](#rpc-teams-commands-run) | true | false | false | — | false |
| [`teams.devices.enroll`](#rpc-teams-devices-enroll) | true | false | false | — | false |
| [`teams.devices.list`](#rpc-teams-devices-list) | false | false | false | — | false |
| [`teams.devices.revoke`](#rpc-teams-devices-revoke) | true | false | false | — | false |
| [`teams.grants.list`](#rpc-teams-grants-list) | false | false | false | — | false |
| [`teams.grants.revoke`](#rpc-teams-grants-revoke) | true | false | false | — | false |
| [`teams.grants.update`](#rpc-teams-grants-update) | true | false | false | — | false |
| [`teams.host.start`](#rpc-teams-host-start) | true | false | false | — | false |
| [`teams.host.status`](#rpc-teams-host-status) | false | false | false | — | false |
| [`teams.host.stop`](#rpc-teams-host-stop) | true | false | false | — | false |
| [`teams.hosts.create`](#rpc-teams-hosts-create) | true | false | false | — | false |
| [`teams.hosts.get`](#rpc-teams-hosts-get) | false | false | false | — | false |
| [`teams.hosts.list`](#rpc-teams-hosts-list) | false | false | false | — | false |
| [`teams.hosts.update`](#rpc-teams-hosts-update) | true | false | false | — | false |
| [`teams.invitations.create`](#rpc-teams-invitations-create) | true | false | false | — | false |
| [`teams.invitations.list`](#rpc-teams-invitations-list) | false | false | false | — | false |
| [`teams.invitations.redeem`](#rpc-teams-invitations-redeem) | true | false | false | — | false |
| [`teams.invitations.revoke`](#rpc-teams-invitations-revoke) | true | false | false | — | false |
| [`teams.people.search`](#rpc-teams-people-search) | false | false | false | — | false |
| [`teams.requests.decide`](#rpc-teams-requests-decide) | true | false | false | — | false |
| [`teams.requests.list`](#rpc-teams-requests-list) | false | false | false | — | false |
| [`teams.sessions.list`](#rpc-teams-sessions-list) | false | false | false | — | false |
| [`teams.sessions.revoke`](#rpc-teams-sessions-revoke) | true | false | false | — | false |
| [`ui.feed.backfill`](#rpc-ui-feed-backfill) | false | false | false | — | false |
| [`ui.feed.browser`](#rpc-ui-feed-browser) | false | false | false | — | false |
| [`ui.feed.creator`](#rpc-ui-feed-creator) | false | false | false | — | false |
| [`widgets.copyCloudOverlayUrl`](#rpc-widgets-copyCloudOverlayUrl) | true | false | true | — | false |
| [`widgets.copyObsUrl`](#rpc-widgets-copyObsUrl) | true | false | true | — | false |
| [`widgets.status`](#rpc-widgets-status) | false | false | false | — | false |
| [`workflows.delete`](#rpc-workflows-delete) | true | true | false | compare-and-set | true |
| [`workflows.disable`](#rpc-workflows-disable) | true | false | false | compare-and-set | true |
| [`workflows.enable`](#rpc-workflows-enable) | true | false | false | compare-and-set | true |
| [`workflows.get`](#rpc-workflows-get) | false | false | false | — | false |
| [`workflows.list`](#rpc-workflows-list) | false | false | false | — | false |
| [`workflows.save`](#rpc-workflows-save) | true | false | false | compare-and-set | true |

### analytics.summary {#rpc-analytics-summary}

Descrição do contrato:

```text
Per-day analytics summary for a creator (defaults to the current one)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "creatorUniqueId": {
      "type": [
        "string",
        "null"
      ]
    },
    "endDay": {
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    },
    "limit": {
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    },
    "startDay": {
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    },
    "tzOffsetSecs": {
      "description": "System-zone offset in seconds east of UTC (local = UTC + offset).\nDay bounds are interpreted in that frame; `None`/0 keeps UTC days.",
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "AnalyticsSummaryParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "summary": true
  },
  "title": "AnalyticsSummaryResult",
  "type": "object"
}
```

:::

### app.state.get {#rpc-app-state-get}

Descrição do contrato:

```text
Reads app state (all keys, or a filtered subset)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "keys": {
      "default": null,
      "items": {
        "type": "string"
      },
      "type": [
        "array",
        "null"
      ]
    }
  },
  "title": "AppStateGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "state": {
      "additionalProperties": {
        "type": "string"
      },
      "type": "object"
    }
  },
  "required": [
    "state"
  ],
  "title": "AppStateResult",
  "type": "object"
}
```

:::

### app.state.set {#rpc-app-state-set}

Descrição do contrato:

```text
Writes one app state key
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "key": {
      "type": "string"
    },
    "value": {
      "type": "string"
    }
  },
  "required": [
    "key",
    "value"
  ],
  "title": "AppStateSetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "state": {
      "additionalProperties": {
        "type": "string"
      },
      "type": "object"
    }
  },
  "required": [
    "state"
  ],
  "title": "AppStateResult",
  "type": "object"
}
```

:::

### app.update.check {#rpc-app-update-check}

Descrição do contrato:

```text
Check signed application update metadata
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "UpdateChannel": {
      "enum": [
        "stable",
        "beta"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "channel": {
      "$ref": "#/$defs/UpdateChannel",
      "default": "stable"
    }
  },
  "title": "CheckParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### app.update.download {#rpc-app-update-download}

Descrição do contrato:

```text
Download and verify the selected application update
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "NoUpdateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### app.update.install {#rpc-app-update-install}

Descrição do contrato:

```text
Install requires trusted native desktop confirmation
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "NoUpdateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### app.update.status {#rpc-app-update-status}

Descrição do contrato:

```text
Application updater status
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "NoUpdateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.adopt {#rpc-automation-adopt}

Descrição do contrato:

```text
Explicit owner authority transfer; always leaves delegated record disabled
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$defs": {
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgeAuthority": {
      "type": "boolean"
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "reviewedVersions": {
      "items": {
        "$ref": "#/$defs/ResourceVersion"
      },
      "type": [
        "array",
        "null"
      ]
    }
  },
  "required": [
    "id",
    "acknowledgeAuthority"
  ],
  "title": "AutomationAdoptParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.adoption.get {#rpc-automation-adoption-get}

Descrição do contrato:

```text
Owner-only durable adoption intent and recent metadata
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerAdoptionEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerAdoptionIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        }
      },
      "required": [
        "commandId",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerAdoptionOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerAdoptionRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerAdoptionIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerAdoptionOutcome"
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        },
        "revision": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerAdoptionIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerAdoptionRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerAdoptionState",
  "type": "object"
}
```

:::

### automation.adoption.resolve {#rpc-automation-adoption-resolve}

Descrição do contrato:

```text
Queries original owner adoption outcome without dispatch
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerAdoptionEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerAdoptionIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        }
      },
      "required": [
        "commandId",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerAdoptionOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerAdoptionRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerAdoptionIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerAdoptionOutcome"
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        },
        "revision": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerAdoptionIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerAdoptionRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerAdoptionState",
  "type": "object"
}
```

:::

### automation.adoption.review_unknown {#rpc-automation-adoption-review_unknown}

Descrição do contrato:

```text
Archives an explicitly reviewed unresolved owner intent without replay
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgeUnknown": {
      "type": "boolean"
    },
    "commandId": {
      "type": "string"
    }
  },
  "required": [
    "commandId",
    "acknowledgeUnknown"
  ],
  "title": "OwnerAdoptionReview",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerAdoptionIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        }
      },
      "required": [
        "commandId",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerAdoptionOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerAdoptionRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerAdoptionIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerAdoptionOutcome"
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        },
        "revision": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerAdoptionIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerAdoptionRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerAdoptionState",
  "type": "object"
}
```

:::

### automation.adoption.submit {#rpc-automation-adoption-submit}

Descrição do contrato:

```text
Persists native owner intent before reviewed authority transfer
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgeAuthority": {
      "type": "boolean"
    },
    "expectedRevision": {
      "type": "string"
    },
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    },
    "reviewedVersions": {
      "items": {
        "$ref": "#/$defs/ResourceVersion"
      },
      "type": "array"
    }
  },
  "required": [
    "kind",
    "id",
    "expectedRevision",
    "hostId",
    "hostInstanceId",
    "reviewedVersions",
    "acknowledgeAuthority"
  ],
  "title": "OwnerAdoptionSubmit",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerAdoptionIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        }
      },
      "required": [
        "commandId",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerAdoptionOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerAdoptionRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerAdoptionIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerAdoptionOutcome"
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        },
        "revision": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerAdoptionIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerAdoptionRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerAdoptionState",
  "type": "object"
}
```

:::

### automation.approval.activate {#rpc-automation-approval-activate}

Descrição do contrato:

```text
Owner-only reviewed delegated approval and explicit activation; authority remains delegated
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$defs": {
    "ApprovalEnablement": {
      "additionalProperties": false,
      "properties": {
        "enabled": {
          "type": "boolean"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        }
      },
      "required": [
        "kind",
        "id",
        "enabled"
      ],
      "type": "object"
    },
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgeEffects": {
      "type": "boolean"
    },
    "enablePlan": {
      "items": {
        "$ref": "#/$defs/ApprovalEnablement"
      },
      "type": "array"
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    },
    "manifestDigest": {
      "type": "string"
    },
    "reviewedVersions": {
      "items": {
        "$ref": "#/$defs/ResourceVersion"
      },
      "type": "array"
    }
  },
  "required": [
    "kind",
    "id",
    "reviewedVersions",
    "manifestDigest",
    "enablePlan",
    "acknowledgeEffects"
  ],
  "title": "ApprovalActivateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "A historical receipt for the exact post-transition binding, never a permit.",
  "properties": {
    "approvalGeneration": {
      "type": "string"
    },
    "approved": {
      "type": "boolean"
    },
    "enabled": {
      "type": "boolean"
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    },
    "manifestDigest": {
      "type": "string"
    },
    "policyVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "kind",
    "id",
    "enabled",
    "approved",
    "policyVersion",
    "manifestDigest",
    "approvalGeneration"
  ],
  "title": "ApprovalActivateReceipt",
  "type": "object"
}
```

:::

### automation.approval.get {#rpc-automation-approval-get}

Descrição do contrato:

```text
Owner-only pending approval operation and recent durable metadata
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerApprovalEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerApprovalIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "$ref": "#/$defs/OwnerApprovalOperation"
        }
      },
      "required": [
        "commandId",
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerApprovalOperation": {
      "enum": [
        "activate",
        "revoke"
      ],
      "type": "string"
    },
    "OwnerApprovalOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerApprovalReceipt": {
      "additionalProperties": false,
      "properties": {
        "approvalGeneration": {
          "type": "string"
        },
        "manifestDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "policyVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "revision": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "approvalGeneration"
      ],
      "type": "object"
    },
    "OwnerApprovalRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerApprovalIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerApprovalOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerApprovalReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerApprovalIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerApprovalRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerApprovalState",
  "type": "object"
}
```

:::

### automation.approval.resolve {#rpc-automation-approval-resolve}

Descrição do contrato:

```text
Looks up the original owner approval outcome without dispatch
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerApprovalEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerApprovalIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "$ref": "#/$defs/OwnerApprovalOperation"
        }
      },
      "required": [
        "commandId",
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerApprovalOperation": {
      "enum": [
        "activate",
        "revoke"
      ],
      "type": "string"
    },
    "OwnerApprovalOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerApprovalReceipt": {
      "additionalProperties": false,
      "properties": {
        "approvalGeneration": {
          "type": "string"
        },
        "manifestDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "policyVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "revision": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "approvalGeneration"
      ],
      "type": "object"
    },
    "OwnerApprovalRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerApprovalIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerApprovalOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerApprovalReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerApprovalIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerApprovalRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerApprovalState",
  "type": "object"
}
```

:::

### automation.approval.review {#rpc-automation-approval-review}

Descrição do contrato:

```text
Owner-only atomic saved source, execution manifest, current grants and approval state
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    }
  },
  "required": [
    "kind",
    "id"
  ],
  "title": "OwnerApprovalReviewParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AudioEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "fileRefs": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxVolume": {
          "format": "double",
          "type": "number"
        },
        "output": {
          "type": "string"
        },
        "overlaps": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "fileRefs",
        "output",
        "maxVolume",
        "overlaps"
      ],
      "type": "object"
    },
    "AutomationDelegation": {
      "additionalProperties": false,
      "properties": {
        "allowDynamicTargets": {
          "type": "boolean"
        },
        "allowOtherAuthorEdits": {
          "description": "Allow scoped definitions authored by another moderator to be replaced.",
          "type": "boolean"
        },
        "allowOtherAuthorSource": {
          "description": "Share full source of scoped definitions authored by other moderators.",
          "type": "boolean"
        },
        "allowOwnerEdits": {
          "description": "Explicitly allow scoped changes to existing Owner definitions.",
          "type": "boolean"
        },
        "allowOwnerSource": {
          "description": "Explicitly share Owner source, which can include sensitive values.",
          "type": "boolean"
        },
        "effects": {
          "items": {
            "$ref": "#/$defs/DelegatedAutomationEffect"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "effects",
        "allowDynamicTargets"
      ],
      "type": "object"
    },
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "DelegatedAutomationEffect": {
      "description": "Explicit policy for future reusable delegated automation execution. This\nis not itself an execution permit: the host must validate the saved closure\nand current subjects before it may be used at runtime.",
      "enum": [
        "log",
        "delay",
        "points",
        "http",
        "audio",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectGrants": {
      "additionalProperties": false,
      "properties": {
        "audio": {
          "default": [],
          "items": {
            "$ref": "#/$defs/AudioEffectGrant"
          },
          "type": "array"
        },
        "emitTopics": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "http": {
          "default": [],
          "items": {
            "$ref": "#/$defs/HttpEffectGrant"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "Grant": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        },
        "automationDelegation": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationDelegation"
            },
            {
              "type": "null"
            }
          ],
          "description": "Missing on legacy grants; reusable execution is bounded by this policy."
        },
        "capabilities": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Capability"
          },
          "type": "array",
          "uniqueItems": true
        },
        "effects": {
          "$ref": "#/$defs/EffectGrants",
          "description": "External effects deny by default. Unlike legacy resource restrictions,\na missing or empty policy never means unrestricted access."
        },
        "maxPointsDelta": {
          "default": null,
          "format": "double",
          "type": [
            "number",
            "null"
          ]
        },
        "resources": {
          "additionalProperties": false,
          "default": {},
          "description": "Missing kind means unrestricted within a capability. An empty set\nexplicitly denies that resource kind; IDs never contain wildcards.",
          "properties": {
            "automation": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "competition": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "live": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "plugin": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "ranking": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "viewer": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "widget": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "workflow": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            }
          },
          "type": "object"
        },
        "settingsFields": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        }
      },
      "type": "object"
    },
    "HttpEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "allowPrivateNetwork": {
          "type": "boolean"
        },
        "allowTextBody": {
          "type": "boolean"
        },
        "bodyFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "headerNames": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxBodyBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "maxResponseBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "methods": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "origin": {
          "type": "string"
        },
        "paths": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "queryFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "renderFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "origin",
        "methods",
        "paths",
        "queryFields",
        "headerNames",
        "renderFields",
        "bodyFields",
        "allowTextBody",
        "maxBodyBytes",
        "maxResponseBytes",
        "allowPrivateNetwork"
      ],
      "type": "object"
    },
    "OwnerApprovalBlockReason": {
      "enum": [
        "definition-missing",
        "no-delegated-subjects",
        "subjects-unavailable",
        "policy-unavailable"
      ],
      "type": "string"
    },
    "OwnerApprovalEligibility": {
      "additionalProperties": false,
      "properties": {
        "eligible": {
          "type": "boolean"
        },
        "reasonCode": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerApprovalBlockReason"
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "eligible"
      ],
      "type": "object"
    },
    "OwnerApprovalStatus": {
      "additionalProperties": false,
      "properties": {
        "current": {
          "type": "boolean"
        },
        "generation": {
          "type": [
            "string",
            "null"
          ]
        },
        "manifestDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "policyVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "state": {
          "$ref": "#/$defs/OwnerApprovalStatusKind"
        }
      },
      "required": [
        "state",
        "current"
      ],
      "type": "object"
    },
    "OwnerApprovalStatusKind": {
      "enum": [
        "none",
        "active",
        "invalidated",
        "revoked"
      ],
      "type": "string"
    },
    "OwnerApprovalSubject": {
      "additionalProperties": false,
      "properties": {
        "configurationRevision": {
          "type": "string"
        },
        "grant": {
          "$ref": "#/$defs/Grant"
        },
        "grantRevision": {
          "type": "string"
        },
        "principalId": {
          "type": "string"
        },
        "sessionId": {
          "type": "string"
        }
      },
      "required": [
        "principalId",
        "sessionId",
        "grantRevision",
        "configurationRevision",
        "grant"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "approval": {
      "$ref": "#/$defs/OwnerApprovalStatus"
    },
    "eligibility": {
      "$ref": "#/$defs/OwnerApprovalEligibility"
    },
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    },
    "manifest": true,
    "source": true,
    "subjects": {
      "items": {
        "$ref": "#/$defs/OwnerApprovalSubject"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "kind",
    "id",
    "source",
    "subjects",
    "eligibility",
    "approval"
  ],
  "title": "OwnerApprovalReviewResult",
  "type": "object"
}
```

:::

### automation.approval.review_unknown {#rpc-automation-approval-review_unknown}

Descrição do contrato:

```text
Archives an explicitly reviewed unresolved owner approval without replay
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgeUnknown": {
      "type": "boolean"
    },
    "commandId": {
      "type": "string"
    }
  },
  "required": [
    "commandId",
    "acknowledgeUnknown"
  ],
  "title": "OwnerApprovalReviewUnknown",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerApprovalIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "$ref": "#/$defs/OwnerApprovalOperation"
        }
      },
      "required": [
        "commandId",
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerApprovalOperation": {
      "enum": [
        "activate",
        "revoke"
      ],
      "type": "string"
    },
    "OwnerApprovalOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerApprovalReceipt": {
      "additionalProperties": false,
      "properties": {
        "approvalGeneration": {
          "type": "string"
        },
        "manifestDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "policyVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "revision": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "approvalGeneration"
      ],
      "type": "object"
    },
    "OwnerApprovalRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerApprovalIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerApprovalOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerApprovalReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerApprovalIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerApprovalRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerApprovalState",
  "type": "object"
}
```

:::

### automation.approval.revoke {#rpc-automation-approval-revoke}

Descrição do contrato:

```text
Owner-only durable revocation of future delegated execution
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgeRevocation": {
      "type": "boolean"
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    }
  },
  "required": [
    "kind",
    "id",
    "acknowledgeRevocation"
  ],
  "title": "ApprovalRevokeParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Revocation fences future delegated work; it does not undo dispatched work.",
  "properties": {
    "approvalGeneration": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    },
    "revoked": {
      "type": "boolean"
    }
  },
  "required": [
    "kind",
    "id",
    "revoked",
    "approvalGeneration"
  ],
  "title": "ApprovalRevokeReceipt",
  "type": "object"
}
```

:::

### automation.approval.submit {#rpc-automation-approval-submit}

Descrição do contrato:

```text
Persists a native owner intent before reviewed activation or revocation
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "ApprovalEnablement": {
      "additionalProperties": false,
      "properties": {
        "enabled": {
          "type": "boolean"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        }
      },
      "required": [
        "kind",
        "id",
        "enabled"
      ],
      "type": "object"
    },
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "additionalProperties": false,
      "properties": {
        "acknowledgeEffects": {
          "type": "boolean"
        },
        "enablePlan": {
          "items": {
            "$ref": "#/$defs/ApprovalEnablement"
          },
          "type": "array"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "manifestDigest": {
          "type": "string"
        },
        "operation": {
          "const": "activate",
          "type": "string"
        },
        "reviewedVersions": {
          "items": {
            "$ref": "#/$defs/ResourceVersion"
          },
          "type": "array"
        }
      },
      "required": [
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostId",
        "hostInstanceId",
        "reviewedVersions",
        "manifestDigest",
        "enablePlan",
        "acknowledgeEffects"
      ],
      "type": "object"
    },
    {
      "additionalProperties": false,
      "properties": {
        "acknowledgeRevocation": {
          "type": "boolean"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "const": "revoke",
          "type": "string"
        }
      },
      "required": [
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostId",
        "hostInstanceId",
        "acknowledgeRevocation"
      ],
      "type": "object"
    }
  ],
  "title": "OwnerApprovalSubmit"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerApprovalIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "$ref": "#/$defs/OwnerApprovalOperation"
        }
      },
      "required": [
        "commandId",
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerApprovalOperation": {
      "enum": [
        "activate",
        "revoke"
      ],
      "type": "string"
    },
    "OwnerApprovalOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerApprovalReceipt": {
      "additionalProperties": false,
      "properties": {
        "approvalGeneration": {
          "type": "string"
        },
        "manifestDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "policyVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "revision": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "approvalGeneration"
      ],
      "type": "object"
    },
    "OwnerApprovalRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerApprovalIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerApprovalOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerApprovalReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerApprovalIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerApprovalRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerApprovalState",
  "type": "object"
}
```

:::

### automation.authority.get {#rpc-automation-authority-get}

Descrição do contrato:

```text
Owner-only retained behavior provenance and generation
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "AutomationGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.context {#rpc-automation-context}

Descrição do contrato:

```text
Last automation event observed, for context panels and previews (eventType narrows to one trigger)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "eventType": {
      "default": null,
      "description": "Return the last envelope of this type instead of the global last\nevent. Callers emulating one trigger pass it so previews replay real\ndata of the right shape.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "AutomationContextParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "capturedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "event": true
  },
  "title": "AutomationContextResult",
  "type": "object"
}
```

:::

### automation.create {#rpc-automation-create}

Descrição do contrato:

```text
Creates a behavior event or action (id generated when omitted)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "record": true
  },
  "required": [
    "record"
  ],
  "title": "AutomationCreateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.definition.get {#rpc-automation-definition-get}

Descrição do contrato:

```text
Owner-only pending definition write and recent durable metadata
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerDefinitionEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerDefinitionIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "$ref": "#/$defs/OwnerDefinitionOperation"
        },
        "recordDigest": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "commandId",
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerDefinitionOperation": {
      "enum": [
        "create",
        "update",
        "delete",
        "enable",
        "disable"
      ],
      "type": "string"
    },
    "OwnerDefinitionOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerDefinitionReceipt": {
      "additionalProperties": false,
      "properties": {
        "affectedResources": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "deleted": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "enabled": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "recordDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "revision": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "affectedResources"
      ],
      "type": "object"
    },
    "OwnerDefinitionRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerDefinitionIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerDefinitionOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerDefinitionReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerDefinitionIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerDefinitionRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerDefinitionState",
  "type": "object"
}
```

:::

### automation.definition.resolve {#rpc-automation-definition-resolve}

Descrição do contrato:

```text
Looks up the original Owner definition outcome without dispatch or historical source
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerDefinitionEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerDefinitionIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "$ref": "#/$defs/OwnerDefinitionOperation"
        },
        "recordDigest": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "commandId",
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerDefinitionOperation": {
      "enum": [
        "create",
        "update",
        "delete",
        "enable",
        "disable"
      ],
      "type": "string"
    },
    "OwnerDefinitionOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerDefinitionReceipt": {
      "additionalProperties": false,
      "properties": {
        "affectedResources": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "deleted": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "enabled": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "recordDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "revision": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "affectedResources"
      ],
      "type": "object"
    },
    "OwnerDefinitionRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerDefinitionIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerDefinitionOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerDefinitionReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerDefinitionIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerDefinitionRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerDefinitionState",
  "type": "object"
}
```

:::

### automation.definition.review_unknown {#rpc-automation-definition-review_unknown}

Descrição do contrato:

```text
Archives explicitly reviewed uncertainty for the original Owner definition identity
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgeUnknown": {
      "type": "boolean"
    },
    "commandId": {
      "type": "string"
    }
  },
  "required": [
    "commandId",
    "acknowledgeUnknown"
  ],
  "title": "OwnerDefinitionReviewUnknown",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerDefinitionIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "$ref": "#/$defs/OwnerDefinitionOperation"
        },
        "recordDigest": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "commandId",
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerDefinitionOperation": {
      "enum": [
        "create",
        "update",
        "delete",
        "enable",
        "disable"
      ],
      "type": "string"
    },
    "OwnerDefinitionOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerDefinitionReceipt": {
      "additionalProperties": false,
      "properties": {
        "affectedResources": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "deleted": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "enabled": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "recordDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "revision": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "affectedResources"
      ],
      "type": "object"
    },
    "OwnerDefinitionRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerDefinitionIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerDefinitionOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerDefinitionReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerDefinitionIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerDefinitionRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerDefinitionState",
  "type": "object"
}
```

:::

### automation.definition.submit {#rpc-automation-definition-submit}

Descrição do contrato:

```text
Persists a native Owner intent before one checked create, update, delete or enablement write
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "additionalProperties": false,
      "properties": {
        "expectedRevision": {
          "type": "string"
        },
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "const": "create",
          "type": "string"
        },
        "record": true
      },
      "required": [
        "operation",
        "kind",
        "record",
        "expectedRevision",
        "hostId",
        "hostInstanceId"
      ],
      "type": "object"
    },
    {
      "additionalProperties": false,
      "properties": {
        "expectedRevision": {
          "type": "string"
        },
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "const": "update",
          "type": "string"
        },
        "record": true
      },
      "required": [
        "operation",
        "kind",
        "id",
        "record",
        "expectedRevision",
        "hostId",
        "hostInstanceId"
      ],
      "type": "object"
    },
    {
      "additionalProperties": false,
      "properties": {
        "expectedRevision": {
          "type": "string"
        },
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "const": "delete",
          "type": "string"
        }
      },
      "required": [
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostId",
        "hostInstanceId"
      ],
      "type": "object"
    },
    {
      "additionalProperties": false,
      "properties": {
        "expectedRevision": {
          "type": "string"
        },
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "const": "enable",
          "type": "string"
        }
      },
      "required": [
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostId",
        "hostInstanceId"
      ],
      "type": "object"
    },
    {
      "additionalProperties": false,
      "properties": {
        "expectedRevision": {
          "type": "string"
        },
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "const": "disable",
          "type": "string"
        }
      },
      "required": [
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostId",
        "hostInstanceId"
      ],
      "type": "object"
    }
  ],
  "title": "OwnerDefinitionSubmit"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "CommandReceipt": {
      "properties": {
        "commandId": {
          "type": "string"
        },
        "committedAt": {
          "format": "int64",
          "type": [
            "integer",
            "null"
          ]
        },
        "resource": {
          "anyOf": [
            {
              "$ref": "#/$defs/CommandResource"
            },
            {
              "type": "null"
            }
          ]
        },
        "state": {
          "$ref": "#/$defs/CommandState"
        }
      },
      "required": [
        "commandId",
        "state"
      ],
      "type": "object"
    },
    "CommandResource": {
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope"
      ],
      "type": "object"
    },
    "CommandState": {
      "enum": [
        "accepted",
        "running",
        "succeeded",
        "failed",
        "unknown"
      ],
      "type": "string"
    },
    "OwnerDefinitionAcknowledgement": {
      "additionalProperties": false,
      "description": "Returned only by the original successful submission. Recovery surfaces\ncontain metadata, so they cannot silently replace an editor's newer draft.",
      "properties": {
        "command": {
          "$ref": "#/$defs/CommandReceipt"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "resourceVersions": {
          "items": {
            "$ref": "#/$defs/ResourceVersion"
          },
          "type": "array"
        },
        "value": true
      },
      "required": [
        "value",
        "resourceVersions",
        "command",
        "hostInstanceId"
      ],
      "type": "object"
    },
    "OwnerDefinitionIntent": {
      "additionalProperties": false,
      "properties": {
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/AutomationKind"
        },
        "operation": {
          "$ref": "#/$defs/OwnerDefinitionOperation"
        },
        "recordDigest": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "commandId",
        "operation",
        "kind",
        "id",
        "expectedRevision",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerDefinitionOperation": {
      "enum": [
        "create",
        "update",
        "delete",
        "enable",
        "disable"
      ],
      "type": "string"
    },
    "OwnerDefinitionOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerDefinitionReceipt": {
      "additionalProperties": false,
      "properties": {
        "affectedResources": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "deleted": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "enabled": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "recordDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "revision": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "affectedResources"
      ],
      "type": "object"
    },
    "OwnerDefinitionRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerDefinitionIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerDefinitionOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerDefinitionReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    },
    "OwnerDefinitionState": {
      "additionalProperties": false,
      "properties": {
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "pending": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerDefinitionIntent"
            },
            {
              "type": "null"
            }
          ]
        },
        "recent": {
          "items": {
            "$ref": "#/$defs/OwnerDefinitionRecord"
          },
          "type": "array"
        }
      },
      "required": [
        "hostId",
        "hostInstanceId",
        "recent"
      ],
      "type": "object"
    },
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgement": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerDefinitionAcknowledgement"
        },
        {
          "type": "null"
        }
      ]
    },
    "state": {
      "$ref": "#/$defs/OwnerDefinitionState"
    }
  },
  "required": [
    "state"
  ],
  "title": "OwnerDefinitionSubmission",
  "type": "object"
}
```

:::

### automation.delegation.preflight {#rpc-automation-delegation-preflight}

Descrição do contrato:

```text
Owner-only advisory reusable-grant check; never an execution permit
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "A saved definition only; caller JSON cannot provide authority or a candidate.",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    }
  },
  "required": [
    "kind",
    "id"
  ],
  "title": "AutomationManifestParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.delete {#rpc-automation-delete}

Descrição do contrato:

```text
Deletes a behavior event or action
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "AutomationGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "deleted": {
      "type": "boolean"
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "deleted"
  ],
  "title": "AutomationDeleteResult",
  "type": "object"
}
```

:::

### automation.disable {#rpc-automation-disable}

Descrição do contrato:

```text
Sets behavior event or action enablement
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "AutomationGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.effects.get {#rpc-automation-effects-get}

Descrição do contrato:

```text
Owner-only metadata receipt lookup; never retries an effect
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "effectId": {
      "type": "string"
    }
  },
  "required": [
    "effectId"
  ],
  "title": "EffectGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "EffectKind": {
      "enum": [
        "http",
        "audio",
        "plugin",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectReceipt": {
      "additionalProperties": false,
      "description": "Deliberately excludes definitions, requests, destinations, settings, errors\nand response payloads. An expired receipt retains its deduplication identity.",
      "properties": {
        "acceptedAt": {
          "format": "int64",
          "type": "integer"
        },
        "effectId": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/EffectKind"
        },
        "outcomeExpired": {
          "type": "boolean"
        },
        "state": {
          "$ref": "#/$defs/EffectState"
        },
        "updatedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "effectId",
        "kind",
        "state",
        "acceptedAt",
        "updatedAt",
        "outcomeExpired"
      ],
      "type": "object"
    },
    "EffectState": {
      "enum": [
        "accepted",
        "dispatched",
        "succeeded",
        "failed",
        "unknown"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "anyOf": [
    {
      "$ref": "#/$defs/EffectReceipt"
    },
    {
      "type": "null"
    }
  ],
  "title": "Nullable_EffectReceipt"
}
```

:::

### automation.effects.history {#rpc-automation-effects-history}

Descrição do contrato:

```text
Owner-only bounded metadata receipt history, including uncertain outcomes
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "before": {
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "limit"
  ],
  "title": "EffectHistoryParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "EffectKind": {
      "enum": [
        "http",
        "audio",
        "plugin",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectReceipt": {
      "additionalProperties": false,
      "description": "Deliberately excludes definitions, requests, destinations, settings, errors\nand response payloads. An expired receipt retains its deduplication identity.",
      "properties": {
        "acceptedAt": {
          "format": "int64",
          "type": "integer"
        },
        "effectId": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/EffectKind"
        },
        "outcomeExpired": {
          "type": "boolean"
        },
        "state": {
          "$ref": "#/$defs/EffectState"
        },
        "updatedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "effectId",
        "kind",
        "state",
        "acceptedAt",
        "updatedAt",
        "outcomeExpired"
      ],
      "type": "object"
    },
    "EffectState": {
      "enum": [
        "accepted",
        "dispatched",
        "succeeded",
        "failed",
        "unknown"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/EffectReceipt"
      },
      "type": "array"
    },
    "nextBefore": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "items"
  ],
  "title": "EffectHistory",
  "type": "object"
}
```

:::

### automation.enable {#rpc-automation-enable}

Descrição do contrato:

```text
Sets behavior event or action enablement
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "AutomationGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.fire {#rpc-automation-fire}

Descrição do contrato:

```text
Fires a synthetic event through the live pipeline: really triggers matching events and executes their actions (no dry run)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "event": {
      "default": null,
      "description": "Optional custom envelope used as-is (fresh id/timestamp, type forced\nto `trigger`). Absent: last live envelope of the trigger, else the\nper-type sample."
    },
    "record": {
      "default": null,
      "description": "Optional editor draft to test-fire: runs when its trigger and\nfilters match, even when unsaved or disabled. Its saved twin, if\nany, is skipped so one fire never executes the same event twice."
    },
    "trigger": {
      "description": "Trigger to fire (`tiktok.gift`, ...). Required: firing needs a type.",
      "type": "string"
    }
  },
  "required": [
    "trigger"
  ],
  "title": "AutomationFireParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.get {#rpc-automation-get}

Descrição do contrato:

```text
One behavior event or action by id
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "AutomationGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.library.get {#rpc-automation-library-get}

Descrição do contrato:

```text
Owner-only pending library intent and recent metadata outcomes
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerLibraryEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "OwnerLibraryCloudProvenance": {
      "additionalProperties": false,
      "description": "Exact remote identity of a Cloud document staged for local review. It is\nevidence bound to one review, never authority to send or apply data.",
      "properties": {
        "accountId": {
          "type": "string"
        },
        "documentId": {
          "type": "string"
        },
        "origin": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "sha256": {
          "type": "string"
        }
      },
      "required": [
        "origin",
        "accountId",
        "documentId",
        "revision",
        "sha256"
      ],
      "type": "object"
    },
    "OwnerLibraryIntent": {
      "additionalProperties": false,
      "properties": {
        "commandHash": {
          "type": "string"
        },
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "definitionDigest": {
          "type": "string"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "operation": {
          "$ref": "#/$defs/OwnerLibraryOperation"
        },
        "provenance": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryCloudProvenance"
            },
            {
              "type": "null"
            }
          ]
        },
        "target": {
          "type": "string"
        }
      },
      "required": [
        "commandId",
        "operation",
        "target",
        "expectedRevision",
        "definitionDigest",
        "commandHash",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerLibraryOperation": {
      "enum": [
        "create-profile",
        "switch-profile",
        "delete-profile",
        "apply-template",
        "import-profile",
        "assign-record",
        "import-templates",
        "delete-template"
      ],
      "type": "string"
    },
    "OwnerLibraryOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerLibraryReceipt": {
      "additionalProperties": false,
      "properties": {
        "affectedResources": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "catalogDigest": {
          "type": "string"
        },
        "definitionDigest": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "versionsDigest": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "catalogDigest",
        "definitionDigest",
        "versionsDigest",
        "affectedResources"
      ],
      "type": "object"
    },
    "OwnerLibraryRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerLibraryIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerLibraryOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerLibraryIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerLibraryRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerLibraryState",
  "type": "object"
}
```

:::

### automation.library.resolve {#rpc-automation-library-resolve}

Descrição do contrato:

```text
Read the original library outcome without source or another dispatch
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerLibraryEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "OwnerLibraryCloudProvenance": {
      "additionalProperties": false,
      "description": "Exact remote identity of a Cloud document staged for local review. It is\nevidence bound to one review, never authority to send or apply data.",
      "properties": {
        "accountId": {
          "type": "string"
        },
        "documentId": {
          "type": "string"
        },
        "origin": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "sha256": {
          "type": "string"
        }
      },
      "required": [
        "origin",
        "accountId",
        "documentId",
        "revision",
        "sha256"
      ],
      "type": "object"
    },
    "OwnerLibraryIntent": {
      "additionalProperties": false,
      "properties": {
        "commandHash": {
          "type": "string"
        },
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "definitionDigest": {
          "type": "string"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "operation": {
          "$ref": "#/$defs/OwnerLibraryOperation"
        },
        "provenance": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryCloudProvenance"
            },
            {
              "type": "null"
            }
          ]
        },
        "target": {
          "type": "string"
        }
      },
      "required": [
        "commandId",
        "operation",
        "target",
        "expectedRevision",
        "definitionDigest",
        "commandHash",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerLibraryOperation": {
      "enum": [
        "create-profile",
        "switch-profile",
        "delete-profile",
        "apply-template",
        "import-profile",
        "assign-record",
        "import-templates",
        "delete-template"
      ],
      "type": "string"
    },
    "OwnerLibraryOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerLibraryReceipt": {
      "additionalProperties": false,
      "properties": {
        "affectedResources": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "catalogDigest": {
          "type": "string"
        },
        "definitionDigest": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "versionsDigest": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "catalogDigest",
        "definitionDigest",
        "versionsDigest",
        "affectedResources"
      ],
      "type": "object"
    },
    "OwnerLibraryRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerLibraryIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerLibraryOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerLibraryIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerLibraryRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerLibraryState",
  "type": "object"
}
```

:::

### automation.library.review_unknown {#rpc-automation-library-review_unknown}

Descrição do contrato:

```text
Archive explicitly reviewed uncertainty for the original Owner library command
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgeUnknown": {
      "type": "boolean"
    },
    "commandId": {
      "type": "string"
    }
  },
  "required": [
    "commandId",
    "acknowledgeUnknown"
  ],
  "title": "OwnerLibraryReviewUnknown",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "OwnerLibraryCloudProvenance": {
      "additionalProperties": false,
      "description": "Exact remote identity of a Cloud document staged for local review. It is\nevidence bound to one review, never authority to send or apply data.",
      "properties": {
        "accountId": {
          "type": "string"
        },
        "documentId": {
          "type": "string"
        },
        "origin": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "sha256": {
          "type": "string"
        }
      },
      "required": [
        "origin",
        "accountId",
        "documentId",
        "revision",
        "sha256"
      ],
      "type": "object"
    },
    "OwnerLibraryIntent": {
      "additionalProperties": false,
      "properties": {
        "commandHash": {
          "type": "string"
        },
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "definitionDigest": {
          "type": "string"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "operation": {
          "$ref": "#/$defs/OwnerLibraryOperation"
        },
        "provenance": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryCloudProvenance"
            },
            {
              "type": "null"
            }
          ]
        },
        "target": {
          "type": "string"
        }
      },
      "required": [
        "commandId",
        "operation",
        "target",
        "expectedRevision",
        "definitionDigest",
        "commandHash",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerLibraryOperation": {
      "enum": [
        "create-profile",
        "switch-profile",
        "delete-profile",
        "apply-template",
        "import-profile",
        "assign-record",
        "import-templates",
        "delete-template"
      ],
      "type": "string"
    },
    "OwnerLibraryOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerLibraryReceipt": {
      "additionalProperties": false,
      "properties": {
        "affectedResources": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "catalogDigest": {
          "type": "string"
        },
        "definitionDigest": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "versionsDigest": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "catalogDigest",
        "definitionDigest",
        "versionsDigest",
        "affectedResources"
      ],
      "type": "object"
    },
    "OwnerLibraryRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerLibraryIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerLibraryOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pending": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerLibraryIntent"
        },
        {
          "type": "null"
        }
      ]
    },
    "recent": {
      "items": {
        "$ref": "#/$defs/OwnerLibraryRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "recent"
  ],
  "title": "OwnerLibraryState",
  "type": "object"
}
```

:::

### automation.library.snapshot {#rpc-automation-library-snapshot}

Descrição do contrato:

```text
Coherent Owner graph, profile and custom template catalog with its reviewed digest
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "title": "OwnerLibraryEmpty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "OwnerLibraryCatalog": {
      "additionalProperties": false,
      "properties": {
        "activeId": {
          "type": "string"
        },
        "packs": {
          "items": {
            "$ref": "#/$defs/OwnerLibraryPack"
          },
          "type": "array"
        },
        "templates": {
          "items": true,
          "type": "array"
        }
      },
      "required": [
        "packs",
        "activeId",
        "templates"
      ],
      "type": "object"
    },
    "OwnerLibraryPack": {
      "additionalProperties": false,
      "properties": {
        "actionIds": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "description": {
          "type": "string"
        },
        "eventIds": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "id": {
          "type": "string"
        },
        "name": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "name",
        "description",
        "eventIds",
        "actionIds"
      ],
      "type": "object"
    },
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "catalog": {
      "$ref": "#/$defs/OwnerLibraryCatalog"
    },
    "definitionDigest": {
      "type": "string"
    },
    "definitions": true,
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "resourceVersions": {
      "items": {
        "$ref": "#/$defs/ResourceVersion"
      },
      "type": "array"
    },
    "revision": {
      "type": "string"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "revision",
    "definitionDigest",
    "catalog",
    "definitions",
    "resourceVersions"
  ],
  "title": "OwnerLibrarySnapshot",
  "type": "object"
}
```

:::

### automation.library.submit {#rpc-automation-library-submit}

Descrição do contrato:

```text
One durable Owner graph/catalog transaction bound to its reviewed library snapshot
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "OwnerLibraryCloudProvenance": {
      "additionalProperties": false,
      "description": "Exact remote identity of a Cloud document staged for local review. It is\nevidence bound to one review, never authority to send or apply data.",
      "properties": {
        "accountId": {
          "type": "string"
        },
        "documentId": {
          "type": "string"
        },
        "origin": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "sha256": {
          "type": "string"
        }
      },
      "required": [
        "origin",
        "accountId",
        "documentId",
        "revision",
        "sha256"
      ],
      "type": "object"
    },
    "OwnerLibraryEntry": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "items": true,
          "type": "array"
        },
        "event": true
      },
      "required": [
        "actions",
        "event"
      ],
      "type": "object"
    },
    "OwnerLibraryMutation": {
      "oneOf": [
        {
          "additionalProperties": false,
          "properties": {
            "description": {
              "type": "string"
            },
            "id": {
              "type": "string"
            },
            "name": {
              "type": "string"
            },
            "operation": {
              "const": "create-profile",
              "type": "string"
            }
          },
          "required": [
            "operation",
            "id",
            "name",
            "description"
          ],
          "type": "object"
        },
        {
          "additionalProperties": false,
          "properties": {
            "id": {
              "type": "string"
            },
            "operation": {
              "const": "switch-profile",
              "type": "string"
            }
          },
          "required": [
            "operation",
            "id"
          ],
          "type": "object"
        },
        {
          "additionalProperties": false,
          "properties": {
            "id": {
              "type": "string"
            },
            "operation": {
              "const": "delete-profile",
              "type": "string"
            }
          },
          "required": [
            "operation",
            "id"
          ],
          "type": "object"
        },
        {
          "additionalProperties": false,
          "properties": {
            "additionalEntries": {
              "items": {
                "$ref": "#/$defs/OwnerLibraryEntry"
              },
              "type": [
                "array",
                "null"
              ]
            },
            "entry": {
              "$ref": "#/$defs/OwnerLibraryEntry"
            },
            "operation": {
              "const": "apply-template",
              "type": "string"
            },
            "profileId": {
              "type": "string"
            },
            "templates": {
              "items": true,
              "type": [
                "array",
                "null"
              ]
            }
          },
          "required": [
            "operation",
            "entry",
            "profileId"
          ],
          "type": "object"
        },
        {
          "additionalProperties": false,
          "properties": {
            "entries": {
              "items": {
                "$ref": "#/$defs/OwnerLibraryEntry"
              },
              "type": "array"
            },
            "operation": {
              "const": "import-profile",
              "type": "string"
            },
            "profile": {
              "$ref": "#/$defs/OwnerLibraryProfile"
            },
            "templates": {
              "items": true,
              "type": [
                "array",
                "null"
              ]
            }
          },
          "required": [
            "operation",
            "profile",
            "entries"
          ],
          "type": "object"
        },
        {
          "additionalProperties": false,
          "properties": {
            "id": {
              "type": "string"
            },
            "kind": {
              "$ref": "#/$defs/AutomationKind"
            },
            "operation": {
              "const": "assign-record",
              "type": "string"
            },
            "profileId": {
              "type": "string"
            }
          },
          "required": [
            "operation",
            "kind",
            "id",
            "profileId"
          ],
          "type": "object"
        },
        {
          "additionalProperties": false,
          "properties": {
            "operation": {
              "const": "import-templates",
              "type": "string"
            },
            "templates": {
              "items": true,
              "type": "array"
            }
          },
          "required": [
            "operation",
            "templates"
          ],
          "type": "object"
        },
        {
          "additionalProperties": false,
          "properties": {
            "id": {
              "type": "string"
            },
            "operation": {
              "const": "delete-template",
              "type": "string"
            }
          },
          "required": [
            "operation",
            "id"
          ],
          "type": "object"
        }
      ]
    },
    "OwnerLibraryProfile": {
      "additionalProperties": false,
      "properties": {
        "description": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "name": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "name",
        "description"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "definitionDigest": {
      "type": "string"
    },
    "expectedRevision": {
      "type": "string"
    },
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "mutation": {
      "$ref": "#/$defs/OwnerLibraryMutation"
    },
    "provenance": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerLibraryCloudProvenance"
        },
        {
          "type": "null"
        }
      ]
    }
  },
  "required": [
    "mutation",
    "expectedRevision",
    "definitionDigest",
    "hostId",
    "hostInstanceId"
  ],
  "title": "OwnerLibrarySubmit",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CommandReceipt": {
      "properties": {
        "commandId": {
          "type": "string"
        },
        "committedAt": {
          "format": "int64",
          "type": [
            "integer",
            "null"
          ]
        },
        "resource": {
          "anyOf": [
            {
              "$ref": "#/$defs/CommandResource"
            },
            {
              "type": "null"
            }
          ]
        },
        "state": {
          "$ref": "#/$defs/CommandState"
        }
      },
      "required": [
        "commandId",
        "state"
      ],
      "type": "object"
    },
    "CommandResource": {
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope"
      ],
      "type": "object"
    },
    "CommandState": {
      "enum": [
        "accepted",
        "running",
        "succeeded",
        "failed",
        "unknown"
      ],
      "type": "string"
    },
    "OwnerLibraryAcknowledgement": {
      "additionalProperties": false,
      "properties": {
        "command": {
          "$ref": "#/$defs/CommandReceipt"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "resourceVersions": {
          "items": {
            "$ref": "#/$defs/ResourceVersion"
          },
          "type": "array"
        },
        "value": {
          "$ref": "#/$defs/OwnerLibraryReceipt"
        }
      },
      "required": [
        "value",
        "resourceVersions",
        "command",
        "hostInstanceId"
      ],
      "type": "object"
    },
    "OwnerLibraryCloudProvenance": {
      "additionalProperties": false,
      "description": "Exact remote identity of a Cloud document staged for local review. It is\nevidence bound to one review, never authority to send or apply data.",
      "properties": {
        "accountId": {
          "type": "string"
        },
        "documentId": {
          "type": "string"
        },
        "origin": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "sha256": {
          "type": "string"
        }
      },
      "required": [
        "origin",
        "accountId",
        "documentId",
        "revision",
        "sha256"
      ],
      "type": "object"
    },
    "OwnerLibraryIntent": {
      "additionalProperties": false,
      "properties": {
        "commandHash": {
          "type": "string"
        },
        "commandId": {
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "definitionDigest": {
          "type": "string"
        },
        "expectedRevision": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "operation": {
          "$ref": "#/$defs/OwnerLibraryOperation"
        },
        "provenance": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryCloudProvenance"
            },
            {
              "type": "null"
            }
          ]
        },
        "target": {
          "type": "string"
        }
      },
      "required": [
        "commandId",
        "operation",
        "target",
        "expectedRevision",
        "definitionDigest",
        "commandHash",
        "hostInstanceId",
        "createdAt"
      ],
      "type": "object"
    },
    "OwnerLibraryOperation": {
      "enum": [
        "create-profile",
        "switch-profile",
        "delete-profile",
        "apply-template",
        "import-profile",
        "assign-record",
        "import-templates",
        "delete-template"
      ],
      "type": "string"
    },
    "OwnerLibraryOutcome": {
      "enum": [
        "succeeded",
        "rejected",
        "reviewed-unknown"
      ],
      "type": "string"
    },
    "OwnerLibraryReceipt": {
      "additionalProperties": false,
      "properties": {
        "affectedResources": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "catalogDigest": {
          "type": "string"
        },
        "definitionDigest": {
          "type": "string"
        },
        "revision": {
          "type": "string"
        },
        "versionsDigest": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "catalogDigest",
        "definitionDigest",
        "versionsDigest",
        "affectedResources"
      ],
      "type": "object"
    },
    "OwnerLibraryRecord": {
      "additionalProperties": false,
      "properties": {
        "errorCode": {
          "type": [
            "string",
            "null"
          ]
        },
        "intent": {
          "$ref": "#/$defs/OwnerLibraryIntent"
        },
        "outcome": {
          "$ref": "#/$defs/OwnerLibraryOutcome"
        },
        "receipt": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryReceipt"
            },
            {
              "type": "null"
            }
          ]
        },
        "recordedAt": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "intent",
        "outcome",
        "recordedAt"
      ],
      "type": "object"
    },
    "OwnerLibraryState": {
      "additionalProperties": false,
      "properties": {
        "hostId": {
          "type": "string"
        },
        "hostInstanceId": {
          "type": "string"
        },
        "pending": {
          "anyOf": [
            {
              "$ref": "#/$defs/OwnerLibraryIntent"
            },
            {
              "type": "null"
            }
          ]
        },
        "recent": {
          "items": {
            "$ref": "#/$defs/OwnerLibraryRecord"
          },
          "type": "array"
        }
      },
      "required": [
        "hostId",
        "hostInstanceId",
        "recent"
      ],
      "type": "object"
    },
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "acknowledgement": {
      "anyOf": [
        {
          "$ref": "#/$defs/OwnerLibraryAcknowledgement"
        },
        {
          "type": "null"
        }
      ]
    },
    "state": {
      "$ref": "#/$defs/OwnerLibraryState"
    }
  },
  "required": [
    "state"
  ],
  "title": "OwnerLibrarySubmission",
  "type": "object"
}
```

:::

### automation.list {#rpc-automation-list}

Descrição do contrato:

```text
Behavior events and actions (kind: event, action, all)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "default": null,
      "description": "`event`, `action`, or `all` (default).",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "AutomationListParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "actions": {
      "items": true,
      "type": "array"
    },
    "events": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "events",
    "actions"
  ],
  "title": "AutomationListResult",
  "type": "object"
}
```

:::

### automation.manifest.get {#rpc-automation-manifest-get}

Descrição do contrato:

```text
Owner-only exact definition and delegated effect manifest preview; never approval
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "A saved definition only; caller JSON cannot provide authority or a candidate.",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    }
  },
  "required": [
    "kind",
    "id"
  ],
  "title": "AutomationManifestParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.nodes.list {#rpc-automation-nodes-list}

Descrição do contrato:

```text
Built-in node catalog for the workflow graph editor
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "nodes": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "nodes"
  ],
  "title": "AutomationNodesResult",
  "type": "object"
}
```

:::

### automation.review {#rpc-automation-review}

Descrição do contrato:

```text
Owner-only atomic definition, provenance and dependency review
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "AutomationGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationKind": {
      "description": "Behavior record family. The storage table is a closed allowlist: table\nnames are never derived from caller input.",
      "enum": [
        "event",
        "action"
      ],
      "type": "string"
    },
    "AutomationReviewAuthority": {
      "oneOf": [
        {
          "additionalProperties": false,
          "properties": {
            "kind": {
              "const": "owner",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "additionalProperties": false,
          "properties": {
            "kind": {
              "const": "delegated",
              "type": "string"
            },
            "principalId": {
              "type": "string"
            },
            "sessionId": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "principalId",
            "sessionId"
          ],
          "type": "object"
        }
      ]
    },
    "AutomationReviewDependency": {
      "additionalProperties": false,
      "properties": {
        "id": {
          "type": "string"
        },
        "provenance": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationReviewProvenance"
            },
            {
              "type": "null"
            }
          ]
        },
        "record": true
      },
      "required": [
        "id"
      ],
      "type": "object"
    },
    "AutomationReviewProvenance": {
      "additionalProperties": false,
      "properties": {
        "authority": {
          "$ref": "#/$defs/AutomationReviewAuthority"
        },
        "generation": {
          "type": "string"
        }
      },
      "required": [
        "authority",
        "generation"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "dependencies": {
      "items": {
        "$ref": "#/$defs/AutomationReviewDependency"
      },
      "type": "array"
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/AutomationKind"
    },
    "provenance": {
      "anyOf": [
        {
          "$ref": "#/$defs/AutomationReviewProvenance"
        },
        {
          "type": "null"
        }
      ]
    },
    "record": true
  },
  "required": [
    "kind",
    "id",
    "dependencies"
  ],
  "title": "AutomationReviewResult",
  "type": "object"
}
```

:::

### automation.runs {#rpc-automation-runs}

Descrição do contrato:

```text
Recent behavior execution runs
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "runs": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "runs"
  ],
  "title": "AutomationRunsResult",
  "type": "object"
}
```

:::

### automation.script.analyze {#rpc-automation-script-analyze}

Descrição do contrato:

```text
Validates an automation script and returns diagnostics
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "eventType": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "nodeId": {
      "type": "string"
    },
    "offset": {
      "default": null,
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "source": {
      "type": "string"
    }
  },
  "required": [
    "nodeId",
    "source"
  ],
  "title": "AutomationScriptParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "completions": {
      "items": true,
      "type": "array"
    },
    "diagnostics": {
      "items": true,
      "type": "array"
    },
    "hover": true,
    "nodeId": {
      "type": "string"
    },
    "source": {
      "type": "string"
    }
  },
  "required": [
    "nodeId",
    "source",
    "diagnostics",
    "completions"
  ],
  "title": "ScriptAnalysisResult",
  "type": "object"
}
```

:::

### automation.set_enablement {#rpc-automation-set_enablement}

Descrição do contrato:

```text
Atomic owner bulk enablement across behavior families
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"exclusive-transition","durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutomationEnablementUpdate": {
      "description": "One record in a bulk enablement pass.",
      "properties": {
        "enabled": {
          "description": "Desired enabled state.",
          "type": "boolean"
        },
        "id": {
          "description": "Record id.",
          "type": "string"
        },
        "kind": {
          "description": "`event` or `action`.",
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "enabled"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "updates": {
      "description": "Batch of enable/disable updates (at most 512).",
      "items": {
        "$ref": "#/$defs/AutomationEnablementUpdate"
      },
      "type": "array"
    }
  },
  "required": [
    "updates"
  ],
  "title": "AutomationSetEnablementParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutomationEnablementMiss": {
      "description": "One record that no longer exists, reported by the bulk\nenablement pass so callers can prune it from their packs.",
      "properties": {
        "id": {
          "description": "Record id that no longer exists.",
          "type": "string"
        },
        "kind": {
          "description": "`event` or `action`.",
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Outcome of one bulk enablement pass: applied count plus\nthe records that were already gone.",
  "properties": {
    "missing": {
      "items": {
        "$ref": "#/$defs/AutomationEnablementMiss"
      },
      "type": "array"
    },
    "updated": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "updated",
    "missing"
  ],
  "title": "AutomationSetEnablementResult",
  "type": "object"
}
```

:::

### automation.snapshot {#rpc-automation-snapshot}

Descrição do contrato:

```text
Merged owner behavior snapshot and runtime catalog
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.source.get {#rpc-automation-source-get}

Descrição do contrato:

```text
Reads an explicitly shared automation definition
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "AutomationGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.test {#rpc-automation-test}

Descrição do contrato:

```text
Dry-runs a saved (id) or inline (record) event or action
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "default": null,
      "description": "Test a saved record by id...",
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "record": {
      "default": null,
      "description": "...or an inline record. One of the two is required."
    },
    "trigger": {
      "default": null,
      "description": "Sample trigger for inline action tests (default `tiktok.chat`).",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "AutomationTestParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### automation.update {#rpc-automation-update}

Descrição do contrato:

```text
Replaces a behavior event or action (must already exist)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "record": true
  },
  "required": [
    "id",
    "record"
  ],
  "title": "AutomationUpdateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### bundles.assets.delete {#rpc-bundles-assets-delete}

Descrição do contrato:

```text
Deletes a Cloud image; conflict while an overlay still uses it; needs media.operations delete
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "AssetIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Acknowledgement of a Cloud delete (`{\"id\", \"deleted\": true}`).",
  "properties": {
    "deleted": {
      "type": "boolean"
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "deleted"
  ],
  "title": "CloudDeleted",
  "type": "object"
}
```

:::

### bundles.assets.list {#rpc-bundles-assets-list}

Descrição do contrato:

```text
Lists the owner's Cloud images, newest first; needs media.operations list
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Paging for the owner's assets and overlays. Cursors come only from the\nprevious page's `nextCursor`.",
  "properties": {
    "cursor": {
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "description": "1 to 50; the Cloud default (50) when absent.",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "CloudPageQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CloudAsset": {
      "properties": {
        "createdAt": {
          "default": null,
          "description": "RFC 3339. Absent on a Cloud that predates asset listing.",
          "type": [
            "string",
            "null"
          ]
        },
        "height": {
          "format": "int64",
          "type": "integer"
        },
        "id": {
          "type": "string"
        },
        "label": {
          "default": null,
          "description": "Owner-chosen name, 1 to 120 characters. Absent when none is set.",
          "type": [
            "string",
            "null"
          ]
        },
        "mime": {
          "type": "string"
        },
        "sha256": {
          "type": "string"
        },
        "size": {
          "format": "int64",
          "type": "integer"
        },
        "width": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "id",
        "sha256",
        "mime",
        "width",
        "height",
        "size"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "One page of the owner's assets, newest first.",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/CloudAsset"
      },
      "type": "array"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "items"
  ],
  "title": "CloudAssetPage",
  "type": "object"
}
```

:::

### bundles.assets.setLabel {#rpc-bundles-assets-setLabel}

Descrição do contrato:

```text
Renames a Cloud image, or clears its label with null; needs media.operations rename
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    },
    "label": {
      "description": "The key is required. `null` clears the label; otherwise 1 to 120\ncharacters after trimming.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "AssetSetLabelParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "createdAt": {
      "default": null,
      "description": "RFC 3339. Absent on a Cloud that predates asset listing.",
      "type": [
        "string",
        "null"
      ]
    },
    "height": {
      "format": "int64",
      "type": "integer"
    },
    "id": {
      "type": "string"
    },
    "label": {
      "default": null,
      "description": "Owner-chosen name, 1 to 120 characters. Absent when none is set.",
      "type": [
        "string",
        "null"
      ]
    },
    "mime": {
      "type": "string"
    },
    "sha256": {
      "type": "string"
    },
    "size": {
      "format": "int64",
      "type": "integer"
    },
    "width": {
      "format": "int64",
      "type": "integer"
    }
  },
  "required": [
    "id",
    "sha256",
    "mime",
    "width",
    "height",
    "size"
  ],
  "title": "CloudAsset",
  "type": "object"
}
```

:::

### bundles.assets.upload {#rpc-bundles-assets-upload}

Descrição do contrato:

```text
Uploads a PNG, JPEG or WebP image to the owner's Cloud asset store
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "dataBase64": {
      "description": "Standard base64 of raw PNG, JPEG or WebP bytes, at most 512 KiB decoded.",
      "type": "string"
    }
  },
  "required": [
    "dataBase64"
  ],
  "title": "AssetUploadParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "createdAt": {
      "default": null,
      "description": "RFC 3339. Absent on a Cloud that predates asset listing.",
      "type": [
        "string",
        "null"
      ]
    },
    "height": {
      "format": "int64",
      "type": "integer"
    },
    "id": {
      "type": "string"
    },
    "label": {
      "default": null,
      "description": "Owner-chosen name, 1 to 120 characters. Absent when none is set.",
      "type": [
        "string",
        "null"
      ]
    },
    "mime": {
      "type": "string"
    },
    "sha256": {
      "type": "string"
    },
    "size": {
      "format": "int64",
      "type": "integer"
    },
    "width": {
      "format": "int64",
      "type": "integer"
    }
  },
  "required": [
    "id",
    "sha256",
    "mime",
    "width",
    "height",
    "size"
  ],
  "title": "CloudAsset",
  "type": "object"
}
```

:::

### bundles.overlays.create {#rpc-bundles-overlays-create}

Descrição do contrato:

```text
Creates a private overlay from a schema version 1 document
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "document": true
  },
  "required": [
    "document"
  ],
  "title": "OverlayCreateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "revision",
    "status"
  ],
  "title": "OverlayWriteResult",
  "type": "object"
}
```

:::

### bundles.overlays.delete {#rpc-bundles-overlays-delete}

Descrição do contrato:

```text
Deletes an overlay and all its revisions; conflict while a package pins it; needs overlays.operations delete
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "OverlayIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Acknowledgement of a Cloud delete (`{\"id\", \"deleted\": true}`).",
  "properties": {
    "deleted": {
      "type": "boolean"
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "deleted"
  ],
  "title": "CloudDeleted",
  "type": "object"
}
```

:::

### bundles.overlays.list {#rpc-bundles-overlays-list}

Descrição do contrato:

```text
Lists the owner's overlays without their documents; needs overlays.operations list
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Paging for the owner's assets and overlays. Cursors come only from the\nprevious page's `nextCursor`.",
  "properties": {
    "cursor": {
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "description": "1 to 50; the Cloud default (50) when absent.",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "CloudPageQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "OverlaySummary": {
      "description": "Overlay list entry. It never carries the document.",
      "properties": {
        "createdAt": {
          "type": "string"
        },
        "elementCount": {
          "format": "int64",
          "type": "integer"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "type": "string"
        },
        "name": {
          "description": "The document title (the overlay schema has no separate name).",
          "type": "string"
        },
        "revision": {
          "description": "Current revision.",
          "format": "int64",
          "type": "integer"
        },
        "status": {
          "type": "string"
        },
        "updatedAt": {
          "description": "When the current revision was written.",
          "type": "string"
        }
      },
      "required": [
        "id",
        "kind",
        "name",
        "revision",
        "status",
        "createdAt",
        "updatedAt",
        "elementCount"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/OverlaySummary"
      },
      "type": "array"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "items"
  ],
  "title": "OverlayPage",
  "type": "object"
}
```

:::

### bundles.overlays.read {#rpc-bundles-overlays-read}

Descrição do contrato:

```text
Reads one owned overlay with its document
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "OverlayIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "document": true,
    "id": {
      "type": "string"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "schemaVersion": {
      "format": "int64",
      "type": "integer"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "revision",
    "status",
    "schemaVersion",
    "document"
  ],
  "title": "OverlayRecord",
  "type": "object"
}
```

:::

### bundles.overlays.revise {#rpc-bundles-overlays-revise}

Descrição do contrato:

```text
Publishes a new overlay revision; requires the current revision
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "document": true,
    "expectedRevision": {
      "format": "int64",
      "type": "integer"
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "document",
    "expectedRevision"
  ],
  "title": "OverlayReviseParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "revision",
    "status"
  ],
  "title": "OverlayWriteResult",
  "type": "object"
}
```

:::

### bundles.overlays.revision {#rpc-bundles-overlays-revision}

Descrição do contrato:

```text
Reads one stored overlay revision with its document; needs overlays.operations revision
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    },
    "revision": {
      "description": "Positive revision number.",
      "format": "int64",
      "type": "integer"
    }
  },
  "required": [
    "id",
    "revision"
  ],
  "title": "OverlayRevisionParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "One stored overlay revision with its document.",
  "properties": {
    "createdAt": {
      "type": "string"
    },
    "current": {
      "type": "boolean"
    },
    "document": true,
    "id": {
      "type": "string"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "schemaVersion": {
      "format": "int64",
      "type": "integer"
    },
    "sha256": {
      "type": "string"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "revision",
    "status",
    "schemaVersion",
    "sha256",
    "createdAt",
    "current",
    "document"
  ],
  "title": "OverlayRevisionRecord",
  "type": "object"
}
```

:::

### bundles.overlays.revisions {#rpc-bundles-overlays-revisions}

Descrição do contrato:

```text
Lists the revisions of one overlay, newest first; needs overlays.operations revisions
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "cursor": {
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "type": "string"
    },
    "limit": {
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "OverlayRevisionsQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "OverlayRevisionSummary": {
      "properties": {
        "createdAt": {
          "type": "string"
        },
        "current": {
          "type": "boolean"
        },
        "name": {
          "type": "string"
        },
        "revision": {
          "format": "int64",
          "type": "integer"
        },
        "sha256": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "revision",
        "status",
        "name",
        "sha256",
        "createdAt",
        "current"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "currentRevision": {
      "format": "int64",
      "type": "integer"
    },
    "id": {
      "type": "string"
    },
    "items": {
      "items": {
        "$ref": "#/$defs/OverlayRevisionSummary"
      },
      "type": "array"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id",
    "currentRevision",
    "items"
  ],
  "title": "OverlayRevisionPage",
  "type": "object"
}
```

:::

### bundles.packages.create {#rpc-bundles-packages-create}

Descrição do contrato:

```text
Creates a package that pins plugin versions and document revisions
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "PackageItemParams": {
      "additionalProperties": false,
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "description": "`plugin`, `template`, `profile` or `overlay`.",
          "type": "string"
        },
        "required": {
          "type": "boolean"
        },
        "revision": {
          "description": "Required for templates, profiles and overlays: the pinned revision.",
          "format": "int64",
          "type": [
            "integer",
            "null"
          ]
        },
        "version": {
          "description": "Required for plugins: the pinned semantic version.",
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "kind",
        "id",
        "required"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/PackageItemParams"
      },
      "type": "array"
    },
    "title": {
      "type": "string"
    }
  },
  "required": [
    "title",
    "items"
  ],
  "title": "PackageCreateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "packageVersion": {
      "format": "int64",
      "type": "integer"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "revision",
    "status",
    "packageVersion"
  ],
  "title": "PackageCreated",
  "type": "object"
}
```

:::

### bundles.packages.delete {#rpc-bundles-packages-delete}

Descrição do contrato:

```text
Deletes a package, which releases the overlays it pinned; needs packages.operations delete
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "PackageIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Acknowledgement of a Cloud delete (`{\"id\", \"deleted\": true}`).",
  "properties": {
    "deleted": {
      "type": "boolean"
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "deleted"
  ],
  "title": "CloudDeleted",
  "type": "object"
}
```

:::

### bundles.packages.preflight {#rpc-bundles-packages-preflight}

Descrição do contrato:

```text
Reports what applying a package needs; applies nothing
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "PackageIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "PreflightItem": {
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "type": "string"
        },
        "permissions": {
          "description": "`reviewRequired` for plugins (the host consent step shows them) or `none`.",
          "type": "string"
        },
        "required": {
          "type": "boolean"
        },
        "revision": {
          "format": "int64",
          "type": [
            "integer",
            "null"
          ]
        },
        "status": {
          "description": "`ready`, `missing`, `notInstallable`, `revisionChanged` or `noActiveGeneration`.",
          "type": "string"
        },
        "version": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "kind",
        "id",
        "required",
        "status",
        "permissions"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "applyMode": {
      "type": "string"
    },
    "blocking": {
      "items": true,
      "type": "array"
    },
    "canApply": {
      "type": "boolean"
    },
    "items": {
      "items": {
        "$ref": "#/$defs/PreflightItem"
      },
      "type": "array"
    },
    "packageVersion": {
      "format": "int64",
      "type": "integer"
    }
  },
  "required": [
    "packageVersion",
    "items",
    "blocking",
    "canApply",
    "applyMode"
  ],
  "title": "PackagePreflight",
  "type": "object"
}
```

:::

### bundles.packages.read {#rpc-bundles-packages-read}

Descrição do contrato:

```text
Reads one owned package
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "PackageIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "document": true,
    "id": {
      "type": "string"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    }
  },
  "required": [
    "id",
    "revision",
    "document"
  ],
  "title": "PackageRecord",
  "type": "object"
}
```

:::

### catalog.fork {#rpc-catalog-fork}

Descrição do contrato:

```text
Copies a public template or profile into the signed-in owner's library as a private draft; needs library.fork
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Fork a document that is visible in the public catalog.",
  "properties": {
    "id": {
      "description": "The public document id.",
      "type": "string"
    },
    "revision": {
      "description": "A revision that is still published; the current one when absent.",
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "CatalogForkParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CatalogForkOrigin": {
      "description": "Where a forked document came from (row metadata, not part of the document).",
      "properties": {
        "documentId": {
          "type": "string"
        },
        "revision": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "documentId",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A private draft created by forking a public template or profile.",
  "properties": {
    "forkOf": {
      "$ref": "#/$defs/CatalogForkOrigin"
    },
    "id": {
      "type": "string"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "revision",
    "status",
    "forkOf"
  ],
  "title": "CatalogForkResult",
  "type": "object"
}
```

:::

### catalog.item {#rpc-catalog-item}

Descrição do contrato:

```text
Reads one unified catalog item; installation still requires the signed release
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    },
    "kind": {
      "type": "string"
    },
    "target": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "kind",
    "id"
  ],
  "title": "CloudCatalogItemQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### catalog.items {#rpc-catalog-items}

Descrição do contrato:

```text
Reads one bounded page of the unified catalog (plugins, templates, profiles); presentation only
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Unified catalog query (v3). `kind` is restricted to the three served kinds.",
  "properties": {
    "cursor": {
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "query": {
      "default": "",
      "type": "string"
    },
    "target": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "CloudCatalogItemsQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "dataState": {
      "description": "`ready` or `noActiveGeneration` (no plugin generation is published yet,\nwhich is an empty registry and not an outage). Absent when the Cloud\ndoes not announce it or announces a value this host does not know.",
      "type": [
        "string",
        "null"
      ]
    },
    "items": {
      "items": true,
      "type": "array"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "items"
  ],
  "title": "CloudCatalogItemPage",
  "type": "object"
}
```

:::

### catalog.mine {#rpc-catalog-mine}

Descrição do contrato:

```text
Lists the signed-in owner's templates and profiles in any status (Míos)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "cursor": {
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "description": "`template` or `profile`. Plugins are not library documents.",
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "query": {
      "default": "",
      "type": "string"
    }
  },
  "title": "LibraryItemsQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "description": "Same item shape as the public catalog, in any publication status.",
      "items": true,
      "type": "array"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "items"
  ],
  "title": "LibraryItemPage",
  "type": "object"
}
```

:::

### cloud.login.begin {#rpc-cloud-login-begin}

Descrição do contrato:

```text
Starts the system-browser login and returns the URL to open
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "operationId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "CloudLoginOperationParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "browserUrl": {
      "type": "string"
    }
  },
  "required": [
    "browserUrl"
  ],
  "title": "CloudLoginUrlResult",
  "type": "object"
}
```

:::

### cloud.login.cancel {#rpc-cloud-login-cancel}

Descrição do contrato:

```text
Aborts an in-progress login and drops its secrets
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "operationId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "CloudLoginOperationParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AccountState": {
      "enum": [
        "unconfigured",
        "signed-out",
        "signing-in",
        "signed-in"
      ],
      "type": "string"
    },
    "Persistence": {
      "oneOf": [
        {
          "const": "keychain",
          "description": "Tokens persist across restarts in the OS credential store.",
          "type": "string"
        },
        {
          "const": "memory-only",
          "description": "Tokens live in process memory only; a restart signs out. This is\nreported, never a silent fallback to a file.",
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the frontend may show. No tokens, ever.",
  "properties": {
    "accountId": {
      "type": [
        "string",
        "null"
      ]
    },
    "displayName": {
      "description": "Verified display name from `/me`, when the provider supplies one.",
      "type": [
        "string",
        "null"
      ]
    },
    "email": {
      "description": "Verified account email from `/me`, when the sign-in resolved it.\nBest-effort: a signed-in status without an email is still valid.",
      "type": [
        "string",
        "null"
      ]
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "loginOperationId": {
      "description": "Opaque identifier for the currently observed login attempt. It binds\ncancellation to that attempt and is not a bearer/session credential.",
      "type": [
        "string",
        "null"
      ]
    },
    "origin": {
      "type": [
        "string",
        "null"
      ]
    },
    "persistence": {
      "$ref": "#/$defs/Persistence"
    },
    "state": {
      "$ref": "#/$defs/AccountState"
    }
  },
  "required": [
    "state",
    "persistence"
  ],
  "title": "CloudStatusResult",
  "type": "object"
}
```

:::

### cloud.login.complete {#rpc-cloud-login-complete}

Descrição do contrato:

```text
Waits for the browser callback, redeems it, and signs in
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "operationId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "CloudLoginOperationParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AccountState": {
      "enum": [
        "unconfigured",
        "signed-out",
        "signing-in",
        "signed-in"
      ],
      "type": "string"
    },
    "Persistence": {
      "oneOf": [
        {
          "const": "keychain",
          "description": "Tokens persist across restarts in the OS credential store.",
          "type": "string"
        },
        {
          "const": "memory-only",
          "description": "Tokens live in process memory only; a restart signs out. This is\nreported, never a silent fallback to a file.",
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the frontend may show. No tokens, ever.",
  "properties": {
    "accountId": {
      "type": [
        "string",
        "null"
      ]
    },
    "displayName": {
      "description": "Verified display name from `/me`, when the provider supplies one.",
      "type": [
        "string",
        "null"
      ]
    },
    "email": {
      "description": "Verified account email from `/me`, when the sign-in resolved it.\nBest-effort: a signed-in status without an email is still valid.",
      "type": [
        "string",
        "null"
      ]
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "loginOperationId": {
      "description": "Opaque identifier for the currently observed login attempt. It binds\ncancellation to that attempt and is not a bearer/session credential.",
      "type": [
        "string",
        "null"
      ]
    },
    "origin": {
      "type": [
        "string",
        "null"
      ]
    },
    "persistence": {
      "$ref": "#/$defs/Persistence"
    },
    "state": {
      "$ref": "#/$defs/AccountState"
    }
  },
  "required": [
    "state",
    "persistence"
  ],
  "title": "CloudStatusResult",
  "type": "object"
}
```

:::

### cloud.login.open {#rpc-cloud-login-open}

Descrição do contrato:

```text
Opens the pending login URL in the system browser
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "operationId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "CloudLoginOperationParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### cloud.login.reserve {#rpc-cloud-login-reserve}

Descrição do contrato:

```text
Reserves a correlated login operation without starting a callback
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "operationId": {
      "type": "string"
    }
  },
  "required": [
    "operationId"
  ],
  "title": "CloudLoginReservationResult",
  "type": "object"
}
```

:::

### cloud.logout {#rpc-cloud-logout}

Descrição do contrato:

```text
Revokes the server session and wipes local credentials
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AccountState": {
      "enum": [
        "unconfigured",
        "signed-out",
        "signing-in",
        "signed-in"
      ],
      "type": "string"
    },
    "Persistence": {
      "oneOf": [
        {
          "const": "keychain",
          "description": "Tokens persist across restarts in the OS credential store.",
          "type": "string"
        },
        {
          "const": "memory-only",
          "description": "Tokens live in process memory only; a restart signs out. This is\nreported, never a silent fallback to a file.",
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the frontend may show. No tokens, ever.",
  "properties": {
    "accountId": {
      "type": [
        "string",
        "null"
      ]
    },
    "displayName": {
      "description": "Verified display name from `/me`, when the provider supplies one.",
      "type": [
        "string",
        "null"
      ]
    },
    "email": {
      "description": "Verified account email from `/me`, when the sign-in resolved it.\nBest-effort: a signed-in status without an email is still valid.",
      "type": [
        "string",
        "null"
      ]
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "loginOperationId": {
      "description": "Opaque identifier for the currently observed login attempt. It binds\ncancellation to that attempt and is not a bearer/session credential.",
      "type": [
        "string",
        "null"
      ]
    },
    "origin": {
      "type": [
        "string",
        "null"
      ]
    },
    "persistence": {
      "$ref": "#/$defs/Persistence"
    },
    "state": {
      "$ref": "#/$defs/AccountState"
    }
  },
  "required": [
    "state",
    "persistence"
  ],
  "title": "CloudStatusResult",
  "type": "object"
}
```

:::

### cloud.restore {#rpc-cloud-restore}

Descrição do contrato:

```text
Reconnects from the OS credential store after a restart
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AccountState": {
      "enum": [
        "unconfigured",
        "signed-out",
        "signing-in",
        "signed-in"
      ],
      "type": "string"
    },
    "Persistence": {
      "oneOf": [
        {
          "const": "keychain",
          "description": "Tokens persist across restarts in the OS credential store.",
          "type": "string"
        },
        {
          "const": "memory-only",
          "description": "Tokens live in process memory only; a restart signs out. This is\nreported, never a silent fallback to a file.",
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the frontend may show. No tokens, ever.",
  "properties": {
    "accountId": {
      "type": [
        "string",
        "null"
      ]
    },
    "displayName": {
      "description": "Verified display name from `/me`, when the provider supplies one.",
      "type": [
        "string",
        "null"
      ]
    },
    "email": {
      "description": "Verified account email from `/me`, when the sign-in resolved it.\nBest-effort: a signed-in status without an email is still valid.",
      "type": [
        "string",
        "null"
      ]
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "loginOperationId": {
      "description": "Opaque identifier for the currently observed login attempt. It binds\ncancellation to that attempt and is not a bearer/session credential.",
      "type": [
        "string",
        "null"
      ]
    },
    "origin": {
      "type": [
        "string",
        "null"
      ]
    },
    "persistence": {
      "$ref": "#/$defs/Persistence"
    },
    "state": {
      "$ref": "#/$defs/AccountState"
    }
  },
  "required": [
    "state",
    "persistence"
  ],
  "title": "CloudStatusResult",
  "type": "object"
}
```

:::

### cloud.status {#rpc-cloud-status}

Descrição do contrato:

```text
Shows the Cloud account state without exposing any credentials
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AccountState": {
      "enum": [
        "unconfigured",
        "signed-out",
        "signing-in",
        "signed-in"
      ],
      "type": "string"
    },
    "Persistence": {
      "oneOf": [
        {
          "const": "keychain",
          "description": "Tokens persist across restarts in the OS credential store.",
          "type": "string"
        },
        {
          "const": "memory-only",
          "description": "Tokens live in process memory only; a restart signs out. This is\nreported, never a silent fallback to a file.",
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the frontend may show. No tokens, ever.",
  "properties": {
    "accountId": {
      "type": [
        "string",
        "null"
      ]
    },
    "displayName": {
      "description": "Verified display name from `/me`, when the provider supplies one.",
      "type": [
        "string",
        "null"
      ]
    },
    "email": {
      "description": "Verified account email from `/me`, when the sign-in resolved it.\nBest-effort: a signed-in status without an email is still valid.",
      "type": [
        "string",
        "null"
      ]
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "loginOperationId": {
      "description": "Opaque identifier for the currently observed login attempt. It binds\ncancellation to that attempt and is not a bearer/session credential.",
      "type": [
        "string",
        "null"
      ]
    },
    "origin": {
      "type": [
        "string",
        "null"
      ]
    },
    "persistence": {
      "$ref": "#/$defs/Persistence"
    },
    "state": {
      "$ref": "#/$defs/AccountState"
    }
  },
  "required": [
    "state",
    "persistence"
  ],
  "title": "CloudStatusResult",
  "type": "object"
}
```

:::

### cloudLibrary.list {#rpc-cloudLibrary-list}

Descrição do contrato:

```text
Lists published or private Cloud documents without importing them
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "CloudLibraryKind": {
      "enum": [
        "template",
        "profile"
      ],
      "type": "string"
    },
    "CloudLibraryScope": {
      "enum": [
        "public",
        "mine"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "after": {
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "anyOf": [
        {
          "$ref": "#/$defs/CloudLibraryKind"
        },
        {
          "type": "null"
        }
      ]
    },
    "query": {
      "type": [
        "string",
        "null"
      ]
    },
    "scope": {
      "$ref": "#/$defs/CloudLibraryScope"
    }
  },
  "required": [
    "scope"
  ],
  "title": "CloudLibraryQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CloudLibraryDocumentMeta": {
      "additionalProperties": false,
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/CloudLibraryKind"
        },
        "revision": {
          "description": "Exact decimal string; never a JavaScript number.",
          "type": "string"
        },
        "sha256": {
          "type": "string"
        },
        "slug": {
          "type": "string"
        },
        "status": {
          "$ref": "#/$defs/CloudLibraryStatus"
        },
        "title": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "kind",
        "slug",
        "revision",
        "title",
        "sha256",
        "status"
      ],
      "type": "object"
    },
    "CloudLibraryKind": {
      "enum": [
        "template",
        "profile"
      ],
      "type": "string"
    },
    "CloudLibraryStatus": {
      "enum": [
        "draft",
        "pending",
        "published",
        "rejected",
        "withdrawn"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/CloudLibraryDocumentMeta"
      },
      "type": "array"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "items"
  ],
  "title": "CloudLibraryPage",
  "type": "object"
}
```

:::

### cloudLibrary.read {#rpc-cloudLibrary-read}

Descrição do contrato:

```text
Reads a Cloud document revision without importing it
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "CloudLibraryScope": {
      "enum": [
        "public",
        "mine"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    },
    "revision": {
      "type": [
        "string",
        "null"
      ]
    },
    "scope": {
      "$ref": "#/$defs/CloudLibraryScope"
    }
  },
  "required": [
    "scope",
    "id"
  ],
  "title": "CloudLibraryRead",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CloudLibraryDocumentMeta": {
      "additionalProperties": false,
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/CloudLibraryKind"
        },
        "revision": {
          "description": "Exact decimal string; never a JavaScript number.",
          "type": "string"
        },
        "sha256": {
          "type": "string"
        },
        "slug": {
          "type": "string"
        },
        "status": {
          "$ref": "#/$defs/CloudLibraryStatus"
        },
        "title": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "kind",
        "slug",
        "revision",
        "title",
        "sha256",
        "status"
      ],
      "type": "object"
    },
    "CloudLibraryKind": {
      "enum": [
        "template",
        "profile"
      ],
      "type": "string"
    },
    "CloudLibraryStatus": {
      "enum": [
        "draft",
        "pending",
        "published",
        "rejected",
        "withdrawn"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "document": true,
    "meta": {
      "$ref": "#/$defs/CloudLibraryDocumentMeta"
    }
  },
  "required": [
    "meta",
    "document"
  ],
  "title": "CloudLibraryDocument",
  "type": "object"
}
```

:::

### collaboration.audit.list {#rpc-collaboration-audit-list}

Descrição do contrato:

```text
Returns a bounded redacted collaboration audit page
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "after": {
      "default": 0,
      "format": "int64",
      "type": "integer"
    },
    "limit": {
      "default": null,
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "AuditQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AuditEntry": {
      "properties": {
        "action": {
          "type": "string"
        },
        "cursor": {
          "format": "int64",
          "type": "integer"
        },
        "details": {
          "anyOf": [
            {
              "$ref": "#/$defs/AuditRecord"
            },
            {
              "type": "null"
            }
          ]
        },
        "outcome": {
          "type": "string"
        },
        "principalId": {
          "type": "string"
        },
        "resourceId": {
          "type": "string"
        },
        "timestamp": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "cursor",
        "timestamp",
        "principalId",
        "action",
        "resourceId",
        "outcome"
      ],
      "type": "object"
    },
    "AuditRecord": {
      "additionalProperties": false,
      "properties": {
        "afterRevision": {
          "type": "string"
        },
        "beforeRevision": {
          "type": "string"
        },
        "commandId": {
          "type": [
            "string",
            "null"
          ]
        },
        "correlationId": {
          "type": "string"
        },
        "eventId": {
          "type": "string"
        },
        "method": {
          "type": "string"
        },
        "principalId": {
          "type": "string"
        },
        "resource": {
          "$ref": "#/$defs/CommandResource"
        },
        "resultCode": {
          "type": "string"
        },
        "sessionId": {
          "type": "string"
        },
        "timestamp": {
          "format": "int64",
          "type": "integer"
        }
      },
      "required": [
        "eventId",
        "timestamp",
        "principalId",
        "sessionId",
        "method",
        "resource",
        "beforeRevision",
        "afterRevision",
        "resultCode",
        "correlationId"
      ],
      "type": "object"
    },
    "CommandResource": {
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope"
      ],
      "type": "object"
    },
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "entries": {
      "items": {
        "$ref": "#/$defs/AuditEntry"
      },
      "type": "array"
    },
    "nextAfter": {
      "format": "int64",
      "type": "integer"
    }
  },
  "required": [
    "entries",
    "nextAfter"
  ],
  "title": "AuditResult",
  "type": "object"
}
```

:::

### collaboration.commands.get {#rpc-collaboration-commands-get}

Descrição do contrato:

```text
Retrieves an authorized retained command outcome without executing it
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "commandId": {
      "type": "string"
    },
    "principalId": {
      "default": null,
      "description": "Only the local owner can select another principal.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "commandId"
  ],
  "title": "CommandGet",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CommandReceipt": {
      "properties": {
        "commandId": {
          "type": "string"
        },
        "committedAt": {
          "format": "int64",
          "type": [
            "integer",
            "null"
          ]
        },
        "resource": {
          "anyOf": [
            {
              "$ref": "#/$defs/CommandResource"
            },
            {
              "type": "null"
            }
          ]
        },
        "state": {
          "$ref": "#/$defs/CommandState"
        }
      },
      "required": [
        "commandId",
        "state"
      ],
      "type": "object"
    },
    "CommandResource": {
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope"
      ],
      "type": "object"
    },
    "CommandState": {
      "enum": [
        "accepted",
        "running",
        "succeeded",
        "failed",
        "unknown"
      ],
      "type": "string"
    },
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "command": {
      "$ref": "#/$defs/CommandReceipt"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "method": {
      "type": "string"
    },
    "principalId": {
      "type": "string"
    },
    "resourceVersions": {
      "items": {
        "$ref": "#/$defs/ResourceVersion"
      },
      "type": "array"
    },
    "result": true
  },
  "required": [
    "principalId",
    "method",
    "hostInstanceId",
    "command",
    "result",
    "resourceVersions"
  ],
  "title": "CommandOutcome",
  "type": "object"
}
```

:::

### collaboration.configure {#rpc-collaboration-configure}

Descrição do contrato:

```text
Configures or disables authenticated remote collaboration
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "PeerConfiguration": {
      "additionalProperties": false,
      "properties": {
        "relayUrls": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "RemoteLimits": {
      "additionalProperties": false,
      "properties": {
        "commandIdsPerPrincipal": {
          "default": 100000,
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "connections": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "connectionsPerPrincipal": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "frameBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "paramsBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "pendingCalls": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "requestBurst": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "requestsPerSecond": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "connections",
        "connectionsPerPrincipal",
        "frameBytes",
        "paramsBytes",
        "pendingCalls",
        "requestsPerSecond",
        "requestBurst"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Optional owner configuration CAS, kept separate from persisted settings.\nLegacy callers can keep sending the original bare Configuration object.",
  "properties": {
    "advertisedOrigin": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "allowedOrigins": {
      "default": [],
      "items": {
        "type": "string"
      },
      "type": "array",
      "uniqueItems": true
    },
    "bindAddress": {
      "default": "127.0.0.1:8788",
      "type": "string"
    },
    "enabled": {
      "type": "boolean"
    },
    "expectedRevision": {
      "type": [
        "string",
        "null"
      ]
    },
    "limits": {
      "$ref": "#/$defs/RemoteLimits",
      "default": {
        "commandIdsPerPrincipal": 100000,
        "connections": 8,
        "connectionsPerPrincipal": 4,
        "frameBytes": 1048576,
        "paramsBytes": 262144,
        "pendingCalls": 32,
        "requestBurst": 60,
        "requestsPerSecond": 30
      }
    },
    "peer": {
      "anyOf": [
        {
          "$ref": "#/$defs/PeerConfiguration"
        },
        {
          "type": "null"
        }
      ]
    },
    "tlsCertificate": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "tlsPrivateKey": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "tlsProxy": {
      "default": false,
      "type": "boolean"
    }
  },
  "required": [
    "enabled"
  ],
  "title": "ConfigureInput",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Configuration": {
      "additionalProperties": false,
      "properties": {
        "advertisedOrigin": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "allowedOrigins": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "bindAddress": {
          "default": "127.0.0.1:8788",
          "type": "string"
        },
        "enabled": {
          "type": "boolean"
        },
        "limits": {
          "$ref": "#/$defs/RemoteLimits",
          "default": {
            "commandIdsPerPrincipal": 100000,
            "connections": 8,
            "connectionsPerPrincipal": 4,
            "frameBytes": 1048576,
            "paramsBytes": 262144,
            "pendingCalls": 32,
            "requestBurst": 60,
            "requestsPerSecond": 30
          }
        },
        "peer": {
          "anyOf": [
            {
              "$ref": "#/$defs/PeerConfiguration"
            },
            {
              "type": "null"
            }
          ]
        },
        "tlsCertificate": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "tlsPrivateKey": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "tlsProxy": {
          "default": false,
          "type": "boolean"
        }
      },
      "required": [
        "enabled"
      ],
      "type": "object"
    },
    "GatewayStatus": {
      "properties": {
        "address": {
          "type": [
            "string",
            "null"
          ]
        },
        "available": {
          "type": "boolean"
        },
        "certificateFingerprint": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "configurationRevision": {
          "type": [
            "string",
            "null"
          ]
        },
        "connections": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "lastError": {
          "type": [
            "string",
            "null"
          ]
        },
        "running": {
          "type": "boolean"
        },
        "sessionConnections": {
          "additionalProperties": {
            "format": "uint32",
            "minimum": 0,
            "type": "integer"
          },
          "default": {},
          "type": "object"
        }
      },
      "required": [
        "available",
        "running",
        "connections"
      ],
      "type": "object"
    },
    "PeerConfiguration": {
      "additionalProperties": false,
      "properties": {
        "relayUrls": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "PeerGatewayStatus": {
      "properties": {
        "address": {
          "type": [
            "string",
            "null"
          ]
        },
        "addresses": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "available": {
          "type": "boolean"
        },
        "certificateFingerprint": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "configurationRevision": {
          "type": [
            "string",
            "null"
          ]
        },
        "connections": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "endpointId": {
          "type": [
            "string",
            "null"
          ]
        },
        "lastError": {
          "type": [
            "string",
            "null"
          ]
        },
        "relayUrls": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "running": {
          "type": "boolean"
        },
        "sessionConnections": {
          "additionalProperties": {
            "format": "uint32",
            "minimum": 0,
            "type": "integer"
          },
          "default": {},
          "type": "object"
        }
      },
      "required": [
        "available",
        "running",
        "connections",
        "addresses",
        "relayUrls"
      ],
      "type": "object"
    },
    "RemoteLimits": {
      "additionalProperties": false,
      "properties": {
        "commandIdsPerPrincipal": {
          "default": 100000,
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "connections": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "connectionsPerPrincipal": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "frameBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "paramsBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "pendingCalls": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "requestBurst": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "requestsPerSecond": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "connections",
        "connectionsPerPrincipal",
        "frameBytes",
        "paramsBytes",
        "pendingCalls",
        "requestsPerSecond",
        "requestBurst"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "activeSessions": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "configuration": {
      "$ref": "#/$defs/Configuration"
    },
    "configurationRevision": {
      "type": "string"
    },
    "gateway": {
      "$ref": "#/$defs/GatewayStatus"
    },
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "peerGateway": {
      "$ref": "#/$defs/PeerGatewayStatus",
      "default": {
        "address": null,
        "addresses": [],
        "available": false,
        "certificateFingerprint": null,
        "configurationRevision": null,
        "connections": 0,
        "endpointId": null,
        "lastError": null,
        "relayUrls": [],
        "running": false,
        "sessionConnections": {}
      }
    },
    "pendingRequests": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "configuration",
    "configurationRevision",
    "activeSessions",
    "pendingRequests",
    "gateway"
  ],
  "title": "Status",
  "type": "object"
}
```

:::

### collaboration.grants.set {#rpc-collaboration-grants-set}

Descrição do contrato:

```text
Updates an identity grant with an atomic revision precondition
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AudioEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "fileRefs": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxVolume": {
          "format": "double",
          "type": "number"
        },
        "output": {
          "type": "string"
        },
        "overlaps": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "fileRefs",
        "output",
        "maxVolume",
        "overlaps"
      ],
      "type": "object"
    },
    "AutomationDelegation": {
      "additionalProperties": false,
      "properties": {
        "allowDynamicTargets": {
          "type": "boolean"
        },
        "allowOtherAuthorEdits": {
          "description": "Allow scoped definitions authored by another moderator to be replaced.",
          "type": "boolean"
        },
        "allowOtherAuthorSource": {
          "description": "Share full source of scoped definitions authored by other moderators.",
          "type": "boolean"
        },
        "allowOwnerEdits": {
          "description": "Explicitly allow scoped changes to existing Owner definitions.",
          "type": "boolean"
        },
        "allowOwnerSource": {
          "description": "Explicitly share Owner source, which can include sensitive values.",
          "type": "boolean"
        },
        "effects": {
          "items": {
            "$ref": "#/$defs/DelegatedAutomationEffect"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "effects",
        "allowDynamicTargets"
      ],
      "type": "object"
    },
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "DelegatedAutomationEffect": {
      "description": "Explicit policy for future reusable delegated automation execution. This\nis not itself an execution permit: the host must validate the saved closure\nand current subjects before it may be used at runtime.",
      "enum": [
        "log",
        "delay",
        "points",
        "http",
        "audio",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectGrants": {
      "additionalProperties": false,
      "properties": {
        "audio": {
          "default": [],
          "items": {
            "$ref": "#/$defs/AudioEffectGrant"
          },
          "type": "array"
        },
        "emitTopics": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "http": {
          "default": [],
          "items": {
            "$ref": "#/$defs/HttpEffectGrant"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "Grant": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        },
        "automationDelegation": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationDelegation"
            },
            {
              "type": "null"
            }
          ],
          "description": "Missing on legacy grants; reusable execution is bounded by this policy."
        },
        "capabilities": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Capability"
          },
          "type": "array",
          "uniqueItems": true
        },
        "effects": {
          "$ref": "#/$defs/EffectGrants",
          "description": "External effects deny by default. Unlike legacy resource restrictions,\na missing or empty policy never means unrestricted access."
        },
        "maxPointsDelta": {
          "default": null,
          "format": "double",
          "type": [
            "number",
            "null"
          ]
        },
        "resources": {
          "additionalProperties": false,
          "default": {},
          "description": "Missing kind means unrestricted within a capability. An empty set\nexplicitly denies that resource kind; IDs never contain wildcards.",
          "properties": {
            "automation": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "competition": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "live": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "plugin": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "ranking": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "viewer": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "widget": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "workflow": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            }
          },
          "type": "object"
        },
        "settingsFields": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        }
      },
      "type": "object"
    },
    "HttpEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "allowPrivateNetwork": {
          "type": "boolean"
        },
        "allowTextBody": {
          "type": "boolean"
        },
        "bodyFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "headerNames": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxBodyBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "maxResponseBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "methods": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "origin": {
          "type": "string"
        },
        "paths": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "queryFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "renderFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "origin",
        "methods",
        "paths",
        "queryFields",
        "headerNames",
        "renderFields",
        "bodyFields",
        "allowTextBody",
        "maxBodyBytes",
        "maxResponseBytes",
        "allowPrivateNetwork"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedRevision": {
      "type": "string"
    },
    "grant": {
      "$ref": "#/$defs/Grant"
    },
    "principalId": {
      "type": "string"
    }
  },
  "required": [
    "principalId",
    "expectedRevision",
    "grant"
  ],
  "title": "GrantSet",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### collaboration.hello {#rpc-collaboration-hello}

Descrição do contrato:

```text
Negotiates the authenticated remote protocol
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "ClientKind": {
      "enum": [
        "desktop",
        "browser",
        "agent"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "clientKind": {
      "$ref": "#/$defs/ClientKind"
    },
    "clientVersion": {
      "type": "string"
    },
    "protocolVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "protocolVersion",
    "clientVersion",
    "clientKind"
  ],
  "title": "HelloParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "RemoteLimits": {
      "additionalProperties": false,
      "properties": {
        "commandIdsPerPrincipal": {
          "default": 100000,
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "connections": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "connectionsPerPrincipal": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "frameBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "paramsBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "pendingCalls": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "requestBurst": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "requestsPerSecond": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "connections",
        "connectionsPerPrincipal",
        "frameBytes",
        "paramsBytes",
        "pendingCalls",
        "requestsPerSecond",
        "requestBurst"
      ],
      "type": "object"
    },
    "StreamDomain": {
      "enum": [
        "live",
        "points",
        "rankings",
        "competitions",
        "workflows",
        "automation",
        "plugins",
        "widgets",
        "commands"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "capabilities": {
      "items": {
        "$ref": "#/$defs/Capability"
      },
      "type": "array",
      "uniqueItems": true
    },
    "connectionId": {
      "type": "string"
    },
    "features": {
      "additionalProperties": {
        "type": "boolean"
      },
      "type": "object"
    },
    "grantRevision": {
      "type": "string"
    },
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "limits": {
      "$ref": "#/$defs/RemoteLimits"
    },
    "principalId": {
      "type": "string"
    },
    "protocolVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "resourceRestrictions": {
      "additionalProperties": false,
      "properties": {
        "automation": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "competition": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "live": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "plugin": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "ranking": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "viewer": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "widget": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "workflow": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "type": "object"
    },
    "serverTime": {
      "format": "int64",
      "type": "integer"
    },
    "sessionId": {
      "type": "string"
    },
    "subscriptionDomains": {
      "default": [],
      "items": {
        "$ref": "#/$defs/StreamDomain"
      },
      "type": "array"
    }
  },
  "required": [
    "protocolVersion",
    "hostId",
    "hostInstanceId",
    "connectionId",
    "principalId",
    "sessionId",
    "grantRevision",
    "capabilities",
    "resourceRestrictions",
    "serverTime",
    "limits",
    "features"
  ],
  "title": "HelloResult",
  "type": "object"
}
```

:::

### collaboration.invites.create {#rpc-collaboration-invites-create}

Descrição do contrato:

```text
Creates a one-use owner-approved invitation; secret returned once
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AudioEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "fileRefs": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxVolume": {
          "format": "double",
          "type": "number"
        },
        "output": {
          "type": "string"
        },
        "overlaps": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "fileRefs",
        "output",
        "maxVolume",
        "overlaps"
      ],
      "type": "object"
    },
    "AutomationDelegation": {
      "additionalProperties": false,
      "properties": {
        "allowDynamicTargets": {
          "type": "boolean"
        },
        "allowOtherAuthorEdits": {
          "description": "Allow scoped definitions authored by another moderator to be replaced.",
          "type": "boolean"
        },
        "allowOtherAuthorSource": {
          "description": "Share full source of scoped definitions authored by other moderators.",
          "type": "boolean"
        },
        "allowOwnerEdits": {
          "description": "Explicitly allow scoped changes to existing Owner definitions.",
          "type": "boolean"
        },
        "allowOwnerSource": {
          "description": "Explicitly share Owner source, which can include sensitive values.",
          "type": "boolean"
        },
        "effects": {
          "items": {
            "$ref": "#/$defs/DelegatedAutomationEffect"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "effects",
        "allowDynamicTargets"
      ],
      "type": "object"
    },
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "DelegatedAutomationEffect": {
      "description": "Explicit policy for future reusable delegated automation execution. This\nis not itself an execution permit: the host must validate the saved closure\nand current subjects before it may be used at runtime.",
      "enum": [
        "log",
        "delay",
        "points",
        "http",
        "audio",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectGrants": {
      "additionalProperties": false,
      "properties": {
        "audio": {
          "default": [],
          "items": {
            "$ref": "#/$defs/AudioEffectGrant"
          },
          "type": "array"
        },
        "emitTopics": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "http": {
          "default": [],
          "items": {
            "$ref": "#/$defs/HttpEffectGrant"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "Grant": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        },
        "automationDelegation": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationDelegation"
            },
            {
              "type": "null"
            }
          ],
          "description": "Missing on legacy grants; reusable execution is bounded by this policy."
        },
        "capabilities": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Capability"
          },
          "type": "array",
          "uniqueItems": true
        },
        "effects": {
          "$ref": "#/$defs/EffectGrants",
          "description": "External effects deny by default. Unlike legacy resource restrictions,\na missing or empty policy never means unrestricted access."
        },
        "maxPointsDelta": {
          "default": null,
          "format": "double",
          "type": [
            "number",
            "null"
          ]
        },
        "resources": {
          "additionalProperties": false,
          "default": {},
          "description": "Missing kind means unrestricted within a capability. An empty set\nexplicitly denies that resource kind; IDs never contain wildcards.",
          "properties": {
            "automation": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "competition": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "live": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "plugin": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "ranking": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "viewer": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "widget": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "workflow": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            }
          },
          "type": "object"
        },
        "settingsFields": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        }
      },
      "type": "object"
    },
    "HttpEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "allowPrivateNetwork": {
          "type": "boolean"
        },
        "allowTextBody": {
          "type": "boolean"
        },
        "bodyFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "headerNames": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxBodyBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "maxResponseBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "methods": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "origin": {
          "type": "string"
        },
        "paths": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "queryFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "renderFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "origin",
        "methods",
        "paths",
        "queryFields",
        "headerNames",
        "renderFields",
        "bodyFields",
        "allowTextBody",
        "maxBodyBytes",
        "maxResponseBytes",
        "allowPrivateNetwork"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "grant": {
      "$ref": "#/$defs/Grant",
      "default": {
        "actions": {},
        "capabilities": [
          "live.read"
        ],
        "maxPointsDelta": null,
        "resources": {},
        "settingsFields": {}
      }
    }
  },
  "title": "InviteCreate",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Never implements Debug: the secret is returned only once to the owner.",
  "properties": {
    "expiresAt": {
      "format": "int64",
      "type": "integer"
    },
    "inviteId": {
      "type": "string"
    },
    "secret": {
      "type": "string"
    }
  },
  "required": [
    "inviteId",
    "secret",
    "expiresAt"
  ],
  "title": "InviteSecret",
  "type": "object"
}
```

:::

### collaboration.invites.list {#rpc-collaboration-invites-list}

Descrição do contrato:

```text
Returns a redacted invitation page
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Keyset pagination over immutable IDs, not a snapshot. Restart without a\ncursor to discover records created before an already-consumed cursor.",
  "properties": {
    "after": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "default": null,
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "ListQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AudioEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "fileRefs": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxVolume": {
          "format": "double",
          "type": "number"
        },
        "output": {
          "type": "string"
        },
        "overlaps": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "fileRefs",
        "output",
        "maxVolume",
        "overlaps"
      ],
      "type": "object"
    },
    "AutomationDelegation": {
      "additionalProperties": false,
      "properties": {
        "allowDynamicTargets": {
          "type": "boolean"
        },
        "allowOtherAuthorEdits": {
          "description": "Allow scoped definitions authored by another moderator to be replaced.",
          "type": "boolean"
        },
        "allowOtherAuthorSource": {
          "description": "Share full source of scoped definitions authored by other moderators.",
          "type": "boolean"
        },
        "allowOwnerEdits": {
          "description": "Explicitly allow scoped changes to existing Owner definitions.",
          "type": "boolean"
        },
        "allowOwnerSource": {
          "description": "Explicitly share Owner source, which can include sensitive values.",
          "type": "boolean"
        },
        "effects": {
          "items": {
            "$ref": "#/$defs/DelegatedAutomationEffect"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "effects",
        "allowDynamicTargets"
      ],
      "type": "object"
    },
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "DelegatedAutomationEffect": {
      "description": "Explicit policy for future reusable delegated automation execution. This\nis not itself an execution permit: the host must validate the saved closure\nand current subjects before it may be used at runtime.",
      "enum": [
        "log",
        "delay",
        "points",
        "http",
        "audio",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectGrants": {
      "additionalProperties": false,
      "properties": {
        "audio": {
          "default": [],
          "items": {
            "$ref": "#/$defs/AudioEffectGrant"
          },
          "type": "array"
        },
        "emitTopics": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "http": {
          "default": [],
          "items": {
            "$ref": "#/$defs/HttpEffectGrant"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "Grant": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        },
        "automationDelegation": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationDelegation"
            },
            {
              "type": "null"
            }
          ],
          "description": "Missing on legacy grants; reusable execution is bounded by this policy."
        },
        "capabilities": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Capability"
          },
          "type": "array",
          "uniqueItems": true
        },
        "effects": {
          "$ref": "#/$defs/EffectGrants",
          "description": "External effects deny by default. Unlike legacy resource restrictions,\na missing or empty policy never means unrestricted access."
        },
        "maxPointsDelta": {
          "default": null,
          "format": "double",
          "type": [
            "number",
            "null"
          ]
        },
        "resources": {
          "additionalProperties": false,
          "default": {},
          "description": "Missing kind means unrestricted within a capability. An empty set\nexplicitly denies that resource kind; IDs never contain wildcards.",
          "properties": {
            "automation": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "competition": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "live": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "plugin": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "ranking": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "viewer": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "widget": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "workflow": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            }
          },
          "type": "object"
        },
        "settingsFields": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        }
      },
      "type": "object"
    },
    "HttpEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "allowPrivateNetwork": {
          "type": "boolean"
        },
        "allowTextBody": {
          "type": "boolean"
        },
        "bodyFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "headerNames": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxBodyBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "maxResponseBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "methods": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "origin": {
          "type": "string"
        },
        "paths": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "queryFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "renderFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "origin",
        "methods",
        "paths",
        "queryFields",
        "headerNames",
        "renderFields",
        "bodyFields",
        "allowTextBody",
        "maxBodyBytes",
        "maxResponseBytes",
        "allowPrivateNetwork"
      ],
      "type": "object"
    },
    "Invitation": {
      "properties": {
        "expiresAt": {
          "format": "int64",
          "type": "integer"
        },
        "grant": {
          "$ref": "#/$defs/Grant"
        },
        "inviteId": {
          "type": "string"
        },
        "state": {
          "type": "string"
        }
      },
      "required": [
        "inviteId",
        "grant",
        "expiresAt",
        "state"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "invitations": {
      "items": {
        "$ref": "#/$defs/Invitation"
      },
      "type": "array"
    },
    "nextAfter": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "invitations"
  ],
  "title": "InvitationsResult",
  "type": "object"
}
```

:::

### collaboration.invites.revoke {#rpc-collaboration-invites-revoke}

Descrição do contrato:

```text
Revokes an invitation and its pending exchange
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "IdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### collaboration.me {#rpc-collaboration-me}

Descrição do contrato:

```text
Shows only the authenticated caller's effective identity and grants
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AudioEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "fileRefs": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxVolume": {
          "format": "double",
          "type": "number"
        },
        "output": {
          "type": "string"
        },
        "overlaps": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "fileRefs",
        "output",
        "maxVolume",
        "overlaps"
      ],
      "type": "object"
    },
    "AutomationDelegation": {
      "additionalProperties": false,
      "properties": {
        "allowDynamicTargets": {
          "type": "boolean"
        },
        "allowOtherAuthorEdits": {
          "description": "Allow scoped definitions authored by another moderator to be replaced.",
          "type": "boolean"
        },
        "allowOtherAuthorSource": {
          "description": "Share full source of scoped definitions authored by other moderators.",
          "type": "boolean"
        },
        "allowOwnerEdits": {
          "description": "Explicitly allow scoped changes to existing Owner definitions.",
          "type": "boolean"
        },
        "allowOwnerSource": {
          "description": "Explicitly share Owner source, which can include sensitive values.",
          "type": "boolean"
        },
        "effects": {
          "items": {
            "$ref": "#/$defs/DelegatedAutomationEffect"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "effects",
        "allowDynamicTargets"
      ],
      "type": "object"
    },
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "ClientKind": {
      "enum": [
        "desktop",
        "browser",
        "agent"
      ],
      "type": "string"
    },
    "DelegatedAutomationEffect": {
      "description": "Explicit policy for future reusable delegated automation execution. This\nis not itself an execution permit: the host must validate the saved closure\nand current subjects before it may be used at runtime.",
      "enum": [
        "log",
        "delay",
        "points",
        "http",
        "audio",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectGrants": {
      "additionalProperties": false,
      "properties": {
        "audio": {
          "default": [],
          "items": {
            "$ref": "#/$defs/AudioEffectGrant"
          },
          "type": "array"
        },
        "emitTopics": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "http": {
          "default": [],
          "items": {
            "$ref": "#/$defs/HttpEffectGrant"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "Grant": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        },
        "automationDelegation": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationDelegation"
            },
            {
              "type": "null"
            }
          ],
          "description": "Missing on legacy grants; reusable execution is bounded by this policy."
        },
        "capabilities": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Capability"
          },
          "type": "array",
          "uniqueItems": true
        },
        "effects": {
          "$ref": "#/$defs/EffectGrants",
          "description": "External effects deny by default. Unlike legacy resource restrictions,\na missing or empty policy never means unrestricted access."
        },
        "maxPointsDelta": {
          "default": null,
          "format": "double",
          "type": [
            "number",
            "null"
          ]
        },
        "resources": {
          "additionalProperties": false,
          "default": {},
          "description": "Missing kind means unrestricted within a capability. An empty set\nexplicitly denies that resource kind; IDs never contain wildcards.",
          "properties": {
            "automation": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "competition": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "live": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "plugin": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "ranking": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "viewer": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "widget": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "workflow": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            }
          },
          "type": "object"
        },
        "settingsFields": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        }
      },
      "type": "object"
    },
    "HttpEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "allowPrivateNetwork": {
          "type": "boolean"
        },
        "allowTextBody": {
          "type": "boolean"
        },
        "bodyFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "headerNames": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxBodyBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "maxResponseBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "methods": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "origin": {
          "type": "string"
        },
        "paths": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "queryFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "renderFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "origin",
        "methods",
        "paths",
        "queryFields",
        "headerNames",
        "renderFields",
        "bodyFields",
        "allowTextBody",
        "maxBodyBytes",
        "maxResponseBytes",
        "allowPrivateNetwork"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "accessExpiresAt": {
      "format": "int64",
      "type": "integer"
    },
    "clientKind": {
      "$ref": "#/$defs/ClientKind"
    },
    "grant": {
      "$ref": "#/$defs/Grant"
    },
    "grantRevision": {
      "type": "string"
    },
    "principalId": {
      "type": "string"
    },
    "sessionId": {
      "type": "string"
    }
  },
  "required": [
    "principalId",
    "sessionId",
    "grant",
    "grantRevision",
    "clientKind",
    "accessExpiresAt"
  ],
  "title": "Authenticated",
  "type": "object"
}
```

:::

### collaboration.principals.get {#rpc-collaboration-principals-get}

Descrição do contrato:

```text
Returns one owner-visible identity and its current permission revision
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "IdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AudioEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "fileRefs": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxVolume": {
          "format": "double",
          "type": "number"
        },
        "output": {
          "type": "string"
        },
        "overlaps": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "fileRefs",
        "output",
        "maxVolume",
        "overlaps"
      ],
      "type": "object"
    },
    "AutomationDelegation": {
      "additionalProperties": false,
      "properties": {
        "allowDynamicTargets": {
          "type": "boolean"
        },
        "allowOtherAuthorEdits": {
          "description": "Allow scoped definitions authored by another moderator to be replaced.",
          "type": "boolean"
        },
        "allowOtherAuthorSource": {
          "description": "Share full source of scoped definitions authored by other moderators.",
          "type": "boolean"
        },
        "allowOwnerEdits": {
          "description": "Explicitly allow scoped changes to existing Owner definitions.",
          "type": "boolean"
        },
        "allowOwnerSource": {
          "description": "Explicitly share Owner source, which can include sensitive values.",
          "type": "boolean"
        },
        "effects": {
          "items": {
            "$ref": "#/$defs/DelegatedAutomationEffect"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "effects",
        "allowDynamicTargets"
      ],
      "type": "object"
    },
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "ClientKind": {
      "enum": [
        "desktop",
        "browser",
        "agent"
      ],
      "type": "string"
    },
    "DelegatedAutomationEffect": {
      "description": "Explicit policy for future reusable delegated automation execution. This\nis not itself an execution permit: the host must validate the saved closure\nand current subjects before it may be used at runtime.",
      "enum": [
        "log",
        "delay",
        "points",
        "http",
        "audio",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectGrants": {
      "additionalProperties": false,
      "properties": {
        "audio": {
          "default": [],
          "items": {
            "$ref": "#/$defs/AudioEffectGrant"
          },
          "type": "array"
        },
        "emitTopics": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "http": {
          "default": [],
          "items": {
            "$ref": "#/$defs/HttpEffectGrant"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "Grant": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        },
        "automationDelegation": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationDelegation"
            },
            {
              "type": "null"
            }
          ],
          "description": "Missing on legacy grants; reusable execution is bounded by this policy."
        },
        "capabilities": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Capability"
          },
          "type": "array",
          "uniqueItems": true
        },
        "effects": {
          "$ref": "#/$defs/EffectGrants",
          "description": "External effects deny by default. Unlike legacy resource restrictions,\na missing or empty policy never means unrestricted access."
        },
        "maxPointsDelta": {
          "default": null,
          "format": "double",
          "type": [
            "number",
            "null"
          ]
        },
        "resources": {
          "additionalProperties": false,
          "default": {},
          "description": "Missing kind means unrestricted within a capability. An empty set\nexplicitly denies that resource kind; IDs never contain wildcards.",
          "properties": {
            "automation": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "competition": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "live": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "plugin": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "ranking": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "viewer": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "widget": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "workflow": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            }
          },
          "type": "object"
        },
        "settingsFields": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        }
      },
      "type": "object"
    },
    "HttpEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "allowPrivateNetwork": {
          "type": "boolean"
        },
        "allowTextBody": {
          "type": "boolean"
        },
        "bodyFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "headerNames": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxBodyBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "maxResponseBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "methods": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "origin": {
          "type": "string"
        },
        "paths": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "queryFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "renderFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "origin",
        "methods",
        "paths",
        "queryFields",
        "headerNames",
        "renderFields",
        "bodyFields",
        "allowTextBody",
        "maxBodyBytes",
        "maxResponseBytes",
        "allowPrivateNetwork"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "clientKind": {
      "$ref": "#/$defs/ClientKind"
    },
    "clientLabel": {
      "type": "string"
    },
    "grant": {
      "$ref": "#/$defs/Grant"
    },
    "grantRevision": {
      "type": "string"
    },
    "principalId": {
      "type": "string"
    },
    "revoked": {
      "type": "boolean"
    }
  },
  "required": [
    "principalId",
    "clientLabel",
    "clientKind",
    "grant",
    "grantRevision",
    "revoked"
  ],
  "title": "Principal",
  "type": "object"
}
```

:::

### collaboration.principals.list {#rpc-collaboration-principals-list}

Descrição do contrato:

```text
Returns approved identities and grants in a bounded page
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Keyset pagination over immutable IDs, not a snapshot. Restart without a\ncursor to discover records created before an already-consumed cursor.",
  "properties": {
    "after": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "default": null,
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "ListQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AudioEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "fileRefs": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxVolume": {
          "format": "double",
          "type": "number"
        },
        "output": {
          "type": "string"
        },
        "overlaps": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "fileRefs",
        "output",
        "maxVolume",
        "overlaps"
      ],
      "type": "object"
    },
    "AutomationDelegation": {
      "additionalProperties": false,
      "properties": {
        "allowDynamicTargets": {
          "type": "boolean"
        },
        "allowOtherAuthorEdits": {
          "description": "Allow scoped definitions authored by another moderator to be replaced.",
          "type": "boolean"
        },
        "allowOtherAuthorSource": {
          "description": "Share full source of scoped definitions authored by other moderators.",
          "type": "boolean"
        },
        "allowOwnerEdits": {
          "description": "Explicitly allow scoped changes to existing Owner definitions.",
          "type": "boolean"
        },
        "allowOwnerSource": {
          "description": "Explicitly share Owner source, which can include sensitive values.",
          "type": "boolean"
        },
        "effects": {
          "items": {
            "$ref": "#/$defs/DelegatedAutomationEffect"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "effects",
        "allowDynamicTargets"
      ],
      "type": "object"
    },
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "ClientKind": {
      "enum": [
        "desktop",
        "browser",
        "agent"
      ],
      "type": "string"
    },
    "DelegatedAutomationEffect": {
      "description": "Explicit policy for future reusable delegated automation execution. This\nis not itself an execution permit: the host must validate the saved closure\nand current subjects before it may be used at runtime.",
      "enum": [
        "log",
        "delay",
        "points",
        "http",
        "audio",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectGrants": {
      "additionalProperties": false,
      "properties": {
        "audio": {
          "default": [],
          "items": {
            "$ref": "#/$defs/AudioEffectGrant"
          },
          "type": "array"
        },
        "emitTopics": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "http": {
          "default": [],
          "items": {
            "$ref": "#/$defs/HttpEffectGrant"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "Grant": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        },
        "automationDelegation": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationDelegation"
            },
            {
              "type": "null"
            }
          ],
          "description": "Missing on legacy grants; reusable execution is bounded by this policy."
        },
        "capabilities": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Capability"
          },
          "type": "array",
          "uniqueItems": true
        },
        "effects": {
          "$ref": "#/$defs/EffectGrants",
          "description": "External effects deny by default. Unlike legacy resource restrictions,\na missing or empty policy never means unrestricted access."
        },
        "maxPointsDelta": {
          "default": null,
          "format": "double",
          "type": [
            "number",
            "null"
          ]
        },
        "resources": {
          "additionalProperties": false,
          "default": {},
          "description": "Missing kind means unrestricted within a capability. An empty set\nexplicitly denies that resource kind; IDs never contain wildcards.",
          "properties": {
            "automation": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "competition": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "live": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "plugin": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "ranking": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "viewer": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "widget": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "workflow": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            }
          },
          "type": "object"
        },
        "settingsFields": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        }
      },
      "type": "object"
    },
    "HttpEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "allowPrivateNetwork": {
          "type": "boolean"
        },
        "allowTextBody": {
          "type": "boolean"
        },
        "bodyFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "headerNames": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxBodyBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "maxResponseBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "methods": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "origin": {
          "type": "string"
        },
        "paths": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "queryFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "renderFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "origin",
        "methods",
        "paths",
        "queryFields",
        "headerNames",
        "renderFields",
        "bodyFields",
        "allowTextBody",
        "maxBodyBytes",
        "maxResponseBytes",
        "allowPrivateNetwork"
      ],
      "type": "object"
    },
    "Principal": {
      "properties": {
        "clientKind": {
          "$ref": "#/$defs/ClientKind"
        },
        "clientLabel": {
          "type": "string"
        },
        "grant": {
          "$ref": "#/$defs/Grant"
        },
        "grantRevision": {
          "type": "string"
        },
        "principalId": {
          "type": "string"
        },
        "revoked": {
          "type": "boolean"
        }
      },
      "required": [
        "principalId",
        "clientLabel",
        "clientKind",
        "grant",
        "grantRevision",
        "revoked"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "nextAfter": {
      "type": [
        "string",
        "null"
      ]
    },
    "principals": {
      "items": {
        "$ref": "#/$defs/Principal"
      },
      "type": "array"
    }
  },
  "required": [
    "principals"
  ],
  "title": "PrincipalsResult",
  "type": "object"
}
```

:::

### collaboration.principals.revoke {#rpc-collaboration-principals-revoke}

Descrição do contrato:

```text
Revokes an identity and all its sessions
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "IdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### collaboration.requests.approve {#rpc-collaboration-requests-approve}

Descrição do contrato:

```text
Approves a device with its invitation's maximum grant
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "IdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### collaboration.requests.list {#rpc-collaboration-requests-list}

Descrição do contrato:

```text
Returns devices awaiting owner approval in a bounded page
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Keyset pagination over immutable IDs, not a snapshot. Restart without a\ncursor to discover records created before an already-consumed cursor.",
  "properties": {
    "after": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "default": null,
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "ListQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AudioEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "fileRefs": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxVolume": {
          "format": "double",
          "type": "number"
        },
        "output": {
          "type": "string"
        },
        "overlaps": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "fileRefs",
        "output",
        "maxVolume",
        "overlaps"
      ],
      "type": "object"
    },
    "AutomationDelegation": {
      "additionalProperties": false,
      "properties": {
        "allowDynamicTargets": {
          "type": "boolean"
        },
        "allowOtherAuthorEdits": {
          "description": "Allow scoped definitions authored by another moderator to be replaced.",
          "type": "boolean"
        },
        "allowOtherAuthorSource": {
          "description": "Share full source of scoped definitions authored by other moderators.",
          "type": "boolean"
        },
        "allowOwnerEdits": {
          "description": "Explicitly allow scoped changes to existing Owner definitions.",
          "type": "boolean"
        },
        "allowOwnerSource": {
          "description": "Explicitly share Owner source, which can include sensitive values.",
          "type": "boolean"
        },
        "effects": {
          "items": {
            "$ref": "#/$defs/DelegatedAutomationEffect"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "effects",
        "allowDynamicTargets"
      ],
      "type": "object"
    },
    "Capability": {
      "description": "Strings are stable protocol capability identifiers, never role names.",
      "enum": [
        "live.read",
        "points.read",
        "points.adjust",
        "rankings.read",
        "rankings.manage",
        "competitions.read",
        "competitions.manage",
        "workflows.read",
        "workflows.edit",
        "workflows.toggle",
        "automation.read",
        "automation.edit",
        "automation.toggle",
        "plugins.read",
        "plugins.toggle",
        "plugins.settings.read",
        "plugins.settings.write",
        "plugins.action.execute",
        "widgets.read",
        "widgets.configure",
        "commands.read",
        "http.request",
        "audio.play",
        "events.emit",
        "scripts.execute"
      ],
      "type": "string"
    },
    "ClientKind": {
      "enum": [
        "desktop",
        "browser",
        "agent"
      ],
      "type": "string"
    },
    "DelegatedAutomationEffect": {
      "description": "Explicit policy for future reusable delegated automation execution. This\nis not itself an execution permit: the host must validate the saved closure\nand current subjects before it may be used at runtime.",
      "enum": [
        "log",
        "delay",
        "points",
        "http",
        "audio",
        "emit",
        "script"
      ],
      "type": "string"
    },
    "EffectGrants": {
      "additionalProperties": false,
      "properties": {
        "audio": {
          "default": [],
          "items": {
            "$ref": "#/$defs/AudioEffectGrant"
          },
          "type": "array"
        },
        "emitTopics": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "http": {
          "default": [],
          "items": {
            "$ref": "#/$defs/HttpEffectGrant"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "Grant": {
      "additionalProperties": false,
      "properties": {
        "actions": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        },
        "automationDelegation": {
          "anyOf": [
            {
              "$ref": "#/$defs/AutomationDelegation"
            },
            {
              "type": "null"
            }
          ],
          "description": "Missing on legacy grants; reusable execution is bounded by this policy."
        },
        "capabilities": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Capability"
          },
          "type": "array",
          "uniqueItems": true
        },
        "effects": {
          "$ref": "#/$defs/EffectGrants",
          "description": "External effects deny by default. Unlike legacy resource restrictions,\na missing or empty policy never means unrestricted access."
        },
        "maxPointsDelta": {
          "default": null,
          "format": "double",
          "type": [
            "number",
            "null"
          ]
        },
        "resources": {
          "additionalProperties": false,
          "default": {},
          "description": "Missing kind means unrestricted within a capability. An empty set\nexplicitly denies that resource kind; IDs never contain wildcards.",
          "properties": {
            "automation": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "competition": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "live": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "plugin": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "ranking": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "viewer": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "widget": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            },
            "workflow": {
              "items": {
                "type": "string"
              },
              "type": "array",
              "uniqueItems": true
            }
          },
          "type": "object"
        },
        "settingsFields": {
          "additionalProperties": {
            "items": {
              "type": "string"
            },
            "type": "array",
            "uniqueItems": true
          },
          "default": {},
          "type": "object"
        }
      },
      "type": "object"
    },
    "HttpEffectGrant": {
      "additionalProperties": false,
      "properties": {
        "allowPrivateNetwork": {
          "type": "boolean"
        },
        "allowTextBody": {
          "type": "boolean"
        },
        "bodyFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "headerNames": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "maxBodyBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "maxResponseBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "methods": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "origin": {
          "type": "string"
        },
        "paths": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "queryFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "renderFields": {
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        }
      },
      "required": [
        "origin",
        "methods",
        "paths",
        "queryFields",
        "headerNames",
        "renderFields",
        "bodyFields",
        "allowTextBody",
        "maxBodyBytes",
        "maxResponseBytes",
        "allowPrivateNetwork"
      ],
      "type": "object"
    },
    "PairingRequest": {
      "properties": {
        "clientKind": {
          "$ref": "#/$defs/ClientKind"
        },
        "clientLabel": {
          "type": "string"
        },
        "expiresAt": {
          "format": "int64",
          "type": "integer"
        },
        "grant": {
          "$ref": "#/$defs/Grant"
        },
        "requestId": {
          "type": "string"
        },
        "state": {
          "type": "string"
        }
      },
      "required": [
        "requestId",
        "clientLabel",
        "clientKind",
        "grant",
        "state",
        "expiresAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "nextAfter": {
      "type": [
        "string",
        "null"
      ]
    },
    "requests": {
      "items": {
        "$ref": "#/$defs/PairingRequest"
      },
      "type": "array"
    }
  },
  "required": [
    "requests"
  ],
  "title": "RequestsResult",
  "type": "object"
}
```

:::

### collaboration.requests.reject {#rpc-collaboration-requests-reject}

Descrição do contrato:

```text
Rejects a pending device
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "IdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### collaboration.resume {#rpc-collaboration-resume}

Descrição do contrato:

```text
Resumes retained reliable notifications under the same session and grant
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostInstanceId": {
      "type": "string"
    },
    "lastAppliedSequence": {
      "type": "string"
    },
    "subscriptionId": {
      "type": "string"
    }
  },
  "required": [
    "subscriptionId",
    "hostInstanceId",
    "lastAppliedSequence"
  ],
  "title": "ResumeParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "afterSequence": {
      "type": "string"
    },
    "grantRevision": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "replaying": {
      "type": "boolean"
    },
    "subscriptionId": {
      "type": "string"
    },
    "throughSequence": {
      "type": "string"
    }
  },
  "required": [
    "subscriptionId",
    "hostInstanceId",
    "grantRevision",
    "afterSequence",
    "throughSequence",
    "replaying"
  ],
  "title": "ResumeResult",
  "type": "object"
}
```

:::

### collaboration.sessions.list {#rpc-collaboration-sessions-list}

Descrição do contrato:

```text
Returns a redacted independent device session page
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "description": "Keyset pagination over immutable IDs, not a snapshot. Restart without a\ncursor to discover records created before an already-consumed cursor.",
  "properties": {
    "after": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "default": null,
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "ListQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "ClientKind": {
      "enum": [
        "desktop",
        "browser",
        "agent"
      ],
      "type": "string"
    },
    "Session": {
      "properties": {
        "clientKind": {
          "$ref": "#/$defs/ClientKind"
        },
        "clientLabel": {
          "default": "",
          "type": "string"
        },
        "createdAt": {
          "format": "int64",
          "type": "integer"
        },
        "expiresAt": {
          "format": "int64",
          "type": "integer"
        },
        "lastActivityAt": {
          "format": "int64",
          "type": "integer"
        },
        "principalId": {
          "type": "string"
        },
        "revoked": {
          "type": "boolean"
        },
        "sessionId": {
          "type": "string"
        }
      },
      "required": [
        "sessionId",
        "principalId",
        "clientKind",
        "createdAt",
        "lastActivityAt",
        "expiresAt",
        "revoked"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "nextAfter": {
      "type": [
        "string",
        "null"
      ]
    },
    "sessions": {
      "items": {
        "$ref": "#/$defs/Session"
      },
      "type": "array"
    }
  },
  "required": [
    "sessions"
  ],
  "title": "SessionsResult",
  "type": "object"
}
```

:::

### collaboration.sessions.revoke {#rpc-collaboration-sessions-revoke}

Descrição do contrato:

```text
Revokes only the selected device session
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "IdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### collaboration.snapshot {#rpc-collaboration-snapshot}

Descrição do contrato:

```text
Captures an immutable authorized baseline or retrieves its next page
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "pageToken": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "subscriptionId": {
      "type": "string"
    }
  },
  "required": [
    "subscriptionId"
  ],
  "title": "SnapshotParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "ResourceKind": {
      "enum": [
        "live",
        "viewer",
        "ranking",
        "competition",
        "workflow",
        "automation",
        "plugin",
        "widget"
      ],
      "type": "string"
    },
    "ResourceVersion": {
      "additionalProperties": false,
      "description": "Persisted resource version, shared with metadata-aware domain adapters.",
      "properties": {
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/ResourceKind"
        },
        "revision": {
          "type": "string"
        },
        "scope": {
          "type": "string"
        }
      },
      "required": [
        "kind",
        "id",
        "scope",
        "revision"
      ],
      "type": "object"
    },
    "StreamDomain": {
      "enum": [
        "live",
        "points",
        "rankings",
        "competitions",
        "workflows",
        "automation",
        "plugins",
        "widgets",
        "commands"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "automation": {
      "items": true,
      "type": "array"
    },
    "competitions": {
      "items": true,
      "type": "array"
    },
    "domains": {
      "items": {
        "$ref": "#/$defs/StreamDomain"
      },
      "type": "array",
      "uniqueItems": true
    },
    "grantRevision": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "nextPageToken": {
      "type": [
        "string",
        "null"
      ]
    },
    "points": {
      "items": true,
      "type": "array"
    },
    "pointsLimit": {
      "format": "uint16",
      "maximum": 65535,
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "resourceVersions": {
      "items": {
        "$ref": "#/$defs/ResourceVersion"
      },
      "type": "array"
    },
    "snapshotId": {
      "type": "string"
    },
    "subscriptionId": {
      "type": "string"
    },
    "throughSequence": {
      "type": "string"
    },
    "workflows": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "subscriptionId",
    "snapshotId",
    "hostInstanceId",
    "grantRevision",
    "throughSequence",
    "domains",
    "workflows",
    "resourceVersions"
  ],
  "title": "SnapshotPage",
  "type": "object"
}
```

:::

### collaboration.status {#rpc-collaboration-status}

Descrição do contrato:

```text
Local-owner collaboration configuration and session counts
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Configuration": {
      "additionalProperties": false,
      "properties": {
        "advertisedOrigin": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "allowedOrigins": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array",
          "uniqueItems": true
        },
        "bindAddress": {
          "default": "127.0.0.1:8788",
          "type": "string"
        },
        "enabled": {
          "type": "boolean"
        },
        "limits": {
          "$ref": "#/$defs/RemoteLimits",
          "default": {
            "commandIdsPerPrincipal": 100000,
            "connections": 8,
            "connectionsPerPrincipal": 4,
            "frameBytes": 1048576,
            "paramsBytes": 262144,
            "pendingCalls": 32,
            "requestBurst": 60,
            "requestsPerSecond": 30
          }
        },
        "peer": {
          "anyOf": [
            {
              "$ref": "#/$defs/PeerConfiguration"
            },
            {
              "type": "null"
            }
          ]
        },
        "tlsCertificate": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "tlsPrivateKey": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "tlsProxy": {
          "default": false,
          "type": "boolean"
        }
      },
      "required": [
        "enabled"
      ],
      "type": "object"
    },
    "GatewayStatus": {
      "properties": {
        "address": {
          "type": [
            "string",
            "null"
          ]
        },
        "available": {
          "type": "boolean"
        },
        "certificateFingerprint": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "configurationRevision": {
          "type": [
            "string",
            "null"
          ]
        },
        "connections": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "lastError": {
          "type": [
            "string",
            "null"
          ]
        },
        "running": {
          "type": "boolean"
        },
        "sessionConnections": {
          "additionalProperties": {
            "format": "uint32",
            "minimum": 0,
            "type": "integer"
          },
          "default": {},
          "type": "object"
        }
      },
      "required": [
        "available",
        "running",
        "connections"
      ],
      "type": "object"
    },
    "PeerConfiguration": {
      "additionalProperties": false,
      "properties": {
        "relayUrls": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "PeerGatewayStatus": {
      "properties": {
        "address": {
          "type": [
            "string",
            "null"
          ]
        },
        "addresses": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "available": {
          "type": "boolean"
        },
        "certificateFingerprint": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "configurationRevision": {
          "type": [
            "string",
            "null"
          ]
        },
        "connections": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "endpointId": {
          "type": [
            "string",
            "null"
          ]
        },
        "lastError": {
          "type": [
            "string",
            "null"
          ]
        },
        "relayUrls": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "running": {
          "type": "boolean"
        },
        "sessionConnections": {
          "additionalProperties": {
            "format": "uint32",
            "minimum": 0,
            "type": "integer"
          },
          "default": {},
          "type": "object"
        }
      },
      "required": [
        "available",
        "running",
        "connections",
        "addresses",
        "relayUrls"
      ],
      "type": "object"
    },
    "RemoteLimits": {
      "additionalProperties": false,
      "properties": {
        "commandIdsPerPrincipal": {
          "default": 100000,
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "connections": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "connectionsPerPrincipal": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "frameBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "paramsBytes": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "pendingCalls": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "requestBurst": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "requestsPerSecond": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "connections",
        "connectionsPerPrincipal",
        "frameBytes",
        "paramsBytes",
        "pendingCalls",
        "requestsPerSecond",
        "requestBurst"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "activeSessions": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "configuration": {
      "$ref": "#/$defs/Configuration"
    },
    "configurationRevision": {
      "type": "string"
    },
    "gateway": {
      "$ref": "#/$defs/GatewayStatus"
    },
    "hostId": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "peerGateway": {
      "$ref": "#/$defs/PeerGatewayStatus",
      "default": {
        "address": null,
        "addresses": [],
        "available": false,
        "certificateFingerprint": null,
        "configurationRevision": null,
        "connections": 0,
        "endpointId": null,
        "lastError": null,
        "relayUrls": [],
        "running": false,
        "sessionConnections": {}
      }
    },
    "pendingRequests": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "hostId",
    "hostInstanceId",
    "configuration",
    "configurationRevision",
    "activeSessions",
    "pendingRequests",
    "gateway"
  ],
  "title": "Status",
  "type": "object"
}
```

:::

### collaboration.subscribe {#rpc-collaboration-subscribe}

Descrição do contrato:

```text
Creates one authorized bounded recovery stream for this connection
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "StreamDomain": {
      "enum": [
        "live",
        "points",
        "rankings",
        "competitions",
        "workflows",
        "automation",
        "plugins",
        "widgets",
        "commands"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "domains": {
      "items": {
        "$ref": "#/$defs/StreamDomain"
      },
      "type": "array",
      "uniqueItems": true
    },
    "feed": {
      "default": false,
      "type": "boolean"
    },
    "pointsLimit": {
      "description": "None is the complete permitted set; Some(n) is a top-N display view.\nSome(0) requests only reliable invalidations, with no baseline point rows.",
      "format": "uint16",
      "maximum": 65535,
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "required": [
    "domains"
  ],
  "title": "SubscribeParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "StreamDomain": {
      "enum": [
        "live",
        "points",
        "rankings",
        "competitions",
        "workflows",
        "automation",
        "plugins",
        "widgets",
        "commands"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "domains": {
      "items": {
        "$ref": "#/$defs/StreamDomain"
      },
      "type": "array",
      "uniqueItems": true
    },
    "grantRevision": {
      "type": "string"
    },
    "hostInstanceId": {
      "type": "string"
    },
    "pointsLimit": {
      "format": "uint16",
      "maximum": 65535,
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "retainedBytes": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "retainedEvents": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "retentionSeconds": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "snapshotSeconds": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "subscriptionId": {
      "type": "string"
    }
  },
  "required": [
    "subscriptionId",
    "hostInstanceId",
    "grantRevision",
    "domains",
    "retainedEvents",
    "retainedBytes",
    "retentionSeconds",
    "snapshotSeconds"
  ],
  "title": "SubscribeResult",
  "type": "object"
}
```

:::

### competitions.action {#rpc-competitions-action}

Descrição do contrato:

```text
Read winner action status
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "generation": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "generation"
  ],
  "title": "CompetitionActionId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "ActionExecution": {
      "properties": {
        "actionId": {
          "type": "string"
        },
        "competitionId": {
          "type": "string"
        },
        "createdAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "generation": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "reason": {
          "description": "Sanitized, machine-readable reason; never includes action inputs.",
          "type": [
            "string",
            "null"
          ]
        },
        "status": {
          "$ref": "#/$defs/ActionStatus"
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "competitionId",
        "generation",
        "actionId",
        "status",
        "createdAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "ActionStatus": {
      "description": "Public execution status omits action inputs and idempotency keys.",
      "enum": [
        "pending",
        "running",
        "completed",
        "failed",
        "uncertain"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "action": {
      "anyOf": [
        {
          "$ref": "#/$defs/ActionExecution"
        },
        {
          "type": "null"
        }
      ]
    }
  },
  "title": "CompetitionActionResult",
  "type": "object"
}
```

:::

### competitions.cancel {#rpc-competitions-cancel}

Descrição do contrato:

```text
Transition a competition with a revision-bound command
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedGeneration": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionStopParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### competitions.candidates {#rpc-competitions-candidates}

Descrição do contrato:

```text
Read tied leaders awaiting manual selection
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Entry": {
      "description": "Optional display data supplied by a source adapter. Identity and scores\nremain the ranking key and integer score, never these presentation fields.",
      "properties": {
        "avatarUrl": {
          "type": [
            "string",
            "null"
          ]
        },
        "giftId": {
          "type": [
            "string",
            "null"
          ]
        },
        "iconUrl": {
          "type": [
            "string",
            "null"
          ]
        },
        "key": {
          "type": "string"
        },
        "label": {
          "default": "",
          "type": "string"
        },
        "nickname": {
          "type": [
            "string",
            "null"
          ]
        },
        "rank": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "score": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "uniqueId": {
          "type": [
            "string",
            "null"
          ]
        },
        "userId": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "key",
        "rank",
        "score"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "candidates": {
      "items": {
        "$ref": "#/$defs/Entry"
      },
      "type": "array"
    },
    "generation": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "generation",
    "candidates"
  ],
  "title": "ManualCandidates",
  "type": "object"
}
```

:::

### competitions.create {#rpc-competitions-create}

Descrição do contrato:

```text
Create a competition definition
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "CompetitionKind": {
      "enum": [
        "poll",
        "weighted_poll",
        "leaderboard_challenge",
        "team_battle",
        "random_draw",
        "weighted_draw",
        "roulette"
      ],
      "type": "string"
    },
    "OptionDefinition": {
      "properties": {
        "commands": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "giftIds": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "id": {
          "type": "string"
        },
        "imageUrl": {
          "type": [
            "string",
            "null"
          ]
        },
        "label": {
          "type": "string"
        },
        "onWin": {
          "anyOf": [
            {
              "$ref": "#/$defs/WinnerAction"
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "id",
        "label"
      ],
      "type": "object"
    },
    "SelectionMode": {
      "enum": [
        "highest_score",
        "uniform_random",
        "weighted_random"
      ],
      "type": "string"
    },
    "TiePolicy": {
      "enum": [
        "manual",
        "stable",
        "random",
        "no_winner"
      ],
      "type": "string"
    },
    "WinnerAction": {
      "description": "A reference to an existing automation action; execution remains the\nresponsibility of the host automation runtime and its capability checks.",
      "properties": {
        "actionId": {
          "type": "string"
        },
        "inputs": true
      },
      "required": [
        "actionId",
        "inputs"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "allowSwitch": {
      "type": "boolean"
    },
    "durationMs": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/CompetitionKind"
    },
    "minimumParticipants": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "name": {
      "type": "string"
    },
    "onUserWin": {
      "anyOf": [
        {
          "$ref": "#/$defs/WinnerAction"
        },
        {
          "type": "null"
        }
      ],
      "description": "Action for winners that are users rather than options (participant\ndraws and leaderboard challenges)."
    },
    "options": {
      "items": {
        "$ref": "#/$defs/OptionDefinition"
      },
      "type": "array"
    },
    "rankingId": {
      "type": "string"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "selectionMode": {
      "$ref": "#/$defs/SelectionMode"
    },
    "targetScore": {
      "description": "Leaderboard challenges: the leader must reach this score to win.",
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "tiePolicy": {
      "$ref": "#/$defs/TiePolicy"
    },
    "voteLimit": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "schemaVersion",
    "id",
    "name",
    "kind",
    "rankingId",
    "options",
    "selectionMode",
    "tiePolicy",
    "voteLimit",
    "allowSwitch",
    "minimumParticipants"
  ],
  "title": "Definition",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### competitions.delete {#rpc-competitions-delete}

Descrição do contrato:

```text
Delete a competition definition without results
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### competitions.finish {#rpc-competitions-finish}

Descrição do contrato:

```text
Close and finalize an eligible competition
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Candidate": {
      "properties": {
        "key": {
          "type": "string"
        },
        "weight": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "key",
        "weight"
      ],
      "type": "object"
    },
    "Selection": {
      "properties": {
        "eligible": {
          "items": {
            "$ref": "#/$defs/Candidate"
          },
          "type": "array"
        },
        "mode": {
          "$ref": "#/$defs/SelectionMode"
        },
        "tiePolicy": {
          "$ref": "#/$defs/TiePolicy"
        },
        "winner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "mode",
        "tiePolicy",
        "eligible"
      ],
      "type": "object"
    },
    "SelectionMode": {
      "enum": [
        "highest_score",
        "uniform_random",
        "weighted_random"
      ],
      "type": "string"
    },
    "TiePolicy": {
      "enum": [
        "manual",
        "stable",
        "random",
        "no_winner"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A finalized generation's immutable selection. The persisted eligible set\nis the authority for replay and audit, not a widget animation.",
  "properties": {
    "competitionId": {
      "type": "string"
    },
    "finalizedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "generation": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "rankingId": {
      "type": "string"
    },
    "rankingRevision": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "selection": {
      "$ref": "#/$defs/Selection"
    }
  },
  "required": [
    "schemaVersion",
    "competitionId",
    "generation",
    "rankingId",
    "rankingRevision",
    "finalizedAt",
    "selection"
  ],
  "title": "ResultRecord",
  "type": "object"
}
```

:::

### competitions.get {#rpc-competitions-get}

Descrição do contrato:

```text
Get a competition definition
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CompetitionKind": {
      "enum": [
        "poll",
        "weighted_poll",
        "leaderboard_challenge",
        "team_battle",
        "random_draw",
        "weighted_draw",
        "roulette"
      ],
      "type": "string"
    },
    "OptionDefinition": {
      "properties": {
        "commands": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "giftIds": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "id": {
          "type": "string"
        },
        "imageUrl": {
          "type": [
            "string",
            "null"
          ]
        },
        "label": {
          "type": "string"
        },
        "onWin": {
          "anyOf": [
            {
              "$ref": "#/$defs/WinnerAction"
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "id",
        "label"
      ],
      "type": "object"
    },
    "SelectionMode": {
      "enum": [
        "highest_score",
        "uniform_random",
        "weighted_random"
      ],
      "type": "string"
    },
    "TiePolicy": {
      "enum": [
        "manual",
        "stable",
        "random",
        "no_winner"
      ],
      "type": "string"
    },
    "WinnerAction": {
      "description": "A reference to an existing automation action; execution remains the\nresponsibility of the host automation runtime and its capability checks.",
      "properties": {
        "actionId": {
          "type": "string"
        },
        "inputs": true
      },
      "required": [
        "actionId",
        "inputs"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "allowSwitch": {
      "type": "boolean"
    },
    "durationMs": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/CompetitionKind"
    },
    "minimumParticipants": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "name": {
      "type": "string"
    },
    "onUserWin": {
      "anyOf": [
        {
          "$ref": "#/$defs/WinnerAction"
        },
        {
          "type": "null"
        }
      ],
      "description": "Action for winners that are users rather than options (participant\ndraws and leaderboard challenges)."
    },
    "options": {
      "items": {
        "$ref": "#/$defs/OptionDefinition"
      },
      "type": "array"
    },
    "rankingId": {
      "type": "string"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "selectionMode": {
      "$ref": "#/$defs/SelectionMode"
    },
    "targetScore": {
      "description": "Leaderboard challenges: the leader must reach this score to win.",
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "tiePolicy": {
      "$ref": "#/$defs/TiePolicy"
    },
    "voteLimit": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "schemaVersion",
    "id",
    "name",
    "kind",
    "rankingId",
    "options",
    "selectionMode",
    "tiePolicy",
    "voteLimit",
    "allowSwitch",
    "minimumParticipants"
  ],
  "title": "Definition",
  "type": "object"
}
```

:::

### competitions.list {#rpc-competitions-list}

Descrição do contrato:

```text
List competition definitions
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CompetitionKind": {
      "enum": [
        "poll",
        "weighted_poll",
        "leaderboard_challenge",
        "team_battle",
        "random_draw",
        "weighted_draw",
        "roulette"
      ],
      "type": "string"
    },
    "Definition": {
      "properties": {
        "allowSwitch": {
          "type": "boolean"
        },
        "durationMs": {
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/CompetitionKind"
        },
        "minimumParticipants": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "name": {
          "type": "string"
        },
        "onUserWin": {
          "anyOf": [
            {
              "$ref": "#/$defs/WinnerAction"
            },
            {
              "type": "null"
            }
          ],
          "description": "Action for winners that are users rather than options (participant\ndraws and leaderboard challenges)."
        },
        "options": {
          "items": {
            "$ref": "#/$defs/OptionDefinition"
          },
          "type": "array"
        },
        "rankingId": {
          "type": "string"
        },
        "schemaVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "selectionMode": {
          "$ref": "#/$defs/SelectionMode"
        },
        "targetScore": {
          "description": "Leaderboard challenges: the leader must reach this score to win.",
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "tiePolicy": {
          "$ref": "#/$defs/TiePolicy"
        },
        "voteLimit": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "schemaVersion",
        "id",
        "name",
        "kind",
        "rankingId",
        "options",
        "selectionMode",
        "tiePolicy",
        "voteLimit",
        "allowSwitch",
        "minimumParticipants"
      ],
      "type": "object"
    },
    "OptionDefinition": {
      "properties": {
        "commands": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "giftIds": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "id": {
          "type": "string"
        },
        "imageUrl": {
          "type": [
            "string",
            "null"
          ]
        },
        "label": {
          "type": "string"
        },
        "onWin": {
          "anyOf": [
            {
              "$ref": "#/$defs/WinnerAction"
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "id",
        "label"
      ],
      "type": "object"
    },
    "SelectionMode": {
      "enum": [
        "highest_score",
        "uniform_random",
        "weighted_random"
      ],
      "type": "string"
    },
    "TiePolicy": {
      "enum": [
        "manual",
        "stable",
        "random",
        "no_winner"
      ],
      "type": "string"
    },
    "WinnerAction": {
      "description": "A reference to an existing automation action; execution remains the\nresponsibility of the host automation runtime and its capability checks.",
      "properties": {
        "actionId": {
          "type": "string"
        },
        "inputs": true
      },
      "required": [
        "actionId",
        "inputs"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "competitions": {
      "items": {
        "$ref": "#/$defs/Definition"
      },
      "type": "array"
    }
  },
  "required": [
    "competitions"
  ],
  "title": "CompetitionList",
  "type": "object"
}
```

:::

### competitions.markActionUncertain {#rpc-competitions-markActionUncertain}

Descrição do contrato:

```text
Mark a stranded running winner action uncertain
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "generation": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "generation"
  ],
  "title": "CompetitionActionId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### competitions.pause {#rpc-competitions-pause}

Descrição do contrato:

```text
Transition a competition with a revision-bound command
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedGeneration": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionStopParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### competitions.resolveAction {#rpc-competitions-resolveAction}

Descrição do contrato:

```text
Resolve an uncertain winner action without dispatching it
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "ActionResolution": {
      "enum": [
        "completed",
        "failed"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "generation": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "id": {
      "type": "string"
    },
    "resolution": {
      "$ref": "#/$defs/ActionResolution"
    }
  },
  "required": [
    "id",
    "generation",
    "resolution"
  ],
  "title": "ResolveCompetitionAction",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### competitions.results {#rpc-competitions-results}

Descrição do contrato:

```text
Read committed competition result history
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "beforeGeneration": {
      "default": null,
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "id": {
      "type": "string"
    },
    "limit": {
      "default": null,
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionResultsRequest",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Candidate": {
      "properties": {
        "key": {
          "type": "string"
        },
        "weight": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "key",
        "weight"
      ],
      "type": "object"
    },
    "ResultRecord": {
      "description": "A finalized generation's immutable selection. The persisted eligible set\nis the authority for replay and audit, not a widget animation.",
      "properties": {
        "competitionId": {
          "type": "string"
        },
        "finalizedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "generation": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "rankingId": {
          "type": "string"
        },
        "rankingRevision": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "schemaVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "selection": {
          "$ref": "#/$defs/Selection"
        }
      },
      "required": [
        "schemaVersion",
        "competitionId",
        "generation",
        "rankingId",
        "rankingRevision",
        "finalizedAt",
        "selection"
      ],
      "type": "object"
    },
    "Selection": {
      "properties": {
        "eligible": {
          "items": {
            "$ref": "#/$defs/Candidate"
          },
          "type": "array"
        },
        "mode": {
          "$ref": "#/$defs/SelectionMode"
        },
        "tiePolicy": {
          "$ref": "#/$defs/TiePolicy"
        },
        "winner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "mode",
        "tiePolicy",
        "eligible"
      ],
      "type": "object"
    },
    "SelectionMode": {
      "enum": [
        "highest_score",
        "uniform_random",
        "weighted_random"
      ],
      "type": "string"
    },
    "TiePolicy": {
      "enum": [
        "manual",
        "stable",
        "random",
        "no_winner"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "nextBeforeGeneration": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "results": {
      "items": {
        "$ref": "#/$defs/ResultRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "results"
  ],
  "title": "CompetitionResults",
  "type": "object"
}
```

:::

### competitions.resume {#rpc-competitions-resume}

Descrição do contrato:

```text
Transition a competition with a revision-bound command
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedGeneration": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionStopParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### competitions.selectWinner {#rpc-competitions-selectWinner}

Descrição do contrato:

```text
Resolve a closed manual highest-score tie
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "generation": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "id": {
      "type": "string"
    },
    "winnerId": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "winnerId",
    "generation"
  ],
  "title": "SelectWinner",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Candidate": {
      "properties": {
        "key": {
          "type": "string"
        },
        "weight": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "key",
        "weight"
      ],
      "type": "object"
    },
    "Selection": {
      "properties": {
        "eligible": {
          "items": {
            "$ref": "#/$defs/Candidate"
          },
          "type": "array"
        },
        "mode": {
          "$ref": "#/$defs/SelectionMode"
        },
        "tiePolicy": {
          "$ref": "#/$defs/TiePolicy"
        },
        "winner": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "mode",
        "tiePolicy",
        "eligible"
      ],
      "type": "object"
    },
    "SelectionMode": {
      "enum": [
        "highest_score",
        "uniform_random",
        "weighted_random"
      ],
      "type": "string"
    },
    "TiePolicy": {
      "enum": [
        "manual",
        "stable",
        "random",
        "no_winner"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A finalized generation's immutable selection. The persisted eligible set\nis the authority for replay and audit, not a widget animation.",
  "properties": {
    "competitionId": {
      "type": "string"
    },
    "finalizedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "generation": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "rankingId": {
      "type": "string"
    },
    "rankingRevision": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "selection": {
      "$ref": "#/$defs/Selection"
    }
  },
  "required": [
    "schemaVersion",
    "competitionId",
    "generation",
    "rankingId",
    "rankingRevision",
    "finalizedAt",
    "selection"
  ],
  "title": "ResultRecord",
  "type": "object"
}
```

:::

### competitions.start {#rpc-competitions-start}

Descrição do contrato:

```text
Transition a competition with a revision-bound command
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedGeneration": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionStopParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### competitions.state {#rpc-competitions-state}

Descrição do contrato:

```text
Read durable competition lifecycle state
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "CompetitionId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Lifecycle": {
      "enum": [
        "draft",
        "active",
        "paused",
        "finalizing",
        "finished",
        "cancelled",
        "failed"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Durable lifecycle; absolute deadlines are supplied by the host clock.",
  "properties": {
    "deadlineMs": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "generation": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "lifecycle": {
      "$ref": "#/$defs/Lifecycle"
    },
    "remainingMs": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "required": [
    "lifecycle",
    "generation"
  ],
  "title": "LifecycleState",
  "type": "object"
}
```

:::

### competitions.summary {#rpc-competitions-summary}

Descrição do contrato:

```text
Read bounded authorized competition status without definitions or participants
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "after": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "includeRevision": {
      "default": false,
      "type": "boolean"
    },
    "limit": {
      "default": null,
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "CompetitionSummaryParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CompetitionKind": {
      "enum": [
        "poll",
        "weighted_poll",
        "leaderboard_challenge",
        "team_battle",
        "random_draw",
        "weighted_draw",
        "roulette"
      ],
      "type": "string"
    },
    "CompetitionSummary": {
      "additionalProperties": false,
      "description": "Read-only remote projection. Definitions, commands, winner actions,\nranking IDs and participant information never leave the host here.",
      "properties": {
        "deadlineMs": {
          "type": [
            "string",
            "null"
          ]
        },
        "generation": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/CompetitionKind"
        },
        "lifecycle": {
          "$ref": "#/$defs/Lifecycle"
        },
        "name": {
          "type": "string"
        },
        "remainingMs": {
          "type": [
            "string",
            "null"
          ]
        },
        "revision": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "id",
        "name",
        "kind",
        "lifecycle",
        "generation"
      ],
      "type": "object"
    },
    "Lifecycle": {
      "enum": [
        "draft",
        "active",
        "paused",
        "finalizing",
        "finished",
        "cancelled",
        "failed"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "competitions": {
      "items": {
        "$ref": "#/$defs/CompetitionSummary"
      },
      "type": "array"
    },
    "nextAfter": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "competitions"
  ],
  "title": "CompetitionSummaryPage",
  "type": "object"
}
```

:::

### competitions.update {#rpc-competitions-update}

Descrição do contrato:

```text
Update a competition definition without results
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "CompetitionKind": {
      "enum": [
        "poll",
        "weighted_poll",
        "leaderboard_challenge",
        "team_battle",
        "random_draw",
        "weighted_draw",
        "roulette"
      ],
      "type": "string"
    },
    "OptionDefinition": {
      "properties": {
        "commands": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "giftIds": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "id": {
          "type": "string"
        },
        "imageUrl": {
          "type": [
            "string",
            "null"
          ]
        },
        "label": {
          "type": "string"
        },
        "onWin": {
          "anyOf": [
            {
              "$ref": "#/$defs/WinnerAction"
            },
            {
              "type": "null"
            }
          ]
        }
      },
      "required": [
        "id",
        "label"
      ],
      "type": "object"
    },
    "SelectionMode": {
      "enum": [
        "highest_score",
        "uniform_random",
        "weighted_random"
      ],
      "type": "string"
    },
    "TiePolicy": {
      "enum": [
        "manual",
        "stable",
        "random",
        "no_winner"
      ],
      "type": "string"
    },
    "WinnerAction": {
      "description": "A reference to an existing automation action; execution remains the\nresponsibility of the host automation runtime and its capability checks.",
      "properties": {
        "actionId": {
          "type": "string"
        },
        "inputs": true
      },
      "required": [
        "actionId",
        "inputs"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "allowSwitch": {
      "type": "boolean"
    },
    "durationMs": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "id": {
      "type": "string"
    },
    "kind": {
      "$ref": "#/$defs/CompetitionKind"
    },
    "minimumParticipants": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "name": {
      "type": "string"
    },
    "onUserWin": {
      "anyOf": [
        {
          "$ref": "#/$defs/WinnerAction"
        },
        {
          "type": "null"
        }
      ],
      "description": "Action for winners that are users rather than options (participant\ndraws and leaderboard challenges)."
    },
    "options": {
      "items": {
        "$ref": "#/$defs/OptionDefinition"
      },
      "type": "array"
    },
    "rankingId": {
      "type": "string"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "selectionMode": {
      "$ref": "#/$defs/SelectionMode"
    },
    "targetScore": {
      "description": "Leaderboard challenges: the leader must reach this score to win.",
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "tiePolicy": {
      "$ref": "#/$defs/TiePolicy"
    },
    "voteLimit": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "schemaVersion",
    "id",
    "name",
    "kind",
    "rankingId",
    "options",
    "selectionMode",
    "tiePolicy",
    "voteLimit",
    "allowSwitch",
    "minimumParticipants"
  ],
  "title": "Definition",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### creators.get {#rpc-creators-get}

Descrição do contrato:

```text
Creator record by uniqueId (defaults to the last connected creator)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "uniqueId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "CreatorGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "creator": true
  },
  "title": "CreatorGetResult",
  "type": "object"
}
```

:::

### creators.history.clear {#rpc-creators-history-clear}

Descrição do contrato:

```text
Clears creator history and the last-creator pointer
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### creators.recent {#rpc-creators-recent}

Descrição do contrato:

```text
Recently connected creators, most recent first
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "default": null,
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "CreatorsRecentParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "creators": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "creators"
  ],
  "title": "CreatorsRecentResult",
  "type": "object"
}
```

:::

### gifts.debug {#rpc-gifts-debug}

Descrição do contrato:

```text
Gift icon lookup plus the catalog size
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "giftId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "GiftDebugParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "giftId": {
      "type": [
        "string",
        "null"
      ]
    },
    "hasIcon": {
      "type": "boolean"
    },
    "iconUrl": {
      "type": [
        "string",
        "null"
      ]
    },
    "totalGifts": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "hasIcon",
    "totalGifts"
  ],
  "title": "GiftDebugResult",
  "type": "object"
}
```

:::

### gifts.list {#rpc-gifts-list}

Descrição do contrato:

```text
Persisted gift catalog ordered by diamond count
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "gifts": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "gifts"
  ],
  "title": "GiftCatalogResult",
  "type": "object"
}
```

:::

### globals.delete {#rpc-globals-delete}

Descrição do contrato:

```text
Deletes one runtime global
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "key": {
      "type": "string"
    }
  },
  "required": [
    "key"
  ],
  "title": "GlobalsDeleteParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "deleted": {
      "type": "boolean"
    },
    "key": {
      "type": "string"
    }
  },
  "required": [
    "key",
    "deleted"
  ],
  "title": "GlobalsDeleteResult",
  "type": "object"
}
```

:::

### globals.get {#rpc-globals-get}

Descrição do contrato:

```text
Reads one runtime global by bare key
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "key": {
      "type": "string"
    }
  },
  "required": [
    "key"
  ],
  "title": "GlobalsGetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "key": {
      "type": "string"
    },
    "value": {
      "type": "string"
    }
  },
  "required": [
    "key",
    "value"
  ],
  "title": "GlobalsValueResult",
  "type": "object"
}
```

:::

### globals.list {#rpc-globals-list}

Descrição do contrato:

```text
Lists every runtime global (bare key to text value)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "globals": {
      "additionalProperties": {
        "type": "string"
      },
      "type": "object"
    }
  },
  "required": [
    "globals"
  ],
  "title": "GlobalsListResult",
  "type": "object"
}
```

:::

### globals.set {#rpc-globals-set}

Descrição do contrato:

```text
Creates or replaces one runtime global
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "key": {
      "type": "string"
    },
    "value": {
      "type": "string"
    }
  },
  "required": [
    "key",
    "value"
  ],
  "title": "GlobalsSetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "key": {
      "type": "string"
    },
    "value": {
      "type": "string"
    }
  },
  "required": [
    "key",
    "value"
  ],
  "title": "GlobalsValueResult",
  "type": "object"
}
```

:::

### globals.typed.list {#rpc-globals-typed-list}

Descrição do contrato:

```text
Reads local variable types, values and the revision required for saving
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "RuntimeVariable": {
      "additionalProperties": false,
      "description": "`value` retains the existing numeric API. Exact text accompanies integers\noutside JavaScript's safe range so a browser can edit another row without\nrounding an untouched native number. It is never emitted for a secret.",
      "properties": {
        "numberText": {
          "description": "Required for integers outside JavaScript's exact range. The numeric\n`value` stays for compatibility; this text preserves the native value.",
          "maxLength": 4096,
          "type": [
            "string",
            "null"
          ]
        },
        "type": {
          "$ref": "#/$defs/RuntimeVariableKind"
        },
        "value": true
      },
      "required": [
        "type",
        "value"
      ],
      "type": "object"
    },
    "RuntimeVariableKind": {
      "enum": [
        "text",
        "number",
        "boolean",
        "list",
        "secret"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "revision": {
      "type": "string"
    },
    "variables": {
      "additionalProperties": {
        "$ref": "#/$defs/RuntimeVariable"
      },
      "type": "object"
    }
  },
  "required": [
    "revision",
    "variables"
  ],
  "title": "RuntimeVariablesSnapshot",
  "type": "object"
}
```

:::

### globals.typed.replace {#rpc-globals-typed-replace}

Descrição do contrato:

```text
Atomically saves the reviewed local variable revision; never syncs values
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "RuntimeVariable": {
      "additionalProperties": false,
      "description": "`value` retains the existing numeric API. Exact text accompanies integers\noutside JavaScript's safe range so a browser can edit another row without\nrounding an untouched native number. It is never emitted for a secret.",
      "properties": {
        "numberText": {
          "description": "Required for integers outside JavaScript's exact range. The numeric\n`value` stays for compatibility; this text preserves the native value.",
          "maxLength": 4096,
          "type": [
            "string",
            "null"
          ]
        },
        "type": {
          "$ref": "#/$defs/RuntimeVariableKind"
        },
        "value": true
      },
      "required": [
        "type",
        "value"
      ],
      "type": "object"
    },
    "RuntimeVariableKind": {
      "enum": [
        "text",
        "number",
        "boolean",
        "list",
        "secret"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedRevision": {
      "type": "string"
    },
    "variables": {
      "additionalProperties": {
        "$ref": "#/$defs/RuntimeVariable"
      },
      "type": "object"
    }
  },
  "required": [
    "expectedRevision",
    "variables"
  ],
  "title": "RuntimeVariablesReplace",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "RuntimeVariable": {
      "additionalProperties": false,
      "description": "`value` retains the existing numeric API. Exact text accompanies integers\noutside JavaScript's safe range so a browser can edit another row without\nrounding an untouched native number. It is never emitted for a secret.",
      "properties": {
        "numberText": {
          "description": "Required for integers outside JavaScript's exact range. The numeric\n`value` stays for compatibility; this text preserves the native value.",
          "maxLength": 4096,
          "type": [
            "string",
            "null"
          ]
        },
        "type": {
          "$ref": "#/$defs/RuntimeVariableKind"
        },
        "value": true
      },
      "required": [
        "type",
        "value"
      ],
      "type": "object"
    },
    "RuntimeVariableKind": {
      "enum": [
        "text",
        "number",
        "boolean",
        "list",
        "secret"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "revision": {
      "type": "string"
    },
    "variables": {
      "additionalProperties": {
        "$ref": "#/$defs/RuntimeVariable"
      },
      "type": "object"
    }
  },
  "required": [
    "revision",
    "variables"
  ],
  "title": "RuntimeVariablesSnapshot",
  "type": "object"
}
```

:::

### integrations.connect {#rpc-integrations-connect}

Descrição do contrato:

```text
Creates a webhook connection; the secret is returned once and is not stored by the host
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "label": {
      "type": "string"
    }
  },
  "required": [
    "label"
  ],
  "title": "IntegrationConnectParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "The only reply that carries a secret. Callers must show it once and drop it.",
  "properties": {
    "connectionId": {
      "type": "string"
    },
    "deliveryPath": {
      "type": "string"
    },
    "label": {
      "type": "string"
    },
    "provider": {
      "type": "string"
    },
    "secret": {
      "type": "string"
    }
  },
  "required": [
    "connectionId",
    "provider",
    "label",
    "secret",
    "deliveryPath"
  ],
  "title": "IntegrationConnectResult",
  "type": "object"
}
```

:::

### integrations.connections {#rpc-integrations-connections}

Descrição do contrato:

```text
Lists the Cloud integration connections owned by the signed-in account
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "IntegrationConnection": {
      "properties": {
        "createdAt": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "label": {
          "type": "string"
        },
        "provider": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "provider",
        "label",
        "status",
        "createdAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/IntegrationConnection"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "IntegrationConnectionList",
  "type": "object"
}
```

:::

### integrations.disconnect {#rpc-integrations-disconnect}

Descrição do contrato:

```text
Revokes a webhook connection; later deliveries are refused and stored events stay readable
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "IntegrationIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "status"
  ],
  "title": "IntegrationDisconnectResult",
  "type": "object"
}
```

:::

### integrations.events {#rpc-integrations-events}

Descrição do contrato:

```text
Reads the newest webhook events of one owned connection, bounded to the reply budget
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "IntegrationIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "IntegrationEvent": {
      "properties": {
        "deliveryId": {
          "type": "string"
        },
        "payload": true,
        "receivedAt": {
          "type": "string"
        }
      },
      "required": [
        "deliveryId",
        "receivedAt",
        "payload"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/IntegrationEvent"
      },
      "type": "array"
    },
    "truncated": {
      "description": "True when older events were dropped to keep the reply within bounds.",
      "type": "boolean"
    }
  },
  "required": [
    "items",
    "truncated"
  ],
  "title": "IntegrationEventList",
  "type": "object"
}
```

:::

### live.channels {#rpc-live-channels}

Descrição do contrato:

```text
Browses currently live rooms without connecting
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "keyword": {
      "default": null,
      "description": "Search hint; trimmed and capped by the host. Defaults to `live`.",
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "default": null,
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "offset": {
      "default": null,
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "sessionCookie": {
      "default": "",
      "description": "Optional signed-in session. Empty uses TikTok's guest session and is\nnever logged or returned.",
      "type": "string"
    }
  },
  "title": "LiveChannelsParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "LiveChannel": {
      "description": "One currently-live room returned by TikTok's browser-free live search.",
      "properties": {
        "nickname": {
          "type": "string"
        },
        "roomId": {
          "description": "Kept a string on purpose: TikTok room ids exceed the JavaScript safe\ninteger range and would be corrupted by a JSON number.",
          "type": "string"
        },
        "title": {
          "type": "string"
        },
        "uniqueId": {
          "type": "string"
        },
        "viewers": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "uniqueId",
        "roomId",
        "nickname",
        "title",
        "viewers"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A bounded page of live rooms. `nextOffset` is absent when the page was\nempty; an empty page is not proof that nobody is broadcasting.",
  "properties": {
    "fetchedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "nextOffset": {
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "rooms": {
      "items": {
        "$ref": "#/$defs/LiveChannel"
      },
      "type": "array"
    },
    "source": {
      "type": "string"
    }
  },
  "required": [
    "rooms",
    "fetchedAt",
    "source"
  ],
  "title": "LiveChannelsResult",
  "type": "object"
}
```

:::

### live.connect {#rpc-live-connect}

Descrição do contrato:

```text
Connects the native TikTok live client
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "roomId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "sessionCookie": {
      "type": "string"
    },
    "uniqueId": {
      "type": "string"
    }
  },
  "required": [
    "uniqueId",
    "sessionCookie"
  ],
  "title": "LiveConnectParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "connected": {
      "type": "boolean"
    },
    "connectionId": {
      "type": [
        "string",
        "null"
      ]
    },
    "native": {
      "type": "boolean"
    },
    "roomId": {
      "type": [
        "string",
        "null"
      ]
    },
    "uniqueId": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "connected",
    "native"
  ],
  "title": "LiveStatus",
  "type": "object"
}
```

:::

### live.disconnect {#rpc-live-disconnect}

Descrição do contrato:

```text
Disconnects the live client
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "connected": {
      "type": "boolean"
    },
    "connectionId": {
      "type": [
        "string",
        "null"
      ]
    },
    "native": {
      "type": "boolean"
    },
    "roomId": {
      "type": [
        "string",
        "null"
      ]
    },
    "uniqueId": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "connected",
    "native"
  ],
  "title": "LiveStatus",
  "type": "object"
}
```

:::

### live.monitor.start {#rpc-live-monitor-start}

Descrição do contrato:

```text
Monitors a creator and connects when LIVE
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pollIntervalSeconds": {
      "default": null,
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "sessionCookie": {
      "default": "",
      "type": "string"
    },
    "uniqueId": {
      "type": "string"
    }
  },
  "required": [
    "uniqueId"
  ],
  "title": "LiveMonitorStartParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "checks": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "enabled": {
      "type": "boolean"
    },
    "failures": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "lastCheckedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "nextCheckAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "phase": {
      "type": "string"
    },
    "pollIntervalSeconds": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "uniqueId": {
      "type": "string"
    }
  },
  "required": [
    "enabled",
    "uniqueId",
    "phase",
    "pollIntervalSeconds",
    "checks",
    "failures"
  ],
  "title": "LiveMonitorStatus",
  "type": "object"
}
```

:::

### live.monitor.status {#rpc-live-monitor-status}

Descrição do contrato:

```text
Current native creator monitor state
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "checks": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "enabled": {
      "type": "boolean"
    },
    "failures": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "lastCheckedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "nextCheckAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "phase": {
      "type": "string"
    },
    "pollIntervalSeconds": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "uniqueId": {
      "type": "string"
    }
  },
  "required": [
    "enabled",
    "uniqueId",
    "phase",
    "pollIntervalSeconds",
    "checks",
    "failures"
  ],
  "title": "LiveMonitorStatus",
  "type": "object"
}
```

:::

### live.monitor.stop {#rpc-live-monitor-stop}

Descrição do contrato:

```text
Stops monitoring and disconnects its LIVE session
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "checks": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "enabled": {
      "type": "boolean"
    },
    "failures": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "lastCheckedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "nextCheckAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "phase": {
      "type": "string"
    },
    "pollIntervalSeconds": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "uniqueId": {
      "type": "string"
    }
  },
  "required": [
    "enabled",
    "uniqueId",
    "phase",
    "pollIntervalSeconds",
    "checks",
    "failures"
  ],
  "title": "LiveMonitorStatus",
  "type": "object"
}
```

:::

### live.pick {#rpc-live-pick}

Descrição do contrato:

```text
Connects to the top live room for a session cookie
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "sessionCookie": {
      "type": "string"
    }
  },
  "required": [
    "sessionCookie"
  ],
  "title": "LivePickParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "connected": {
      "type": "boolean"
    },
    "connectionId": {
      "type": [
        "string",
        "null"
      ]
    },
    "native": {
      "type": "boolean"
    },
    "roomId": {
      "type": [
        "string",
        "null"
      ]
    },
    "uniqueId": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "connected",
    "native"
  ],
  "title": "LiveStatus",
  "type": "object"
}
```

:::

### live.status {#rpc-live-status}

Descrição do contrato:

```text
Live connection status
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "connected": {
      "type": "boolean"
    },
    "connectionId": {
      "type": [
        "string",
        "null"
      ]
    },
    "native": {
      "type": "boolean"
    },
    "roomId": {
      "type": [
        "string",
        "null"
      ]
    },
    "uniqueId": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "connected",
    "native"
  ],
  "title": "LiveStatus",
  "type": "object"
}
```

:::

### media.pick {#rpc-media-pick}

Descrição do contrato:

```text
Opens the native file dialog (desktop hosts only)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":true,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "MediaKind": {
      "enum": [
        "audio",
        "video",
        "image",
        "other"
      ],
      "type": "string"
    },
    "MediaPickerMode": {
      "enum": [
        "file",
        "directory"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "extensions": {
      "default": [],
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "initialDirectory": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "anyOf": [
        {
          "$ref": "#/$defs/MediaKind"
        },
        {
          "type": "null"
        }
      ],
      "default": null
    },
    "mode": {
      "anyOf": [
        {
          "$ref": "#/$defs/MediaPickerMode"
        },
        {
          "type": "null"
        }
      ],
      "default": null
    },
    "title": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "MediaPickParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "MediaDirectoryRef": {
      "properties": {
        "name": {
          "default": "",
          "type": "string"
        },
        "path": {
          "type": "string"
        },
        "referenceVersion": {
          "default": 1,
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "path"
      ],
      "type": "object"
    },
    "MediaFileRef": {
      "properties": {
        "directory": {
          "default": "",
          "type": "string"
        },
        "extension": {
          "default": "",
          "type": "string"
        },
        "kind": {
          "anyOf": [
            {
              "$ref": "#/$defs/MediaKind"
            },
            {
              "type": "null"
            }
          ],
          "default": null
        },
        "mimeType": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "modifiedAt": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "name": {
          "default": "",
          "type": "string"
        },
        "path": {
          "type": "string"
        },
        "referenceVersion": {
          "default": 1,
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "sizeBytes": {
          "default": 0,
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "path"
      ],
      "type": "object"
    },
    "MediaKind": {
      "enum": [
        "audio",
        "video",
        "image",
        "other"
      ],
      "type": "string"
    },
    "MediaSelection": {
      "oneOf": [
        {
          "properties": {
            "file": {
              "$ref": "#/$defs/MediaFileRef"
            },
            "type": {
              "const": "file",
              "type": "string"
            }
          },
          "required": [
            "type",
            "file"
          ],
          "type": "object"
        },
        {
          "properties": {
            "directory": {
              "$ref": "#/$defs/MediaDirectoryRef"
            },
            "type": {
              "const": "directory",
              "type": "string"
            }
          },
          "required": [
            "type",
            "directory"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "selection": {
      "anyOf": [
        {
          "$ref": "#/$defs/MediaSelection"
        },
        {
          "type": "null"
        }
      ]
    }
  },
  "title": "MediaPickResult",
  "type": "object"
}
```

:::

### media.play {#rpc-media-play}

Descrição do contrato:

```text
Plays a local audio file through the host audio backend
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "MediaKind": {
      "enum": [
        "audio",
        "video",
        "image",
        "other"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "anyOf": [
        {
          "$ref": "#/$defs/MediaKind"
        },
        {
          "type": "null"
        }
      ],
      "default": null
    },
    "path": {
      "type": "string"
    },
    "volume": {
      "default": null,
      "format": "float",
      "type": [
        "number",
        "null"
      ]
    }
  },
  "required": [
    "path"
  ],
  "title": "MediaPlayParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "activePlayers": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "played": {
      "type": "boolean"
    },
    "reason": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "played",
    "activePlayers"
  ],
  "title": "AudioPlaybackResult",
  "type": "object"
}
```

:::

### media.validate {#rpc-media-validate}

Descrição do contrato:

```text
Validates a media path and returns its canonical reference
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "MediaKind": {
      "enum": [
        "audio",
        "video",
        "image",
        "other"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "anyOf": [
        {
          "$ref": "#/$defs/MediaKind"
        },
        {
          "type": "null"
        }
      ],
      "default": null
    },
    "path": {
      "type": "string"
    }
  },
  "required": [
    "path"
  ],
  "title": "MediaValidateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "MediaDirectoryRef": {
      "properties": {
        "name": {
          "default": "",
          "type": "string"
        },
        "path": {
          "type": "string"
        },
        "referenceVersion": {
          "default": 1,
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "path"
      ],
      "type": "object"
    },
    "MediaFileRef": {
      "properties": {
        "directory": {
          "default": "",
          "type": "string"
        },
        "extension": {
          "default": "",
          "type": "string"
        },
        "kind": {
          "anyOf": [
            {
              "$ref": "#/$defs/MediaKind"
            },
            {
              "type": "null"
            }
          ],
          "default": null
        },
        "mimeType": {
          "default": null,
          "type": [
            "string",
            "null"
          ]
        },
        "modifiedAt": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "name": {
          "default": "",
          "type": "string"
        },
        "path": {
          "type": "string"
        },
        "referenceVersion": {
          "default": 1,
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "sizeBytes": {
          "default": 0,
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "path"
      ],
      "type": "object"
    },
    "MediaKind": {
      "enum": [
        "audio",
        "video",
        "image",
        "other"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "oneOf": [
    {
      "properties": {
        "file": {
          "$ref": "#/$defs/MediaFileRef"
        },
        "type": {
          "const": "file",
          "type": "string"
        }
      },
      "required": [
        "type",
        "file"
      ],
      "type": "object"
    },
    {
      "properties": {
        "directory": {
          "$ref": "#/$defs/MediaDirectoryRef"
        },
        "type": {
          "const": "directory",
          "type": "string"
        }
      },
      "required": [
        "type",
        "directory"
      ],
      "type": "object"
    }
  ],
  "title": "MediaSelection"
}
```

:::

### plugins.action.execute {#rpc-plugins-action-execute}

Descrição do contrato:

```text
Executes one plugin action (dry run unless live is true)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "actionType": {
      "type": "string"
    },
    "config": {
      "additionalProperties": true,
      "default": {},
      "type": "object"
    },
    "live": {
      "default": false,
      "description": "Dry run by default; `live: true` performs the real execution.",
      "type": "boolean"
    },
    "pluginId": {
      "description": "Requesting plugin id. When present, the host verifies the action\nowner matches before executing (plugin UI broker scoping).",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "actionType"
  ],
  "title": "PluginActionParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "actionType": {
      "type": "string"
    },
    "durationMs": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "logs": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "ok": {
      "type": "boolean"
    },
    "summary": {
      "type": "string"
    }
  },
  "required": [
    "actionType",
    "ok",
    "summary",
    "logs",
    "durationMs"
  ],
  "title": "PluginActionOutcome",
  "type": "object"
}
```

:::

### plugins.diagnostics {#rpc-plugins-diagnostics}

Descrição do contrato:

```text
In-memory plugin runtime diagnostics: last event, poll drops, hotkey sync state
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### plugins.disable {#rpc-plugins-disable}

Descrição do contrato:

```text
Stops a plugin runtime and marks it disabled
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### plugins.enable {#rpc-plugins-enable}

Descrição do contrato:

```text
Marks a plugin installed+enabled and starts its runtime
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### plugins.get {#rpc-plugins-get}

Descrição do contrato:

```text
One plugin summary by id
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "plugin": true
  },
  "required": [
    "plugin"
  ],
  "title": "PluginResult",
  "type": "object"
}
```

:::

### plugins.health {#rpc-plugins-health}

Descrição do contrato:

```text
Probes a plugin health endpoint (schema v3 http.health)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "details": {
      "type": [
        "string",
        "null"
      ]
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "gameplayReady": {
      "type": [
        "boolean",
        "null"
      ]
    },
    "latencyMs": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "ok": {
      "type": "boolean"
    },
    "pluginId": {
      "type": "string"
    },
    "reachable": {
      "default": false,
      "type": "boolean"
    },
    "serverInfo": true
  },
  "required": [
    "pluginId",
    "ok",
    "latencyMs"
  ],
  "title": "PluginConnectionResult",
  "type": "object"
}
```

:::

### plugins.install {#rpc-plugins-install}

Descrição do contrato:

```text
Installs a .plugin archive (identity comes from plugin.json)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "path": {
      "type": "string"
    },
    "replaceExisting": {
      "default": false,
      "type": "boolean"
    }
  },
  "required": [
    "path"
  ],
  "title": "PluginInstallParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "directory": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "replaced": {
      "type": "boolean"
    },
    "version": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "version",
    "directory",
    "replaced"
  ],
  "title": "PluginInstallResult",
  "type": "object"
}
```

:::

### plugins.install.set {#rpc-plugins-install-set}

Descrição do contrato:

```text
Logical installed-state toggle (never touches the filesystem)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "installed": {
      "type": "boolean"
    },
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId",
    "installed"
  ],
  "title": "PluginInstallSetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### plugins.jobs.cancel {#rpc-plugins-jobs-cancel}

Descrição do contrato:

```text
Requests cancellation of a running plugin package job
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "jobId": {
      "type": "string"
    }
  },
  "required": [
    "jobId"
  ],
  "title": "JobParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "cancelled": {
      "type": "boolean"
    }
  },
  "required": [
    "cancelled"
  ],
  "title": "JobCancelResult",
  "type": "object"
}
```

:::

### plugins.jobs.get {#rpc-plugins-jobs-get}

Descrição do contrato:

```text
One plugin package job
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "jobId": {
      "type": "string"
    }
  },
  "required": [
    "jobId"
  ],
  "title": "JobParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "InstallError": {
      "properties": {
        "code": {
          "$ref": "#/$defs/InstallErrorCode"
        },
        "details": true,
        "message": {
          "type": "string"
        }
      },
      "required": [
        "code",
        "message"
      ],
      "type": "object"
    },
    "InstallErrorCode": {
      "enum": [
        "already-installed",
        "not-installed",
        "not-found",
        "no-compatible-release",
        "download-failed",
        "digest-mismatch",
        "signature-required",
        "signature-invalid",
        "untrusted-publisher",
        "revoked",
        "invalid-package",
        "incompatible",
        "consent-required",
        "downgrade-blocked",
        "busy",
        "cancelled",
        "io-error",
        "unsupported"
      ],
      "type": "string"
    },
    "JobKind": {
      "enum": [
        "install",
        "update",
        "uninstall",
        "rollback"
      ],
      "type": "string"
    },
    "JobProgress": {
      "properties": {
        "done": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "total": {
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        }
      },
      "required": [
        "done"
      ],
      "type": "object"
    },
    "JobSnapshot": {
      "properties": {
        "createdAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "error": {
          "anyOf": [
            {
              "$ref": "#/$defs/InstallError"
            },
            {
              "type": "null"
            }
          ]
        },
        "fromVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/JobKind"
        },
        "pluginId": {
          "type": [
            "string",
            "null"
          ]
        },
        "progress": {
          "anyOf": [
            {
              "$ref": "#/$defs/JobProgress"
            },
            {
              "type": "null"
            }
          ]
        },
        "spec": {
          "type": [
            "string",
            "null"
          ]
        },
        "stage": {
          "$ref": "#/$defs/JobStage"
        },
        "status": {
          "$ref": "#/$defs/JobStatus"
        },
        "toVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "id",
        "kind",
        "stage",
        "status",
        "createdAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "JobStage": {
      "enum": [
        "queued",
        "resolve",
        "download",
        "verify",
        "extract",
        "activate",
        "cleanup",
        "done"
      ],
      "type": "string"
    },
    "JobStatus": {
      "enum": [
        "queued",
        "running",
        "succeeded",
        "failed",
        "cancelled"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "job": {
      "$ref": "#/$defs/JobSnapshot"
    }
  },
  "required": [
    "job"
  ],
  "title": "JobResult",
  "type": "object"
}
```

:::

### plugins.jobs.list {#rpc-plugins-jobs-list}

Descrição do contrato:

```text
Recent plugin package jobs, newest first
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "InstallError": {
      "properties": {
        "code": {
          "$ref": "#/$defs/InstallErrorCode"
        },
        "details": true,
        "message": {
          "type": "string"
        }
      },
      "required": [
        "code",
        "message"
      ],
      "type": "object"
    },
    "InstallErrorCode": {
      "enum": [
        "already-installed",
        "not-installed",
        "not-found",
        "no-compatible-release",
        "download-failed",
        "digest-mismatch",
        "signature-required",
        "signature-invalid",
        "untrusted-publisher",
        "revoked",
        "invalid-package",
        "incompatible",
        "consent-required",
        "downgrade-blocked",
        "busy",
        "cancelled",
        "io-error",
        "unsupported"
      ],
      "type": "string"
    },
    "JobKind": {
      "enum": [
        "install",
        "update",
        "uninstall",
        "rollback"
      ],
      "type": "string"
    },
    "JobProgress": {
      "properties": {
        "done": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "total": {
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        }
      },
      "required": [
        "done"
      ],
      "type": "object"
    },
    "JobSnapshot": {
      "properties": {
        "createdAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "error": {
          "anyOf": [
            {
              "$ref": "#/$defs/InstallError"
            },
            {
              "type": "null"
            }
          ]
        },
        "fromVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/JobKind"
        },
        "pluginId": {
          "type": [
            "string",
            "null"
          ]
        },
        "progress": {
          "anyOf": [
            {
              "$ref": "#/$defs/JobProgress"
            },
            {
              "type": "null"
            }
          ]
        },
        "spec": {
          "type": [
            "string",
            "null"
          ]
        },
        "stage": {
          "$ref": "#/$defs/JobStage"
        },
        "status": {
          "$ref": "#/$defs/JobStatus"
        },
        "toVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "id",
        "kind",
        "stage",
        "status",
        "createdAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "JobStage": {
      "enum": [
        "queued",
        "resolve",
        "download",
        "verify",
        "extract",
        "activate",
        "cleanup",
        "done"
      ],
      "type": "string"
    },
    "JobStatus": {
      "enum": [
        "queued",
        "running",
        "succeeded",
        "failed",
        "cancelled"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "jobs": {
      "items": {
        "$ref": "#/$defs/JobSnapshot"
      },
      "type": "array"
    }
  },
  "required": [
    "jobs"
  ],
  "title": "JobListResult",
  "type": "object"
}
```

:::

### plugins.list {#rpc-plugins-list}

Descrição do contrato:

```text
Safe plugin summaries (never includes settings values)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "plugins": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "plugins"
  ],
  "title": "PluginListResult",
  "type": "object"
}
```

:::

### plugins.options {#rpc-plugins-options}

Descrição do contrato:

```text
Resolves action-type/field option documents
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "description": "Requesting plugin id. When present, the host verifies the option\nsource owner matches before resolving (plugin UI broker scoping).",
      "type": [
        "string",
        "null"
      ]
    },
    "refresh": {
      "default": false,
      "description": "Bypass the option cache for one read (manual Refresh, post-mutation\nre-read). Defaults to false: normal reads stay cached.",
      "type": "boolean"
    },
    "source": {
      "type": "string"
    }
  },
  "required": [
    "source"
  ],
  "title": "PluginOptionsParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "options": {
      "items": true,
      "type": "array"
    },
    "selected": {
      "type": [
        "string",
        "null"
      ]
    },
    "source": {
      "type": "string"
    }
  },
  "required": [
    "source",
    "options"
  ],
  "title": "PluginOptionsResult",
  "type": "object"
}
```

:::

### plugins.packages.inspect {#rpc-plugins-packages-inspect}

Descrição do contrato:

```text
Resolves, downloads, and verifies a package spec and returns its review (permissions, signature, consent token)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "PackageSpecInput": {
      "oneOf": [
        {
          "properties": {
            "kind": {
              "const": "file",
              "type": "string"
            },
            "path": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "path"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "url",
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "id": {
              "type": "string"
            },
            "kind": {
              "const": "registry",
              "type": "string"
            },
            "registry": {
              "type": [
                "string",
                "null"
              ]
            },
            "version": {
              "type": [
                "string",
                "null"
              ]
            }
          },
          "required": [
            "kind",
            "id"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "spec",
              "type": "string"
            },
            "spec": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "spec"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "spec": {
      "$ref": "#/$defs/PackageSpecInput"
    }
  },
  "required": [
    "spec"
  ],
  "title": "PackageInspectParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "ChangeKind": {
      "enum": [
        "install",
        "update",
        "reinstall",
        "downgrade"
      ],
      "type": "string"
    },
    "PackageOrigin": {
      "oneOf": [
        {
          "properties": {
            "kind": {
              "const": "registry",
              "type": "string"
            },
            "registry": {
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "registry",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "url",
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "file",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "builtin",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "development",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "adopted",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        }
      ]
    },
    "PermissionDiff": {
      "properties": {
        "addedCapabilities": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "addedPermissions": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "removedCapabilities": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "removedPermissions": {
          "items": {
            "type": "string"
          },
          "type": "array"
        }
      },
      "required": [
        "addedPermissions",
        "removedPermissions",
        "addedCapabilities",
        "removedCapabilities"
      ],
      "type": "object"
    },
    "SecurityModel": {
      "enum": [
        "trusted",
        "isolated",
        "sandboxed"
      ],
      "type": "string"
    },
    "SignatureState": {
      "properties": {
        "keyId": {
          "type": [
            "string",
            "null"
          ]
        },
        "publisher": {
          "type": [
            "string",
            "null"
          ]
        },
        "reason": {
          "type": [
            "string",
            "null"
          ]
        },
        "source": {
          "type": [
            "string",
            "null"
          ]
        },
        "status": {
          "$ref": "#/$defs/SignatureStatus"
        }
      },
      "required": [
        "status"
      ],
      "type": "object"
    },
    "SignatureStatus": {
      "enum": [
        "verified",
        "unsigned",
        "untrusted"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "capabilities": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "change": {
      "$ref": "#/$defs/ChangeKind"
    },
    "changelog": {
      "type": [
        "string",
        "null"
      ]
    },
    "consentToken": {
      "type": "string"
    },
    "description": {
      "type": [
        "string",
        "null"
      ]
    },
    "digest": {
      "type": "string"
    },
    "icon": {
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "type": "string"
    },
    "incompatibility": true,
    "installedVersion": {
      "type": [
        "string",
        "null"
      ]
    },
    "metadata": true,
    "name": {
      "type": "string"
    },
    "origin": {
      "$ref": "#/$defs/PackageOrigin"
    },
    "permissionDiff": {
      "$ref": "#/$defs/PermissionDiff"
    },
    "permissions": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "requiresSideloadConsent": {
      "type": "boolean"
    },
    "runtime": {
      "type": "string"
    },
    "securityModel": {
      "$ref": "#/$defs/SecurityModel"
    },
    "signature": {
      "$ref": "#/$defs/SignatureState"
    },
    "size": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "version": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "name",
    "version",
    "runtime",
    "securityModel",
    "permissions",
    "capabilities",
    "size",
    "digest",
    "origin",
    "signature",
    "change",
    "permissionDiff",
    "requiresSideloadConsent",
    "metadata",
    "consentToken"
  ],
  "title": "PackagePreview",
  "type": "object"
}
```

:::

### plugins.packages.install {#rpc-plugins-packages-install}

Descrição do contrato:

```text
Starts an install job for a reviewed package; the consent token comes from plugins.packages.inspect
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutoUpdate": {
      "enum": [
        "off",
        "notify",
        "patch",
        "minor"
      ],
      "type": "string"
    },
    "PackageSpecInput": {
      "oneOf": [
        {
          "properties": {
            "kind": {
              "const": "file",
              "type": "string"
            },
            "path": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "path"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "url",
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "id": {
              "type": "string"
            },
            "kind": {
              "const": "registry",
              "type": "string"
            },
            "registry": {
              "type": [
                "string",
                "null"
              ]
            },
            "version": {
              "type": [
                "string",
                "null"
              ]
            }
          },
          "required": [
            "kind",
            "id"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "spec",
              "type": "string"
            },
            "spec": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "spec"
          ],
          "type": "object"
        }
      ]
    },
    "UpdatePolicy": {
      "properties": {
        "autoUpdate": {
          "$ref": "#/$defs/AutoUpdate",
          "default": "notify"
        },
        "channel": {
          "default": "stable",
          "type": "string"
        },
        "pinned": {
          "default": false,
          "type": "boolean"
        }
      },
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "consentToken": {
      "type": "string"
    },
    "policy": {
      "anyOf": [
        {
          "$ref": "#/$defs/UpdatePolicy"
        },
        {
          "type": "null"
        }
      ]
    },
    "spec": {
      "$ref": "#/$defs/PackageSpecInput"
    }
  },
  "required": [
    "spec",
    "consentToken"
  ],
  "title": "PackageInstallParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "InstallError": {
      "properties": {
        "code": {
          "$ref": "#/$defs/InstallErrorCode"
        },
        "details": true,
        "message": {
          "type": "string"
        }
      },
      "required": [
        "code",
        "message"
      ],
      "type": "object"
    },
    "InstallErrorCode": {
      "enum": [
        "already-installed",
        "not-installed",
        "not-found",
        "no-compatible-release",
        "download-failed",
        "digest-mismatch",
        "signature-required",
        "signature-invalid",
        "untrusted-publisher",
        "revoked",
        "invalid-package",
        "incompatible",
        "consent-required",
        "downgrade-blocked",
        "busy",
        "cancelled",
        "io-error",
        "unsupported"
      ],
      "type": "string"
    },
    "JobKind": {
      "enum": [
        "install",
        "update",
        "uninstall",
        "rollback"
      ],
      "type": "string"
    },
    "JobProgress": {
      "properties": {
        "done": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "total": {
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        }
      },
      "required": [
        "done"
      ],
      "type": "object"
    },
    "JobSnapshot": {
      "properties": {
        "createdAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "error": {
          "anyOf": [
            {
              "$ref": "#/$defs/InstallError"
            },
            {
              "type": "null"
            }
          ]
        },
        "fromVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/JobKind"
        },
        "pluginId": {
          "type": [
            "string",
            "null"
          ]
        },
        "progress": {
          "anyOf": [
            {
              "$ref": "#/$defs/JobProgress"
            },
            {
              "type": "null"
            }
          ]
        },
        "spec": {
          "type": [
            "string",
            "null"
          ]
        },
        "stage": {
          "$ref": "#/$defs/JobStage"
        },
        "status": {
          "$ref": "#/$defs/JobStatus"
        },
        "toVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "id",
        "kind",
        "stage",
        "status",
        "createdAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "JobStage": {
      "enum": [
        "queued",
        "resolve",
        "download",
        "verify",
        "extract",
        "activate",
        "cleanup",
        "done"
      ],
      "type": "string"
    },
    "JobStatus": {
      "enum": [
        "queued",
        "running",
        "succeeded",
        "failed",
        "cancelled"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "job": {
      "$ref": "#/$defs/JobSnapshot"
    }
  },
  "required": [
    "job"
  ],
  "title": "JobResult",
  "type": "object"
}
```

:::

### plugins.packages.policy {#rpc-plugins-packages-policy}

Descrição do contrato:

```text
Updates a plugin's update channel, auto-update rule, or pin
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutoUpdate": {
      "enum": [
        "off",
        "notify",
        "patch",
        "minor"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "autoUpdate": {
      "anyOf": [
        {
          "$ref": "#/$defs/AutoUpdate"
        },
        {
          "type": "null"
        }
      ]
    },
    "channel": {
      "type": [
        "string",
        "null"
      ]
    },
    "pinned": {
      "type": [
        "boolean",
        "null"
      ]
    },
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PackagePolicyParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutoUpdate": {
      "enum": [
        "off",
        "notify",
        "patch",
        "minor"
      ],
      "type": "string"
    },
    "CloudArtifactIdentity": {
      "description": "Artifact provenance recorded by the trusted Cloud install path after its\nsigned metadata and current generation have been verified. Older receipts\nintentionally have no such proof and cannot infer it from today’s index.",
      "properties": {
        "generation": {
          "description": "Decimal string preserves backend generation precision across clients.",
          "type": "string"
        },
        "target": {
          "type": "string"
        }
      },
      "required": [
        "target",
        "generation"
      ],
      "type": "object"
    },
    "InstallReceipt": {
      "properties": {
        "archiveDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "cloudArtifact": {
          "anyOf": [
            {
              "$ref": "#/$defs/CloudArtifactIdentity"
            },
            {
              "type": "null"
            }
          ]
        },
        "grantedCapabilities": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "grantedPermissions": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "id": {
          "type": "string"
        },
        "installedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "origin": {
          "$ref": "#/$defs/PackageOrigin"
        },
        "policy": {
          "$ref": "#/$defs/UpdatePolicy",
          "default": {
            "autoUpdate": "notify",
            "channel": "stable",
            "pinned": false
          }
        },
        "retained": {
          "default": [],
          "items": {
            "$ref": "#/$defs/RetainedVersion"
          },
          "type": "array"
        },
        "runtime": {
          "type": "string"
        },
        "signature": {
          "$ref": "#/$defs/SignatureState"
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "version": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "version",
        "origin",
        "signature",
        "runtime",
        "installedAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "PackageOrigin": {
      "oneOf": [
        {
          "properties": {
            "kind": {
              "const": "registry",
              "type": "string"
            },
            "registry": {
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "registry",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "url",
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "file",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "builtin",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "development",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "adopted",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        }
      ]
    },
    "RetainedVersion": {
      "properties": {
        "retainedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "version": {
          "type": "string"
        }
      },
      "required": [
        "version",
        "retainedAt"
      ],
      "type": "object"
    },
    "SignatureState": {
      "properties": {
        "keyId": {
          "type": [
            "string",
            "null"
          ]
        },
        "publisher": {
          "type": [
            "string",
            "null"
          ]
        },
        "reason": {
          "type": [
            "string",
            "null"
          ]
        },
        "source": {
          "type": [
            "string",
            "null"
          ]
        },
        "status": {
          "$ref": "#/$defs/SignatureStatus"
        }
      },
      "required": [
        "status"
      ],
      "type": "object"
    },
    "SignatureStatus": {
      "enum": [
        "verified",
        "unsigned",
        "untrusted"
      ],
      "type": "string"
    },
    "UpdatePolicy": {
      "properties": {
        "autoUpdate": {
          "$ref": "#/$defs/AutoUpdate",
          "default": "notify"
        },
        "channel": {
          "default": "stable",
          "type": "string"
        },
        "pinned": {
          "default": false,
          "type": "boolean"
        }
      },
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "receipt": {
      "$ref": "#/$defs/InstallReceipt"
    }
  },
  "required": [
    "receipt"
  ],
  "title": "ReceiptResult",
  "type": "object"
}
```

:::

### plugins.packages.receipts {#rpc-plugins-packages-receipts}

Descrição do contrato:

```text
Install receipts (origin, signature, granted access, update policy) for managed plugins
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutoUpdate": {
      "enum": [
        "off",
        "notify",
        "patch",
        "minor"
      ],
      "type": "string"
    },
    "CloudArtifactIdentity": {
      "description": "Artifact provenance recorded by the trusted Cloud install path after its\nsigned metadata and current generation have been verified. Older receipts\nintentionally have no such proof and cannot infer it from today’s index.",
      "properties": {
        "generation": {
          "description": "Decimal string preserves backend generation precision across clients.",
          "type": "string"
        },
        "target": {
          "type": "string"
        }
      },
      "required": [
        "target",
        "generation"
      ],
      "type": "object"
    },
    "InstallReceipt": {
      "properties": {
        "archiveDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "cloudArtifact": {
          "anyOf": [
            {
              "$ref": "#/$defs/CloudArtifactIdentity"
            },
            {
              "type": "null"
            }
          ]
        },
        "grantedCapabilities": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "grantedPermissions": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "id": {
          "type": "string"
        },
        "installedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "origin": {
          "$ref": "#/$defs/PackageOrigin"
        },
        "policy": {
          "$ref": "#/$defs/UpdatePolicy",
          "default": {
            "autoUpdate": "notify",
            "channel": "stable",
            "pinned": false
          }
        },
        "retained": {
          "default": [],
          "items": {
            "$ref": "#/$defs/RetainedVersion"
          },
          "type": "array"
        },
        "runtime": {
          "type": "string"
        },
        "signature": {
          "$ref": "#/$defs/SignatureState"
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "version": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "version",
        "origin",
        "signature",
        "runtime",
        "installedAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "PackageOrigin": {
      "oneOf": [
        {
          "properties": {
            "kind": {
              "const": "registry",
              "type": "string"
            },
            "registry": {
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "registry",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "url",
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "file",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "builtin",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "development",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "adopted",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        }
      ]
    },
    "RetainedVersion": {
      "properties": {
        "retainedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "version": {
          "type": "string"
        }
      },
      "required": [
        "version",
        "retainedAt"
      ],
      "type": "object"
    },
    "SignatureState": {
      "properties": {
        "keyId": {
          "type": [
            "string",
            "null"
          ]
        },
        "publisher": {
          "type": [
            "string",
            "null"
          ]
        },
        "reason": {
          "type": [
            "string",
            "null"
          ]
        },
        "source": {
          "type": [
            "string",
            "null"
          ]
        },
        "status": {
          "$ref": "#/$defs/SignatureStatus"
        }
      },
      "required": [
        "status"
      ],
      "type": "object"
    },
    "SignatureStatus": {
      "enum": [
        "verified",
        "unsigned",
        "untrusted"
      ],
      "type": "string"
    },
    "UpdatePolicy": {
      "properties": {
        "autoUpdate": {
          "$ref": "#/$defs/AutoUpdate",
          "default": "notify"
        },
        "channel": {
          "default": "stable",
          "type": "string"
        },
        "pinned": {
          "default": false,
          "type": "boolean"
        }
      },
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "receipts": {
      "items": {
        "$ref": "#/$defs/InstallReceipt"
      },
      "type": "array"
    }
  },
  "required": [
    "receipts"
  ],
  "title": "ReceiptListResult",
  "type": "object"
}
```

:::

### plugins.packages.rollback {#rpc-plugins-packages-rollback}

Descrição do contrato:

```text
Restores a retained previous version of a plugin and pins it
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    },
    "version": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PackageRollbackParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "InstallError": {
      "properties": {
        "code": {
          "$ref": "#/$defs/InstallErrorCode"
        },
        "details": true,
        "message": {
          "type": "string"
        }
      },
      "required": [
        "code",
        "message"
      ],
      "type": "object"
    },
    "InstallErrorCode": {
      "enum": [
        "already-installed",
        "not-installed",
        "not-found",
        "no-compatible-release",
        "download-failed",
        "digest-mismatch",
        "signature-required",
        "signature-invalid",
        "untrusted-publisher",
        "revoked",
        "invalid-package",
        "incompatible",
        "consent-required",
        "downgrade-blocked",
        "busy",
        "cancelled",
        "io-error",
        "unsupported"
      ],
      "type": "string"
    },
    "JobKind": {
      "enum": [
        "install",
        "update",
        "uninstall",
        "rollback"
      ],
      "type": "string"
    },
    "JobProgress": {
      "properties": {
        "done": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "total": {
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        }
      },
      "required": [
        "done"
      ],
      "type": "object"
    },
    "JobSnapshot": {
      "properties": {
        "createdAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "error": {
          "anyOf": [
            {
              "$ref": "#/$defs/InstallError"
            },
            {
              "type": "null"
            }
          ]
        },
        "fromVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/JobKind"
        },
        "pluginId": {
          "type": [
            "string",
            "null"
          ]
        },
        "progress": {
          "anyOf": [
            {
              "$ref": "#/$defs/JobProgress"
            },
            {
              "type": "null"
            }
          ]
        },
        "spec": {
          "type": [
            "string",
            "null"
          ]
        },
        "stage": {
          "$ref": "#/$defs/JobStage"
        },
        "status": {
          "$ref": "#/$defs/JobStatus"
        },
        "toVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "id",
        "kind",
        "stage",
        "status",
        "createdAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "JobStage": {
      "enum": [
        "queued",
        "resolve",
        "download",
        "verify",
        "extract",
        "activate",
        "cleanup",
        "done"
      ],
      "type": "string"
    },
    "JobStatus": {
      "enum": [
        "queued",
        "running",
        "succeeded",
        "failed",
        "cancelled"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "job": {
      "$ref": "#/$defs/JobSnapshot"
    }
  },
  "required": [
    "job"
  ],
  "title": "JobResult",
  "type": "object"
}
```

:::

### plugins.packages.update {#rpc-plugins-packages-update}

Descrição do contrato:

```text
Starts an update job for a registry plugin; omitting the consent token only applies updates that add no access
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "consentToken": {
      "type": [
        "string",
        "null"
      ]
    },
    "pluginId": {
      "type": "string"
    },
    "version": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PackageUpdateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "InstallError": {
      "properties": {
        "code": {
          "$ref": "#/$defs/InstallErrorCode"
        },
        "details": true,
        "message": {
          "type": "string"
        }
      },
      "required": [
        "code",
        "message"
      ],
      "type": "object"
    },
    "InstallErrorCode": {
      "enum": [
        "already-installed",
        "not-installed",
        "not-found",
        "no-compatible-release",
        "download-failed",
        "digest-mismatch",
        "signature-required",
        "signature-invalid",
        "untrusted-publisher",
        "revoked",
        "invalid-package",
        "incompatible",
        "consent-required",
        "downgrade-blocked",
        "busy",
        "cancelled",
        "io-error",
        "unsupported"
      ],
      "type": "string"
    },
    "JobKind": {
      "enum": [
        "install",
        "update",
        "uninstall",
        "rollback"
      ],
      "type": "string"
    },
    "JobProgress": {
      "properties": {
        "done": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "total": {
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        }
      },
      "required": [
        "done"
      ],
      "type": "object"
    },
    "JobSnapshot": {
      "properties": {
        "createdAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "error": {
          "anyOf": [
            {
              "$ref": "#/$defs/InstallError"
            },
            {
              "type": "null"
            }
          ]
        },
        "fromVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "$ref": "#/$defs/JobKind"
        },
        "pluginId": {
          "type": [
            "string",
            "null"
          ]
        },
        "progress": {
          "anyOf": [
            {
              "$ref": "#/$defs/JobProgress"
            },
            {
              "type": "null"
            }
          ]
        },
        "spec": {
          "type": [
            "string",
            "null"
          ]
        },
        "stage": {
          "$ref": "#/$defs/JobStage"
        },
        "status": {
          "$ref": "#/$defs/JobStatus"
        },
        "toVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "id",
        "kind",
        "stage",
        "status",
        "createdAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "JobStage": {
      "enum": [
        "queued",
        "resolve",
        "download",
        "verify",
        "extract",
        "activate",
        "cleanup",
        "done"
      ],
      "type": "string"
    },
    "JobStatus": {
      "enum": [
        "queued",
        "running",
        "succeeded",
        "failed",
        "cancelled"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "job": {
      "$ref": "#/$defs/JobSnapshot"
    }
  },
  "required": [
    "job"
  ],
  "title": "JobResult",
  "type": "object"
}
```

:::

### plugins.packages.versions {#rpc-plugins-packages-versions}

Descrição do contrato:

```text
Installed, retained, and registry-available versions of one plugin
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PackagePluginParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Advisory": {
      "properties": {
        "message": {
          "type": "string"
        },
        "severity": {
          "$ref": "#/$defs/AdvisorySeverity"
        },
        "url": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "severity",
        "message"
      ],
      "type": "object"
    },
    "AdvisorySeverity": {
      "enum": [
        "low",
        "moderate",
        "high",
        "critical"
      ],
      "type": "string"
    },
    "AutoUpdate": {
      "enum": [
        "off",
        "notify",
        "patch",
        "minor"
      ],
      "type": "string"
    },
    "AvailableRelease": {
      "properties": {
        "advisory": {
          "anyOf": [
            {
              "$ref": "#/$defs/Advisory"
            },
            {
              "type": "null"
            }
          ]
        },
        "changelog": {
          "type": [
            "string",
            "null"
          ]
        },
        "channel": {
          "type": "string"
        },
        "compatible": {
          "type": "boolean"
        },
        "liveStatus": {
          "$ref": "#/$defs/LiveReleaseStatus",
          "default": "unknown"
        },
        "publishedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "version": {
          "type": "string"
        },
        "yanked": {
          "type": "boolean"
        }
      },
      "required": [
        "version",
        "channel",
        "publishedAt",
        "yanked",
        "compatible"
      ],
      "type": "object"
    },
    "CloudArtifactIdentity": {
      "description": "Artifact provenance recorded by the trusted Cloud install path after its\nsigned metadata and current generation have been verified. Older receipts\nintentionally have no such proof and cannot infer it from today’s index.",
      "properties": {
        "generation": {
          "description": "Decimal string preserves backend generation precision across clients.",
          "type": "string"
        },
        "target": {
          "type": "string"
        }
      },
      "required": [
        "target",
        "generation"
      ],
      "type": "object"
    },
    "InstallReceipt": {
      "properties": {
        "archiveDigest": {
          "type": [
            "string",
            "null"
          ]
        },
        "cloudArtifact": {
          "anyOf": [
            {
              "$ref": "#/$defs/CloudArtifactIdentity"
            },
            {
              "type": "null"
            }
          ]
        },
        "grantedCapabilities": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "grantedPermissions": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "id": {
          "type": "string"
        },
        "installedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "origin": {
          "$ref": "#/$defs/PackageOrigin"
        },
        "policy": {
          "$ref": "#/$defs/UpdatePolicy",
          "default": {
            "autoUpdate": "notify",
            "channel": "stable",
            "pinned": false
          }
        },
        "retained": {
          "default": [],
          "items": {
            "$ref": "#/$defs/RetainedVersion"
          },
          "type": "array"
        },
        "runtime": {
          "type": "string"
        },
        "signature": {
          "$ref": "#/$defs/SignatureState"
        },
        "updatedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "version": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "version",
        "origin",
        "signature",
        "runtime",
        "installedAt",
        "updatedAt"
      ],
      "type": "object"
    },
    "LiveReleaseStatus": {
      "description": "Current Cloud status of the exact host artifact, separate from the\nimmutable signed snapshot. `Unknown` never authorizes installation.",
      "enum": [
        "unknown",
        "approved",
        "yanked",
        "key-revoked",
        "publisher-unapproved",
        "advisory",
        "incompatible"
      ],
      "type": "string"
    },
    "PackageOrigin": {
      "oneOf": [
        {
          "properties": {
            "kind": {
              "const": "registry",
              "type": "string"
            },
            "registry": {
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "registry",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "url",
              "type": "string"
            },
            "url": {
              "type": "string"
            }
          },
          "required": [
            "kind",
            "url"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "file",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "builtin",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "development",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        },
        {
          "properties": {
            "kind": {
              "const": "adopted",
              "type": "string"
            }
          },
          "required": [
            "kind"
          ],
          "type": "object"
        }
      ]
    },
    "RetainedVersion": {
      "properties": {
        "retainedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "version": {
          "type": "string"
        }
      },
      "required": [
        "version",
        "retainedAt"
      ],
      "type": "object"
    },
    "SignatureState": {
      "properties": {
        "keyId": {
          "type": [
            "string",
            "null"
          ]
        },
        "publisher": {
          "type": [
            "string",
            "null"
          ]
        },
        "reason": {
          "type": [
            "string",
            "null"
          ]
        },
        "source": {
          "type": [
            "string",
            "null"
          ]
        },
        "status": {
          "$ref": "#/$defs/SignatureStatus"
        }
      },
      "required": [
        "status"
      ],
      "type": "object"
    },
    "SignatureStatus": {
      "enum": [
        "verified",
        "unsigned",
        "untrusted"
      ],
      "type": "string"
    },
    "UpdatePolicy": {
      "properties": {
        "autoUpdate": {
          "$ref": "#/$defs/AutoUpdate",
          "default": "notify"
        },
        "channel": {
          "default": "stable",
          "type": "string"
        },
        "pinned": {
          "default": false,
          "type": "boolean"
        }
      },
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "available": {
      "items": {
        "$ref": "#/$defs/AvailableRelease"
      },
      "type": "array"
    },
    "id": {
      "type": "string"
    },
    "installed": {
      "type": [
        "string",
        "null"
      ]
    },
    "receipt": {
      "anyOf": [
        {
          "$ref": "#/$defs/InstallReceipt"
        },
        {
          "type": "null"
        }
      ]
    },
    "registry": {
      "type": [
        "string",
        "null"
      ]
    },
    "registryError": {
      "type": [
        "string",
        "null"
      ]
    },
    "retained": {
      "items": {
        "$ref": "#/$defs/RetainedVersion"
      },
      "type": "array"
    }
  },
  "required": [
    "id",
    "retained",
    "available"
  ],
  "title": "PluginVersions",
  "type": "object"
}
```

:::

### plugins.settings.get {#rpc-plugins-settings-get}

Descrição do contrato:

```text
Plugin settings schema plus redacted values
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginSettingsGet",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    },
    "schema": true,
    "uiHints": true,
    "values": {
      "additionalProperties": true,
      "type": "object"
    }
  },
  "required": [
    "pluginId",
    "schema",
    "values"
  ],
  "title": "PluginSettingsResult",
  "type": "object"
}
```

:::

### plugins.settings.reset {#rpc-plugins-settings-reset}

Descrição do contrato:

```text
Deletes stored settings so schema defaults apply again
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginSettingsGet",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    },
    "schema": true,
    "uiHints": true,
    "values": {
      "additionalProperties": true,
      "type": "object"
    }
  },
  "required": [
    "pluginId",
    "schema",
    "values"
  ],
  "title": "PluginSettingsResult",
  "type": "object"
}
```

:::

### plugins.settings.set {#rpc-plugins-settings-set}

Descrição do contrato:

```text
Updates plugin settings (placeholder preserves stored secrets)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    },
    "values": {
      "additionalProperties": true,
      "type": "object"
    }
  },
  "required": [
    "pluginId",
    "values"
  ],
  "title": "PluginSettingsSet",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    },
    "schema": true,
    "uiHints": true,
    "values": {
      "additionalProperties": true,
      "type": "object"
    }
  },
  "required": [
    "pluginId",
    "schema",
    "values"
  ],
  "title": "PluginSettingsResult",
  "type": "object"
}
```

:::

### plugins.start {#rpc-plugins-start}

Descrição do contrato:

```text
Starts a plugin runtime without changing persisted state
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### plugins.stop {#rpc-plugins-stop}

Descrição do contrato:

```text
Stops a plugin runtime without changing persisted state
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### plugins.token.import {#rpc-plugins-token-import}

Descrição do contrato:

```text
Imports a secret token from a user-selected local configuration via the native picker
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "candidateId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "mode": {
      "default": "file",
      "type": "string"
    },
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginTokenImportParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "TokenImportCandidate": {
      "properties": {
        "id": {
          "type": "string"
        },
        "label": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "label"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "candidates": {
      "items": {
        "$ref": "#/$defs/TokenImportCandidate"
      },
      "type": "array"
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "ok": {
      "type": "boolean"
    },
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId",
    "ok"
  ],
  "title": "TokenImportResult",
  "type": "object"
}
```

:::

### plugins.token.provision {#rpc-plugins-token-provision}

Descrição do contrato:

```text
Mints an API token via the plugin provisioning flow (password is used once, never stored)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "password": {
      "type": "string"
    },
    "pluginId": {
      "type": "string"
    },
    "username": {
      "type": "string"
    }
  },
  "required": [
    "pluginId",
    "username",
    "password"
  ],
  "title": "PluginProvisionParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "ok": {
      "type": "boolean"
    },
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId",
    "ok"
  ],
  "title": "PluginProvisionResult",
  "type": "object"
}
```

:::

### plugins.ui.assets {#rpc-plugins-ui-assets}

Descrição do contrato:

```text
Validated plugin UI assets for local transports
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Validated UI asset root for trusted local transport adapters, not a file API.",
  "properties": {
    "assetRoot": {
      "type": "string"
    },
    "entryFile": {
      "type": "string"
    },
    "pageIds": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "pluginId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId",
    "assetRoot",
    "entryFile",
    "pageIds"
  ],
  "title": "PluginUiAssetsResult",
  "type": "object"
}
```

:::

### plugins.uninstall {#rpc-plugins-uninstall}

Descrição do contrato:

```text
Removes a user-installed plugin package, its retained versions, and optionally its data
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    },
    "purgeData": {
      "default": false,
      "type": "boolean"
    }
  },
  "required": [
    "pluginId"
  ],
  "title": "PluginUninstallParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### plugins.updates.check {#rpc-plugins-updates-check}

Descrição do contrato:

```text
Checks registries for plugin updates and advisories, applying permitted automatic updates
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Advisory": {
      "properties": {
        "message": {
          "type": "string"
        },
        "severity": {
          "$ref": "#/$defs/AdvisorySeverity"
        },
        "url": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "severity",
        "message"
      ],
      "type": "object"
    },
    "AdvisorySeverity": {
      "enum": [
        "low",
        "moderate",
        "high",
        "critical"
      ],
      "type": "string"
    },
    "LiveReleaseStatus": {
      "description": "Current Cloud status of the exact host artifact, separate from the\nimmutable signed snapshot. `Unknown` never authorizes installation.",
      "enum": [
        "unknown",
        "approved",
        "yanked",
        "key-revoked",
        "publisher-unapproved",
        "advisory",
        "incompatible"
      ],
      "type": "string"
    },
    "PermissionDiff": {
      "properties": {
        "addedCapabilities": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "addedPermissions": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "removedCapabilities": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "removedPermissions": {
          "items": {
            "type": "string"
          },
          "type": "array"
        }
      },
      "required": [
        "addedPermissions",
        "removedPermissions",
        "addedCapabilities",
        "removedCapabilities"
      ],
      "type": "object"
    },
    "UpdateInfo": {
      "properties": {
        "advisory": {
          "anyOf": [
            {
              "$ref": "#/$defs/Advisory"
            },
            {
              "type": "null"
            }
          ]
        },
        "autoApplicable": {
          "type": "boolean"
        },
        "changelog": {
          "type": [
            "string",
            "null"
          ]
        },
        "channel": {
          "type": "string"
        },
        "current": {
          "type": "string"
        },
        "currentStatus": {
          "$ref": "#/$defs/LiveReleaseStatus",
          "default": "unknown"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "anyOf": [
            {
              "$ref": "#/$defs/UpdateKind"
            },
            {
              "type": "null"
            }
          ]
        },
        "latest": {
          "type": [
            "string",
            "null"
          ]
        },
        "latestStatus": {
          "$ref": "#/$defs/LiveReleaseStatus",
          "default": "unknown"
        },
        "permissionDiff": {
          "$ref": "#/$defs/PermissionDiff"
        },
        "pinned": {
          "type": "boolean"
        },
        "registry": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "registry",
        "current",
        "channel",
        "pinned",
        "permissionDiff",
        "autoApplicable"
      ],
      "type": "object"
    },
    "UpdateKind": {
      "enum": [
        "patch",
        "minor",
        "major",
        "prerelease"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "checkedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "errors": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "updates": {
      "items": {
        "$ref": "#/$defs/UpdateInfo"
      },
      "type": "array"
    }
  },
  "required": [
    "updates",
    "checkedAt"
  ],
  "title": "UpdateReport",
  "type": "object"
}
```

:::

### plugins.updates.list {#rpc-plugins-updates-list}

Descrição do contrato:

```text
The most recent plugin update report
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Advisory": {
      "properties": {
        "message": {
          "type": "string"
        },
        "severity": {
          "$ref": "#/$defs/AdvisorySeverity"
        },
        "url": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "severity",
        "message"
      ],
      "type": "object"
    },
    "AdvisorySeverity": {
      "enum": [
        "low",
        "moderate",
        "high",
        "critical"
      ],
      "type": "string"
    },
    "LiveReleaseStatus": {
      "description": "Current Cloud status of the exact host artifact, separate from the\nimmutable signed snapshot. `Unknown` never authorizes installation.",
      "enum": [
        "unknown",
        "approved",
        "yanked",
        "key-revoked",
        "publisher-unapproved",
        "advisory",
        "incompatible"
      ],
      "type": "string"
    },
    "PermissionDiff": {
      "properties": {
        "addedCapabilities": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "addedPermissions": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "removedCapabilities": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "removedPermissions": {
          "items": {
            "type": "string"
          },
          "type": "array"
        }
      },
      "required": [
        "addedPermissions",
        "removedPermissions",
        "addedCapabilities",
        "removedCapabilities"
      ],
      "type": "object"
    },
    "UpdateInfo": {
      "properties": {
        "advisory": {
          "anyOf": [
            {
              "$ref": "#/$defs/Advisory"
            },
            {
              "type": "null"
            }
          ]
        },
        "autoApplicable": {
          "type": "boolean"
        },
        "changelog": {
          "type": [
            "string",
            "null"
          ]
        },
        "channel": {
          "type": "string"
        },
        "current": {
          "type": "string"
        },
        "currentStatus": {
          "$ref": "#/$defs/LiveReleaseStatus",
          "default": "unknown"
        },
        "id": {
          "type": "string"
        },
        "kind": {
          "anyOf": [
            {
              "$ref": "#/$defs/UpdateKind"
            },
            {
              "type": "null"
            }
          ]
        },
        "latest": {
          "type": [
            "string",
            "null"
          ]
        },
        "latestStatus": {
          "$ref": "#/$defs/LiveReleaseStatus",
          "default": "unknown"
        },
        "permissionDiff": {
          "$ref": "#/$defs/PermissionDiff"
        },
        "pinned": {
          "type": "boolean"
        },
        "registry": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "registry",
        "current",
        "channel",
        "pinned",
        "permissionDiff",
        "autoApplicable"
      ],
      "type": "object"
    },
    "UpdateKind": {
      "enum": [
        "patch",
        "minor",
        "major",
        "prerelease"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "checkedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "errors": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "updates": {
      "items": {
        "$ref": "#/$defs/UpdateInfo"
      },
      "type": "array"
    }
  },
  "required": [
    "updates",
    "checkedAt"
  ],
  "title": "UpdateReport",
  "type": "object"
}
```

:::

### points.adjust {#rpc-points-adjust}

Descrição do contrato:

```text
Manual points adjustment (creates the viewer record if missing)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"additive","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "delta": {
      "format": "double",
      "type": "number"
    },
    "expectedCurrency": {
      "type": [
        "string",
        "null"
      ]
    },
    "uniqueId": {
      "type": "string"
    }
  },
  "required": [
    "uniqueId",
    "delta"
  ],
  "title": "PointsAdjustParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "currencyName": {
      "type": "string"
    },
    "delta": {
      "format": "double",
      "type": "number"
    },
    "level": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "totalPoints": {
      "format": "double",
      "type": "number"
    },
    "uniqueId": {
      "type": "string"
    }
  },
  "required": [
    "uniqueId",
    "delta",
    "totalPoints",
    "level",
    "currencyName"
  ],
  "title": "PointAward",
  "type": "object"
}
```

:::

### points.config.get {#rpc-points-config-get}

Descrição do contrato:

```text
Current points configuration
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "currencyName": {
      "type": "string"
    },
    "pointsPerChat": {
      "format": "double",
      "type": "number"
    },
    "pointsPerChatEnabled": {
      "type": "boolean"
    },
    "pointsPerCoin": {
      "format": "double",
      "type": "number"
    },
    "pointsPerCoinEnabled": {
      "type": "boolean"
    },
    "pointsPerFollow": {
      "format": "double",
      "type": "number"
    },
    "pointsPerFollowEnabled": {
      "type": "boolean"
    },
    "pointsPerJoin": {
      "format": "double",
      "type": "number"
    },
    "pointsPerJoinEnabled": {
      "type": "boolean"
    },
    "pointsPerLevel": {
      "format": "double",
      "type": "number"
    },
    "pointsPerLike": {
      "format": "double",
      "type": "number"
    },
    "pointsPerLikeEnabled": {
      "type": "boolean"
    },
    "pointsPerShare": {
      "format": "double",
      "type": "number"
    },
    "pointsPerShareEnabled": {
      "type": "boolean"
    },
    "subBonusMultiplier": {
      "format": "double",
      "type": "number"
    }
  },
  "required": [
    "currencyName",
    "pointsPerCoin",
    "pointsPerCoinEnabled",
    "pointsPerShare",
    "pointsPerShareEnabled",
    "pointsPerChat",
    "pointsPerChatEnabled",
    "pointsPerLike",
    "pointsPerLikeEnabled",
    "pointsPerFollow",
    "pointsPerFollowEnabled",
    "pointsPerJoin",
    "pointsPerJoinEnabled",
    "subBonusMultiplier",
    "pointsPerLevel"
  ],
  "title": "PointsConfig",
  "type": "object"
}
```

:::

### points.config.set {#rpc-points-config-set}

Descrição do contrato:

```text
Updates points configuration (partial object)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "currencyName": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "pointsPerChat": {
      "default": null,
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "pointsPerChatEnabled": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "pointsPerCoin": {
      "default": null,
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "pointsPerCoinEnabled": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "pointsPerFollow": {
      "default": null,
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "pointsPerFollowEnabled": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "pointsPerJoin": {
      "default": null,
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "pointsPerJoinEnabled": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "pointsPerLevel": {
      "default": null,
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "pointsPerLike": {
      "default": null,
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "pointsPerLikeEnabled": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "pointsPerShare": {
      "default": null,
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    },
    "pointsPerShareEnabled": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "subBonusMultiplier": {
      "default": null,
      "format": "double",
      "type": [
        "number",
        "null"
      ]
    }
  },
  "title": "PartialPointsConfig",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "currencyName": {
      "type": "string"
    },
    "pointsPerChat": {
      "format": "double",
      "type": "number"
    },
    "pointsPerChatEnabled": {
      "type": "boolean"
    },
    "pointsPerCoin": {
      "format": "double",
      "type": "number"
    },
    "pointsPerCoinEnabled": {
      "type": "boolean"
    },
    "pointsPerFollow": {
      "format": "double",
      "type": "number"
    },
    "pointsPerFollowEnabled": {
      "type": "boolean"
    },
    "pointsPerJoin": {
      "format": "double",
      "type": "number"
    },
    "pointsPerJoinEnabled": {
      "type": "boolean"
    },
    "pointsPerLevel": {
      "format": "double",
      "type": "number"
    },
    "pointsPerLike": {
      "format": "double",
      "type": "number"
    },
    "pointsPerLikeEnabled": {
      "type": "boolean"
    },
    "pointsPerShare": {
      "format": "double",
      "type": "number"
    },
    "pointsPerShareEnabled": {
      "type": "boolean"
    },
    "subBonusMultiplier": {
      "format": "double",
      "type": "number"
    }
  },
  "required": [
    "currencyName",
    "pointsPerCoin",
    "pointsPerCoinEnabled",
    "pointsPerShare",
    "pointsPerShareEnabled",
    "pointsPerChat",
    "pointsPerChatEnabled",
    "pointsPerLike",
    "pointsPerLikeEnabled",
    "pointsPerFollow",
    "pointsPerFollowEnabled",
    "pointsPerJoin",
    "pointsPerJoinEnabled",
    "subBonusMultiplier",
    "pointsPerLevel"
  ],
  "title": "PointsConfig",
  "type": "object"
}
```

:::

### points.info {#rpc-points-info}

Descrição do contrato:

```text
Points unit label for confirmation; no earning configuration
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "currencyName": {
      "type": "string"
    }
  },
  "required": [
    "currencyName"
  ],
  "title": "PointsInfo",
  "type": "object"
}
```

:::

### points.leaderboard {#rpc-points-leaderboard}

Descrição do contrato:

```text
Points leaderboard, highest first
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "default": null,
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "title": "PointsLeaderboardParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "viewers": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "viewers"
  ],
  "title": "PointsLeaderboardResult",
  "type": "object"
}
```

:::

### points.reset {#rpc-points-reset}

Descrição do contrato:

```text
Resets points for one viewer, or all viewers when uniqueId is omitted
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "uniqueId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "PointsResetParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### points.view {#rpc-points-view}

Descrição do contrato:

```text
Committed bounded leaderboard and selected viewer
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "limit": {
      "default": null,
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    },
    "uniqueId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "PointsViewParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "currencyName": {
      "type": "string"
    },
    "selected": true,
    "selectedId": {
      "type": [
        "string",
        "null"
      ]
    },
    "viewers": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "currencyName",
    "viewers"
  ],
  "title": "PointsViewResult",
  "type": "object"
}
```

:::

### points.viewer.get {#rpc-points-viewer-get}

Descrição do contrato:

```text
One viewer points record by uniqueId
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "uniqueId": {
      "type": "string"
    }
  },
  "required": [
    "uniqueId"
  ],
  "title": "PointsViewerParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "viewer": true
  },
  "required": [
    "viewer"
  ],
  "title": "PointsViewerResult",
  "type": "object"
}
```

:::

### processors.list {#rpc-processors-list}

Descrição do contrato:

```text
Indexed event processors
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "processors": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "processors"
  ],
  "title": "ProcessorListResult",
  "type": "object"
}
```

:::

### processors.status {#rpc-processors-status}

Descrição do contrato:

```text
Processor health and metrics snapshot
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "ProcessorStatusEntry": {
      "properties": {
        "eventTypes": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "metrics": {
          "$ref": "#/$defs/ProcessorStatusMetrics"
        },
        "pluginId": {
          "type": "string"
        },
        "priority": {
          "format": "int64",
          "type": "integer"
        },
        "processorId": {
          "type": "string"
        },
        "status": {
          "type": "string"
        },
        "timeoutMs": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "pluginId",
        "processorId",
        "eventTypes",
        "timeoutMs",
        "priority",
        "status",
        "metrics"
      ],
      "type": "object"
    },
    "ProcessorStatusMetrics": {
      "description": "Typed health/metrics snapshot. The RPC shape is identical to the\nfrontend `ProcessorStatusEntry`: `{ processors: [...] }`.",
      "properties": {
        "averageLatencyMs": {
          "format": "double",
          "type": "number"
        },
        "calls": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "failures": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "lastError": {
          "type": [
            "string",
            "null"
          ]
        },
        "lastSuccessAt": {
          "format": "int64",
          "type": [
            "integer",
            "null"
          ]
        },
        "maxLatencyMs": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "skippedCircuitOpen": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "skippedOverloaded": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "successes": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "timeouts": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "calls",
        "successes",
        "failures",
        "timeouts",
        "skippedCircuitOpen",
        "skippedOverloaded",
        "averageLatencyMs",
        "maxLatencyMs"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "processors": {
      "items": {
        "$ref": "#/$defs/ProcessorStatusEntry"
      },
      "type": "array"
    }
  },
  "required": [
    "processors"
  ],
  "title": "ProcessorStatusResult",
  "type": "object"
}
```

:::

### processors.test {#rpc-processors-test}

Descrição do contrato:

```text
Runs one processor against a sample event
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "event": true,
    "pluginId": {
      "type": "string"
    },
    "processorId": {
      "type": "string"
    }
  },
  "required": [
    "pluginId",
    "processorId",
    "event"
  ],
  "title": "ProcessorTestParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "durationMs": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "error": {
      "type": [
        "string",
        "null"
      ]
    },
    "ok": {
      "type": "boolean"
    },
    "pluginId": {
      "type": "string"
    },
    "processorId": {
      "type": "string"
    },
    "result": true
  },
  "required": [
    "pluginId",
    "processorId",
    "ok",
    "durationMs",
    "result"
  ],
  "title": "ProcessorOutcomeDto",
  "type": "object"
}
```

:::

### publisher.create {#rpc-publisher-create}

Descrição do contrato:

```text
Requests a Cloud publisher namespace (starts pending admin approval)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "namespace": {
      "type": "string"
    }
  },
  "required": [
    "namespace"
  ],
  "title": "PublisherCreateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A publisher namespace the signed-in account belongs to.",
  "properties": {
    "id": {
      "type": "string"
    },
    "namespace": {
      "type": "string"
    },
    "role": {
      "type": "string"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "namespace",
    "status",
    "role"
  ],
  "title": "PublisherSummary",
  "type": "object"
}
```

:::

### publisher.delete {#rpc-publisher-delete}

Descrição do contrato:

```text
Deletes an owned publisher namespace that has no other members, keys, uploads, releases, transfers or step-up records
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "publisherId": {
      "type": "string"
    }
  },
  "required": [
    "publisherId"
  ],
  "title": "PublisherIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Result of deleting a publisher namespace. `publisher_id` is the resolved\nid, which can differ from the namespace the caller typed.",
  "properties": {
    "deleted": {
      "type": "boolean"
    },
    "publisherId": {
      "type": "string"
    }
  },
  "required": [
    "publisherId",
    "deleted"
  ],
  "title": "PublisherDeleteResult",
  "type": "object"
}
```

:::

### publisher.keys {#rpc-publisher-keys}

Descrição do contrato:

```text
Lists the signing keys of one publisher, pending and approved
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "publisherId": {
      "type": "string"
    }
  },
  "required": [
    "publisherId"
  ],
  "title": "PublisherIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "PublisherKeyRecord": {
      "description": "A signing-key record as the backend reports it. `status` is `pending`\nuntil an independent Cloud admin approves the key; pending keys sign\nnothing the registry trusts.",
      "properties": {
        "fingerprint": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "keyId": {
          "type": "string"
        },
        "publicKey": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "keyId",
        "publicKey",
        "fingerprint",
        "status"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "keys": {
      "items": {
        "$ref": "#/$defs/PublisherKeyRecord"
      },
      "type": "array"
    }
  },
  "required": [
    "keys"
  ],
  "title": "PublisherKeysResult",
  "type": "object"
}
```

:::

### publisher.keys.enroll {#rpc-publisher-keys-enroll}

Descrição do contrato:

```text
Enrolls a signing key: step-up, challenge, local key generation and registration
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "keyId": {
      "type": [
        "string",
        "null"
      ]
    },
    "password": {
      "description": "Backend account password for the session-bound step-up grant. Held in\nprocess memory only for this call; never stored.",
      "type": "string"
    },
    "publisherId": {
      "type": "string"
    }
  },
  "required": [
    "publisherId",
    "password"
  ],
  "title": "PublisherEnrollParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "PublisherKeyRecord": {
      "description": "A signing-key record as the backend reports it. `status` is `pending`\nuntil an independent Cloud admin approves the key; pending keys sign\nnothing the registry trusts.",
      "properties": {
        "fingerprint": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "keyId": {
          "type": "string"
        },
        "publicKey": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "keyId",
        "publicKey",
        "fingerprint",
        "status"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "fingerprint": {
      "type": "string"
    },
    "keyFile": {
      "description": "Local private-key file. Never send this file to Cloud.",
      "type": "string"
    },
    "record": {
      "$ref": "#/$defs/PublisherKeyRecord"
    }
  },
  "required": [
    "record",
    "fingerprint",
    "keyFile"
  ],
  "title": "PublisherEnrollResult",
  "type": "object"
}
```

:::

### publisher.list {#rpc-publisher-list}

Descrição do contrato:

```text
Lists the Cloud publisher namespaces the signed-in account belongs to
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "PublisherSummary": {
      "description": "A publisher namespace the signed-in account belongs to.",
      "properties": {
        "id": {
          "type": "string"
        },
        "namespace": {
          "type": "string"
        },
        "role": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "namespace",
        "status",
        "role"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "publishers": {
      "items": {
        "$ref": "#/$defs/PublisherSummary"
      },
      "type": "array"
    }
  },
  "required": [
    "publishers"
  ],
  "title": "PublisherListResult",
  "type": "object"
}
```

:::

### publisher.publication.request {#rpc-publisher-publication-request}

Descrição do contrato:

```text
Requests publication review for a release (needs a reviewer role)
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "PublisherReleaseParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Publication review request as the backend reports it.",
  "properties": {
    "jobId": {
      "type": "string"
    },
    "processingAvailable": {
      "type": "boolean"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "jobId",
    "status",
    "processingAvailable"
  ],
  "title": "PublisherPublicationResult",
  "type": "object"
}
```

:::

### publisher.releases.get {#rpc-publisher-releases-get}

Descrição do contrato:

```text
Fetches one plugin release for status review
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "PublisherReleaseParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "PublisherReleaseView": {
      "description": "A plugin release as reported by Cloud. Unknown backend fields are\nignored so newer servers stay compatible.",
      "properties": {
        "id": {
          "type": "string"
        },
        "pluginId": {
          "type": "string"
        },
        "publisherId": {
          "type": "string"
        },
        "publisherKeyId": {
          "type": "string"
        },
        "status": {
          "type": "string"
        },
        "target": {
          "type": "string"
        },
        "uploadId": {
          "type": "string"
        },
        "version": {
          "type": "string"
        },
        "yankReason": {
          "type": [
            "string",
            "null"
          ]
        },
        "yanked": {
          "default": false,
          "type": "boolean"
        }
      },
      "required": [
        "id",
        "publisherId",
        "uploadId",
        "publisherKeyId",
        "pluginId",
        "version",
        "target",
        "status"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "release": {
      "$ref": "#/$defs/PublisherReleaseView"
    }
  },
  "required": [
    "release"
  ],
  "title": "PublisherReleaseResult",
  "type": "object"
}
```

:::

### publisher.releases.submit {#rpc-publisher-releases-submit}

Descrição do contrato:

```text
Submits an uploaded artifact as a plugin release for review
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "pluginId": {
      "type": "string"
    },
    "publisherKeyId": {
      "type": "string"
    },
    "target": {
      "default": "any",
      "type": "string"
    },
    "uploadId": {
      "type": "string"
    },
    "version": {
      "type": "string"
    }
  },
  "required": [
    "uploadId",
    "publisherKeyId",
    "pluginId",
    "version"
  ],
  "title": "PublisherSubmitParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "PublisherReleaseView": {
      "description": "A plugin release as reported by Cloud. Unknown backend fields are\nignored so newer servers stay compatible.",
      "properties": {
        "id": {
          "type": "string"
        },
        "pluginId": {
          "type": "string"
        },
        "publisherId": {
          "type": "string"
        },
        "publisherKeyId": {
          "type": "string"
        },
        "status": {
          "type": "string"
        },
        "target": {
          "type": "string"
        },
        "uploadId": {
          "type": "string"
        },
        "version": {
          "type": "string"
        },
        "yankReason": {
          "type": [
            "string",
            "null"
          ]
        },
        "yanked": {
          "default": false,
          "type": "boolean"
        }
      },
      "required": [
        "id",
        "publisherId",
        "uploadId",
        "publisherKeyId",
        "pluginId",
        "version",
        "target",
        "status"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "release": {
      "$ref": "#/$defs/PublisherReleaseView"
    }
  },
  "required": [
    "release"
  ],
  "title": "PublisherReleaseResult",
  "type": "object"
}
```

:::

### publisher.uploads.send {#rpc-publisher-uploads-send}

Descrição do contrato:

```text
Uploads a local plugin artifact to object storage for a publisher
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "path": {
      "description": "Local `.plugin` artifact. The host reads and uploads it; the path\nmust exist on the host filesystem.",
      "type": "string"
    },
    "publisherId": {
      "type": "string"
    }
  },
  "required": [
    "publisherId",
    "path"
  ],
  "title": "PublisherUploadParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "status": {
      "type": "string"
    },
    "uploadId": {
      "type": "string"
    }
  },
  "required": [
    "uploadId",
    "status"
  ],
  "title": "PublisherUploadResult",
  "type": "object"
}
```

:::

### rankings.contribute {#rpc-rankings-contribute}

Descrição do contrato:

```text
Apply an idempotent plugin or custom ranking contribution
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "One externally reported contribution (plugin or agent). `eventId` is a\ncaller-chosen idempotency key; replays are acknowledged without scoring.",
  "properties": {
    "creatorId": {
      "type": "string"
    },
    "customKey": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "eventId": {
      "type": "string"
    },
    "label": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "metrics": {
      "additionalProperties": {
        "format": "uint64",
        "minimum": 0,
        "type": "integer"
      },
      "type": "object"
    },
    "option": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "roomId": {
      "type": "string"
    },
    "source": {
      "description": "Namespaced source such as `plugin.event` or `plugin.<id>.<name>`.",
      "type": "string"
    },
    "team": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "user": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "eventId",
    "creatorId",
    "roomId",
    "source",
    "metrics"
  ],
  "title": "ExternalContribution",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "duplicate": {
      "type": "boolean"
    }
  },
  "required": [
    "duplicate"
  ],
  "title": "ContributionOutcome",
  "type": "object"
}
```

:::

### rankings.create {#rpc-rankings-create}

Descrição do contrato:

```text
Create a ranking definition
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "GroupBy": {
      "description": "Available grouping keys. Missing keys are rejected, never guessed.",
      "enum": [
        "user",
        "gift",
        "event_type",
        "option",
        "team",
        "custom_key"
      ],
      "type": "string"
    },
    "SourceRule": {
      "description": "An integer multiplier. Scores are in whole units; no rounding is performed.",
      "properties": {
        "enabled": {
          "type": "boolean"
        },
        "giftMultipliers": {
          "additionalProperties": {
            "format": "uint64",
            "minimum": 0,
            "type": "integer"
          },
          "description": "Per-gift integer multiplier, defaulting to 1 when no override exists.\nA value of 0 disables this metric for that gift.",
          "type": "object"
        },
        "metric": {
          "type": "string"
        },
        "source": {
          "type": "string"
        },
        "weight": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "source",
        "metric",
        "weight",
        "enabled"
      ],
      "type": "object"
    },
    "UserLimits": {
      "description": "Limits apply to scored contributions per user until the ranking is reset.",
      "properties": {
        "cooldownMs": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "maxContributions": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "maxUsers": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "maxUsers"
      ],
      "type": "object"
    },
    "Window": {
      "description": "Rolling windows retain at most `max_events` accepted scored contributions.\nEntries expire at `accepted_at + duration_ms`, inclusive.",
      "oneOf": [
        {
          "properties": {
            "type": {
              "const": "session",
              "type": "string"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        },
        {
          "properties": {
            "durationMs": {
              "format": "uint64",
              "minimum": 0,
              "type": "integer"
            },
            "maxEvents": {
              "format": "uint",
              "minimum": 0,
              "type": "integer"
            },
            "type": {
              "const": "rolling",
              "type": "string"
            }
          },
          "required": [
            "type",
            "durationMs",
            "maxEvents"
          ],
          "type": "object"
        },
        {
          "description": "Accumulates for one calendar day at `utc_offset_minutes` (-840..=840)\nand resets scores, limits and integrity counters at local midnight.",
          "properties": {
            "type": {
              "const": "daily",
              "type": "string"
            },
            "utcOffsetMinutes": {
              "default": 0,
              "format": "int32",
              "type": "integer"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "capacity": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "creatorId": {
      "type": "string"
    },
    "groupBy": {
      "$ref": "#/$defs/GroupBy",
      "default": "user"
    },
    "id": {
      "type": "string"
    },
    "limit": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "roomId": {
      "type": "string"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "sources": {
      "items": {
        "$ref": "#/$defs/SourceRule"
      },
      "type": "array"
    },
    "userLimits": {
      "anyOf": [
        {
          "$ref": "#/$defs/UserLimits"
        },
        {
          "type": "null"
        }
      ]
    },
    "window": {
      "$ref": "#/$defs/Window",
      "default": {
        "type": "session"
      }
    }
  },
  "required": [
    "schemaVersion",
    "id",
    "creatorId",
    "roomId",
    "sources",
    "limit",
    "capacity"
  ],
  "title": "Definition",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### rankings.delete {#rpc-rankings-delete}

Descrição do contrato:

```text
Delete a ranking
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "RankingId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### rankings.get {#rpc-rankings-get}

Descrição do contrato:

```text
Get a ranking definition
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "RankingId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "GroupBy": {
      "description": "Available grouping keys. Missing keys are rejected, never guessed.",
      "enum": [
        "user",
        "gift",
        "event_type",
        "option",
        "team",
        "custom_key"
      ],
      "type": "string"
    },
    "SourceRule": {
      "description": "An integer multiplier. Scores are in whole units; no rounding is performed.",
      "properties": {
        "enabled": {
          "type": "boolean"
        },
        "giftMultipliers": {
          "additionalProperties": {
            "format": "uint64",
            "minimum": 0,
            "type": "integer"
          },
          "description": "Per-gift integer multiplier, defaulting to 1 when no override exists.\nA value of 0 disables this metric for that gift.",
          "type": "object"
        },
        "metric": {
          "type": "string"
        },
        "source": {
          "type": "string"
        },
        "weight": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "source",
        "metric",
        "weight",
        "enabled"
      ],
      "type": "object"
    },
    "UserLimits": {
      "description": "Limits apply to scored contributions per user until the ranking is reset.",
      "properties": {
        "cooldownMs": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "maxContributions": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "maxUsers": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "maxUsers"
      ],
      "type": "object"
    },
    "Window": {
      "description": "Rolling windows retain at most `max_events` accepted scored contributions.\nEntries expire at `accepted_at + duration_ms`, inclusive.",
      "oneOf": [
        {
          "properties": {
            "type": {
              "const": "session",
              "type": "string"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        },
        {
          "properties": {
            "durationMs": {
              "format": "uint64",
              "minimum": 0,
              "type": "integer"
            },
            "maxEvents": {
              "format": "uint",
              "minimum": 0,
              "type": "integer"
            },
            "type": {
              "const": "rolling",
              "type": "string"
            }
          },
          "required": [
            "type",
            "durationMs",
            "maxEvents"
          ],
          "type": "object"
        },
        {
          "description": "Accumulates for one calendar day at `utc_offset_minutes` (-840..=840)\nand resets scores, limits and integrity counters at local midnight.",
          "properties": {
            "type": {
              "const": "daily",
              "type": "string"
            },
            "utcOffsetMinutes": {
              "default": 0,
              "format": "int32",
              "type": "integer"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "capacity": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "creatorId": {
      "type": "string"
    },
    "groupBy": {
      "$ref": "#/$defs/GroupBy",
      "default": "user"
    },
    "id": {
      "type": "string"
    },
    "limit": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "roomId": {
      "type": "string"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "sources": {
      "items": {
        "$ref": "#/$defs/SourceRule"
      },
      "type": "array"
    },
    "userLimits": {
      "anyOf": [
        {
          "$ref": "#/$defs/UserLimits"
        },
        {
          "type": "null"
        }
      ]
    },
    "window": {
      "$ref": "#/$defs/Window",
      "default": {
        "type": "session"
      }
    }
  },
  "required": [
    "schemaVersion",
    "id",
    "creatorId",
    "roomId",
    "sources",
    "limit",
    "capacity"
  ],
  "title": "Definition",
  "type": "object"
}
```

:::

### rankings.list {#rpc-rankings-list}

Descrição do contrato:

```text
List rankings
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Definition": {
      "properties": {
        "capacity": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "creatorId": {
          "type": "string"
        },
        "groupBy": {
          "$ref": "#/$defs/GroupBy",
          "default": "user"
        },
        "id": {
          "type": "string"
        },
        "limit": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "roomId": {
          "type": "string"
        },
        "schemaVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "sources": {
          "items": {
            "$ref": "#/$defs/SourceRule"
          },
          "type": "array"
        },
        "userLimits": {
          "anyOf": [
            {
              "$ref": "#/$defs/UserLimits"
            },
            {
              "type": "null"
            }
          ]
        },
        "window": {
          "$ref": "#/$defs/Window",
          "default": {
            "type": "session"
          }
        }
      },
      "required": [
        "schemaVersion",
        "id",
        "creatorId",
        "roomId",
        "sources",
        "limit",
        "capacity"
      ],
      "type": "object"
    },
    "GroupBy": {
      "description": "Available grouping keys. Missing keys are rejected, never guessed.",
      "enum": [
        "user",
        "gift",
        "event_type",
        "option",
        "team",
        "custom_key"
      ],
      "type": "string"
    },
    "SourceRule": {
      "description": "An integer multiplier. Scores are in whole units; no rounding is performed.",
      "properties": {
        "enabled": {
          "type": "boolean"
        },
        "giftMultipliers": {
          "additionalProperties": {
            "format": "uint64",
            "minimum": 0,
            "type": "integer"
          },
          "description": "Per-gift integer multiplier, defaulting to 1 when no override exists.\nA value of 0 disables this metric for that gift.",
          "type": "object"
        },
        "metric": {
          "type": "string"
        },
        "source": {
          "type": "string"
        },
        "weight": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "source",
        "metric",
        "weight",
        "enabled"
      ],
      "type": "object"
    },
    "UserLimits": {
      "description": "Limits apply to scored contributions per user until the ranking is reset.",
      "properties": {
        "cooldownMs": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "maxContributions": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "maxUsers": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "maxUsers"
      ],
      "type": "object"
    },
    "Window": {
      "description": "Rolling windows retain at most `max_events` accepted scored contributions.\nEntries expire at `accepted_at + duration_ms`, inclusive.",
      "oneOf": [
        {
          "properties": {
            "type": {
              "const": "session",
              "type": "string"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        },
        {
          "properties": {
            "durationMs": {
              "format": "uint64",
              "minimum": 0,
              "type": "integer"
            },
            "maxEvents": {
              "format": "uint",
              "minimum": 0,
              "type": "integer"
            },
            "type": {
              "const": "rolling",
              "type": "string"
            }
          },
          "required": [
            "type",
            "durationMs",
            "maxEvents"
          ],
          "type": "object"
        },
        {
          "description": "Accumulates for one calendar day at `utc_offset_minutes` (-840..=840)\nand resets scores, limits and integrity counters at local midnight.",
          "properties": {
            "type": {
              "const": "daily",
              "type": "string"
            },
            "utcOffsetMinutes": {
              "default": 0,
              "format": "int32",
              "type": "integer"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "rankings": {
      "items": {
        "$ref": "#/$defs/Definition"
      },
      "type": "array"
    }
  },
  "required": [
    "rankings"
  ],
  "title": "RankingList",
  "type": "object"
}
```

:::

### rankings.metrics {#rpc-rankings-metrics}

Descrição do contrato:

```text
List available native delta metrics
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "MetricDescriptor": {
      "properties": {
        "description": {
          "type": "string"
        },
        "metric": {
          "type": "string"
        },
        "source": {
          "type": "string"
        },
        "unit": {
          "type": "string"
        }
      },
      "required": [
        "source",
        "metric",
        "description",
        "unit"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "metrics": {
      "items": {
        "$ref": "#/$defs/MetricDescriptor"
      },
      "type": "array"
    }
  },
  "required": [
    "metrics"
  ],
  "title": "MetricList",
  "type": "object"
}
```

:::

### rankings.pause {#rpc-rankings-pause}

Descrição do contrato:

```text
Pause a ranking
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "RankingId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### rankings.reset {#rpc-rankings-reset}

Descrição do contrato:

```text
Reset a ranking
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "RankingId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### rankings.resume {#rpc-rankings-resume}

Descrição do contrato:

```text
Resume a ranking
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "RankingId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### rankings.snapshot {#rpc-rankings-snapshot}

Descrição do contrato:

```text
Get a ranking snapshot
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "RankingId",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Entry": {
      "description": "Optional display data supplied by a source adapter. Identity and scores\nremain the ranking key and integer score, never these presentation fields.",
      "properties": {
        "avatarUrl": {
          "type": [
            "string",
            "null"
          ]
        },
        "giftId": {
          "type": [
            "string",
            "null"
          ]
        },
        "iconUrl": {
          "type": [
            "string",
            "null"
          ]
        },
        "key": {
          "type": "string"
        },
        "label": {
          "default": "",
          "type": "string"
        },
        "nickname": {
          "type": [
            "string",
            "null"
          ]
        },
        "rank": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "score": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "uniqueId": {
          "type": [
            "string",
            "null"
          ]
        },
        "userId": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "key",
        "rank",
        "score"
      ],
      "type": "object"
    },
    "Window": {
      "description": "Rolling windows retain at most `max_events` accepted scored contributions.\nEntries expire at `accepted_at + duration_ms`, inclusive.",
      "oneOf": [
        {
          "properties": {
            "type": {
              "const": "session",
              "type": "string"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        },
        {
          "properties": {
            "durationMs": {
              "format": "uint64",
              "minimum": 0,
              "type": "integer"
            },
            "maxEvents": {
              "format": "uint",
              "minimum": 0,
              "type": "integer"
            },
            "type": {
              "const": "rolling",
              "type": "string"
            }
          },
          "required": [
            "type",
            "durationMs",
            "maxEvents"
          ],
          "type": "object"
        },
        {
          "description": "Accumulates for one calendar day at `utc_offset_minutes` (-840..=840)\nand resets scores, limits and integrity counters at local midnight.",
          "properties": {
            "type": {
              "const": "daily",
              "type": "string"
            },
            "utcOffsetMinutes": {
              "default": 0,
              "format": "int32",
              "type": "integer"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "cooldownRejected": {
      "default": 0,
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "creatorId": {
      "type": "string"
    },
    "dropped": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "entries": {
      "items": {
        "$ref": "#/$defs/Entry"
      },
      "type": "array"
    },
    "instanceId": {
      "description": "Opaque identity of this ranking instance, retained by clean checkpoints.",
      "type": [
        "string",
        "null"
      ]
    },
    "integrityStatus": {
      "description": "Accounting completeness cannot be inferred from local state alone.",
      "type": "string"
    },
    "limited": {
      "default": 0,
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "paused": {
      "type": "boolean"
    },
    "rankingId": {
      "type": "string"
    },
    "rejected": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "revision": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "roomId": {
      "type": "string"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "totalParticipants": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "totalScore": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "window": {
      "$ref": "#/$defs/Window"
    }
  },
  "required": [
    "rankingId",
    "schemaVersion",
    "creatorId",
    "roomId",
    "window",
    "revision",
    "integrityStatus",
    "rejected",
    "dropped",
    "paused",
    "entries",
    "totalScore",
    "totalParticipants"
  ],
  "title": "Snapshot",
  "type": "object"
}
```

:::

### rankings.update {#rpc-rankings-update}

Descrição do contrato:

```text
Update a paused ranking; score changes require reset
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "Definition": {
      "properties": {
        "capacity": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "creatorId": {
          "type": "string"
        },
        "groupBy": {
          "$ref": "#/$defs/GroupBy",
          "default": "user"
        },
        "id": {
          "type": "string"
        },
        "limit": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        },
        "roomId": {
          "type": "string"
        },
        "schemaVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "sources": {
          "items": {
            "$ref": "#/$defs/SourceRule"
          },
          "type": "array"
        },
        "userLimits": {
          "anyOf": [
            {
              "$ref": "#/$defs/UserLimits"
            },
            {
              "type": "null"
            }
          ]
        },
        "window": {
          "$ref": "#/$defs/Window",
          "default": {
            "type": "session"
          }
        }
      },
      "required": [
        "schemaVersion",
        "id",
        "creatorId",
        "roomId",
        "sources",
        "limit",
        "capacity"
      ],
      "type": "object"
    },
    "GroupBy": {
      "description": "Available grouping keys. Missing keys are rejected, never guessed.",
      "enum": [
        "user",
        "gift",
        "event_type",
        "option",
        "team",
        "custom_key"
      ],
      "type": "string"
    },
    "SourceRule": {
      "description": "An integer multiplier. Scores are in whole units; no rounding is performed.",
      "properties": {
        "enabled": {
          "type": "boolean"
        },
        "giftMultipliers": {
          "additionalProperties": {
            "format": "uint64",
            "minimum": 0,
            "type": "integer"
          },
          "description": "Per-gift integer multiplier, defaulting to 1 when no override exists.\nA value of 0 disables this metric for that gift.",
          "type": "object"
        },
        "metric": {
          "type": "string"
        },
        "source": {
          "type": "string"
        },
        "weight": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "source",
        "metric",
        "weight",
        "enabled"
      ],
      "type": "object"
    },
    "UserLimits": {
      "description": "Limits apply to scored contributions per user until the ranking is reset.",
      "properties": {
        "cooldownMs": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "maxContributions": {
          "default": null,
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "maxUsers": {
          "format": "uint",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "maxUsers"
      ],
      "type": "object"
    },
    "Window": {
      "description": "Rolling windows retain at most `max_events` accepted scored contributions.\nEntries expire at `accepted_at + duration_ms`, inclusive.",
      "oneOf": [
        {
          "properties": {
            "type": {
              "const": "session",
              "type": "string"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        },
        {
          "properties": {
            "durationMs": {
              "format": "uint64",
              "minimum": 0,
              "type": "integer"
            },
            "maxEvents": {
              "format": "uint",
              "minimum": 0,
              "type": "integer"
            },
            "type": {
              "const": "rolling",
              "type": "string"
            }
          },
          "required": [
            "type",
            "durationMs",
            "maxEvents"
          ],
          "type": "object"
        },
        {
          "description": "Accumulates for one calendar day at `utc_offset_minutes` (-840..=840)\nand resets scores, limits and integrity counters at local midnight.",
          "properties": {
            "type": {
              "const": "daily",
              "type": "string"
            },
            "utcOffsetMinutes": {
              "default": 0,
              "format": "int32",
              "type": "integer"
            }
          },
          "required": [
            "type"
          ],
          "type": "object"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "definition": {
      "$ref": "#/$defs/Definition"
    },
    "reset": {
      "default": false,
      "type": "boolean"
    }
  },
  "required": [
    "definition"
  ],
  "title": "RankingUpdate",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### registry.cloudCatalog {#rpc-registry-cloudCatalog}

Descrição do contrato:

```text
Discovers plugin metadata from the configured Cloud instance; installation requires separate signature verification
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "after": {
      "type": [
        "string",
        "null"
      ]
    },
    "generation": {
      "description": "Exact decimal generation; JavaScript numbers cannot represent all i64s.",
      "type": [
        "string",
        "null"
      ]
    },
    "query": {
      "default": "",
      "type": "string"
    },
    "target": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "CloudCatalogQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "generation": {
      "type": [
        "string",
        "null"
      ]
    },
    "items": {
      "items": true,
      "type": "array"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "items"
  ],
  "title": "CloudCatalogPage",
  "type": "object"
}
```

:::

### registry.cloudCatalogPaginationCapabilities {#rpc-registry-cloudCatalogPaginationCapabilities}

Descrição do contrato:

```text
Discovers bounded Cloud catalog pagination capabilities
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "maxBytes": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "maxItems": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "schemaVersion": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "summaryPath": {
      "type": "string"
    },
    "versionOrder": {
      "type": "string"
    },
    "versionPagePath": {
      "type": "string"
    }
  },
  "required": [
    "schemaVersion",
    "summaryPath",
    "versionPagePath",
    "maxItems",
    "maxBytes",
    "versionOrder"
  ],
  "title": "CloudCatalogPaginationCapabilities",
  "type": "object"
}
```

:::

### registry.cloudCatalogSummaries {#rpc-registry-cloudCatalogSummaries}

Descrição do contrato:

```text
Reads bounded Cloud plugin summaries without granting installation trust
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "cursor": {
      "type": [
        "string",
        "null"
      ]
    },
    "generation": {
      "type": [
        "string",
        "null"
      ]
    },
    "query": {
      "type": [
        "string",
        "null"
      ]
    },
    "target": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "CloudCatalogSummariesQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CloudCatalogSummary": {
      "additionalProperties": false,
      "properties": {
        "id": {
          "type": "string"
        },
        "latestStableVersion": {
          "type": [
            "string",
            "null"
          ]
        },
        "metadataPath": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "metadataPath"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "generation": {
      "type": [
        "string",
        "null"
      ]
    },
    "items": {
      "items": {
        "$ref": "#/$defs/CloudCatalogSummary"
      },
      "type": "array"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "items"
  ],
  "title": "CloudCatalogSummariesPage",
  "type": "object"
}
```

:::

### registry.cloudCatalogVersions {#rpc-registry-cloudCatalogVersions}

Descrição do contrato:

```text
Reads one bounded Cloud plugin version page
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "cursor": {
      "type": [
        "string",
        "null"
      ]
    },
    "generation": {
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "type": "string"
    },
    "target": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "CloudCatalogVersionsQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CloudCatalogAdvisory": {
      "additionalProperties": false,
      "properties": {
        "message": {
          "type": [
            "string",
            "null"
          ]
        },
        "severity": {
          "type": "string"
        },
        "url": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "severity"
      ],
      "type": "object"
    },
    "CloudCatalogVersion": {
      "additionalProperties": false,
      "properties": {
        "advisory": {
          "anyOf": [
            {
              "$ref": "#/$defs/CloudCatalogAdvisory"
            },
            {
              "type": "null"
            }
          ]
        },
        "keyRevoked": {
          "type": "boolean"
        },
        "publisherApproved": {
          "type": "boolean"
        },
        "sha256": {
          "type": "string"
        },
        "size": {
          "type": "string"
        },
        "target": {
          "type": "string"
        },
        "version": {
          "type": "string"
        },
        "yanked": {
          "type": "boolean"
        }
      },
      "required": [
        "version",
        "target",
        "sha256",
        "size",
        "yanked",
        "keyRevoked",
        "publisherApproved"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "generation": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "items": {
      "items": {
        "$ref": "#/$defs/CloudCatalogVersion"
      },
      "type": "array"
    },
    "metadataPath": {
      "type": "string"
    },
    "nextCursor": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "generation",
    "id",
    "metadataPath",
    "items"
  ],
  "title": "CloudCatalogVersionsPage",
  "type": "object"
}
```

:::

### registry.defaults.get {#rpc-registry-defaults-get}

Descrição do contrato:

```text
Default update channel, auto-update rule, check interval, and sideload policy
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutoUpdate": {
      "enum": [
        "off",
        "notify",
        "patch",
        "minor"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "allowSideload": {
      "default": true,
      "type": "boolean"
    },
    "autoUpdate": {
      "$ref": "#/$defs/AutoUpdate",
      "default": "notify"
    },
    "channel": {
      "default": "stable",
      "type": "string"
    },
    "checkIntervalHours": {
      "default": 12,
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "title": "RegistryDefaults",
  "type": "object"
}
```

:::

### registry.defaults.set {#rpc-registry-defaults-set}

Descrição do contrato:

```text
Updates the default plugin update and sideload policy
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "AutoUpdate": {
      "enum": [
        "off",
        "notify",
        "patch",
        "minor"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "allowSideload": {
      "default": true,
      "type": "boolean"
    },
    "autoUpdate": {
      "$ref": "#/$defs/AutoUpdate",
      "default": "notify"
    },
    "channel": {
      "default": "stable",
      "type": "string"
    },
    "checkIntervalHours": {
      "default": 12,
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "title": "RegistryDefaults",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "AutoUpdate": {
      "enum": [
        "off",
        "notify",
        "patch",
        "minor"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "allowSideload": {
      "default": true,
      "type": "boolean"
    },
    "autoUpdate": {
      "$ref": "#/$defs/AutoUpdate",
      "default": "notify"
    },
    "channel": {
      "default": "stable",
      "type": "string"
    },
    "checkIntervalHours": {
      "default": 12,
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "title": "RegistryDefaults",
  "type": "object"
}
```

:::

### registry.list {#rpc-registry-list}

Descrição do contrato:

```text
Configured plugin registries with verification and refresh state
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "RegistryStatus": {
      "properties": {
        "builtin": {
          "type": "boolean"
        },
        "description": {
          "type": [
            "string",
            "null"
          ]
        },
        "enabled": {
          "type": "boolean"
        },
        "error": {
          "type": [
            "string",
            "null"
          ]
        },
        "fingerprints": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "name": {
          "type": "string"
        },
        "priority": {
          "format": "int32",
          "type": "integer"
        },
        "refreshedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "scopes": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "url": {
          "type": "string"
        },
        "verified": {
          "type": "boolean"
        }
      },
      "required": [
        "name",
        "url",
        "enabled",
        "priority",
        "scopes",
        "builtin",
        "verified",
        "fingerprints"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "registries": {
      "items": {
        "$ref": "#/$defs/RegistryStatus"
      },
      "type": "array"
    }
  },
  "required": [
    "registries"
  ],
  "title": "RegistryListResult",
  "type": "object"
}
```

:::

### registry.package {#rpc-registry-package}

Descrição do contrato:

```text
The verified registry document of one plugin
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "registry": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "RegistryPackageParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "Advisory": {
      "properties": {
        "message": {
          "type": "string"
        },
        "severity": {
          "$ref": "#/$defs/AdvisorySeverity"
        },
        "url": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "severity",
        "message"
      ],
      "type": "object"
    },
    "AdvisorySeverity": {
      "enum": [
        "low",
        "moderate",
        "high",
        "critical"
      ],
      "type": "string"
    },
    "Artifact": {
      "properties": {
        "keyId": {
          "type": [
            "string",
            "null"
          ]
        },
        "sha256": {
          "type": "string"
        },
        "size": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "target": {
          "type": "string"
        },
        "url": {
          "type": "string"
        }
      },
      "required": [
        "target",
        "url",
        "size",
        "sha256"
      ],
      "type": "object"
    },
    "LiveReleaseStatus": {
      "description": "Current Cloud status of the exact host artifact, separate from the\nimmutable signed snapshot. `Unknown` never authorizes installation.",
      "enum": [
        "unknown",
        "approved",
        "yanked",
        "key-revoked",
        "publisher-unapproved",
        "advisory",
        "incompatible"
      ],
      "type": "string"
    },
    "PackageDocument": {
      "properties": {
        "categories": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "description": {
          "type": [
            "string",
            "null"
          ]
        },
        "dist-tags": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "homepage": {
          "type": [
            "string",
            "null"
          ]
        },
        "icon": {
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "license": {
          "type": [
            "string",
            "null"
          ]
        },
        "name": {
          "type": "string"
        },
        "publisher": {
          "type": [
            "string",
            "null"
          ]
        },
        "releases": {
          "default": [],
          "items": {
            "$ref": "#/$defs/Release"
          },
          "type": "array"
        },
        "repository": {
          "type": [
            "string",
            "null"
          ]
        },
        "screenshots": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "tags": {
          "items": {
            "type": "string"
          },
          "type": "array"
        }
      },
      "required": [
        "id",
        "name"
      ],
      "type": "object"
    },
    "Release": {
      "properties": {
        "abiVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "advisory": {
          "anyOf": [
            {
              "$ref": "#/$defs/Advisory"
            },
            {
              "type": "null"
            }
          ]
        },
        "artifacts": {
          "items": {
            "$ref": "#/$defs/Artifact"
          },
          "type": "array"
        },
        "capabilities": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "changelog": {
          "type": [
            "string",
            "null"
          ]
        },
        "channel": {
          "default": "stable",
          "type": "string"
        },
        "dependencies": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "engines": {
          "additionalProperties": {
            "type": "string"
          },
          "type": "object"
        },
        "permissions": {
          "default": [],
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "protocolVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "publishedAt": {
          "default": 0,
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "runtime": {
          "type": "string"
        },
        "schemaVersion": {
          "format": "uint32",
          "minimum": 0,
          "type": "integer"
        },
        "version": {
          "type": "string"
        },
        "yanked": {
          "default": false,
          "type": "boolean"
        }
      },
      "required": [
        "version",
        "runtime",
        "schemaVersion",
        "protocolVersion",
        "artifacts"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "document": {
      "$ref": "#/$defs/PackageDocument"
    },
    "liveStatuses": {
      "additionalProperties": {
        "$ref": "#/$defs/LiveReleaseStatus"
      },
      "default": {},
      "type": "object"
    },
    "registry": {
      "type": "string"
    }
  },
  "required": [
    "registry",
    "document"
  ],
  "title": "RegistryPackageView",
  "type": "object"
}
```

:::

### registry.refresh {#rpc-registry-refresh}

Descrição do contrato:

```text
Re-reads and re-verifies registry keys and configuration
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "name": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "RegistryRefreshParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "RegistryStatus": {
      "properties": {
        "builtin": {
          "type": "boolean"
        },
        "description": {
          "type": [
            "string",
            "null"
          ]
        },
        "enabled": {
          "type": "boolean"
        },
        "error": {
          "type": [
            "string",
            "null"
          ]
        },
        "fingerprints": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "name": {
          "type": "string"
        },
        "priority": {
          "format": "int32",
          "type": "integer"
        },
        "refreshedAt": {
          "format": "uint64",
          "minimum": 0,
          "type": [
            "integer",
            "null"
          ]
        },
        "scopes": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "url": {
          "type": "string"
        },
        "verified": {
          "type": "boolean"
        }
      },
      "required": [
        "name",
        "url",
        "enabled",
        "priority",
        "scopes",
        "builtin",
        "verified",
        "fingerprints"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "registries": {
      "items": {
        "$ref": "#/$defs/RegistryStatus"
      },
      "type": "array"
    }
  },
  "required": [
    "registries"
  ],
  "title": "RegistryListResult",
  "type": "object"
}
```

:::

### registry.search {#rpc-registry-search}

Descrição do contrato:

```text
Searches enabled registries' catalogs
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "category": {
      "type": [
        "string",
        "null"
      ]
    },
    "page": {
      "default": 0,
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "query": {
      "default": "",
      "type": "string"
    },
    "registry": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "SearchQuery",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "SearchEntry": {
      "properties": {
        "categories": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "description": {
          "type": [
            "string",
            "null"
          ]
        },
        "icon": {
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "latest": {
          "type": "string"
        },
        "name": {
          "type": "string"
        },
        "publisher": {
          "type": [
            "string",
            "null"
          ]
        },
        "registry": {
          "type": "string"
        },
        "runtime": {
          "type": "string"
        },
        "tags": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "updatedAt": {
          "default": 0,
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        }
      },
      "required": [
        "registry",
        "id",
        "name",
        "latest",
        "runtime"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "entries": {
      "items": {
        "$ref": "#/$defs/SearchEntry"
      },
      "type": "array"
    },
    "errors": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "page": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "pages": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    },
    "total": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "entries",
    "total",
    "page",
    "pages"
  ],
  "title": "SearchResults",
  "type": "object"
}
```

:::

### rpc.discover {#rpc-rpc-discover}

Descrição do contrato:

```text
Lists every method with descriptions and JSON Schemas
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "ConcurrencyStrategy": {
      "enum": [
        "compare-and-set",
        "additive",
        "exclusive-transition"
      ],
      "type": "string"
    },
    "MethodMeta": {
      "description": "Discoverable method metadata (see `rpc.discover` / `rpc.schema`).",
      "properties": {
        "concurrency": {
          "anyOf": [
            {
              "$ref": "#/$defs/ConcurrencyStrategy"
            },
            {
              "type": "null"
            }
          ]
        },
        "description": {
          "type": "string"
        },
        "destructive": {
          "default": false,
          "description": "Deletes or resets persisted state (agents should confirm first).",
          "type": "boolean"
        },
        "durableCommands": {
          "default": false,
          "type": "boolean"
        },
        "name": {
          "type": "string"
        },
        "paramsSchema": true,
        "requiresDesktop": {
          "default": false,
          "description": "Needs a desktop host (native dialogs); headless hosts refuse it.",
          "type": "boolean"
        },
        "resultSchema": true,
        "sideEffect": {
          "type": "boolean"
        }
      },
      "required": [
        "name",
        "description",
        "sideEffect",
        "paramsSchema",
        "resultSchema"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "methods": {
      "items": {
        "$ref": "#/$defs/MethodMeta"
      },
      "type": "array"
    }
  },
  "required": [
    "methods"
  ],
  "title": "DiscoverResult",
  "type": "object"
}
```

:::

### rpc.schema {#rpc-rpc-schema}

Descrição do contrato:

```text
Returns one method metadata entry with its schemas
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "method": {
      "type": "string"
    }
  },
  "required": [
    "method"
  ],
  "title": "SchemaParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "ConcurrencyStrategy": {
      "enum": [
        "compare-and-set",
        "additive",
        "exclusive-transition"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Discoverable method metadata (see `rpc.discover` / `rpc.schema`).",
  "properties": {
    "concurrency": {
      "anyOf": [
        {
          "$ref": "#/$defs/ConcurrencyStrategy"
        },
        {
          "type": "null"
        }
      ]
    },
    "description": {
      "type": "string"
    },
    "destructive": {
      "default": false,
      "description": "Deletes or resets persisted state (agents should confirm first).",
      "type": "boolean"
    },
    "durableCommands": {
      "default": false,
      "type": "boolean"
    },
    "name": {
      "type": "string"
    },
    "paramsSchema": true,
    "requiresDesktop": {
      "default": false,
      "description": "Needs a desktop host (native dialogs); headless hosts refuse it.",
      "type": "boolean"
    },
    "resultSchema": true,
    "sideEffect": {
      "type": "boolean"
    }
  },
  "required": [
    "name",
    "description",
    "sideEffect",
    "paramsSchema",
    "resultSchema"
  ],
  "title": "MethodMeta",
  "type": "object"
}
```

:::

### sync.disable {#rpc-sync-disable}

Descrição do contrato:

```text
Opts this device out of Cloud plugin-selection sync
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the UI may show. No tokens, no plugin code, ever.",
  "properties": {
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "enabled": {
      "type": "boolean"
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "lastSyncedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "pending": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "signedIn": {
      "type": "boolean"
    },
    "syncing": {
      "type": "boolean"
    }
  },
  "required": [
    "enabled",
    "signedIn",
    "syncing",
    "pending",
    "conflicts"
  ],
  "title": "SyncStatusResult",
  "type": "object"
}
```

:::

### sync.enable {#rpc-sync-enable}

Descrição do contrato:

```text
Opts this device into Cloud plugin-selection sync
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the UI may show. No tokens, no plugin code, ever.",
  "properties": {
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "enabled": {
      "type": "boolean"
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "lastSyncedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "pending": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "signedIn": {
      "type": "boolean"
    },
    "syncing": {
      "type": "boolean"
    }
  },
  "required": [
    "enabled",
    "signedIn",
    "syncing",
    "pending",
    "conflicts"
  ],
  "title": "SyncStatusResult",
  "type": "object"
}
```

:::

### sync.now {#rpc-sync-now}

Descrição do contrato:

```text
Pulls and pushes plugin selections without automatically applying remote changes
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "applicationDeferred": {
      "default": false,
      "description": "At least one remote selection was retained without automatic local\napplication. This is not a count of local differences or failures.",
      "type": "boolean"
    },
    "applied": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "pulled": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "pushed": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "skipped": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "pushed",
    "pulled",
    "applied",
    "skipped",
    "conflicts"
  ],
  "title": "SyncNowResult",
  "type": "object"
}
```

:::

### sync.resolve {#rpc-sync-resolve}

Descrição do contrato:

```text
Conflict resolution is unavailable until local recovery is protected
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "keepLocal": {
      "default": false,
      "type": "boolean"
    },
    "key": {
      "type": "string"
    }
  },
  "required": [
    "key"
  ],
  "title": "SyncResolveParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the UI may show. No tokens, no plugin code, ever.",
  "properties": {
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "enabled": {
      "type": "boolean"
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "lastSyncedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "pending": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "signedIn": {
      "type": "boolean"
    },
    "syncing": {
      "type": "boolean"
    }
  },
  "required": [
    "enabled",
    "signedIn",
    "syncing",
    "pending",
    "conflicts"
  ],
  "title": "SyncStatusResult",
  "type": "object"
}
```

:::

### sync.status {#rpc-sync-status}

Descrição do contrato:

```text
Shows the Cloud sync state without exposing any credentials
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Everything the UI may show. No tokens, no plugin code, ever.",
  "properties": {
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "enabled": {
      "type": "boolean"
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "lastSyncedAt": {
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "pending": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "signedIn": {
      "type": "boolean"
    },
    "syncing": {
      "type": "boolean"
    }
  },
  "required": [
    "enabled",
    "signedIn",
    "syncing",
    "pending",
    "conflicts"
  ],
  "title": "SyncStatusResult",
  "type": "object"
}
```

:::

### sync.uncertain.list {#rpc-sync-uncertain-list}

Descrição do contrato:

```text
Lists selections with an uncertain upload outcome
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "type": "string"
  },
  "title": "Array_of_string",
  "type": "array"
}
```

:::

### sync.uncertain.resolve {#rpc-sync-uncertain-resolve}

Descrição do contrato:

```text
Resolves a reviewed uncertain upload
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "SyncRecoveryChoice": {
      "enum": [
        "keepLocal",
        "takeCloud",
        "takeCloudIncludingNewer"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "choice": {
      "$ref": "#/$defs/SyncRecoveryChoice"
    },
    "handle": {
      "type": "string"
    }
  },
  "required": [
    "handle",
    "choice"
  ],
  "title": "SyncUncertainResolveParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    "resolved",
    "newerEditNeedsReview"
  ],
  "title": "SyncRecoveryOutcome",
  "type": "string"
}
```

:::

### sync.uncertain.review {#rpc-sync-uncertain-review}

Descrição do contrato:

```text
Reads the current Cloud value for an uncertain upload
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "key": {
      "type": "string"
    }
  },
  "required": [
    "key"
  ],
  "title": "SyncUncertainReviewParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "SyncRecoveryValue": {
      "description": "A nonsecret projection of a single selection shown during explicit review.",
      "properties": {
        "enabled": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "state": {
          "type": "string"
        },
        "version": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "state"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "base": {
      "$ref": "#/$defs/SyncRecoveryValue"
    },
    "cloud": {
      "$ref": "#/$defs/SyncRecoveryValue"
    },
    "handle": {
      "type": "string"
    },
    "key": {
      "type": "string"
    },
    "local": {
      "$ref": "#/$defs/SyncRecoveryValue"
    },
    "original": {
      "$ref": "#/$defs/SyncRecoveryValue"
    }
  },
  "required": [
    "handle",
    "key",
    "base",
    "original",
    "local",
    "cloud"
  ],
  "title": "SyncRecoveryReview",
  "type": "object"
}
```

:::

### sync.v2.changes.list {#rpc-sync-v2-changes-list}

Descrição do contrato:

```text
Reads the durable v2 workspace (cached Cloud values and pending edits) without contacting Cloud
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "V2ChangeEntry": {
      "description": "Read-only projection of one durable v2 workspace entry: the last fetched\nCloud record, the pending local edit when there is one, and whether a newer\nlocal decision waits behind it. Nothing here contacts Cloud. Values are\nprojected only for the credential-free `plugin-selections` and\n`preferences` and `variables` collections; every other collection (notably\n`plugin-settings`) is reduced to metadata and never leaves the host.",
      "properties": {
        "collection": {
          "type": "string"
        },
        "hasNewer": {
          "type": "boolean"
        },
        "key": {
          "type": "string"
        },
        "pending": {
          "anyOf": [
            {
              "$ref": "#/$defs/V2PendingChange"
            },
            {
              "type": "null"
            }
          ]
        },
        "remote": {
          "$ref": "#/$defs/V2ConflictValue"
        }
      },
      "required": [
        "collection",
        "key",
        "remote",
        "hasNewer"
      ],
      "type": "object"
    },
    "V2ConflictValue": {
      "description": "A generic nonsecret projection for UI comparison. `revision` is present\nfor Cloud-backed values; `value` carries the portable JSON when present.",
      "properties": {
        "enabled": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "revision": {
          "format": "int64",
          "type": [
            "integer",
            "null"
          ]
        },
        "state": {
          "type": "string"
        },
        "value": true,
        "version": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "state"
      ],
      "type": "object"
    },
    "V2PendingChange": {
      "description": "One pending local edit of a v2 workspace entry, with the frozen Cloud base it\nwas made against. `status` is `queued`, `unconfirmed` or `conflict`.\n\nFor `plugin-settings` entries `base` and `local` carry no value (their\n`value` is always null): settings are arbitrary plugin data that may be\nsensitive, so only `changed_keys`, the sorted top-level setting names whose\nvalue differs from the base (or that were added or removed), is projected.\nIt is `null` for every other collection.",
      "properties": {
        "base": {
          "$ref": "#/$defs/V2ConflictValue"
        },
        "changedKeys": {
          "items": {
            "type": "string"
          },
          "type": [
            "array",
            "null"
          ]
        },
        "local": {
          "$ref": "#/$defs/V2ConflictValue"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "status",
        "base",
        "local"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$ref": "#/$defs/V2ChangeEntry"
  },
  "title": "Array_of_V2ChangeEntry",
  "type": "array"
}
```

:::

### sync.v2.conflict.list {#rpc-sync-v2-conflict-list}

Descrição do contrato:

```text
Lists v2 conflicts for the current account without contacting Cloud
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "V2ConflictTarget": {
      "description": "One v2 conflict key for the current account.",
      "properties": {
        "collection": {
          "type": "string"
        },
        "key": {
          "type": "string"
        }
      },
      "required": [
        "collection",
        "key"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "items": {
    "$ref": "#/$defs/V2ConflictTarget"
  },
  "title": "Array_of_V2ConflictTarget",
  "type": "array"
}
```

:::

### sync.v2.conflict.resolve {#rpc-sync-v2-conflict-resolve}

Descrição do contrato:

```text
Resolves a reviewed v2 conflict
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "V2ConflictChoice": {
      "enum": [
        "keepLocal",
        "takeCloud"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "choice": {
      "$ref": "#/$defs/V2ConflictChoice"
    },
    "handle": {
      "type": "string"
    }
  },
  "required": [
    "handle",
    "choice"
  ],
  "title": "SyncV2ConflictResolveParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "enum": [
    "resolved"
  ],
  "title": "V2ConflictOutcome",
  "type": "string"
}
```

:::

### sync.v2.conflict.review {#rpc-sync-v2-conflict-review}

Descrição do contrato:

```text
Reads the Cloud comparison for one reviewed v2 conflict
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "collection": {
      "type": "string"
    },
    "key": {
      "type": "string"
    }
  },
  "required": [
    "collection",
    "key"
  ],
  "title": "SyncV2ConflictReviewParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "V2ConflictValue": {
      "description": "A generic nonsecret projection for UI comparison. `revision` is present\nfor Cloud-backed values; `value` carries the portable JSON when present.",
      "properties": {
        "enabled": {
          "type": [
            "boolean",
            "null"
          ]
        },
        "revision": {
          "format": "int64",
          "type": [
            "integer",
            "null"
          ]
        },
        "state": {
          "type": "string"
        },
        "value": true,
        "version": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "required": [
        "state"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Reviewed v2 conflict comparison bound to a one-time handle. Resolution\nre-reads Cloud and rejects the handle unless the fresh comparison matches.",
  "properties": {
    "base": {
      "$ref": "#/$defs/V2ConflictValue"
    },
    "cloud": {
      "$ref": "#/$defs/V2ConflictValue"
    },
    "collection": {
      "type": "string"
    },
    "handle": {
      "type": "string"
    },
    "hasNewer": {
      "type": "boolean"
    },
    "key": {
      "type": "string"
    },
    "local": {
      "$ref": "#/$defs/V2ConflictValue"
    }
  },
  "required": [
    "handle",
    "collection",
    "key",
    "base",
    "local",
    "cloud",
    "hasNewer"
  ],
  "title": "V2ConflictReview",
  "type": "object"
}
```

:::

### sync.v2.discover {#rpc-sync-v2-discover}

Descrição do contrato:

```text
Checks Cloud v2 compatibility without enabling synchronization
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "WorkspaceV2DiscoveryState": {
      "enum": [
        "syncDisabled",
        "signInRequired",
        "backendNotSupported",
        "backendCompatible"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Compatibility is informational, not permission to write or apply data.",
  "properties": {
    "desktopSyncAvailable": {
      "type": "boolean"
    },
    "selected": {
      "description": "True only for the currently signed-in account with a completed v2\nmarker and matching, enabled account-scoped consent.",
      "type": "boolean"
    },
    "state": {
      "$ref": "#/$defs/WorkspaceV2DiscoveryState"
    }
  },
  "required": [
    "state",
    "desktopSyncAvailable",
    "selected"
  ],
  "title": "WorkspaceV2Discovery",
  "type": "object"
}
```

:::

### sync.v2.inventory.repair {#rpc-sync-v2-inventory-repair}

Descrição do contrato:

```text
Archives and clears only reviewed absent local install flags; keeps Cloud queues and plugin data
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "review": {
      "type": "string"
    }
  },
  "required": [
    "review"
  ],
  "title": "WorkspaceInventoryRepairParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Reviewed local-only reconciliation of install flags for absent packages.",
  "properties": {
    "plugins": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "review": {
      "type": "string"
    }
  },
  "required": [
    "review",
    "plugins"
  ],
  "title": "WorkspaceInventoryReview",
  "type": "object"
}
```

:::

### sync.v2.inventory.review {#rpc-sync-v2-inventory-review}

Descrição do contrato:

```text
Reviews absent local packages without changing sync queues
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Reviewed local-only reconciliation of install flags for absent packages.",
  "properties": {
    "plugins": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "review": {
      "type": "string"
    }
  },
  "required": [
    "review",
    "plugins"
  ],
  "title": "WorkspaceInventoryReview",
  "type": "object"
}
```

:::

### sync.v2.migrate {#rpc-sync-v2-migrate}

Descrição do contrato:

```text
Migrates this account from v1 to v2 and activates the server write-epoch policy
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Result of the one-time v1 to v2 migration. `completed` means the local\ncutover committed; `policy_activated` means the server write-epoch policy\nis active for this account. A completed migration with\n`policy_activated` false can be retried by calling migrate again.",
  "properties": {
    "completed": {
      "type": "boolean"
    },
    "conflicts": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "pending": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "policyActivated": {
      "type": "boolean"
    },
    "retainedV1Only": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "selections": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "selections",
    "pending",
    "conflicts",
    "retainedV1Only",
    "completed",
    "policyActivated"
  ],
  "title": "WorkspaceV2Migration",
  "type": "object"
}
```

:::

### sync.v2.now {#rpc-sync-v2-now}

Descrição do contrato:

```text
Synchronize an already selected v2 workspace without applying Cloud records
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A v2 network pass persists its cache and attempts supported plugin state\nchanges; other Cloud records are not applied by this operation.",
  "properties": {
    "availableRecords": {
      "description": "Cached records available for application or review.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "moreQueued": {
      "type": "boolean"
    },
    "pulled": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "pushed": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "workspacePending": {
      "description": "Pending edits in the v2 workspace; additional SQLite intents may be\nindicated by `more_queued`.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "pushed",
    "pulled",
    "workspacePending",
    "availableRecords",
    "conflicts",
    "moreQueued"
  ],
  "title": "WorkspaceV2SyncResult",
  "type": "object"
}
```

:::

### sync.v2.preview {#rpc-sync-v2-preview}

Descrição do contrato:

```text
Reviews local migration inputs without changing or enabling synchronization
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Informational local migration estimate; it is not an authorization token.",
  "properties": {
    "conflicts": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "migrationAvailable": {
      "type": "boolean"
    },
    "pending": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "retainedV1Only": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "selections": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "selections",
    "pending",
    "conflicts",
    "retainedV1Only",
    "migrationAvailable"
  ],
  "title": "WorkspaceV2Preview",
  "type": "object"
}
```

:::

### sync.v2.rebuild {#rpc-sync-v2-rebuild}

Descrição do contrato:

```text
Rebuilds the fetched Cloud baseline after cursor expiry
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "null",
  "type": "null"
}
```

:::

### sync.v2.section {#rpc-sync-v2-section}

Descrição do contrato:

```text
Publishes only the requested section and returns its application candidates; other downloaded records remain cached
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "SyncSection": {
      "description": "Sections with native publication and application adapters.",
      "enum": [
        "plugins",
        "variables"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "section": {
      "$ref": "#/$defs/SyncSection"
    }
  },
  "required": [
    "section"
  ],
  "title": "SyncSectionParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A v2 network pass persists its cache and attempts supported plugin state\nchanges; other Cloud records are not applied by this operation.",
  "properties": {
    "availableRecords": {
      "description": "Cached records available for application or review.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "moreQueued": {
      "type": "boolean"
    },
    "pulled": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "pushed": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "workspacePending": {
      "description": "Pending edits in the v2 workspace; additional SQLite intents may be\nindicated by `more_queued`.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "pushed",
    "pulled",
    "workspacePending",
    "availableRecords",
    "conflicts",
    "moreQueued"
  ],
  "title": "WorkspaceV2SyncResult",
  "type": "object"
}
```

:::

### sync.v2.selected {#rpc-sync-v2-selected}

Descrição do contrato:

```text
Reads the selected Cloud protocol without contacting the server
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "boolean",
  "type": "boolean"
}
```

:::

### sync.v2.stagePluginSettings {#rpc-sync-v2-stagePluginSettings}

Descrição do contrato:

```text
Stages plugin settings for v2 sync
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "pluginId": {
      "type": "string"
    },
    "settings": true
  },
  "required": [
    "pluginId",
    "settings"
  ],
  "title": "SyncV2StagePluginSettingsParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "null",
  "type": "null"
}
```

:::

### sync.v2.stagePreferences {#rpc-sync-v2-stagePreferences}

Descrição do contrato:

```text
Stages desktop preferences for v2 sync
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "null",
  "type": "null"
}
```

:::

### sync.v2.stageVariables {#rpc-sync-v2-stageVariables}

Descrição do contrato:

```text
Stages runtime variables for v2 sync; secrets are excluded and never leave the host; needs the Cloud variables collection
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$defs": {
    "VariableInput": {
      "additionalProperties": false,
      "description": "One local variable offered for sync.",
      "properties": {
        "name": {
          "type": "string"
        },
        "type": {
          "$ref": "#/$defs/VariableKind"
        },
        "value": {
          "description": "`text`: string; `number`: finite number; `boolean`; `list`: array of\nstrings. For `secret` the value is ignored and dropped."
        }
      },
      "required": [
        "name",
        "type",
        "value"
      ],
      "type": "object"
    },
    "VariableKind": {
      "description": "Local variable types. `Secret` exists only so the host can recognise and\nexclude it; it has no portable representation.",
      "enum": [
        "text",
        "number",
        "boolean",
        "list",
        "secret"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "remove": {
      "default": [],
      "description": "Names whose Cloud record should be deleted (also how a variable that\nbecame secret is withdrawn from the Cloud).",
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "variables": {
      "default": [],
      "description": "Variables to create or update in the Cloud workspace.",
      "items": {
        "$ref": "#/$defs/VariableInput"
      },
      "type": "array"
    }
  },
  "title": "SyncV2StageVariablesParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "excludedSecrets": {
      "description": "Names of `secret` variables that were left out. Nothing about them was\nstaged or sent.",
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "rejected": {
      "description": "Names the portable contract refuses (bad name, wrong value type, too\nlarge, or a value that looks like a credential). Never staged.",
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "removed": {
      "description": "Names with a pending delete now.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "staged": {
      "description": "Variables with a pending put now.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "staged",
    "removed",
    "excludedSecrets",
    "rejected"
  ],
  "title": "SyncV2StageVariablesResult",
  "type": "object"
}
```

:::

### sync.v2.variables.apply {#rpc-sync-v2-variables-apply}

Descrição do contrato:

```text
Apply one exact reviewed cached record; rejects changed local, account or Cloud state
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "review": {
      "type": "string"
    }
  },
  "required": [
    "review"
  ],
  "title": "VariableSyncApply",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "null",
  "type": "null"
}
```

:::

### sync.v2.variables.fetch {#rpc-sync-v2-variables-fetch}

Descrição do contrato:

```text
Refresh the account cache without publishing queues or applying records
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A v2 network pass persists its cache and attempts supported plugin state\nchanges; other Cloud records are not applied by this operation.",
  "properties": {
    "availableRecords": {
      "description": "Cached records available for application or review.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "moreQueued": {
      "type": "boolean"
    },
    "pulled": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "pushed": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "workspacePending": {
      "description": "Pending edits in the v2 workspace; additional SQLite intents may be\nindicated by `more_queued`.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "pushed",
    "pulled",
    "workspacePending",
    "availableRecords",
    "conflicts",
    "moreQueued"
  ],
  "title": "WorkspaceV2SyncResult",
  "type": "object"
}
```

:::

### sync.v2.variables.review {#rpc-sync-v2-variables-review}

Descrição do contrato:

```text
Compare local variables with the current account cache without network or application
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "RuntimeVariable": {
      "additionalProperties": false,
      "description": "`value` retains the existing numeric API. Exact text accompanies integers\noutside JavaScript's safe range so a browser can edit another row without\nrounding an untouched native number. It is never emitted for a secret.",
      "properties": {
        "numberText": {
          "description": "Required for integers outside JavaScript's exact range. The numeric\n`value` stays for compatibility; this text preserves the native value.",
          "maxLength": 4096,
          "type": [
            "string",
            "null"
          ]
        },
        "type": {
          "$ref": "#/$defs/RuntimeVariableKind"
        },
        "value": true
      },
      "required": [
        "type",
        "value"
      ],
      "type": "object"
    },
    "RuntimeVariableKind": {
      "enum": [
        "text",
        "number",
        "boolean",
        "list",
        "secret"
      ],
      "type": "string"
    },
    "VariableSyncChange": {
      "properties": {
        "blockedLocal": {
          "type": "boolean"
        },
        "canApply": {
          "type": "boolean"
        },
        "canSend": {
          "type": "boolean"
        },
        "canWithdraw": {
          "type": "boolean"
        },
        "cloud": {
          "anyOf": [
            {
              "$ref": "#/$defs/RuntimeVariable"
            },
            {
              "type": "null"
            }
          ]
        },
        "cloudDeleted": {
          "type": "boolean"
        },
        "local": {
          "anyOf": [
            {
              "$ref": "#/$defs/RuntimeVariable"
            },
            {
              "type": "null"
            }
          ]
        },
        "localSecret": {
          "type": "boolean"
        },
        "name": {
          "type": "string"
        },
        "pendingStatus": {
          "type": [
            "string",
            "null"
          ]
        },
        "review": {
          "description": "Native comparison reference, never a value or a credential.",
          "type": "string"
        }
      },
      "required": [
        "review",
        "name",
        "cloudDeleted",
        "localSecret",
        "blockedLocal",
        "canSend",
        "canApply",
        "canWithdraw"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "changes": {
      "items": {
        "$ref": "#/$defs/VariableSyncChange"
      },
      "type": "array"
    }
  },
  "required": [
    "changes"
  ],
  "title": "VariableSyncReview",
  "type": "object"
}
```

:::

### sync.v2.variables.send {#rpc-sync-v2-variables-send}

Descrição do contrato:

```text
Publish only the native variable in an exact reviewed comparison
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "review": {
      "type": "string"
    }
  },
  "required": [
    "review"
  ],
  "title": "VariableSyncApply",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A v2 network pass persists its cache and attempts supported plugin state\nchanges; other Cloud records are not applied by this operation.",
  "properties": {
    "availableRecords": {
      "description": "Cached records available for application or review.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "moreQueued": {
      "type": "boolean"
    },
    "pulled": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "pushed": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "workspacePending": {
      "description": "Pending edits in the v2 workspace; additional SQLite intents may be\nindicated by `more_queued`.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "pushed",
    "pulled",
    "workspacePending",
    "availableRecords",
    "conflicts",
    "moreQueued"
  ],
  "title": "WorkspaceV2SyncResult",
  "type": "object"
}
```

:::

### sync.v2.variables.withdraw {#rpc-sync-v2-variables-withdraw}

Descrição do contrato:

```text
Withdraw one reviewed Cloud variable without changing its local value
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "review": {
      "type": "string"
    }
  },
  "required": [
    "review"
  ],
  "title": "VariableSyncApply",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A v2 network pass persists its cache and attempts supported plugin state\nchanges; other Cloud records are not applied by this operation.",
  "properties": {
    "availableRecords": {
      "description": "Cached records available for application or review.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "conflicts": {
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "moreQueued": {
      "type": "boolean"
    },
    "pulled": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "pushed": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "workspacePending": {
      "description": "Pending edits in the v2 workspace; additional SQLite intents may be\nindicated by `more_queued`.",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "pushed",
    "pulled",
    "workspacePending",
    "availableRecords",
    "conflicts",
    "moreQueued"
  ],
  "title": "WorkspaceV2SyncResult",
  "type": "object"
}
```

:::

### system.doctor {#rpc-system-doctor}

Descrição do contrato:

```text
Structured diagnostics: storage, database, plugins, live, processors
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "DoctorCheck": {
      "properties": {
        "id": {
          "type": "string"
        },
        "message": {
          "type": [
            "string",
            "null"
          ]
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "status"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "checks": {
      "items": {
        "$ref": "#/$defs/DoctorCheck"
      },
      "type": "array"
    },
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok",
    "checks"
  ],
  "title": "DoctorReport",
  "type": "object"
}
```

:::

### system.health {#rpc-system-health}

Descrição do contrato:

```text
Liveness summary: status, live, plugin and processor counts
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### system.info {#rpc-system-info}

Descrição do contrato:

```text
Host info: version, features, paths, pid
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### system.ping {#rpc-system-ping}

Descrição do contrato:

```text
Liveness probe (always { ok: true })
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "`{ ok: true }` result shared by fire-and-forget mutations.",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "OkResult",
  "type": "object"
}
```

:::

### system.requestInputAccess {#rpc-system-requestInputAccess}

Descrição do contrato:

```text
Probe raw-input access; install the seat rule via one polkit prompt when blocked
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Outcome of [`request_input_access`]: whether capture is usable now,\nplus a human sentence for the UI (success) or the failure detail.",
  "properties": {
    "granted": {
      "type": "boolean"
    },
    "message": {
      "type": "string"
    }
  },
  "required": [
    "granted",
    "message"
  ],
  "title": "InputAccessResult",
  "type": "object"
}
```

:::

### system.saveFile {#rpc-system-saveFile}

Descrição do contrato:

```text
Writes one text file into the user's Downloads folder
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "content": {
      "type": "string"
    },
    "filename": {
      "type": "string"
    }
  },
  "required": [
    "filename",
    "content"
  ],
  "title": "SaveFileParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "ok": {
      "type": "boolean"
    },
    "path": {
      "type": "string"
    }
  },
  "required": [
    "ok",
    "path"
  ],
  "title": "SaveFileResult",
  "type": "object"
}
```

:::

### system.shutdown {#rpc-system-shutdown}

Descrição do contrato:

```text
Stops live, plugin polling, and plugin runtimes
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "message": {
      "type": "string"
    },
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok",
    "message"
  ],
  "title": "ShutdownResult",
  "type": "object"
}
```

:::

### system.snapshot {#rpc-system-snapshot}

Descrição do contrato:

```text
Safe observable state (never includes secrets)
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### teams.accessible.list {#rpc-teams-accessible-list}

Descrição do contrato:

```text
Lists only teams approved for this PC, with owner nicknames instead of email addresses
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "PeerMembership": {
      "additionalProperties": false,
      "properties": {
        "clientKind": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "name": {
          "type": "string"
        },
        "ownerNickname": {
          "default": "",
          "type": "string"
        },
        "role": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "name",
        "role",
        "clientKind"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/PeerMembership"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "PeerMemberships",
  "type": "object"
}
```

:::

### teams.commands.run {#rpc-teams-commands-run}

Descrição do contrato:

```text
Sends one command to a team as this PC's viewer and waits for the host's result
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "args": {
      "default": null
    },
    "hostId": {
      "type": "string"
    },
    "operation": {
      "type": "string"
    }
  },
  "required": [
    "hostId",
    "operation"
  ],
  "title": "CommandRunParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "code": {
      "type": [
        "string",
        "null"
      ]
    },
    "commandId": {
      "type": "string"
    },
    "result": true,
    "status": {
      "description": "The host's terminal status: `completed`, `failed`, `cancelled`, `expired` or `unknown`.",
      "type": "string"
    }
  },
  "required": [
    "commandId",
    "status"
  ],
  "title": "CommandOutcome",
  "type": "object"
}
```

:::

### teams.devices.enroll {#rpc-teams-devices-enroll}

Descrição do contrato:

```text
Enrolls this PC's device key as a controller or as a team host, after a signed challenge
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "description": "Required for `host`; the team must belong to the signed-in account.",
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "description": "`controller` for this PC as a viewer, or `host` for this PC as a served team host.",
      "type": "string"
    },
    "label": {
      "type": "string"
    }
  },
  "required": [
    "kind",
    "label"
  ],
  "title": "TeamEnrollParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "createdAt": {
      "default": "",
      "description": "Absent from the enrollment reply; present in the list.",
      "type": "string"
    },
    "hostId": {
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "type": "string"
    },
    "isThisDevice": {
      "default": false,
      "description": "Set only by the local core after matching this PC's persisted host binding.",
      "type": "boolean"
    },
    "kind": {
      "type": "string"
    },
    "label": {
      "type": "string"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "label",
    "kind",
    "status"
  ],
  "title": "TeamDevice",
  "type": "object"
}
```

:::

### teams.devices.list {#rpc-teams-devices-list}

Descrição do contrato:

```text
Lists the devices enrolled for the signed-in account
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "TeamDevice": {
      "properties": {
        "createdAt": {
          "default": "",
          "description": "Absent from the enrollment reply; present in the list.",
          "type": "string"
        },
        "hostId": {
          "type": [
            "string",
            "null"
          ]
        },
        "id": {
          "type": "string"
        },
        "isThisDevice": {
          "default": false,
          "description": "Set only by the local core after matching this PC's persisted host binding.",
          "type": "boolean"
        },
        "kind": {
          "type": "string"
        },
        "label": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "label",
        "kind",
        "status"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/TeamDevice"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "TeamDeviceList",
  "type": "object"
}
```

:::

### teams.devices.revoke {#rpc-teams-devices-revoke}

Descrição do contrato:

```text
Revokes an enrolled device and everything it authorizes
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "deviceId": {
      "type": "string"
    }
  },
  "required": [
    "deviceId"
  ],
  "title": "TeamDeviceParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### teams.grants.list {#rpc-teams-grants-list}

Descrição do contrato:

```text
Lists grants of an owned team
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    }
  },
  "required": [
    "hostId"
  ],
  "title": "TeamHostParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "TeamGrant": {
      "properties": {
        "deviceId": {
          "type": "string"
        },
        "deviceLabel": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "policy": true,
        "revision": {
          "format": "int64",
          "type": "integer"
        },
        "role": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "deviceId",
        "deviceLabel",
        "role",
        "policy",
        "status",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/TeamGrant"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "TeamGrantList",
  "type": "object"
}
```

:::

### teams.grants.revoke {#rpc-teams-grants-revoke}

Descrição do contrato:

```text
Revokes a grant and closes its sessions
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedRevision": {
      "format": "int64",
      "type": "integer"
    },
    "grantId": {
      "type": "string"
    },
    "hostId": {
      "type": "string"
    }
  },
  "required": [
    "hostId",
    "grantId",
    "expectedRevision"
  ],
  "title": "TeamGrantRevokeParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### teams.grants.update {#rpc-teams-grants-update}

Descrição do contrato:

```text
Changes a grant's role or policy; the host must confirm the new revision
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedRevision": {
      "format": "int64",
      "type": "integer"
    },
    "grantId": {
      "type": "string"
    },
    "hostId": {
      "type": "string"
    },
    "policy": true,
    "role": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "hostId",
    "grantId",
    "expectedRevision"
  ],
  "title": "TeamGrantUpdateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### teams.host.start {#rpc-teams-host-start}

Descrição do contrato:

```text
Starts this PC's outbound link for the team it serves; the team must be enrolled first
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "hostId": {
      "type": [
        "string",
        "null"
      ]
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "state": {
      "description": "`stopped`, `connecting`, `ready` or `retrying`.",
      "type": "string"
    }
  },
  "required": [
    "state"
  ],
  "title": "RemoteHostStatus",
  "type": "object"
}
```

:::

### teams.host.status {#rpc-teams-host-status}

Descrição do contrato:

```text
Status of this PC's outbound link as a team host
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "hostId": {
      "type": [
        "string",
        "null"
      ]
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "state": {
      "description": "`stopped`, `connecting`, `ready` or `retrying`.",
      "type": "string"
    }
  },
  "required": [
    "state"
  ],
  "title": "RemoteHostStatus",
  "type": "object"
}
```

:::

### teams.host.stop {#rpc-teams-host-stop}

Descrição do contrato:

```text
Stops this PC's outbound link; pending commands become unknown on the relay
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "hostId": {
      "type": [
        "string",
        "null"
      ]
    },
    "lastError": {
      "type": [
        "string",
        "null"
      ]
    },
    "state": {
      "description": "`stopped`, `connecting`, `ready` or `retrying`.",
      "type": "string"
    }
  },
  "required": [
    "state"
  ],
  "title": "RemoteHostStatus",
  "type": "object"
}
```

:::

### teams.hosts.create {#rpc-teams-hosts-create}

Descrição do contrato:

```text
Creates a remote-control team owned by the signed-in account
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "name": {
      "type": "string"
    }
  },
  "required": [
    "name"
  ],
  "title": "TeamCreateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "name": {
      "type": "string"
    },
    "online": {
      "default": false,
      "type": "boolean"
    },
    "protocolVersion": {
      "format": "int32",
      "type": "integer"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "name",
    "status",
    "protocolVersion",
    "revision"
  ],
  "title": "TeamHost",
  "type": "object"
}
```

:::

### teams.hosts.get {#rpc-teams-hosts-get}

Descrição do contrato:

```text
Reads one owned team
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    }
  },
  "required": [
    "hostId"
  ],
  "title": "TeamHostParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "name": {
      "type": "string"
    },
    "online": {
      "default": false,
      "type": "boolean"
    },
    "protocolVersion": {
      "format": "int32",
      "type": "integer"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "name",
    "status",
    "protocolVersion",
    "revision"
  ],
  "title": "TeamHost",
  "type": "object"
}
```

:::

### teams.hosts.list {#rpc-teams-hosts-list}

Descrição do contrato:

```text
Lists the remote-control teams owned by the signed-in account
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "TeamHost": {
      "properties": {
        "id": {
          "type": "string"
        },
        "name": {
          "type": "string"
        },
        "online": {
          "default": false,
          "type": "boolean"
        },
        "protocolVersion": {
          "format": "int32",
          "type": "integer"
        },
        "revision": {
          "format": "int64",
          "type": "integer"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "name",
        "status",
        "protocolVersion",
        "revision"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/TeamHost"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "TeamHostList",
  "type": "object"
}
```

:::

### teams.hosts.update {#rpc-teams-hosts-update}

Descrição do contrato:

```text
Renames, enables or disables an owned team; requires the current revision
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "expectedRevision": {
      "format": "int64",
      "type": "integer"
    },
    "hostId": {
      "type": "string"
    },
    "name": {
      "type": [
        "string",
        "null"
      ]
    },
    "status": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "hostId",
    "expectedRevision"
  ],
  "title": "TeamUpdateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    },
    "name": {
      "type": "string"
    },
    "online": {
      "default": false,
      "type": "boolean"
    },
    "protocolVersion": {
      "format": "int32",
      "type": "integer"
    },
    "revision": {
      "format": "int64",
      "type": "integer"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "name",
    "status",
    "protocolVersion",
    "revision"
  ],
  "title": "TeamHost",
  "type": "object"
}
```

:::

### teams.invitations.create {#rpc-teams-invitations-create}

Descrição do contrato:

```text
Creates a single-use invitation; the token is returned once and not stored by the host
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "recipientId": {
      "type": [
        "string",
        "null"
      ]
    },
    "requestedRole": {
      "type": "string"
    },
    "ttlSeconds": {
      "format": "int64",
      "type": [
        "integer",
        "null"
      ]
    }
  },
  "required": [
    "hostId",
    "requestedRole"
  ],
  "title": "TeamInvitationCreateParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "The only reply that carries an invitation token. The owner shares it once.",
  "properties": {
    "expiresAt": {
      "type": "string"
    },
    "id": {
      "type": "string"
    },
    "requestedRole": {
      "type": "string"
    },
    "status": {
      "type": "string"
    },
    "token": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "requestedRole",
    "status",
    "expiresAt",
    "token"
  ],
  "title": "TeamInvitationCreated",
  "type": "object"
}
```

:::

### teams.invitations.list {#rpc-teams-invitations-list}

Descrição do contrato:

```text
Lists invitations of an owned team; tokens are never listed
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    }
  },
  "required": [
    "hostId"
  ],
  "title": "TeamHostParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "TeamInvitation": {
      "properties": {
        "expiresAt": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "redeemedAt": {
          "type": [
            "string",
            "null"
          ]
        },
        "requestedRole": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "requestedRole",
        "status",
        "expiresAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/TeamInvitation"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "TeamInvitationList",
  "type": "object"
}
```

:::

### teams.invitations.redeem {#rpc-teams-invitations-redeem}

Descrição do contrato:

```text
Redeems an invitation token with this PC's viewer device; the owner still approves the request
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "token": {
      "description": "The invitation token shown once by the owner.",
      "type": "string"
    }
  },
  "required": [
    "token"
  ],
  "title": "TeamRedeemParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "hostId": {
      "type": "string"
    },
    "requestId": {
      "type": "string"
    },
    "requestedRole": {
      "type": "string"
    },
    "status": {
      "type": "string"
    }
  },
  "required": [
    "requestId",
    "hostId",
    "status",
    "requestedRole"
  ],
  "title": "TeamRedeemResult",
  "type": "object"
}
```

:::

### teams.invitations.revoke {#rpc-teams-invitations-revoke}

Descrição do contrato:

```text
Revokes an unspent invitation
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "invitationId": {
      "type": "string"
    }
  },
  "required": [
    "hostId",
    "invitationId"
  ],
  "title": "TeamInvitationParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### teams.people.search {#rpc-teams-people-search}

Descrição do contrato:

```text
Finds invitation recipients by nickname without returning email addresses
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "query": {
      "type": "string"
    }
  },
  "required": [
    "query"
  ],
  "title": "TeamPeopleSearchParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "TeamPerson": {
      "additionalProperties": false,
      "properties": {
        "ambiguous": {
          "type": "boolean"
        },
        "id": {
          "type": "string"
        },
        "nickname": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "nickname",
        "ambiguous"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/TeamPerson"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "TeamPeople",
  "type": "object"
}
```

:::

### teams.requests.decide {#rpc-teams-requests-decide}

Descrição do contrato:

```text
Approves or denies a join request; approval creates a grant awaiting host confirmation
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "decision": {
      "description": "`approve` or `deny`.",
      "type": "string"
    },
    "hostId": {
      "type": "string"
    },
    "requestId": {
      "type": "string"
    },
    "role": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "hostId",
    "requestId",
    "decision"
  ],
  "title": "TeamDecisionParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### teams.requests.list {#rpc-teams-requests-list}

Descrição do contrato:

```text
Lists join requests of an owned team
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    }
  },
  "required": [
    "hostId"
  ],
  "title": "TeamHostParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "TeamRequest": {
      "properties": {
        "createdAt": {
          "type": "string"
        },
        "deviceLabel": {
          "type": "string"
        },
        "id": {
          "type": "string"
        },
        "requestedRole": {
          "type": "string"
        },
        "status": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "requestedRole",
        "status",
        "deviceLabel",
        "createdAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/TeamRequest"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "TeamRequestList",
  "type": "object"
}
```

:::

### teams.sessions.list {#rpc-teams-sessions-list}

Descrição do contrato:

```text
Lists open controller sessions of an owned team
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    }
  },
  "required": [
    "hostId"
  ],
  "title": "TeamHostParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "TeamSession": {
      "properties": {
        "deviceId": {
          "type": "string"
        },
        "grantId": {
          "type": "string"
        },
        "grantRevision": {
          "format": "int64",
          "type": "integer"
        },
        "id": {
          "type": "string"
        },
        "openedAt": {
          "type": "string"
        }
      },
      "required": [
        "id",
        "grantId",
        "deviceId",
        "grantRevision",
        "openedAt"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "items": {
      "items": {
        "$ref": "#/$defs/TeamSession"
      },
      "type": "array"
    }
  },
  "required": [
    "items"
  ],
  "title": "TeamSessionList",
  "type": "object"
}
```

:::

### teams.sessions.revoke {#rpc-teams-sessions-revoke}

Descrição do contrato:

```text
Closes one controller session
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "hostId": {
      "type": "string"
    },
    "sessionId": {
      "type": "string"
    }
  },
  "required": [
    "hostId",
    "sessionId"
  ],
  "title": "TeamSessionParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### ui.feed.backfill {#rpc-ui-feed-backfill}

Descrição do contrato:

```text
Bounded read-only LIVE render history
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "after": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "limit": {
      "default": null,
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "version": {
      "format": "uint32",
      "minimum": 0,
      "type": "integer"
    }
  },
  "required": [
    "version",
    "after"
  ],
  "title": "FeedBackfillParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "FeedEntry": {
      "properties": {
        "cursor": {
          "format": "uint64",
          "minimum": 0,
          "type": "integer"
        },
        "event": true
      },
      "required": [
        "cursor",
        "event"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Bounded feed replay payload, matching the existing desktop recovery contract.",
  "properties": {
    "depth": {
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "events": {
      "items": {
        "$ref": "#/$defs/FeedEntry"
      },
      "type": "array"
    },
    "gap": {
      "type": "boolean"
    },
    "latestCursor": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "oldestCursor": {
      "format": "uint64",
      "minimum": 0,
      "type": "integer"
    },
    "truncated": {
      "type": "boolean"
    }
  },
  "required": [
    "oldestCursor",
    "latestCursor",
    "depth",
    "gap",
    "truncated",
    "events"
  ],
  "title": "FeedBackfill",
  "type": "object"
}
```

:::

### ui.feed.browser {#rpc-ui-feed-browser}

Descrição do contrato:

```text
Bounded browser-safe LIVE history
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "after": {
      "type": "string"
    }
  },
  "required": [
    "after"
  ],
  "title": "BrowserFeedParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### ui.feed.creator {#rpc-ui-feed-creator}

Descrição do contrato:

```text
Creator-scoped bounded LIVE history
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "after": {
      "type": "string"
    },
    "creatorId": {
      "type": "string"
    },
    "limit": {
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "streamId": {
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "creatorId",
    "after"
  ],
  "title": "CreatorFeedParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "CreatorFeedEvent": {
      "properties": {
        "author": {
          "type": "string"
        },
        "authorLookupId": {
          "type": [
            "string",
            "null"
          ]
        },
        "cursor": {
          "type": "string"
        },
        "kind": {
          "type": "string"
        },
        "nickname": {
          "type": [
            "string",
            "null"
          ]
        },
        "text": {
          "type": "string"
        }
      },
      "required": [
        "cursor",
        "kind",
        "author",
        "text"
      ],
      "type": "object"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "A creator-local projection. Neither cursors nor the stream identity reveal\nthe sequence or volume of other creators' activity.",
  "properties": {
    "creatorId": {
      "type": "string"
    },
    "events": {
      "items": {
        "$ref": "#/$defs/CreatorFeedEvent"
      },
      "type": "array"
    },
    "gap": {
      "type": "boolean"
    },
    "latestCursor": {
      "type": "string"
    },
    "nextCursor": {
      "type": "string"
    },
    "oldestCursor": {
      "type": "string"
    },
    "streamId": {
      "type": [
        "string",
        "null"
      ]
    },
    "truncated": {
      "type": "boolean"
    }
  },
  "required": [
    "creatorId",
    "oldestCursor",
    "latestCursor",
    "nextCursor",
    "gap",
    "truncated",
    "events"
  ],
  "title": "CreatorFeedPage",
  "type": "object"
}
```

:::

### widgets.copyCloudOverlayUrl {#rpc-widgets-copyCloudOverlayUrl}

Descrição do contrato:

```text
Copies the OBS Browser Source URL of a cloud overlay, with its widget credential, to the OS clipboard
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":true,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "additionalProperties": false,
  "properties": {
    "overlay": {
      "description": "base64url JSON of the cloud overlay document, at most 6000 characters.",
      "type": "string"
    }
  },
  "required": [
    "overlay"
  ],
  "title": "WidgetsCopyCloudOverlayParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "WidgetsCopyResult",
  "type": "object"
}
```

:::

### widgets.copyObsUrl {#rpc-widgets-copyObsUrl}

Descrição do contrato:

```text
Copies one OBS Browser Source URL (with its widget credential) to the OS clipboard; the URL is never returned
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":true,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "competitionId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "rankingId": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "showLabels": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "showLegend": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "showPercentages": {
      "default": null,
      "type": [
        "boolean",
        "null"
      ]
    },
    "visualization": {
      "default": null,
      "type": [
        "string",
        "null"
      ]
    },
    "widget": {
      "type": "string"
    }
  },
  "required": [
    "widget"
  ],
  "title": "WidgetsCopyParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "ok": {
      "type": "boolean"
    }
  },
  "required": [
    "ok"
  ],
  "title": "WidgetsCopyResult",
  "type": "object"
}
```

:::

### widgets.status {#rpc-widgets-status}

Descrição do contrato:

```text
OBS widgets readiness: gateway state, loopback port, and a secret-free hint
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$defs": {
    "WidgetsState": {
      "oneOf": [
        {
          "const": "missing",
          "description": "The Event Gateway plugin is not installed/discovered.",
          "type": "string"
        },
        {
          "const": "disabled",
          "description": "Installed but disabled (or unavailable).",
          "type": "string"
        },
        {
          "const": "stopped",
          "description": "Enabled but its process is not running.",
          "type": "string"
        },
        {
          "const": "starting",
          "description": "Process running, loopback port not accepting yet (mid-startup).",
          "type": "string"
        },
        {
          "const": "credential-unavailable",
          "description": "Running (or ready to run) but no usable widget credential is stored.",
          "type": "string"
        },
        {
          "const": "assets-missing",
          "description": "Gateway answers but `/widgets/*` is not served (assets missing).",
          "type": "string"
        },
        {
          "const": "unreachable",
          "description": "Reachable socket but no valid gateway HTTP answers.",
          "type": "string"
        },
        {
          "const": "ready",
          "description": "Credential present, health OK, every widget bundle served.",
          "type": "string"
        }
      ]
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "error": {
      "description": "Human-safe detail; never contains secrets.",
      "type": [
        "string",
        "null"
      ]
    },
    "port": {
      "format": "uint16",
      "maximum": 65535,
      "minimum": 0,
      "type": "integer"
    },
    "state": {
      "$ref": "#/$defs/WidgetsState"
    }
  },
  "required": [
    "state",
    "port"
  ],
  "title": "WidgetsStatusResult",
  "type": "object"
}
```

:::

### workflows.delete {#rpc-workflows-delete}

Descrição do contrato:

```text
Deletes a graph workflow
```

`{"sideEffect":true,"destructive":true,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "WorkflowIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "deleted": {
      "type": "boolean"
    },
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id",
    "deleted"
  ],
  "title": "WorkflowDeleteResult",
  "type": "object"
}
```

:::

### workflows.disable {#rpc-workflows-disable}

Descrição do contrato:

```text
Disables a graph workflow
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "WorkflowIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### workflows.enable {#rpc-workflows-enable}

Descrição do contrato:

```text
Enables a graph workflow
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "WorkflowIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### workflows.get {#rpc-workflows-get}

Descrição do contrato:

```text
One graph workflow by id
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": [
    "id"
  ],
  "title": "WorkflowIdParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::

### workflows.list {#rpc-workflows-list}

Descrição do contrato:

```text
Graph workflows for the node-canvas editor
```

`{"sideEffect":false,"destructive":false,"requiresDesktop":false,"concurrency":null,"durableCommands":false}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty `{}` params shared by read-only methods.",
  "title": "Empty",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "workflows": {
      "items": true,
      "type": "array"
    }
  },
  "required": [
    "workflows"
  ],
  "title": "WorkflowListResult",
  "type": "object"
}
```

:::

### workflows.save {#rpc-workflows-save}

Descrição do contrato:

```text
Creates or replaces a graph workflow
```

`{"sideEffect":true,"destructive":false,"requiresDesktop":false,"concurrency":"compare-and-set","durableCommands":true}`

::: details Parâmetros

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "graph": true
  },
  "required": [
    "graph"
  ],
  "title": "WorkflowSaveParams",
  "type": "object"
}
```

:::

::: details Resultado

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "AnyValue"
}
```

:::
