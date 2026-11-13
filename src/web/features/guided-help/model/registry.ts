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
  automation: {
    id: 'automation', version: 1, route: '/automations', scope: 'automations', task: 'automation.saved',
    steps: [
      { id: 'profiles', version: 1, target: 'automation-profiles', titleKey: 'automation.profiles.title', descriptionKey: 'automation.profiles.description', requiredCapabilities: [], missingTargetPolicy: 'cancel' },
      { id: 'library', version: 1, target: 'automation-library', titleKey: 'automation.library.title', descriptionKey: 'automation.library.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
      { id: 'templates', version: 1, target: 'automation-templates', titleKey: 'automation.templates.title', descriptionKey: 'automation.templates.description', requiredCapabilities: [], missingTargetPolicy: 'skip' },
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
} as const satisfies Record<TourId, TourDefinition>;

export function isTourId(value: string): value is TourId {
  return Object.hasOwn(tourRegistry, value);
}
