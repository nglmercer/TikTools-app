/**
 * Shared alert controller: validates routed events of one kind, drops
 * historical and duplicate deliveries, and serves one alert at a time from a
 * bounded queue. Rendering-agnostic; the Vue component only observes
 * show/hide. Follow, share and subscribe alerts differ only in the routed
 * kind, their default duration and their log scope.
 */

import { DEFAULT_QUEUE_CAPACITY, DEFAULT_RECENT_IDS_CAPACITY } from './config.ts';
import { AlertQueue, RecentEventIds, type QueueDropPolicy } from './alert-queue.ts';
import { realClock, type WidgetClock } from './clock.ts';
import { routeLiveEvent } from './event-router.ts';
import { displayNameFor, eventIdentity, handleFor } from './event-types.ts';
import { createLogger } from './logger.ts';

export type AlertKind = 'follow' | 'share' | 'subscribe';

export interface AlertBase {
  id: string;
  displayName: string;
  uniqueId: string;
  avatarUrl: string | null;
}

export interface AlertControllerHandlers {
  onShow: (alert: AlertBase) => void;
  onHide: (id: string) => void;
}

export interface AlertControllerOptions {
  visibleMs?: number;
  queueCapacity?: number;
  queuePolicy?: QueueDropPolicy;
  recentCapacity?: number;
  clock?: WidgetClock;
}

export interface AlertControllerConfig {
  kind: AlertKind;
  logScope: string;
  defaultVisibleMs: number;
}

export type AlertHandleResult = 'shown' | 'queued' | 'ignored' | 'history' | 'duplicate';

export class AlertController {
  private readonly kind: AlertKind;
  private readonly log: ReturnType<typeof createLogger>;
  private readonly handlers: AlertControllerHandlers;
  private readonly visibleMs: number;
  private readonly queue: AlertQueue<AlertBase>;
  private readonly recentIds: RecentEventIds;
  private readonly clock: WidgetClock;

  private current: AlertBase | null = null;
  private hideTimer: unknown = null;

  constructor(config: AlertControllerConfig, handlers: AlertControllerHandlers, options: AlertControllerOptions = {}) {
    this.kind = config.kind;
    this.log = createLogger(config.logScope);
    this.handlers = handlers;
    this.visibleMs = options.visibleMs ?? config.defaultVisibleMs;
    this.queue = new AlertQueue<AlertBase>(options.queueCapacity ?? DEFAULT_QUEUE_CAPACITY, options.queuePolicy ?? 'drop-oldest');
    this.recentIds = new RecentEventIds(options.recentCapacity ?? DEFAULT_RECENT_IDS_CAPACITY);
    this.clock = options.clock ?? realClock;
  }

  get currentAlert(): AlertBase | null {
    return this.current;
  }

  get queueSize(): number {
    return this.queue.size;
  }

  handleEnvelope(value: unknown): AlertHandleResult {
    const routed = routeLiveEvent(value);
    if (!routed || routed.kind !== this.kind) return 'ignored';
    const { event } = routed;
    if (event.data.isHistory) return 'history';
    if (!this.recentIds.add(eventIdentity(event))) return 'duplicate';
    const alert: AlertBase = {
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
      this.log.warn('onShow handler threw', error);
    }
    this.hideTimer = this.clock.setTimeout(() => {
      this.hideTimer = null;
      if (this.current) {
        const id = this.current.id;
        this.current = null;
        try {
          this.handlers.onHide(id);
        } catch (error) {
          this.log.warn('onHide handler threw', error);
        }
      }
      this.pump();
    }, this.visibleMs);
  }
}
