import { createFeedViewport } from './feed-viewport.ts';
import { scheduleFrame, cancelFrame } from './feed-frame.ts';
import { createFeedCoalescer } from './feed-coalescing.ts';
import { createFeedRecovery } from './feed-recovery.ts';
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
}

export interface GiftCatalogResult {
  gifts: GiftCatalogEntry[];
}

export type { FeedBackfillResult } from './feed-recovery.ts';

export function createLiveFeedState(control: ControlClient, callbacks: LiveFeedCallbacks) {
  const seenMsgIds = new Map<string, number>();
  const seenCursors = new Set<number>();
  let duplicates = 0;
  let needsCursorSort = false;
  let sessionScope = '';
  let sessionRoom = '';
  let sessionCreator = '';
  let rankingCreator = '';
  let rankingRoom = '';
  let disposed = false;
  let presentationActive = true;
  const events = shallowRef<DisplayEvent[]>([]);
  const viewport = createFeedViewport(events, () => !disposed && presentationActive);
  const {
    filter, imagesEnabled, renderLimit, searchQuery, autoScroll, unreadCount, streamContainerRef,
    scrollToBottom, handleToggleAutoScroll, setFilter, setSearchQuery, setStreamContainerRef,
  } = viewport;
  const topViewers = shallowRef<TopViewerPayload[]>([]);
  const likeLeaders = shallowRef<LikeLeaderPayload[]>([]);
  const liveViewers = ref(0);
  const giftCatalog = shallowRef<GiftCatalogEntry[]>([]);
  const droppedUiEvents = ref(0);
  const error = ref<string | null>(null);
  let nextEventId = 0;
  let refreshRevision = 0;
  const unsubscribers: Array<() => void> = [];
  const feedRankings = createFeedRankings(control);

  const MAX_VISIBLE_FEED = 300;
  const MAX_STAGED_EVENTS = 900;
  let staged: DisplayEvent[] = [];
  let flushQueued = false;
  let flushFrame = 0;
  let flushTimer: ReturnType<typeof setTimeout> | null = null;
  let commits = 0;
  let received = 0;
  let presentationCoalesced = 0;
  let retentionEvicted = 0;
  let lastHostSuspended: number | null = null;
  let lastHostBusDropped: number | null = null;
  let lastDeliverySequence: number | null = null;
  let lastFeedCursor: number | null = null;

  const stageEvent = (event: DisplayEvent): void => {
    if (disposed) return;
    if (!presentationActive) { droppedUiEvents.value += 1; return; }
    staged.push(event);
    if (staged.length > MAX_STAGED_EVENTS) {
      const shed = staged.length - MAX_STAGED_EVENTS;
      staged.splice(0, shed);
      presentationCoalesced += shed;
      droppedUiEvents.value += shed;
    }
    if (!flushQueued && !disposed) {
      flushQueued = true;
      if (typeof requestAnimationFrame === 'function') flushTimer = setTimeout(flushStaged, 100);
      flushFrame = scheduleFrame(flushStaged);
    }
  };

  const evictToVisibleLimit = (combined: DisplayEvent[]): void => {
    // Chat floods must never push gifts out of the Regalos tab: evict the
    // oldest non-gift rows first. Gifts are only evicted once no other kind
    // remains, so the buffer stays bounded at MAX_VISIBLE_FEED either way.
    while (combined.length > MAX_VISIBLE_FEED) {
      const victim = combined.findIndex((event) => event.kind !== 'gift');
      combined.splice(victim === -1 ? 0 : victim, 1);
      retentionEvicted += 1;
    }
  };

  const flushStaged = (): void => {
    if (flushTimer !== null) clearTimeout(flushTimer);
    flushTimer = null;
    cancelFrame(flushFrame);
    flushQueued = false;
    flushFrame = 0;
    if (disposed || !presentationActive || staged.length === 0) return;
    const batch = staged;
    staged = [];
    const now = Date.now();
    for (const stagedEvent of batch) stagedEvent.receivedAt = now;
    commits += 1;
    const combined = [...events.value, ...batch];
    if (needsCursorSort) {
      // Sort only journal-backed slots: system notices retain their positions.
      const ordered = combined.filter((event) => event.feedCursor !== undefined).sort((a, b) => a.feedCursor! - b.feedCursor!);
      let position = 0;
      for (let i = 0; i < combined.length; i += 1) if (combined[i]!.feedCursor !== undefined) combined[i] = ordered[position++]!;
      needsCursorSort = false;
    }
    evictToVisibleLimit(combined);
    events.value = combined;
    if (!autoScroll.value) unreadCount.value += batch.length;
    if (autoScroll.value) scrollToBottom();
  };

  const resetEvents = (): void => {
    nextEventId = 0;
    staged = [];
    events.value = [];
    unreadCount.value = 0;
    droppedUiEvents.value = 0;
    topViewers.value = [];
    likeLeaders.value = [];
    liveViewers.value = 0;
  };

  const dispose = (): void => {
    disposed = true;
    feedRankings.dispose();
    viewport.dispose();
    cancelFrame(flushFrame);
    if (flushTimer !== null) clearTimeout(flushTimer);
    flushTimer = null;
    refreshRevision += 1;
    seenMsgIds.clear();
    seenCursors.clear();
    flushFrame = 0;
    flushQueued = false;
    staged = [];
    while (unsubscribers.length > 0) unsubscribers.pop()?.();
  };

  const coalescer = createFeedCoalescer({
    staged: () => staged,
    events,
    session: () => ({ room: sessionRoom, creator: sessionCreator }),
    stage: stageEvent,
  });

  const pushFeedEvent = (event: UiEvent, cursor?: number): boolean => {
    if (disposed) return false;
    received += 1;
    if (!presentationActive) {
      droppedUiEvents.value += 1;
      if (cursor === undefined) recovery.noteUnjournaledDrop();
      return false;
    }
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
    // Streakable combos arrive as N progress packets plus a final
    // `repeatEnd` packet sharing one groupId. Fold them into a single live
    // row that updates in place instead of flooding Regalos with duplicates.
    if (coalescer.tryMergeGiftStreak(event) || coalescer.tryMergeLikeBurst(event, cursor)) {
      presentationCoalesced += 1;
      return true;
    }
    stageEvent({ ...event, feedCursor: cursor, id: nextEventId++, receivedAt: Date.now() });
    return true;
  };

  const insertDiscontinuity = (): void => {
    stageEvent({
      kind: 'member',
      author: callbacks.systemAuthor(),
      text: callbacks.discontinuityText(),
      id: nextEventId++,
      receivedAt: Date.now(),
    });
  };

  const recovery = createFeedRecovery(control, {
    active: () => !disposed && presentationActive,
    cursor: () => lastFeedCursor,
    resetCursor: () => { seenCursors.clear(); lastFeedCursor = null; },
    push: pushFeedEvent,
    discontinuity: insertDiscontinuity,
    dropped: droppedUiEvents,
  });

  if (typeof control.onGap === 'function') {
    unsubscribers.push(control.onGap(() => {
      if (!disposed && presentationActive) void recovery.recover();
    }));
  }

  const setPresentationActive = (active: boolean): void => {
    if (disposed || active === presentationActive) return;
    // Commit already accepted events before suspension; their cursor and
    // identity must never describe discarded staging.
    if (!active) flushStaged();
    presentationActive = active;
    recovery.invalidate();
    if (!active) {
      viewport.pause();
      droppedUiEvents.value += staged.length;
      staged = [];
      cancelFrame(flushFrame);
      if (flushTimer !== null) clearTimeout(flushTimer);
      flushTimer = null;
      flushFrame = 0;
      flushQueued = false;
    } else {
      void recovery.recover();
    }
  };

  unsubscribers.push(control.onTopic<{ feedSuspended: number; feedBusDropped?: number; feedForwarded?: number }>('ui.projection.stats', (data) => {
    if (!Number.isFinite(data.feedSuspended)) return;
    if (lastDeliverySequence === null && typeof data.feedForwarded === 'number') lastDeliverySequence = data.feedForwarded;
    if (lastDeliverySequence !== null && typeof data.feedForwarded === 'number' && data.feedForwarded > lastDeliverySequence && presentationActive) void recovery.recover();
    const previous = lastHostSuspended ?? data.feedSuspended;
    droppedUiEvents.value += Math.max(0, data.feedSuspended - previous);
    lastHostSuspended = data.feedSuspended;
    if (typeof data.feedBusDropped === 'number' && Number.isFinite(data.feedBusDropped)) {
      const previousDropped = lastHostBusDropped ?? data.feedBusDropped;
      droppedUiEvents.value += Math.max(0, data.feedBusDropped - previousDropped);
      if (data.feedBusDropped > previousDropped && presentationActive) void recovery.recover();
      lastHostBusDropped = data.feedBusDropped;
    }
  }));

  unsubscribers.push(control.onTopic<{ uniqueId?: string; roomId?: string }>('live.connected', (data) => {
    const scope = JSON.stringify([data.uniqueId ?? '', data.roomId ?? '']);
    if (scope !== sessionScope) {
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
    if (typeof data.sequence === 'number' && Number.isSafeInteger(data.sequence)) {
      if (lastDeliverySequence !== null) droppedUiEvents.value += Math.max(0, data.sequence - lastDeliverySequence - 1);
      lastDeliverySequence = Math.max(lastDeliverySequence ?? 0, data.sequence);
    }
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
    stageEvent({
      kind: 'member' as const,
      author: callbacks.systemAuthor(),
      text: data.message,
      id: nextEventId++,
      receivedAt: Date.now(),
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
    diagnostics: () => ({ visible: events.value.length, staged: staged.length, dropped: droppedUiEvents.value, presentationActive, commits, received, duplicates, ...recovery.diagnostics(), presentationCoalesced, retentionEvicted, identities: seenMsgIds.size, cursor: lastFeedCursor, timers: Number(flushTimer !== null) + viewport.diagnostics().timers, frames: Number(flushFrame !== 0) + viewport.diagnostics().frames }),
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
    droppedUiEvents,
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
