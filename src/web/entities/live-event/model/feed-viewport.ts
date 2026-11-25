import { ref, watch, type Ref } from 'vue';
import type { DisplayEvent, EventFilter } from '@/types.ts';
import { debounce, loadViewState, saveViewState, type DebouncedCallback } from '@/shared/lib/view-state.ts';
import { scheduleFrame, cancelFrame } from './feed-frame.ts';

/** Presentation preferences and scroll resources; owns no transport subscriptions. */
export function createFeedViewport(events: Ref<DisplayEvent[]>, isActive: () => boolean) {
  let disposed = false;
  const unsubscribers: Array<() => void> = [];
  const savedFeed = loadViewState().feed;
  const filter = ref<EventFilter>(savedFeed?.filter ?? 'all');
  const imagesEnabled = ref(true);
  const renderLimit = ref(100);
  const searchQuery = ref(savedFeed?.searchQuery ?? '');
  const autoScroll = ref(savedFeed?.autoScroll ?? true);
  const unreadCount = ref(0);
  const streamContainerRef = ref<HTMLDivElement | null>(null);
  let restoreScrollTop = savedFeed?.scrollTop;
  let scrollSaveHandler: DebouncedCallback | null = null;
  let restoreFrame = 0;
  let restoreTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollTimer: ReturnType<typeof setTimeout> | null = null;
  let scrollFrame = 0;
  const applyScroll = (): void => {
    if (scrollTimer !== null) clearTimeout(scrollTimer);
    scrollTimer = null;
    cancelFrame(scrollFrame);
    scrollFrame = 0;
    if (disposed || !isActive() || !autoScroll.value) return;
    const container = streamContainerRef.value;
    if (container) container.scrollTop = container.scrollHeight;
  };

  const queueScroll = (): void => {
    if (scrollFrame !== 0 || scrollTimer !== null || disposed || !isActive()) return;
    if (typeof requestAnimationFrame === 'function') scrollTimer = setTimeout(applyScroll, 100);
    scrollFrame = scheduleFrame(applyScroll);
  };

  const scrollToBottom = (): void => {
    queueScroll();
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

  // Pin the scroll in the same DOM commit as an insert/eviction. Doing this in
  // a later frame is what makes new rows look like they "jump" into place.
  let beforeHeight = 0;
  let beforeTop = 0;
  let pausedAnchor: { id: number; offset: number } | null = null;

  const capturePausedAnchor = (container: HTMLElement, nextEvents: readonly DisplayEvent[]): { id: number; offset: number } | null => {
    const surviving = new Set(nextEvents.map((event) => event.id));
    const containerRect = container.getBoundingClientRect();
    let firstSurviving: { id: number; offset: number } | null = null;
    const walker = document.createTreeWalker(container, NodeFilter.SHOW_ELEMENT, {
      acceptNode: (node) => node instanceof HTMLElement && node.dataset.eventId !== undefined
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_SKIP,
    });
    for (let element = walker.nextNode(); element instanceof HTMLElement; element = walker.nextNode()) {
      const id = Number(element.dataset.eventId);
      if (!surviving.has(id)) continue;
      const rect = element.getBoundingClientRect();
      const anchor = { id, offset: rect.top - containerRect.top };
      if (firstSurviving === null) firstSurviving = anchor;
      if (rect.bottom > containerRect.top && rect.top < containerRect.bottom) return anchor;
    }
    return firstSurviving;
  };

  const restorePausedAnchor = (container: HTMLElement): boolean => {
    if (pausedAnchor === null) return false;
    const target = container.querySelector<HTMLElement>(`[data-event-id="${pausedAnchor.id}"]`);
    if (!target) return false;
    const containerTop = container.getBoundingClientRect().top;
    const offset = target.getBoundingClientRect().top - containerTop;
    const delta = offset - pausedAnchor.offset;
    if (delta !== 0) container.scrollTop = Math.max(0, container.scrollTop + delta);
    return true;
  };
  unsubscribers.push(watch(events, () => {
    const container = streamContainerRef.value;
    if (!container) return;
    beforeHeight = container.scrollHeight;
    beforeTop = container.scrollTop;
    pausedAnchor = !autoScroll.value && restoreScrollTop === undefined
      ? capturePausedAnchor(container, events.value)
      : null;
  }, { flush: 'pre' }));
  unsubscribers.push(watch(events, () => {
    const container = streamContainerRef.value;
    if (disposed || !isActive() || !container) return;
    if (autoScroll.value) {
      container.scrollTop = container.scrollHeight;
      pausedAnchor = null;
      return;
    }
    // Rows evicted above must not shift the paused reading position. Prefer a
    // surviving row as the anchor because appends below can hide the removal
    // in the total scrollHeight delta.
    if (!restorePausedAnchor(container)) {
      const delta = container.scrollHeight - beforeHeight;
      if (delta < 0) container.scrollTop = Math.max(0, beforeTop + delta);
    }
    pausedAnchor = null;
  }, { flush: 'post' }));

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

  function pause(): void {
    if (!autoScroll.value && streamContainerRef.value) {
      restoreScrollTop = streamContainerRef.value.scrollTop;
      saveFeedState();
    }
    cancelFrame(scrollFrame);
    cancelFrame(restoreFrame);
    if (restoreTimer !== null) clearTimeout(restoreTimer);
    if (scrollTimer !== null) clearTimeout(scrollTimer);
    restoreTimer = scrollTimer = null;
    restoreFrame = scrollFrame = 0;
  }

  function dispose(): void {
    disposed = true;
    cancelFrame(scrollFrame);
    cancelFrame(restoreFrame);
    if (restoreTimer !== null) clearTimeout(restoreTimer);
    if (scrollTimer !== null) clearTimeout(scrollTimer);
    restoreTimer = scrollTimer = null;
    scrollFrame = 0;
    saveFeedStateDebounced.cancel();
    scrollSaveHandler?.cancel();
    while (unsubscribers.length > 0) unsubscribers.pop()?.();
    const container = streamContainerRef.value;
    if (container && scrollSaveHandler) {
      container.removeEventListener('scroll', scrollSaveHandler);
      scrollSaveHandler = null;
    }
  }

  return {
    filter, imagesEnabled, renderLimit, searchQuery, autoScroll, unreadCount, streamContainerRef,
    scrollToBottom, handleToggleAutoScroll, setFilter, setSearchQuery, setStreamContainerRef,
    pause, dispose,
    diagnostics: () => ({ timers: Number(scrollTimer !== null) + Number(restoreTimer !== null), frames: Number(scrollFrame !== 0) + Number(restoreFrame !== 0) }),
  };
}
