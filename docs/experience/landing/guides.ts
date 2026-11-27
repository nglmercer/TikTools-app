import type { Topic } from '../types.ts';
import type { ShowcaseStepId } from '../../../src/web/demo/showcase/script.ts';

/** User-guide page behind each landing step or topic, relative to `/<locale>/user/`. */
export const userGuidePath: Record<ShowcaseStepId | Topic, string> = {
  feed: 'live/feed.html',
  inspector: 'live/feed.html',
  connection: 'first-live.html',
  automation: 'automations/first-rule.html',
  overlays: 'overlays/',
  points: 'points/rules.html',
};
