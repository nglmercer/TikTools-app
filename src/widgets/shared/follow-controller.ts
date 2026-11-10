/**
 * Follow alert controller: validates routed follow events, drops historical
 * and duplicate deliveries, and serves one alert at a time from a bounded
 * queue. Rendering-agnostic; the Vue component only observes show/hide.
 */

import { AlertController, type AlertBase, type AlertControllerHandlers, type AlertControllerOptions, type AlertHandleResult } from './alert-controller.ts';
import { DEFAULT_FOLLOW_VISIBLE_MS } from './config.ts';

export type FollowAlert = AlertBase;
export type FollowControllerHandlers = AlertControllerHandlers;
export type FollowControllerOptions = AlertControllerOptions;
export type FollowHandleResult = AlertHandleResult;

export class FollowController extends AlertController {
  constructor(handlers: FollowControllerHandlers, options: FollowControllerOptions = {}) {
    super({ kind: 'follow', logScope: 'follow', defaultVisibleMs: DEFAULT_FOLLOW_VISIBLE_MS }, handlers, options);
  }
}
