/**
 * Names a page registers with `registerGuideActions` and that tours or the demo showcase call.
 * One definition keeps the registering page, tour layouts and the showcase script from drifting.
 */
export const guideScope = {
  feed: 'feed',
  automations: 'automations',
  overlays: 'overlays',
  points: 'points',
  plugins: 'plugins',
  analytics: 'analytics',
} as const;

export const guideAction = {
  feed: {
    openInspector: 'feed.openInspector',
    showViewers: 'feed.showViewers',
    showEvent: 'feed.showEvent',
    showViewer: 'feed.showViewer',
  },
  automation: {
    selectEvents: 'automation.selectEvents',
    selectActions: 'automation.selectActions',
    selectVariables: 'automation.selectVariables',
    openEvent: 'automation.openEvent',
    openAction: 'automation.openAction',
    revealActionField: 'automation.revealActionField',
  },
} as const;
