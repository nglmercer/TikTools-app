/**
 * Share alert controller: validates routed share events, drops historical
 * and duplicate deliveries, and serves one alert at a time from a bounded
 * queue. Rendering-agnostic; the Vue component only observes show/hide.
 */

import { AlertController, type AlertBase, type AlertControllerHandlers, type AlertControllerOptions, type AlertHandleResult } from './alert-controller.ts';
import { DEFAULT_SHARE_VISIBLE_MS } from './config.ts';

export type ShareAlert = AlertBase;
export type ShareControllerHandlers = AlertControllerHandlers;
export type ShareControllerOptions = AlertControllerOptions;
export type ShareHandleResult = AlertHandleResult;

export class ShareController extends AlertController {
  constructor(handlers: ShareControllerHandlers, options: ShareControllerOptions = {}) {
    super({ kind: 'share', logScope: 'share', defaultVisibleMs: DEFAULT_SHARE_VISIBLE_MS }, handlers, options);
  }
}
