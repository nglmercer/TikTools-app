import type { TourDefinition, TourId } from './types.ts';

/** Stable targets/IDs are public. Bump the tour version when a step version, target or contract changes. */
export const tourRegistry = {
  connection: {
    id: 'connection', version: 1, route: '/connections', scope: 'connections', task: 'connection.connected',
    steps: [
      { id: 'form', version: 1, target: 'connection-form', titleKey: 'connection.form.title', descriptionKey: 'connection.form.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'explorer', version: 1, target: 'connection-explorer', titleKey: 'connection.explorer.title', descriptionKey: 'connection.explorer.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'plugins', version: 1, target: 'connection-plugins', titleKey: 'connection.plugins.title', descriptionKey: 'connection.plugins.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  automationEvents: {
    id: 'automationEvents', version: 2, route: '/automations', scope: 'automations', task: 'automation.saved',
    steps: [
      { id: 'profiles', version: 1, target: 'automation-profiles', titleKey: 'automationEvents.profiles.title', descriptionKey: 'automationEvents.profiles.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'eventsTab', version: 2, target: 'automation-tab-events', titleKey: 'automationEvents.eventsTab.title', descriptionKey: 'automationEvents.eventsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'events', version: 2, target: 'automation-events', titleKey: 'automationEvents.events.title', descriptionKey: 'automationEvents.events.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]'] },
      { id: 'editor', version: 2, target: 'automation-editor', titleKey: 'automationEvents.editor.title', descriptionKey: 'automationEvents.editor.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-new-record"]'] },
      { id: 'runs', version: 1, target: 'automation-runs', titleKey: 'automationEvents.runs.title', descriptionKey: 'automationEvents.runs.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  plugin: {
    id: 'plugin', version: 1, route: '/plugins', scope: 'plugins', task: 'plugin.installed',
    steps: [
      { id: 'catalog', version: 1, target: 'plugin-catalog', titleKey: 'plugin.catalog.title', descriptionKey: 'plugin.catalog.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'search', version: 1, target: 'plugins-search', titleKey: 'plugin.search.title', descriptionKey: 'plugin.search.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'install', version: 1, target: 'plugins-install', titleKey: 'plugin.install.title', descriptionKey: 'plugin.install.description', requiredCapabilities: ['plugins.packages.install'], missingTargetPolicy: 'skip' },
    ],
  },
  overlay: {
    id: 'overlay', version: 1, route: '/overlays', scope: 'overlays', task: 'overlay.previewed',
    steps: [
      { id: 'gateway', version: 1, target: 'overlay-gateway', titleKey: 'overlay.gateway.title', descriptionKey: 'overlay.gateway.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'preview', version: 1, target: 'overlay-preview', titleKey: 'overlay.preview.title', descriptionKey: 'overlay.preview.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'design', version: 1, target: 'overlay-design', titleKey: 'overlay.design.title', descriptionKey: 'overlay.design.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  points: {
    id: 'points', version: 1, route: '/points', scope: 'points', task: 'points.reviewed',
    steps: [
      { id: 'rules', version: 1, target: 'points-rules', titleKey: 'points.rules.title', descriptionKey: 'points.rules.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'leaderboard', version: 1, target: 'points-leaderboard', titleKey: 'points.leaderboard.title', descriptionKey: 'points.leaderboard.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
    ],
  },
  changes: {
    id: 'changes', version: 2, route: '/automations?view=variables', scope: 'automations', task: 'changes.reviewed',
    steps: [
      { id: 'library', version: 1, target: 'automation-library', titleKey: 'changes.library.title', descriptionKey: 'changes.library.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'review', version: 1, target: 'changes', titleKey: 'changes.review.title', descriptionKey: 'changes.review.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'profiles', version: 1, target: 'automation-profiles', titleKey: 'changes.profiles.title', descriptionKey: 'changes.profiles.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  diagnostics: {
    id: 'diagnostics', version: 1, route: '/debug', scope: 'diagnostics', task: 'diagnostics.inspected',
    steps: [
      { id: 'log', version: 1, target: 'diagnostics-log', titleKey: 'diagnostics.log.title', descriptionKey: 'diagnostics.log.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'back', version: 1, target: 'diagnostics-actions', titleKey: 'diagnostics.back.title', descriptionKey: 'diagnostics.back.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  feed: {
    id: 'feed', version: 2, route: '/feed', scope: 'feed', task: 'feed.reviewed',
    steps: [
      { id: 'view', version: 1, target: 'feed-view', titleKey: 'feed.view.title', descriptionKey: 'feed.view.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'filters', version: 1, target: 'feed-filters', titleKey: 'feed.filters.title', descriptionKey: 'feed.filters.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'stage', version: 2, target: 'feed-stage', titleKey: 'feed.stage.title', descriptionKey: 'feed.stage.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'controls', version: 1, target: 'feed-controls', titleKey: 'feed.controls.title', descriptionKey: 'feed.controls.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  backup: {
    id: 'backup', version: 2, route: '/settings?section=account', scope: 'account', task: 'backup.saved',
    steps: [
      { id: 'plan', version: 1, target: 'account-plan', titleKey: 'backup.plan.title', descriptionKey: 'backup.plan.description', requiredCapabilities: ['cloud.plan.get'], missingTargetPolicy: 'skip' },
      { id: 'tools', version: 2, target: 'backup-tools', titleKey: 'backup.tools.title', descriptionKey: 'backup.tools.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'create', version: 2, target: 'backup-create', titleKey: 'backup.create.title', descriptionKey: 'backup.create.description', requiredCapabilities: ['cloud.backup.createNamed'], missingTargetPolicy: 'skip' },
      { id: 'list', version: 1, target: 'backup-list', titleKey: 'backup.list.title', descriptionKey: 'backup.list.description', requiredCapabilities: ['cloud.backup.list'], missingTargetPolicy: 'skip' },
    ],
  },

  analytics: {
    id: 'analytics', version: 1, route: '/analytics', scope: 'analytics', task: undefined,
    steps: [
      { id: 'range', version: 1, target: 'analytics-range', titleKey: 'analytics.range.title', descriptionKey: 'analytics.range.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'metrics', version: 1, target: 'analytics-metrics', titleKey: 'analytics.metrics.title', descriptionKey: 'analytics.metrics.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'activity', version: 1, target: 'analytics-activity', titleKey: 'analytics.activity.title', descriptionKey: 'analytics.activity.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'facts', version: 1, target: 'analytics-facts', titleKey: 'analytics.facts.title', descriptionKey: 'analytics.facts.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'contributors', version: 1, target: 'analytics-contributors', titleKey: 'analytics.contributors.title', descriptionKey: 'analytics.contributors.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  automationVariables: {
    id: 'automationVariables', version: 2, route: '/automations?view=variables', scope: 'automations', task: undefined,
    steps: [
      { id: 'variablesTab', version: 1, target: 'automation-tab-variables', titleKey: 'automationVariables.variablesTab.title', descriptionKey: 'automationVariables.variablesTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'values', version: 2, target: 'automation-variables', titleKey: 'automationVariables.values.title', descriptionKey: 'automationVariables.values.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-variables"]'] },
      { id: 'review', version: 1, target: 'changes', titleKey: 'automationVariables.review.title', descriptionKey: 'automationVariables.review.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  settings: {
    id: 'settings', version: 1, route: '/settings', scope: 'settings', task: undefined,
    steps: [
      { id: 'sections', version: 1, target: 'settings-sections', titleKey: 'settings.sections.title', descriptionKey: 'settings.sections.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'language', version: 1, target: 'settings-language', titleKey: 'settings.language.title', descriptionKey: 'settings.language.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'review', version: 1, target: 'settings-review', titleKey: 'settings.review.title', descriptionKey: 'settings.review.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'updater', version: 1, target: 'settings-updater', titleKey: 'settings.updater.title', descriptionKey: 'settings.updater.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  overlayObs: {
    id: 'overlayObs', version: 1, route: '/overlays', scope: 'overlays', task: undefined,
    steps: [
      { id: 'gateway', version: 1, target: 'overlay-gateway', titleKey: 'overlayObs.gateway.title', descriptionKey: 'overlayObs.gateway.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'widget', version: 1, target: 'overlay-widgets', titleKey: 'overlayObs.widget.title', descriptionKey: 'overlayObs.widget.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'preview', version: 1, target: 'overlay-preview', titleKey: 'overlayObs.preview.title', descriptionKey: 'overlayObs.preview.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'url', version: 1, target: 'overlay-url', titleKey: 'overlayObs.url.title', descriptionKey: 'overlayObs.url.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'design', version: 1, target: 'overlay-design', titleKey: 'overlayObs.design.title', descriptionKey: 'overlayObs.design.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  cloudOverlays: {
    id: 'cloudOverlays', version: 1, route: '/overlays', scope: 'overlays', task: undefined,
    steps: [
      { id: 'open', version: 1, target: 'overlay-cloud-open', titleKey: 'cloudOverlays.open.title', descriptionKey: 'cloudOverlays.open.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'widgets', version: 1, target: 'overlay-widgets', titleKey: 'cloudOverlays.widgets.title', descriptionKey: 'cloudOverlays.widgets.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'preview', version: 1, target: 'overlay-preview', titleKey: 'cloudOverlays.preview.title', descriptionKey: 'cloudOverlays.preview.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'url', version: 1, target: 'overlay-url', titleKey: 'cloudOverlays.url.title', descriptionKey: 'cloudOverlays.url.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  automationActions: {
    id: 'automationActions', version: 2, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'actionsTab', version: 2, target: 'automation-tab-actions', titleKey: 'automationActions.actionsTab.title', descriptionKey: 'automationActions.actionsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'actions', version: 2, target: 'automation-actions', titleKey: 'automationActions.actions.title', descriptionKey: 'automationActions.actions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]'] },
      { id: 'editor', version: 2, target: 'automation-editor', titleKey: 'automationActions.editor.title', descriptionKey: 'automationActions.editor.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-new-record"]'] },
      { id: 'templates', version: 1, target: 'automation-templates', titleKey: 'automationActions.templates.title', descriptionKey: 'automationActions.templates.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  rankings: {
    id: 'rankings', version: 1, route: '/rankings', scope: 'rankings', task: undefined,
    steps: [
      { id: 'tabs', version: 1, target: 'rankings-tabs', titleKey: 'rankings.tabs.title', descriptionKey: 'rankings.tabs.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'list', version: 1, target: 'rankings-list', titleKey: 'rankings.list.title', descriptionKey: 'rankings.list.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'detail', version: 1, target: 'rankings-detail', titleKey: 'rankings.detail.title', descriptionKey: 'rankings.detail.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'competitions', version: 1, target: 'rankings-competitions', titleKey: 'rankings.competitions.title', descriptionKey: 'rankings.competitions.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  pluginSetup: {
    id: 'pluginSetup', version: 1, route: '/plugins', scope: 'plugins', task: undefined,
    steps: [
      { id: 'catalog', version: 1, target: 'plugin-catalog', titleKey: 'pluginSetup.catalog.title', descriptionKey: 'pluginSetup.catalog.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'details', version: 1, target: 'plugin-details', titleKey: 'pluginSetup.details.title', descriptionKey: 'pluginSetup.details.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'settings', version: 1, target: 'plugin-settings', titleKey: 'pluginSetup.settings.title', descriptionKey: 'pluginSetup.settings.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'connection', version: 1, target: 'plugin-connection', titleKey: 'pluginSetup.connection.title', descriptionKey: 'pluginSetup.connection.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  connectionRecovery: {
    id: 'connectionRecovery', version: 1, route: '/connections', scope: 'connections', task: undefined,
    steps: [
      { id: 'form', version: 1, target: 'connection-form', titleKey: 'connectionRecovery.form.title', descriptionKey: 'connectionRecovery.form.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'explorer', version: 1, target: 'connection-explorer', titleKey: 'connectionRecovery.explorer.title', descriptionKey: 'connectionRecovery.explorer.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'plugins', version: 1, target: 'connection-plugins', titleKey: 'connectionRecovery.plugins.title', descriptionKey: 'connectionRecovery.plugins.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  feedInspector: {
    id: 'feedInspector', version: 1, route: '/feed', scope: 'feed', task: undefined,
    steps: [
      { id: 'controls', version: 1, target: 'feed-controls', titleKey: 'feedInspector.controls.title', descriptionKey: 'feedInspector.controls.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'panel', version: 1, target: 'feed-inspector', titleKey: 'feedInspector.panel.title', descriptionKey: 'feedInspector.panel.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'viewers', version: 1, target: 'feed-inspector-viewers', titleKey: 'feedInspector.viewers.title', descriptionKey: 'feedInspector.viewers.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'details', version: 1, target: 'feed-inspector-details', titleKey: 'feedInspector.details.title', descriptionKey: 'feedInspector.details.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'profile', version: 1, target: 'feed-inspector-profile', titleKey: 'feedInspector.profile.title', descriptionKey: 'feedInspector.profile.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  backupRestore: {
    id: 'backupRestore', version: 1, route: '/settings?section=account', scope: 'account', task: undefined,
    steps: [
      { id: 'tools', version: 1, target: 'backup-tools', titleKey: 'backupRestore.tools.title', descriptionKey: 'backupRestore.tools.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'list', version: 1, target: 'backup-list', titleKey: 'backupRestore.list.title', descriptionKey: 'backupRestore.list.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'plan', version: 1, target: 'account-plan', titleKey: 'backupRestore.plan.title', descriptionKey: 'backupRestore.plan.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
    ],
  },
  automationEventChat: {
    id: 'automationEventChat', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'eventsTab', version: 1, target: 'automation-tab-events', titleKey: 'automationEventChat.eventsTab.title', descriptionKey: 'automationEventChat.eventsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'events', version: 1, target: 'automation-events', titleKey: 'automationEventChat.events.title', descriptionKey: 'automationEventChat.events.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]'] },
      { id: 'trigger', version: 1, target: 'event-editor-trigger', titleKey: 'automationEventChat.trigger.title', descriptionKey: 'automationEventChat.trigger.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'conditions', version: 1, target: 'event-editor-conditions', titleKey: 'automationEventChat.conditions.title', descriptionKey: 'automationEventChat.conditions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
    ],
  },
  automationEventGift: {
    id: 'automationEventGift', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'eventsTab', version: 1, target: 'automation-tab-events', titleKey: 'automationEventGift.eventsTab.title', descriptionKey: 'automationEventGift.eventsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'events', version: 1, target: 'automation-events', titleKey: 'automationEventGift.events.title', descriptionKey: 'automationEventGift.events.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]'] },
      { id: 'trigger', version: 1, target: 'event-editor-trigger', titleKey: 'automationEventGift.trigger.title', descriptionKey: 'automationEventGift.trigger.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'conditions', version: 1, target: 'event-editor-conditions', titleKey: 'automationEventGift.conditions.title', descriptionKey: 'automationEventGift.conditions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'cooldown', version: 1, target: 'event-editor-cooldown', titleKey: 'automationEventGift.cooldown.title', descriptionKey: 'automationEventGift.cooldown.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
    ],
  },
  automationEventSocial: {
    id: 'automationEventSocial', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'eventsTab', version: 1, target: 'automation-tab-events', titleKey: 'automationEventSocial.eventsTab.title', descriptionKey: 'automationEventSocial.eventsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'events', version: 1, target: 'automation-events', titleKey: 'automationEventSocial.events.title', descriptionKey: 'automationEventSocial.events.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]'] },
      { id: 'trigger', version: 1, target: 'event-editor-trigger', titleKey: 'automationEventSocial.trigger.title', descriptionKey: 'automationEventSocial.trigger.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'conditions', version: 1, target: 'event-editor-conditions', titleKey: 'automationEventSocial.conditions.title', descriptionKey: 'automationEventSocial.conditions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
    ],
  },
  automationEventRoom: {
    id: 'automationEventRoom', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'eventsTab', version: 1, target: 'automation-tab-events', titleKey: 'automationEventRoom.eventsTab.title', descriptionKey: 'automationEventRoom.eventsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'events', version: 1, target: 'automation-events', titleKey: 'automationEventRoom.events.title', descriptionKey: 'automationEventRoom.events.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]'] },
      { id: 'trigger', version: 1, target: 'event-editor-trigger', titleKey: 'automationEventRoom.trigger.title', descriptionKey: 'automationEventRoom.trigger.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'conditions', version: 1, target: 'event-editor-conditions', titleKey: 'automationEventRoom.conditions.title', descriptionKey: 'automationEventRoom.conditions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
    ],
  },
  automationActionMedia: {
    id: 'automationActionMedia', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'actionsTab', version: 1, target: 'automation-tab-actions', titleKey: 'automationActionMedia.actionsTab.title', descriptionKey: 'automationActionMedia.actionsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'actions', version: 1, target: 'automation-actions', titleKey: 'automationActionMedia.actions.title', descriptionKey: 'automationActionMedia.actions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]'] },
      { id: 'settings', version: 1, target: 'action-editor-settings', titleKey: 'automationActionMedia.settings.title', descriptionKey: 'automationActionMedia.settings.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]', '[data-guide="automation-new-record"]'] },
      { id: 'permissions', version: 1, target: 'action-editor-permissions', titleKey: 'automationActionMedia.permissions.title', descriptionKey: 'automationActionMedia.permissions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]', '[data-guide="automation-new-record"]'] },
    ],
  },
  automationActionTts: {
    id: 'automationActionTts', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'actionsTab', version: 1, target: 'automation-tab-actions', titleKey: 'automationActionTts.actionsTab.title', descriptionKey: 'automationActionTts.actionsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'actions', version: 1, target: 'automation-actions', titleKey: 'automationActionTts.actions.title', descriptionKey: 'automationActionTts.actions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]'] },
      { id: 'settings', version: 1, target: 'action-editor-settings', titleKey: 'automationActionTts.settings.title', descriptionKey: 'automationActionTts.settings.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]', '[data-guide="automation-new-record"]'] },
      { id: 'permissions', version: 1, target: 'action-editor-permissions', titleKey: 'automationActionTts.permissions.title', descriptionKey: 'automationActionTts.permissions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]', '[data-guide="automation-new-record"]'] },
    ],
  },
  automationActionHttp: {
    id: 'automationActionHttp', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'actionsTab', version: 1, target: 'automation-tab-actions', titleKey: 'automationActionHttp.actionsTab.title', descriptionKey: 'automationActionHttp.actionsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'actions', version: 1, target: 'automation-actions', titleKey: 'automationActionHttp.actions.title', descriptionKey: 'automationActionHttp.actions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]'] },
      { id: 'settings', version: 1, target: 'action-editor-settings', titleKey: 'automationActionHttp.settings.title', descriptionKey: 'automationActionHttp.settings.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]', '[data-guide="automation-new-record"]'] },
      { id: 'permissions', version: 1, target: 'action-editor-permissions', titleKey: 'automationActionHttp.permissions.title', descriptionKey: 'automationActionHttp.permissions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]', '[data-guide="automation-new-record"]'] },
    ],
  },
  automationActionVars: {
    id: 'automationActionVars', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'actionsTab', version: 1, target: 'automation-tab-actions', titleKey: 'automationActionVars.actionsTab.title', descriptionKey: 'automationActionVars.actionsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'actions', version: 1, target: 'automation-actions', titleKey: 'automationActionVars.actions.title', descriptionKey: 'automationActionVars.actions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]'] },
      { id: 'settings', version: 1, target: 'action-editor-settings', titleKey: 'automationActionVars.settings.title', descriptionKey: 'automationActionVars.settings.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]', '[data-guide="automation-new-record"]'] },
      { id: 'permissions', version: 1, target: 'action-editor-permissions', titleKey: 'automationActionVars.permissions.title', descriptionKey: 'automationActionVars.permissions.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]', '[data-guide="automation-new-record"]'] },
    ],
  },
  automationFilters: {
    id: 'automationFilters', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'eventsTab', version: 1, target: 'automation-tab-events', titleKey: 'automationFilters.eventsTab.title', descriptionKey: 'automationFilters.eventsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'operators', version: 1, target: 'event-editor-conditions', titleKey: 'automationFilters.operators.title', descriptionKey: 'automationFilters.operators.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'quantifiers', version: 1, target: 'event-editor-run', titleKey: 'automationFilters.quantifiers.title', descriptionKey: 'automationFilters.quantifiers.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'verify', version: 1, target: 'automation-runs', titleKey: 'automationFilters.verify.title', descriptionKey: 'automationFilters.verify.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]'] },
    ],
  },
  automationCooldowns: {
    id: 'automationCooldowns', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'eventsTab', version: 1, target: 'automation-tab-events', titleKey: 'automationCooldowns.eventsTab.title', descriptionKey: 'automationCooldowns.eventsTab.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'scope', version: 1, target: 'event-editor-cooldown', titleKey: 'automationCooldowns.scope.title', descriptionKey: 'automationCooldowns.scope.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'runMode', version: 1, target: 'event-editor-run', titleKey: 'automationCooldowns.runMode.title', descriptionKey: 'automationCooldowns.runMode.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
      { id: 'verify', version: 1, target: 'automation-runs', titleKey: 'automationCooldowns.verify.title', descriptionKey: 'automationCooldowns.verify.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]'] },
    ],
  },
  automationTemplates: {
    id: 'automationTemplates', version: 1, route: '/automations', scope: 'automations', task: undefined,
    steps: [
      { id: 'gallery', version: 1, target: 'automation-templates', titleKey: 'automationTemplates.gallery.title', descriptionKey: 'automationTemplates.gallery.description', requiredCapabilities: [], missingTargetPolicy: 'cancel', prepareClick: ['[data-guide="automation-editor-close"]'] },
      { id: 'events', version: 1, target: 'automation-events', titleKey: 'automationTemplates.events.title', descriptionKey: 'automationTemplates.events.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]'] },
      { id: 'review', version: 1, target: 'event-editor-trigger', titleKey: 'automationTemplates.review.title', descriptionKey: 'automationTemplates.review.description', requiredCapabilities: [], missingTargetPolicy: 'skip', prepareClick: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-events"]', '[data-guide="automation-new-record"]'] },
    ],
  },
} as const satisfies Record<TourId, TourDefinition>;

export function isTourId(value: string): value is TourId {
  return Object.hasOwn(tourRegistry, value);
}
