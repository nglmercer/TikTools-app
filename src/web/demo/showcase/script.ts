/**
 * The showcase walks the real demo UI. Every operation is either a simulated
 * live event or a named guide action registered by the page that owns the
 * view, so the script never clicks arbitrary DOM.
 */
export type ShowcaseOp =
  /** Advance the simulated LIVE by `count` events, `gap` ms apart. */
  | { kind: 'events'; count: number; gap: number }
  /** Run a registered guide action; the cursor first travels to `cursor` (a data-guide target) and clicks. */
  | { kind: 'guide'; action: string; cursor?: string };

export interface ShowcaseStep {
  id: 'feed' | 'inspector' | 'automations' | 'overlays' | 'points';
  route: string;
  /** Guide scope that owns `kind: 'guide'` operations for this view. */
  scope: string;
  /** `data-guide` value that is spotlighted. */
  target: string;
  /** Spotlight before the operations (the target already exists) instead of after them. */
  focusFirst: boolean;
  /** Where the tooltip sits when the target is too large to place it outside. */
  tip: 'top' | 'bottom';
  ops: readonly ShowcaseOp[];
  /** How long the finished step stays on screen, in ms. */
  dwell: number;
}

export const showcaseSteps: readonly ShowcaseStep[] = [
  { id: 'feed', route: '/feed', scope: 'feed', target: 'feed-stage', focusFirst: true, tip: 'bottom', ops: [{ kind: 'events', count: 6, gap: 520 }], dwell: 1400 },
  { id: 'inspector', route: '/feed', scope: 'feed', target: 'feed-inspector-profile', focusFirst: false, tip: 'bottom',
    ops: [{ kind: 'guide', action: 'feed.showViewer', cursor: 'feed-inspector-tab-user' }], dwell: 2600 },
  { id: 'automations', route: '/automations', scope: 'automations', target: 'automation-editor', focusFirst: false, tip: 'bottom',
    ops: [{ kind: 'guide', action: 'automation.openEvent', cursor: 'automation-library' }], dwell: 2800 },
  { id: 'overlays', route: '/overlays', scope: 'overlays', target: 'overlay-preview', focusFirst: true, tip: 'top', ops: [], dwell: 3600 },
  { id: 'points', route: '/points', scope: 'points', target: 'points-leaderboard', focusFirst: true, tip: 'bottom', ops: [{ kind: 'events', count: 3, gap: 700 }], dwell: 2200 },
];

export type ShowcaseStepId = ShowcaseStep['id'];
