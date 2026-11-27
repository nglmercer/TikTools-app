import { demoRoute } from '../scenarios.ts';
import { guideAction, guideScope } from '@/features/guided-help/model/guide-vocabulary.ts';

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
  id: 'feed' | 'inspector' | 'automation' | 'overlays' | 'points';
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
  { id: 'feed', route: demoRoute.feed, scope: guideScope.feed, target: 'feed-stage', focusFirst: true, tip: 'bottom', ops: [{ kind: 'events', count: 6, gap: 520 }], dwell: 1400 },
  { id: 'inspector', route: demoRoute.feed, scope: guideScope.feed, target: 'feed-inspector-profile', focusFirst: false, tip: 'bottom',
    ops: [{ kind: 'guide', action: guideAction.feed.showViewer, cursor: 'feed-inspector-tab-user' }], dwell: 2600 },
  { id: 'automation', route: demoRoute.automations, scope: guideScope.automations, target: 'automation-editor', focusFirst: false, tip: 'bottom',
    ops: [{ kind: 'guide', action: guideAction.automation.openEvent, cursor: 'automation-library' }], dwell: 2800 },
  { id: 'overlays', route: demoRoute.overlays, scope: guideScope.overlays, target: 'overlay-preview', focusFirst: true, tip: 'top', ops: [], dwell: 3600 },
  { id: 'points', route: demoRoute.points, scope: guideScope.points, target: 'points-leaderboard', focusFirst: true, tip: 'bottom', ops: [{ kind: 'events', count: 3, gap: 700 }], dwell: 2200 },
];

export type ShowcaseStepId = ShowcaseStep['id'];
