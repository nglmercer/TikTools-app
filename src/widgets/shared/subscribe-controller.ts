/**
 * Subscribe alert controller: validates routed subscribe events (member
 * action 3), drops historical and duplicate deliveries, and serves one
 * alert at a time from a bounded queue. Rendering-agnostic; the Vue
 * component only observes show/hide.
 */

import { AlertController, type AlertBase, type AlertControllerHandlers, type AlertControllerOptions, type AlertHandleResult } from './alert-controller.ts';
import { DEFAULT_SUBSCRIBE_VISIBLE_MS } from './config.ts';

export type SubscribeAlert = AlertBase;
export type SubscribeControllerHandlers = AlertControllerHandlers;
export type SubscribeControllerOptions = AlertControllerOptions;
export type SubscribeHandleResult = AlertHandleResult;

export class SubscribeController extends AlertController {
  constructor(handlers: SubscribeControllerHandlers, options: SubscribeControllerOptions = {}) {
    super({ kind: 'subscribe', logScope: 'subscribe', defaultVisibleMs: DEFAULT_SUBSCRIBE_VISIBLE_MS }, handlers, options);
  }
}
