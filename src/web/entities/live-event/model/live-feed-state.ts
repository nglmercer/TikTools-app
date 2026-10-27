import { liveMessageIdentity, LIVE_IDENTITY_CAPACITY, LIVE_IDENTITY_TTL_MS } from '../../../../shared/live-identity.ts';
import { ref, shallowRef, watch } from 'vue';

import type { GiftCatalogEntry, LikeLeaderPayload, UiEvent } from '../../../../shared/messages.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import type { DisplayEvent, EventFilter, TopViewerPayload } from '@/types.ts';
import { debounce, loadViewState, saveViewState, type DebouncedCallback } from '@/shared/lib/view-state.ts';

export interface LiveFeedCallbacks {
  systemAuthor: () => string;
  discontinuityText: () => string;
}

export interface GiftCatalogResult {
  gifts: GiftCatalogEntry[];
}

export interface FeedBackfillResult {
  oldestCursor: number;
  latestCursor: number;
  depth: number;
  gap: boolean;
  truncated: boolean;
  events: Array<{ cursor: number; event: UiEvent }>;
}

export function createLiveFeedState(control: ControlClient, callbacks: LiveFeedCallbacks) {
  const seenMsgIds = new Map<string, number>();
  const seenCursors = new Set<number>();
  let recoveryGeneration = 0;
  let recoveryInFlight = false;
  let recoveryRequested = false;
  let recoveryAfter: number | null = null;
  let duplicates = 0;
  let recoveryGaps = 0;
  let recoveredEvents = 0;
  let recoveredGaps = 0;
  let unjournaledSuspended = 0;
  let needsCursorSort = false;
  let sessionScope = '';
  let sessionRoom = '';
  let sessionCreator = '';
  const savedFeed = loadViewState().feed;
  const events = shallowRef<DisplayEvent[]>([]);
  const filter = ref<EventFilter>(savedFeed?.filter ?? 'all');
  const imagesEnabled = ref(true);
  const renderLimit = ref(100);
  const searchQuery = ref(savedFeed?.searchQuery ?? '');
  const topViewers = shallowRef<TopViewerPayload[]>([]);
  const likeLeaders = shallowRef<LikeLeaderPayload[]>([]);
  const liveViewers = ref(0);
  const giftCatalog = shallowRef<GiftCatalogEntry[]>([]);
  const autoScroll = ref(savedFeed?.autoScroll ?? true);
  const unreadCount = ref(0);
    const droppedUiEvents = ref(0);
  const error = ref<string | null>(null);
  let nextEventId = 0;
  const streamContainerRef = ref<HTMLDivElement | null>(null);
  let restoreScrollTop = savedFeed?.scrollTop;
  let scrollSaveHandler: DebouncedCallback | null = null;
  let restoreFrame = 0;
  let restoreTimer: ReturnType<typeof setTimeout> | null = null;
  let refreshRevision = 0;
  const unsubscribers: Array<() => void> = [];

  const MAX_VISIBLE_FEED = 300;
  const MAX_STAGED_EVENTS = 900;
  let staged: DisplayEvent[] = [];
  let flushQueued = false;
  let flushFrame = 0;
  let flushTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollFrame = 0;
  let disposed = false;
  let presentationActive = true;
  let commits = 0;
  let received = 0;
  let presentationCoalesced = 0;
  let retentionEvicted = 0;
  let lastHostSuspended: number | null = null;
  let lastHostBusDropped: number | null = null;
  let lastDeliverySequence: number | null = null;
    let lastFeedCursor: number | null = null;

  const scheduleFrame = (callback: () => void): number => {
    if (typeof requestAnimationFrame === 'function') return requestAnimationFrame(callback);
    callback();
    return 0;
  };
  const cancelFrame = (id: number): void => {
    if (id !== 0 && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(id);
  };

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

  const giftGroupKey = (event: UiEvent): string | null => {
    const group = event.giftDetails?.groupId?.trim() ?? '';
    if (!group || /^0+$/.test(group)) return null;
    const giftId = event.giftDetails?.giftId?.trim() ?? '';
    if (!giftId || /^0+$/.test(giftId)) return null;
    return JSON.stringify([event.roomId ?? sessionRoom, event.creatorId ?? sessionCreator, event.author, giftId, group]);
  };

  const applyGiftProgress = (target: DisplayEvent, update: UiEvent): void => {
    const count = update.giftDetails?.count ?? target.giftDetails?.count ?? 1;
    const diamonds = update.giftDetails?.diamonds ?? target.giftDetails?.diamonds ?? 0;
    target.giftDetails = {
      ...target.giftDetails,
      name: update.giftDetails?.name ?? target.giftDetails?.name ?? 'Gift',
      count,
      diamonds,
      giftId: update.giftDetails?.giftId ?? target.giftDetails?.giftId,
      imageUrl: update.giftDetails?.imageUrl ?? target.giftDetails?.imageUrl,
      groupId: update.giftDetails?.groupId ?? target.giftDetails?.groupId,
      repeatEnd: update.giftDetails?.repeatEnd ?? target.giftDetails?.repeatEnd,
      streakable: update.giftDetails?.streakable ?? target.giftDetails?.streakable,
    };
    target.text = update.text;
    if (update.nickname !== undefined) target.nickname = update.nickname;
    if (update.avatarUrl !== undefined) target.avatarUrl = update.avatarUrl;
    if (update.i18nKey !== undefined) target.i18nKey = update.i18nKey;
    if (update.i18nParams !== undefined) target.i18nParams = update.i18nParams;
    if (update.msgId !== undefined) target.msgId = update.msgId;
    target.receivedAt = Date.now();
  };

  const tryMergeGiftStreak = (event: UiEvent): boolean => {
    if (event.kind !== 'gift') return false;
    const key = giftGroupKey(event);
    if (key === null) return false;
    for (let i = staged.length - 1; i >= 0; i -= 1) {
      const candidate = staged[i]!;
      if (candidate.kind === 'gift' && giftGroupKey(candidate) === key) {
        applyGiftProgress(candidate, event);
        presentationCoalesced += 1;
        return true;
      }
    }
    const committed = events.value;
    for (let i = committed.length - 1; i >= 0; i -= 1) {
      const candidate = committed[i]!;
      if (candidate.kind === 'gift' && giftGroupKey(candidate) === key) {
        applyGiftProgress(candidate, event);
        events.value = [...committed];
        presentationCoalesced += 1;
        return true;
      }
    }
    return false;
  };

  // A like is not a TikTok gift combo. Only coalesce its visual rows when
  // the backend supplies a reliable per-viewer total and stable viewer key.
  // The feed journal, automation rules, points and analytics retain each packet.
  const LIKE_BURST_WINDOW_MS = 3_000;
  const likeGroupKey = (event: UiEvent): string | null => {
    if (event.kind !== 'like' || !event.likeViewerKey ||
        !Number.isSafeInteger(event.likeUserTotal) || (event.likeUserTotal ?? 0) <= 0) return null;
    return JSON.stringify([
      event.creatorId ?? sessionCreator,
      event.roomId ?? sessionRoom,
      event.likeViewerKey,
    ]);
  };

  const tryMergeLikeBurst = (event: UiEvent, cursor?: number): boolean => {
    const key = likeGroupKey(event);
    if (key === null) return false;
    const now = Date.now();
    const canMerge = (candidate: DisplayEvent): boolean => {
      const elapsed = now - candidate.receivedAt;
      return likeGroupKey(candidate) === key &&
        elapsed >= 0 && elapsed <= LIKE_BURST_WINDOW_MS &&
        candidate.likeUserTotal !== undefined &&
        event.likeUserTotal! > candidate.likeUserTotal;
    };
    const updatedRow = (candidate: DisplayEvent): DisplayEvent => ({
      ...candidate,
      ...event,
      id: candidate.id,
      feedCursor: cursor ?? event.feedCursor ?? candidate.feedCursor,
      receivedAt: now,
    });

    for (let i = staged.length - 1; i >= 0; i -= 1) {
      const candidate = staged[i]!;
      if (!canMerge(candidate)) continue;
      staged.splice(i, 1);
      staged.push(updatedRow(candidate));
      presentationCoalesced += 1;
      return true;
    }

    const committed = events.value;
    for (let i = committed.length - 1; i >= 0; i -= 1) {
      const candidate = committed[i]!;
      if (!canMerge(candidate)) continue;
      // Re-stage the existing row after any already staged events to reflect
      // the latest activity without duplicating the user's row.
      events.value = [...committed.slice(0, i), ...committed.slice(i + 1)];
      stageEvent(updatedRow(candidate));
      presentationCoalesced += 1;
      return true;
    }
    return false;
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
    if (autoScroll.value) queueScroll();
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

  const applyScroll = (): void => {
    if (scrollTimer !== null) clearTimeout(scrollTimer);
    scrollTimer = null;
    cancelFrame(scrollFrame);
    scrollFrame = 0;
    if (disposed || !presentationActive || !autoScroll.value) return;
    const container = streamContainerRef.value;
    if (container) container.scrollTop = container.scrollHeight;
  };

  const queueScroll = (): void => {
    if (scrollFrame !== 0 || scrollTimer !== null || disposed || !presentationActive) return;
    if (typeof requestAnimationFrame === 'function') scrollTimer = setTimeout(applyScroll, 100);
    scrollFrame = scheduleFrame(applyScroll);
  };

  const scrollToBottom = (): void => {
    queueScroll();
  };

  const dispose = (): void => {
    disposed = true;
    cancelFrame(flushFrame);
    cancelFrame(scrollFrame);
    cancelFrame(restoreFrame);
    if (restoreTimer !== null) clearTimeout(restoreTimer);
    restoreTimer = null;
    if (flushTimer !== null) clearTimeout(flushTimer);
    if (scrollTimer !== null) clearTimeout(scrollTimer);
    flushTimer = scrollTimer = null;
    refreshRevision += 1;
    saveFeedStateDebounced.cancel();
    scrollSaveHandler?.cancel();
    seenMsgIds.clear();
    seenCursors.clear();
    flushFrame = 0;
    scrollFrame = 0;
    flushQueued = false;
    staged = [];
    while (unsubscribers.length > 0) unsubscribers.pop()?.();
    const container = streamContainerRef.value;
    if (container && scrollSaveHandler) {
      container.removeEventListener('scroll', scrollSaveHandler);
      scrollSaveHandler = null;
    }
  };

  const pushFeedEvent = (event: UiEvent, cursor?: number): boolean => {
    if (disposed) return false;
    received += 1;
    if (!presentationActive) {
      droppedUiEvents.value += 1;
      if (cursor === undefined) unjournaledSuspended += 1;
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
    if (tryMergeGiftStreak(event)) return true;
    if (tryMergeLikeBurst(event, cursor)) return true;
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

  const recoverFeed = async (): Promise<void> => {
    if (recoveryInFlight) { recoveryRequested = true; return; }
    if (typeof (control as { call?: unknown }).call !== 'function') { unjournaledSuspended = 0; insertDiscontinuity(); return; }
    const generation = recoveryGeneration;
    recoveryInFlight = true;
    let recoveredAny = false;
    let after = recoveryAfter ?? lastFeedCursor ?? 0;
    recoveryAfter = after;
    try {
      // One host journal is bounded. Page by the last returned cursor, never
      // by latestCursor (which could skip the rest of a truncated page).
      for (let page = 0; page < 16; page += 1) {
        const result = await control.call<FeedBackfillResult>('ui.feed.backfill', { version: 1, after });
        if (disposed || !presentationActive || generation !== recoveryGeneration) return;
        if (result.latestCursor < after) {
          seenCursors.clear(); lastFeedCursor = null; after = 0; recoveryAfter = 0;
          recoveryGaps += 1; insertDiscontinuity();
          continue;
        }
        if (unjournaledSuspended > 0) {
          unjournaledSuspended = 0; recoveryGaps += 1; insertDiscontinuity();
        }
        let recovered = 0;
        let next = after;
        for (const entry of result.events ?? []) {
          if (!Number.isSafeInteger(entry?.cursor) || entry.cursor <= after || !entry.event) continue;
          next = Math.max(next, entry.cursor);
          if (pushFeedEvent(entry.event, entry.cursor)) recovered += 1;
        }
        if (recovered > 0) {
          recoveredEvents += recovered;
          if (!recoveredAny) recoveredGaps += 1;
          recoveredAny = true;
        }
        droppedUiEvents.value = Math.max(0, droppedUiEvents.value - recovered);
        if (result.gap) { recoveryGaps += 1; insertDiscontinuity(); }
        if (!result.truncated) { if (!recoveryRequested) recoveryAfter = null; break; }
        if (next <= after || page === 15) { recoveryGaps += 1; insertDiscontinuity(); break; }
        after = next;
        recoveryAfter = after;
      }
    } catch {
      if (!disposed && presentationActive && generation === recoveryGeneration) { recoveryGaps += 1; insertDiscontinuity(); }
    } finally {
      recoveryInFlight = false;
      if (recoveryRequested && !disposed && presentationActive) {
        recoveryRequested = false;
        void recoverFeed();
      }
    }
  };

  if (typeof control.onGap === 'function') {
    unsubscribers.push(control.onGap(() => {
      if (!disposed && presentationActive) void recoverFeed();
    }));
  }

  const setPresentationActive = (active: boolean): void => {
    if (disposed || active === presentationActive) return;
    // Commit already accepted events before suspension; their cursor and
    // identity must never describe discarded staging.
    if (!active) flushStaged();
    presentationActive = active;
    recoveryGeneration += 1;
    if (!active) {
      if (!autoScroll.value && streamContainerRef.value) {
        restoreScrollTop = streamContainerRef.value.scrollTop;
        saveFeedState();
      }
      droppedUiEvents.value += staged.length;
      staged = [];
      cancelFrame(flushFrame);
      cancelFrame(scrollFrame);
      cancelFrame(restoreFrame);
      if (restoreTimer !== null) clearTimeout(restoreTimer);
      restoreTimer = null;
      restoreFrame = 0;
      if (flushTimer !== null) clearTimeout(flushTimer);
      if (scrollTimer !== null) clearTimeout(scrollTimer);
      flushTimer = scrollTimer = null;
      flushFrame = scrollFrame = 0;
      flushQueued = false;
    } else {
      void recoverFeed();
    }
  };

  unsubscribers.push(control.onTopic<{ feedSuspended: number; feedBusDropped?: number; feedForwarded?: number }>('ui.projection.stats', (data) => {
    if (!Number.isFinite(data.feedSuspended)) return;
    if (lastDeliverySequence === null && typeof data.feedForwarded === 'number') lastDeliverySequence = data.feedForwarded;
    if (lastDeliverySequence !== null && typeof data.feedForwarded === 'number' && data.feedForwarded > lastDeliverySequence && presentationActive) void recoverFeed();
    const previous = lastHostSuspended ?? data.feedSuspended;
    droppedUiEvents.value += Math.max(0, data.feedSuspended - previous);
    lastHostSuspended = data.feedSuspended;
    if (typeof data.feedBusDropped === 'number' && Number.isFinite(data.feedBusDropped)) {
      const previousDropped = lastHostBusDropped ?? data.feedBusDropped;
      droppedUiEvents.value += Math.max(0, data.feedBusDropped - previousDropped);
      if (data.feedBusDropped > previousDropped && presentationActive) void recoverFeed();
      lastHostBusDropped = data.feedBusDropped;
    }
  }));

  unsubscribers.push(control.onTopic<{ uniqueId?: string; roomId?: string }>('live.connected', (data) => {
    const scope = JSON.stringify([data.uniqueId ?? '', data.roomId ?? '']);
    if (scope !== sessionScope) {
      seenMsgIds.clear();
      seenCursors.clear();
      likeLeaders.value = [];
      sessionScope = scope;
      sessionRoom = data.roomId ?? '';
      sessionCreator = data.uniqueId ?? '';
      recoveryGeneration += 1;
      recoveryAfter = null;
    }
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
    if (gap) recoveryAfter = Math.min(recoveryAfter ?? previousCursor ?? 0, previousCursor ?? 0);
    pushFeedEvent(data.event, cursor);
    if (gap && presentationActive) void recoverFeed();
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
    if (lastFeedCursor === null) void recoverFeed();
    try {
      const result = await control.call<GiftCatalogResult>('gifts.list', {});
      if (!disposed && revision === refreshRevision) giftCatalog.value = result.gifts;
    } catch (failure) {
      if (!disposed && revision === refreshRevision) error.value = errorMessage(failure);
    }
  };

  const handleToggleAutoScroll = (): void => {
    const nextState = !autoScroll.value;
    autoScroll.value = nextState;
    if (!nextState) {
      cancelFrame(scrollFrame);
      scrollFrame = 0;
      if (scrollTimer !== null) clearTimeout(scrollTimer);
      scrollTimer = null;
    }
    if (nextState) {
      unreadCount.value = 0;
      scrollToBottom();
    }
  };

  const setFilter = (value: EventFilter): void => {
    filter.value = value;
  };
  const setSearchQuery = (value: string): void => {
    searchQuery.value = value;
  };
    const saveFeedState = (): void => {
    saveViewState('feed', {
      filter: filter.value,
      searchQuery: searchQuery.value,
      autoScroll: autoScroll.value,
      scrollTop: streamContainerRef.value?.scrollTop ?? 0,
    });
  };
  const saveFeedStateDebounced = debounce(saveFeedState);
  unsubscribers.push(watch([filter, searchQuery, autoScroll], saveFeedStateDebounced));
    const applySavedScrollTop = (): void => {
    const container = streamContainerRef.value;
    const top = restoreScrollTop;
    if (!container || top === undefined || top <= 0 || autoScroll.value) return;
    if (container.scrollHeight < top) return;
    container.scrollTop = top;
    restoreScrollTop = undefined;
  };
  unsubscribers.push(watch(events, applySavedScrollTop));
  const setStreamContainerRef = (element: Element | null): void => {
    const next = element instanceof HTMLDivElement ? element : null;
    if (streamContainerRef.value === next) return;
    const previous = streamContainerRef.value;
    if (previous && scrollSaveHandler) {
      previous.removeEventListener('scroll', scrollSaveHandler);
      scrollSaveHandler.cancel();
      scrollSaveHandler = null;
    }
    streamContainerRef.value = next;
    const container = streamContainerRef.value;
    if (!container) return;
    cancelFrame(restoreFrame);
    if (restoreTimer !== null) clearTimeout(restoreTimer);
    const restore = (): void => {
      cancelFrame(restoreFrame);
      restoreFrame = 0;
      if (restoreTimer !== null) clearTimeout(restoreTimer);
      restoreTimer = null;
      if (!disposed) applySavedScrollTop();
    };
    if (!autoScroll.value && restoreScrollTop !== undefined && restoreScrollTop > 0) restoreTimer = setTimeout(restore, 100);
    restoreFrame = scheduleFrame(restore);
    scrollSaveHandler = debounce(saveFeedState);
    container.addEventListener('scroll', scrollSaveHandler);
  };

  return {
    diagnostics: () => ({ visible: events.value.length, staged: staged.length, dropped: droppedUiEvents.value, presentationActive, commits, received, duplicates, recoveryGaps, recoveredEvents, recoveredGaps, presentationCoalesced, retentionEvicted, identities: seenMsgIds.size, recoveryInFlight, cursor: lastFeedCursor, timers: Number(flushTimer !== null) + Number(scrollTimer !== null) + Number(restoreTimer !== null), frames: Number(flushFrame !== 0) + Number(scrollFrame !== 0) + Number(restoreFrame !== 0) }),
    events,
    imagesEnabled,
    renderLimit,
    filter,
    searchQuery,
    topViewers,
    likeLeaders,
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
