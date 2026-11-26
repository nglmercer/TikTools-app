/**
 * Canonical semantic metadata for guide targets.
 *
 * `data-guide` attributes identify real DOM elements. This registry is the
 * only place allowed to interpret what those targets mean visually: which
 * schematic region they belong to and where explanatory popovers should
 * prefer to open. Never infer either one from the target name with regular
 * expressions; a target missing here is a contract error.
 */
export type GuideLayoutZone = 'toolbar' | 'tabs' | 'list' | 'editor' | 'preview' | 'detail' | 'footer';
export type GuidePopoverSide = 'top' | 'right' | 'bottom' | 'left';
export type GuidePopoverAlign = 'start' | 'center' | 'end';
export interface GuideTargetMetadata {
  zone: GuideLayoutZone;
  side?: GuidePopoverSide;
  align?: GuidePopoverAlign;
}

const bottomStart = { side: 'bottom', align: 'start' } as const;
const bottomEnd = { side: 'bottom', align: 'end' } as const;
const rightStart = { side: 'right', align: 'start' } as const;
const leftStart = { side: 'left', align: 'start' } as const;
const topStart = { side: 'top', align: 'start' } as const;

/** Exact target metadata, keyed by the stable `data-guide` value. */
export const guideTargets = {
  // Automation library and record editor.
  'automation-library': { zone: 'list', ...rightStart },
  'automation-profiles': { zone: 'toolbar', ...bottomStart },
  'automation-templates': { zone: 'toolbar', ...bottomStart },
  'automation-tab-events': { zone: 'tabs', ...bottomStart },
  'automation-tab-actions': { zone: 'tabs', ...bottomStart },
  'automation-tab-variables': { zone: 'tabs', ...bottomStart },
  'automation-events': { zone: 'list', ...rightStart },
  'automation-actions': { zone: 'list', ...rightStart },
  'automation-variables': { zone: 'editor', ...rightStart },
  'automation-editor': { zone: 'editor', ...leftStart },
  'event-editor-trigger': { zone: 'editor', ...leftStart },
  'event-editor-conditions': { zone: 'editor', ...leftStart },
  'event-editor-cooldown': { zone: 'editor', ...leftStart },
  'event-editor-run': { zone: 'editor', ...leftStart },
  'action-editor-settings': { zone: 'editor', ...leftStart },
  'action-editor-permissions': { zone: 'editor', ...leftStart },
  'automation-runs': { zone: 'footer', ...topStart },
  changes: { zone: 'toolbar', ...bottomEnd },

  // Connection.
  'connection-form': { zone: 'editor', ...rightStart },
  'connection-explorer': { zone: 'detail', ...leftStart },
  'connection-plugins': { zone: 'list', ...rightStart },

  // Live feed and inspector.
  'feed-view': { zone: 'list', ...rightStart },
  'feed-filters': { zone: 'toolbar', ...bottomStart },
  'feed-stage': { zone: 'preview', ...bottomStart },
  'feed-controls': { zone: 'toolbar', ...bottomEnd },
  'feed-inspector': { zone: 'detail', ...leftStart },
  'feed-inspector-viewers': { zone: 'detail', ...leftStart },
  'feed-inspector-details': { zone: 'detail', ...leftStart },
  'feed-inspector-profile': { zone: 'detail', ...leftStart },

  // Plugins.
  'plugin-catalog': { zone: 'list', ...rightStart },
  'plugins-search': { zone: 'toolbar', ...bottomStart },
  'plugins-install': { zone: 'toolbar', ...bottomEnd },
  'plugin-details': { zone: 'detail', ...leftStart },
  'plugin-settings': { zone: 'editor', ...leftStart },
  'plugin-connection': { zone: 'detail', ...leftStart },

  // Overlays.
  'overlay-widgets': { zone: 'list', ...rightStart },
  'overlay-preview': { zone: 'preview', ...leftStart },
  'overlay-design': { zone: 'editor', ...leftStart },
  'overlay-url': { zone: 'editor', ...bottomStart },
  'overlay-gateway': { zone: 'toolbar', ...bottomStart },
  'overlay-cloud-open': { zone: 'toolbar', ...bottomEnd },

  // Points, analytics and diagnostics.
  'points-rules': { zone: 'list', ...rightStart },
  'points-leaderboard': { zone: 'list', ...rightStart },
  'analytics-range': { zone: 'toolbar', ...bottomStart },
  'analytics-metrics': { zone: 'preview', ...bottomStart },
  'analytics-facts': { zone: 'preview', ...bottomStart },
  'analytics-activity': { zone: 'list', ...topStart },
  'analytics-contributors': { zone: 'list', ...topStart },
  'analytics-sessions': { zone: 'list', ...topStart },
  'diagnostics-log': { zone: 'list', ...rightStart },
  'diagnostics-actions': { zone: 'toolbar', ...bottomEnd },

  // Account and backup.
  'account-plan': { zone: 'toolbar', ...bottomStart },
  'backup-tools': { zone: 'toolbar', ...bottomStart },
  'backup-create': { zone: 'editor', ...rightStart },
  'backup-list': { zone: 'detail', ...topStart },

  // Settings.
  'settings-sections': { zone: 'list', ...rightStart },
  'settings-language': { zone: 'editor', ...rightStart },
  'settings-updater': { zone: 'editor', ...rightStart },
  'settings-review': { zone: 'detail', ...leftStart },

  // Rankings are internal while the management page is intentionally absent
  // from navigation. The metadata stays explicit so re-enabling the tour is
  // a lifecycle change, not another layout reinterpretation.
  'rankings-tabs': { zone: 'tabs', ...bottomStart },
  'rankings-list': { zone: 'list', ...rightStart },
  'rankings-detail': { zone: 'detail', ...leftStart },
  'rankings-competitions': { zone: 'list', ...rightStart },
} as const satisfies Record<string, GuideTargetMetadata>;

export type GuideTargetId = keyof typeof guideTargets;

export function guideTargetMetadata(target: string): GuideTargetMetadata {
  const metadata = (guideTargets as Record<string, GuideTargetMetadata>)[target];
  if (metadata) return metadata;
  // Plugin-contributed field steps are dynamic by nature, but their visual
  // meaning is still fixed host layout: a control inside the action editor.
  if (/^action-field:[A-Za-z0-9_.-]{1,128}$/.test(target)) return { zone: 'editor', ...leftStart };
  throw new Error(`Unknown guide target metadata: ${target}`);
}
