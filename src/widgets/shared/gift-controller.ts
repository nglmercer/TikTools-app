/**
 * Gift alert controller: aggregates TikTok streak updates per `groupId` into
 * one live alert, finalizes on `repeatEnd` (or a stale-combo timeout
 * fallback), and serves completed gifts from a bounded queue.
 */

import {
  DEFAULT_GIFT_COMBO_TIMEOUT_MS,
  DEFAULT_GIFT_MINIMUM_DIAMONDS,
  DEFAULT_GIFT_VISIBLE_MS,
  DEFAULT_QUEUE_CAPACITY,
  DEFAULT_RECENT_IDS_CAPACITY,
} from './config.ts';
import { AlertQueue, RecentEventIds, type QueueDropPolicy } from './alert-queue.ts';
import { realClock, type WidgetClock } from './clock.ts';
import { routeLiveEvent } from './event-router.ts';
import { displayNameFor, handleFor, type GiftAutomationData } from './event-types.ts';
import { createLogger } from './logger.ts';

const log = createLogger('gift');

export interface GiftAlertView {
  key: string;
  displayName: string;
  uniqueId: string;
  giftId: string;
  giftName: string;
  count: number;
  diamondsEach: number;
  totalDiamonds: number;
  giftIconUrl: string | null;
  /** True while the streak is still aggregating updates. */
  streaking: boolean;
}

export interface GiftControllerHandlers {
  onShow: (alert: GiftAlertView) => void;
  onUpdate: (alert: GiftAlertView) => void;
  onHide: (key: string) => void;
}

export interface GiftControllerOptions {
  visibleMs?: number;
  comboTimeoutMs?: number;
  minimumDiamonds?: number;
  queueCapacity?: number;
  queuePolicy?: QueueDropPolicy;
  recentCapacity?: number;
  clock?: WidgetClock;
}

export type GiftHandleResult =
  | 'shown'
  | 'updated'
  | 'aggregated'
  | 'queued'
  | 'ignored'
  | 'history'
  | 'duplicate'
  | 'filtered';

export function giftCounts(data: GiftAutomationData): {
  count: number;
  diamondsEach: number;
  totalDiamonds: number;
} {
  const count = Math.max(Math.floor(data.repeatCount), Math.floor(data.comboCount), 1);
  const diamondsEach = Math.max(Math.floor(data.diamondCount), 1);
  return { count, diamondsEach, totalDiamonds: diamondsEach * count };
}

interface ActiveGiftCombo extends GiftAlertView {
  timer: unknown;
  lastUpdatedAt: number;
}

export class GiftController {
  private readonly handlers: GiftControllerHandlers;
  private readonly visibleMs: number;
  private readonly comboTimeoutMs: number;
  private readonly minimumDiamonds: number;
  private readonly completed: AlertQueue<GiftAlertView>;
  private readonly recentIds: RecentEventIds;
  private readonly clock: WidgetClock;

  private readonly active = new Map<string, ActiveGiftCombo>();
  private current: GiftAlertView | null = null;
  private hideTimer: unknown = null;

  constructor(handlers: GiftControllerHandlers, options: GiftControllerOptions = {}) {
    this.handlers = handlers;
    this.visibleMs = options.visibleMs ?? DEFAULT_GIFT_VISIBLE_MS;
    this.comboTimeoutMs = options.comboTimeoutMs ?? DEFAULT_GIFT_COMBO_TIMEOUT_MS;
    this.minimumDiamonds = Math.max(0, Math.floor(options.minimumDiamonds ?? DEFAULT_GIFT_MINIMUM_DIAMONDS));
    this.completed = new AlertQueue<GiftAlertView>(
      options.queueCapacity ?? DEFAULT_QUEUE_CAPACITY,
      options.queuePolicy ?? 'drop-oldest',
    );
    this.recentIds = new RecentEventIds(options.recentCapacity ?? DEFAULT_RECENT_IDS_CAPACITY);
    this.clock = options.clock ?? realClock;
  }

  get currentAlert(): GiftAlertView | null {
    return this.current;
  }

  get activeCount(): number {
    return this.active.size;
  }

  get queueSize(): number {
    return this.completed.size;
  }

