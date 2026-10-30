/**
 * Top-likes board controller: aggregates TikTok like events per sender into
 * a bounded, ranked leaderboard. Rendering-agnostic; the Vue component only
 * observes the snapshot the handler receives.
 *
 * Totals prefer the backend `aggregation.user.total` when present (it is the
 * authoritative cumulative count) and otherwise accumulate `count`
 * increments. Events without a usable sender or count are ignored.
 */

import {
  DEFAULT_LIKES_BOARD_LIMIT,
  DEFAULT_LIKES_TRACKED_CAPACITY,
  DEFAULT_RECENT_IDS_CAPACITY,
} from './config.ts';
import { RecentEventIds } from './alert-queue.ts';
import { routeLiveEvent } from './event-router.ts';
import { displayNameFor, eventIdentity, handleFor } from './event-types.ts';
import { createLogger } from './logger.ts';

const log = createLogger('likes');

export interface LikeBoardEntry {
  userId?: string;
  uniqueId: string;
  displayName: string;
  avatarUrl: string | null;
  total: number;
}

export interface LikesControllerHandlers {
  onBoard: (entries: LikeBoardEntry[]) => void;
}

export interface LikesControllerOptions {
  maxEntries?: number;
  trackedCapacity?: number;
  recentCapacity?: number;
}

export type LikesHandleResult = 'updated' | 'ignored' | 'history' | 'duplicate';

interface TrackedLiker {
  key: string;
  displayName: string;
  uniqueId: string;
  avatarUrl: string | null;
  total: number;
}

export class LikesController {
  private readonly handlers: LikesControllerHandlers;
  private readonly maxEntries: number;
  private readonly trackedCapacity: number;
  private readonly recentIds: RecentEventIds;

  private readonly tracked = new Map<string, TrackedLiker>();

  constructor(handlers: LikesControllerHandlers, options: LikesControllerOptions = {}) {
    this.handlers = handlers;
    this.maxEntries = Math.max(1, Math.floor(options.maxEntries ?? DEFAULT_LIKES_BOARD_LIMIT));
    this.trackedCapacity = Math.max(1, Math.floor(options.trackedCapacity ?? DEFAULT_LIKES_TRACKED_CAPACITY));
    this.recentIds = new RecentEventIds(options.recentCapacity ?? DEFAULT_RECENT_IDS_CAPACITY);
  }

  get board(): LikeBoardEntry[] {
    return this.ranked();
  }

  get trackedCount(): number {
    return this.tracked.size;
  }

  clear(): void {
    this.tracked.clear();
    this.handlers.onBoard([]);
  }

  dispose(): void {
    this.tracked.clear();
    this.recentIds.clear();
  }

  handleEnvelope(value: unknown): LikesHandleResult {
    const routed = routeLiveEvent(value);
    if (!routed || routed.kind !== 'like') return 'ignored';
    const { event } = routed;
    if (event.data.isHistory) return 'history';
    if (!this.recentIds.add(eventIdentity(event))) return 'duplicate';

    const key = event.user?.userId || event.user?.uniqueId;
    if (!key) {
      log.debug('ignoring like without a usable sender');
      return 'ignored';
    }
    const count = Number.isFinite(event.data.count) ? Math.max(0, Math.floor(event.data.count)) : 0;
    const backendTotal = event.data.aggregation?.user?.total;
    const previous = this.tracked.get(key);
    const total = typeof backendTotal === 'number' && Number.isFinite(backendTotal)
      ? Math.max(previous?.total ?? 0, Math.floor(backendTotal))
      : (previous?.total ?? 0) + count;
    if (total <= 0 && !previous) return 'ignored';
    const avatarUrl = typeof event.user?.avatarUrl === 'string' && event.user.avatarUrl
      ? event.user.avatarUrl
      : (previous?.avatarUrl ?? null);
    this.tracked.set(key, {
      key,
      displayName: displayNameFor(event.user),
      uniqueId: handleFor(event.user),
      avatarUrl,
      total,
    });
    if (this.tracked.size > this.trackedCapacity) {
      const ranked = this.rankedAll();
      for (let index = this.trackedCapacity; index < ranked.length; index += 1) {
        this.tracked.delete(ranked[index]!.key);
      }
    }
    this.handlers.onBoard(this.ranked());
    return 'updated';
  }

  private rankedAll(): TrackedLiker[] {
    return [...this.tracked.values()].sort((a, b) => b.total - a.total || a.displayName.localeCompare(b.displayName));
  }

  private ranked(): LikeBoardEntry[] {
    return this.rankedAll().slice(0, this.maxEntries).map(({ uniqueId, displayName, avatarUrl, total }) => ({
      uniqueId,
      displayName,
      avatarUrl,
      total,
    }));
  }
}
