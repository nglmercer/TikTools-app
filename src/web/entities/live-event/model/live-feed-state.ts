import { createFeedViewport } from './feed-viewport.ts';
import { createFeedRecovery } from './feed-recovery.ts';
import { eventCategory, MAX_PER_CATEGORY, retainPerCategory } from '../lib/event-filter.ts';
import type { FeedHistoryAdapter } from '../lib/feed-history.ts';
import { liveMessageIdentity, LIVE_IDENTITY_CAPACITY, LIVE_IDENTITY_TTL_MS } from '../../../../shared/live-identity.ts';
import { ref, shallowRef } from 'vue';

import type { GiftCatalogEntry, LikeLeaderPayload, UiEvent } from '../../../../shared/messages.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import type { DisplayEvent, TopViewerPayload } from '@/types.ts';
import { createFeedRankings } from '@/entities/ranking/model/feed-rankings.ts';

export interface LiveFeedCallbacks {
  systemAuthor: () => string;
  discontinuityText: () => string;
  /** Optional: when supplied, the last events survive a restart. */
  history?: FeedHistoryAdapter;
}

export interface GiftCatalogResult {
  gifts: GiftCatalogEntry[];
}

export type { FeedBackfillResult } from './feed-recovery.ts';

export function createLiveFeedState(control: ControlClient, callbacks: LiveFeedCallbacks) {
  const history = callbacks.history;
  const restored = history?.load() ?? null;
  let historyScope = restored?.scope ?? '';
  const restoredEvents = restored?.events ?? [];
  const seenMsgIds = new Map<string, number>();
  const seenCursors = new Set<number>(restoredEvents
    .map((event) => event.feedCursor)
    .filter((cursor): cursor is number => typeof cursor === 'number' && Number.isSafeInteger(cursor)));
  let duplicates = 0;
  let needsCursorSort = false;
  let sessionScope = '';
  let sessionRoom = '';
  let sessionCreator = '';
  let rankingCreator = '';
  let rankingRoom = '';
  let disposed = false;
  let presentationActive = true;
  const events = shallowRef<DisplayEvent[]>(restoredEvents);
  const countCategories = (list: readonly DisplayEvent[]): Map<ReturnType<typeof eventCategory>, number> => {
    const counts = new Map<ReturnType<typeof eventCategory>, number>();
    for (const event of list) {
      const category = eventCategory(event);
      counts.set(category, (counts.get(category) ?? 0) + 1);
    }
    return counts;
  };
  let categoryCounts = countCategories(restoredEvents);
  const viewport = createFeedViewport(events, () => !disposed && presentationActive);
  const {
    filter, imagesEnabled, renderLimit, searchQuery, autoScroll, unreadCount, streamContainerRef,
    scrollToBottom, handleToggleAutoScroll, setFilter, setSearchQuery, setStreamContainerRef,
  } = viewport;
  const topViewers = shallowRef<TopViewerPayload[]>([]);
  const likeLeaders = shallowRef<LikeLeaderPayload[]>([]);
  const liveViewers = ref(0);
  const giftCatalog = shallowRef<GiftCatalogEntry[]>([]);
  const error = ref<string | null>(null);
  let nextEventId = restoredEvents.reduce((max, event) => Math.max(max, event.id + 1), 0);
  let refreshRevision = 0;
  let historyTimer: ReturnType<typeof setTimeout> | null = null;

  function persistHistory(): void {
    if (historyTimer !== null) { clearTimeout(historyTimer); historyTimer = null; }
    history?.save({ scope: sessionScope || historyScope, savedAt: Date.now(), events: events.value });
  }
  function scheduleHistorySave(): void {
    if (!history) return;
    if (historyTimer !== null) clearTimeout(historyTimer);
    historyTimer = setTimeout(() => { historyTimer = null; persistHistory(); }, 800);
  }
  const unsubscribers: Array<() => void> = [];
  const feedRankings = createFeedRankings(control);

  let received = 0;
  let retentionEvicted = 0;
  let lastBusDropped: number | null = null;
  let lastDeliverySequence: number | null = null;
  let lastFeedCursor: number | null = seenCursors.size > 0 ? Math.max(...seenCursors) : null;

  // Render every accepted event as it arrives: no staging, frame batching or
  // visual coalescing. Retention is bounded per category, so chat survives a
  // flood of likes/joins and each filter keeps its own recent history.
  const appendEvent = (input: Omit<DisplayEvent, 'receivedAt'>): void => {
    if (disposed) return;
    const list = events.value;
    const appendedEvent: DisplayEvent = { ...input, receivedAt: Date.now() };
    list.push(appendedEvent);
    if (needsCursorSort) {
      // Sort only journal-backed slots: system notices retain their positions.
      const ordered = list.filter((row) => row.feedCursor !== undefined).sort((a, b) => a.feedCursor! - b.feedCursor!);
      let position = 0;
      for (let i = 0; i < list.length; i += 1) if (list[i]!.feedCursor !== undefined) list[i] = ordered[position++]!;
      needsCursorSort = false;
    }
    const category = eventCategory(appendedEvent);
    const count = categoryCounts.get(category) ?? 0;
    let next: DisplayEvent[];
    if (count < MAX_PER_CATEGORY) {
      categoryCounts.set(category, count + 1);
      next = [...list];
    } else {
      // The append can overflow only its own category by one. Evicting that
      // category's oldest row avoids the previous full-buffer retention scan
      // on every live packet; the fallback preserves the invariant if a
      // restored or externally mutated buffer ever diverges.
      const evictIndex = list.findIndex((row) => eventCategory(row) === category);
      if (evictIndex >= 0) {
        list.splice(evictIndex, 1);
        retentionEvicted += 1;
        next = [...list];
      } else {
        const kept = retainPerCategory(list);
        retentionEvicted += list.length - kept.length;
        next = [...kept];
        categoryCounts = countCategories(next);
      }
    }
    events.value = next;
    scheduleHistorySave();
    if (!autoScroll.value) unreadCount.value += 1;
    if (autoScroll.value) scrollToBottom();
  };

  const resetEvents = (): void => {
    if (historyTimer !== null) { clearTimeout(historyTimer); historyTimer = null; }
    history?.clear();
    historyScope = sessionScope;
    nextEventId = 0;
    categoryCounts.clear();
    events.value = [];
    unreadCount.value = 0;
    topViewers.value = [];
    likeLeaders.value = [];
    liveViewers.value = 0;
  };

  const dispose = (): void => {
    if (historyTimer !== null) persistHistory();
    disposed = true;
    feedRankings.dispose();
    viewport.dispose();
    refreshRevision += 1;
    seenMsgIds.clear();
    seenCursors.clear();
    while (unsubscribers.length > 0) unsubscribers.pop()?.();
  };

  const pushFeedEvent = (event: UiEvent, cursor?: number): boolean => {
    if (disposed) return false;
    received += 1;
    if ((event.roomId && sessionRoom && event.roomId !== sessionRoom) || (event.creatorId && sessionCreator && event.creatorId !== sessionCreator)) return false;
    if (cursor !== undefined) {
      if (seenCursors.has(cursor)) { duplicates += 1; return false; }
      seenCursors.add(cursor);
      if (seenCursors.size > LIVE_IDENTITY_CAPACITY) seenCursors.delete(seenCursors.values().next().value!);
      if (lastFeedCursor !== null && cursor < lastFeedCursor) needsCursorSort = true;
      lastFeedCursor = Math.max(lastFeedCursor ?? 0, cursor);
    }
    // A journal cursor identifies an accepted backend delivery. Never let a
    // second, expiring message cache contradict that authoritative decision.
    const identity = cursor === undefined ? liveMessageIdentity(event.sourceType ?? event.kind, event.msgId, event.roomId ?? sessionRoom, event.creatorId ?? sessionCreator) : null;
    if (identity !== null) {
      const now = Date.now();
      const seen = seenMsgIds.get(identity);
      if (seen !== undefined && now - seen < LIVE_IDENTITY_TTL_MS) { duplicates += 1; return false; }
      seenMsgIds.delete(identity);
      seenMsgIds.set(identity, now);
      if (seenMsgIds.size > LIVE_IDENTITY_CAPACITY) seenMsgIds.delete(seenMsgIds.keys().next().value!);
    }
    appendEvent({ ...event, feedCursor: cursor, id: nextEventId++ });
    return true;
  };

  const insertDiscontinuity = (): void => {
    appendEvent({
      kind: 'member',
      author: callbacks.systemAuthor(),
      text: callbacks.discontinuityText(),
      id: nextEventId++,
    });
  };

  const recovery = createFeedRecovery(control, {
    active: () => !disposed && presentationActive,
    cursor: () => lastFeedCursor,
    resetCursor: () => { seenCursors.clear(); lastFeedCursor = null; },
    push: pushFeedEvent,
    discontinuity: insertDiscontinuity,
  });

  if (typeof control.onGap === 'function') {
    unsubscribers.push(control.onGap(() => {
      if (!disposed && presentationActive) void recovery.recover();
    }));
  }

  const setPresentationActive = (active: boolean): void => {
    if (disposed || active === presentationActive) return;
    presentationActive = active;
    recovery.invalidate();
    if (!active) viewport.pause();
    else void recovery.recover();
  };

  // Host projection stats no longer drive a "skipped updates" counter. They
  // only signal that the host dropped domain events, which the journal-backed
  // recovery loop can backfill.
  unsubscribers.push(control.onTopic<{ feedSuspended: number; feedBusDropped?: number; feedForwarded?: number }>('ui.projection.stats', (data) => {
    if (!Number.isFinite(data.feedSuspended)) return;
    if (typeof data.feedForwarded === 'number' && Number.isFinite(data.feedForwarded)) {
      if (lastDeliverySequence !== null && data.feedForwarded > lastDeliverySequence && presentationActive) void recovery.recover();
      lastDeliverySequence = data.feedForwarded;
    }
    if (typeof data.feedBusDropped === 'number' && Number.isFinite(data.feedBusDropped)) {
      const previous = lastBusDropped ?? data.feedBusDropped;
      if (data.feedBusDropped > previous && presentationActive) void recovery.recover();
      lastBusDropped = data.feedBusDropped;
    }
  }));

  unsubscribers.push(control.onTopic<{ uniqueId?: string; roomId?: string }>('live.connected', (data) => {
    const scope = JSON.stringify([data.uniqueId ?? '', data.roomId ?? '']);
    if (scope !== sessionScope) {
      if (historyScope && historyScope !== scope) {
        // A restart must not show another creator's saved activity.
        if (historyTimer !== null) { clearTimeout(historyTimer); historyTimer = null; }
        history?.clear();
        events.value = [];
        nextEventId = 0;
        unreadCount.value = 0;
      }
      historyScope = scope;
      seenMsgIds.clear();
      seenCursors.clear();
      // Lossy ranking snapshots can be delivered before reliable
      // live.connected. Preserve a snapshot from the SAME room in that case.
      if (rankingCreator !== (data.uniqueId ?? '') ||
          rankingRoom !== (data.roomId ?? '')) likeLeaders.value = [];
      sessionScope = scope;
      sessionRoom = data.roomId ?? '';
      sessionCreator = data.uniqueId ?? '';
      recovery.invalidate(true);
    }
    // Idempotent: also reloads ranking snapshots after a same-room reconnect.
    void feedRankings.connect(data.uniqueId ?? '', data.roomId ?? '');
  }));
  unsubscribers.push(control.onTopic<{ event: UiEvent; sequence?: number; cursor?: number }>('live.ui-event', (data) => {
    const deliveredCursor = data.cursor ?? data.event.feedCursor;
    const cursor = typeof deliveredCursor === 'number' && Number.isSafeInteger(deliveredCursor) ? deliveredCursor : undefined;
    const previousCursor = lastFeedCursor;
    const gap = cursor !== undefined && previousCursor !== null && cursor > previousCursor + 1;
    if (gap) recovery.noteGap(previousCursor);
    pushFeedEvent(data.event, cursor);
    if (gap && presentationActive) void recovery.recover();
  }));
  unsubscribers.push(control.onTopic<{
    creatorId: string;
    roomId: string;
    leaders: LikeLeaderPayload[];
  }>('live.likes-ranking', (data) => {
    if ((sessionCreator && data.creatorId !== sessionCreator) ||
        (sessionRoom && data.roomId !== sessionRoom)) return;
    if (!Array.isArray(data.leaders)) return;
    // The core owns the ranking, and the UI never re-aggregates folded rows.
    // Bound and validate snapshots from all transports before display.
    rankingCreator = data.creatorId;
    rankingRoom = data.roomId;
    likeLeaders.value = data.leaders
      .filter((leader) => leader && typeof leader.uniqueId === 'string' &&
        typeof leader.score === 'number' && Number.isSafeInteger(leader.score) && leader.score > 0)
      .slice(0, 10);
  }));
  unsubscribers.push(control.onTopic<{ viewers: number; totalUsers: number; topViewers: TopViewerPayload[] }>(
    'room.stats',
    (data) => {
      topViewers.value = data.topViewers;
      liveViewers.value = data.viewers;
    },
  ));
  unsubscribers.push(control.onTopic<{ gifts: GiftCatalogEntry[] }>('gifts.catalog', (data) => {
    refreshRevision += 1;
    giftCatalog.value = data.gifts;
  }));
  unsubscribers.push(control.onTopic<{ phase: string; message: string }>('live.error', (data) => {
    appendEvent({
      kind: 'member' as const,
      author: callbacks.systemAuthor(),
      text: data.message,
      id: nextEventId++,
    });
  }));

  const refresh = async (): Promise<void> => {
    const revision = ++refreshRevision;
    if (lastFeedCursor === null) void recovery.recover();
    try {
      const result = await control.call<GiftCatalogResult>('gifts.list', {});
      if (!disposed && revision === refreshRevision) giftCatalog.value = result.gifts;
    } catch (failure) {
      if (!disposed && revision === refreshRevision) error.value = errorMessage(failure);
    }
  };

  return {
    diagnostics: () => ({ visible: events.value.length, presentationActive, received, duplicates, ...recovery.diagnostics(), retentionEvicted, identities: seenMsgIds.size, cursor: lastFeedCursor, timers: Number(historyTimer !== null) + viewport.diagnostics().timers, frames: viewport.diagnostics().frames }),
    events,
    imagesEnabled,
    renderLimit,
    filter,
    searchQuery,
    topViewers,
    likeLeaders,
    topRanking: feedRankings.topRanking,
    likesRanking: feedRankings.likesRanking,
    rankingsAvailable: feedRankings.rankingsAvailable,
    liveViewers,
    giftCatalog,
    autoScroll,
    unreadCount,
    error,
    streamContainerRef,
    resetEvents,
    setPresentationActive,
    scrollToBottom,
    handleToggleAutoScroll,
    setFilter,
    setSearchQuery,
    setStreamContainerRef,
    refresh,
    dispose,
  };
}