  handleEnvelope(value: unknown): GiftHandleResult {
    const routed = routeLiveEvent(value);
    if (!routed || routed.kind !== 'gift') return 'ignored';
    const { event } = routed;
    if (event.data.isHistory) return 'history';
    if (!this.recentIds.add(event.id)) return 'duplicate';

    const { count, diamondsEach, totalDiamonds } = giftCounts(event.data);
    const base = {
      displayName: displayNameFor(event.user),
      uniqueId: handleFor(event.user),
      giftId: event.data.giftId,
      giftName: event.data.giftName,
      count,
      diamondsEach,
      totalDiamonds,
      giftIconUrl: event.data.giftIconUrl ?? null,
    };

    if (!event.data.streakable) {
      if (totalDiamonds < this.minimumDiamonds) return 'filtered';
      this.completed.enqueue({ ...base, key: event.id, streaking: false });
      const wasIdle = this.current === null;
      this.pump();
      return wasIdle ? 'shown' : 'queued';
    }

    const key = event.data.groupId !== '' ? event.data.groupId : event.id;
    const previous = this.active.get(key);
    if (previous && previous.giftId !== event.data.giftId) {
      // Defensive: a group id must not change gifts mid-streak. Finalize the
      // stale combo instead of merging two different gifts.
      this.finalizeCombo(key);
    }
    if (previous) this.clock.clearTimeout(previous.timer);
    const combo: ActiveGiftCombo = {
      ...base,
      key,
      streaking: true,
      timer: this.clock.setTimeout(() => this.finalizeCombo(key), this.comboTimeoutMs),
      lastUpdatedAt: this.clock.now(),
    };
    this.active.set(key, combo);

    if (event.data.repeatEnd) {
      this.finalizeCombo(key);
      return this.current?.key === key ? 'shown' : 'queued';
    }
    if (this.current?.key === key) {
      this.emitUpdate(comboView(combo));
      return 'updated';
    }
    if (this.current === null) {
      this.pump();
      return 'shown';
    }
    return 'aggregated';
  }

  clear(): void {
    for (const combo of this.active.values()) this.clock.clearTimeout(combo.timer);
    this.active.clear();
    this.completed.clear();
    this.clearHideTimer();
    if (this.current) {
      const key = this.current.key;
      this.current = null;
      this.emitHide(key);
    }
  }

  dispose(): void {
    this.clear();
    this.recentIds.clear();
  }

  private finalizeCombo(key: string): void {
    const combo = this.active.get(key);
    if (!combo) return;
    this.active.delete(key);
    this.clock.clearTimeout(combo.timer);
    const finished: GiftAlertView = { ...comboView(combo), streaking: false };
    if (finished.totalDiamonds < this.minimumDiamonds) {
      if (this.current?.key === key) {
        this.clearHideTimer();
        this.current = null;
        this.emitHide(key);
        this.pump();
      }
      return;
    }
    if (this.current?.key === key) {
      // The live streak becomes a completed alert in place, then holds.
      this.current = finished;
      this.emitUpdate(finished);
      this.scheduleHide(key);
      return;
    }
    this.completed.enqueue(finished);
    this.pump();
  }

  private pump(): void {
    if (this.current !== null) return;
    const nextActive = mostRecentlyUpdated(this.active);
    if (nextActive) {
      this.show(comboView(nextActive), false);
      return;
    }
    const next = this.completed.dequeue();
    if (!next) return;
    this.show(next, true);
  }

  private show(alert: GiftAlertView, scheduleHide: boolean): void {
    this.current = alert;
    try {
      this.handlers.onShow(alert);
    } catch (error) {
      log.warn('onShow handler threw', error);
    }
    if (scheduleHide) this.scheduleHide(alert.key);
  }

  private scheduleHide(key: string): void {
    this.clearHideTimer();
    this.hideTimer = this.clock.setTimeout(() => {
      this.hideTimer = null;
      if (this.current?.key === key) {
        this.current = null;
        this.emitHide(key);
      }
      this.pump();
    }, this.visibleMs);
  }

  private clearHideTimer(): void {
    if (this.hideTimer !== null) {
      this.clock.clearTimeout(this.hideTimer);
      this.hideTimer = null;
    }
  }

  private emitUpdate(alert: GiftAlertView): void {
    if (this.current?.key === alert.key) this.current = alert;
    try {
      this.handlers.onUpdate(alert);
    } catch (error) {
      log.warn('onUpdate handler threw', error);
    }
  }

  private emitHide(key: string): void {
    try {
      this.handlers.onHide(key);
    } catch (error) {
      log.warn('onHide handler threw', error);
    }
  }
}

function comboView(combo: ActiveGiftCombo): GiftAlertView {
  return {
    key: combo.key,
    displayName: combo.displayName,
    uniqueId: combo.uniqueId,
    giftId: combo.giftId,
    giftName: combo.giftName,
    count: combo.count,
    diamondsEach: combo.diamondsEach,
    totalDiamonds: combo.totalDiamonds,
    giftIconUrl: combo.giftIconUrl,
    streaking: combo.streaking,
  };
}

function mostRecentlyUpdated(active: Map<string, ActiveGiftCombo>): ActiveGiftCombo | null {
  let newest: ActiveGiftCombo | null = null;
  for (const combo of active.values()) {
    if (!newest || combo.lastUpdatedAt >= newest.lastUpdatedAt) newest = combo;
  }
  return newest;
}
