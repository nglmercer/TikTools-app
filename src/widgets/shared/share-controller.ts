/**
 * Share alert controller: validates routed share events, drops historical
 * and duplicate deliveries, and serves one alert at a time from a bounded
 * queue. Rendering-agnostic; the Vue component only observes show/hide.
 */

import {
  DEFAULT_QUEUE_CAPACITY,
  DEFAULT_RECENT_IDS_CAPACITY,
  DEFAULT_SHARE_VISIBLE_MS,
} from './config.ts';
import { AlertQueue, RecentEventIds, type QueueDropPolicy } from './alert-queue.ts';
import { realClock, type WidgetClock } from './clock.ts';
import { routeLiveEvent } from './event-router.ts';
import { displayNameFor, handleFor } from './event-types.ts';
import { createLogger } from './logger.ts';

const log = createLogger('share');

export interface ShareAlert {
  id: string;
  displayName: string;
  uniqueId: string;
  avatarUrl: string | null;
}

export interface ShareControllerHandlers {
  onShow: (alert: ShareAlert) => void;
  onHide: (id: string) => void;
}

export interface ShareControllerOptions {
  visibleMs?: number;
  queueCapacity?: number;
  queuePolicy?: QueueDropPolicy;
  recentCapacity?: number;
  clock?: WidgetClock;
}

export type ShareHandleResult = 'shown' | 'queued' | 'ignored' | 'history' | 'duplicate';

export class ShareController {
  private readonly handlers: ShareControllerHandlers;
  private readonly visibleMs: number;
  private readonly queue: AlertQueue<ShareAlert>;
  private readonly recentIds: RecentEventIds;
  private readonly clock: WidgetClock;

  private current: ShareAlert | null = null;
  private hideTimer: unknown = null;

  constructor(handlers: ShareControllerHandlers, options: ShareControllerOptions = {}) {
    this.handlers = handlers;
    this.visibleMs = options.visibleMs ?? DEFAULT_SHARE_VISIBLE_MS;
    this.queue = new AlertQueue<ShareAlert>(options.queueCapacity ?? DEFAULT_QUEUE_CAPACITY, options.queuePolicy ?? 'drop-oldest');
    this.recentIds = new RecentEventIds(options.recentCapacity ?? DEFAULT_RECENT_IDS_CAPACITY);
    this.clock = options.clock ?? realClock;
  }

  get currentAlert(): ShareAlert | null {
    return this.current;
  }

  get queueSize(): number {
    return this.queue.size;
  }

  handleEnvelope(value: unknown): ShareHandleResult {
    const routed = routeLiveEvent(value);
    if (!routed || routed.kind !== 'share') return 'ignored';
    const { event } = routed;
    if (event.data.isHistory) return 'history';
    if (!this.recentIds.add(event.id)) return 'duplicate';
    const alert: ShareAlert = {
      id: event.id,
      displayName: displayNameFor(event.user),
      uniqueId: handleFor(event.user),
      avatarUrl: event.user?.avatarUrl ?? null,
    };
    const wasIdle = this.current === null && this.queue.size === 0;
    this.queue.enqueue(alert);
    this.pump();
    return wasIdle ? 'shown' : 'queued';
  }

  clear(): void {
    this.queue.clear();
    if (this.hideTimer !== null) {
      this.clock.clearTimeout(this.hideTimer);
      this.hideTimer = null;
    }
    if (this.current) {
      const id = this.current.id;
      this.current = null;
      this.handlers.onHide(id);
    }
  }

  dispose(): void {
    this.clear();
    this.recentIds.clear();
  }

  private pump(): void {
    if (this.current !== null) return;
    const next = this.queue.dequeue();
    if (!next) return;
    this.current = next;
    try {
      this.handlers.onShow(next);
    } catch (error) {
      log.warn('onShow handler threw', error);
    }
    this.hideTimer = this.clock.setTimeout(() => {
      this.hideTimer = null;
      if (this.current) {
        const id = this.current.id;
        this.current = null;
        try {
          this.handlers.onHide(id);
        } catch (error) {
          log.warn('onHide handler threw', error);
        }
      }
      this.pump();
    }, this.visibleMs);
  }
}
