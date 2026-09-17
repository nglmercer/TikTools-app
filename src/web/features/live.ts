import { ref } from 'vue';

import type { GiftCatalogEntry } from '../../shared/messages.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';
import type { DisplayEvent, EventFilter, TopViewerPayload } from '../types.ts';

export interface LiveCallbacks {
  onChat: (
    author: string,
    text: string,
    points: number | undefined,
    isSubscriber: boolean | undefined,
  ) => void;
  systemAuthor: () => string;
}

export interface GiftCatalogResult {
  gifts: GiftCatalogEntry[];
}

/** Live feed: events, room stats, gift catalog, and scroll state. */
export function useLive(control: ControlClient, callbacks: LiveCallbacks) {
  const events = ref<DisplayEvent[]>([]);
  const filter = ref<EventFilter>('all');
  const searchQuery = ref('');
  const topViewers = ref<TopViewerPayload[]>([]);
  const liveViewers = ref(0);
  const giftCatalog = ref<GiftCatalogEntry[]>([]);
  const autoScroll = ref(true);
  const unreadCount = ref(0);
  const nextEventId = ref(0);
  const streamContainerRef = ref<HTMLDivElement | null>(null);

  const resetEvents = (): void => {
    nextEventId.value = 0;
    events.value = [];
    unreadCount.value = 0;
    topViewers.value = [];
    liveViewers.value = 0;
  };

  const scrollToBottom = (): void => {
    const container = streamContainerRef.value;
    if (container) container.scrollTop = container.scrollHeight;
  };

  control.onPush('live-event', (message) => {
    if (message.type !== 'live-event') return;
    const event = message.event;
    events.value = [
      ...events.value,
      { ...event, id: nextEventId.value++, receivedAt: Date.now() },
    ].slice(-300);
    if (!autoScroll.value) unreadCount.value += 1;
    if (event.kind === 'chat' && event.text) {
      callbacks.onChat(event.author, event.text, event.points, event.isSubscriber);
    }
  });
  control.onPush('room-stats', (message) => {
    if (message.type !== 'room-stats') return;
    topViewers.value = message.topViewers;
    liveViewers.value = message.viewers;
  });
  control.onPush('gift-catalog', (message) => {
    if (message.type !== 'gift-catalog') return;
    giftCatalog.value = message.gifts;
  });
  control.onPush('error', (message) => {
    if (message.type !== 'error') return;
    events.value = [
      ...events.value,
      {
        kind: 'member' as const,
        author: callbacks.systemAuthor(),
        text: message.message,
        id: nextEventId.value++,
        receivedAt: Date.now(),
      },
    ].slice(-300);
  });

  const refresh = async (): Promise<void> => {
    try {
      const result = await control.call<GiftCatalogResult>('gifts.list', {});
      giftCatalog.value = result.gifts;
    } catch (failure) {
      console.warn(`gifts.list failed: ${errorMessage(failure)}`);
    }
  };

  const handleToggleAutoScroll = (): void => {
    const nextState = !autoScroll.value;
    autoScroll.value = nextState;
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
  const setStreamContainerRef = (element: Element | null): void => {
    streamContainerRef.value = element instanceof HTMLDivElement ? element : null;
  };

  return {
    events,
    filter,
    searchQuery,
    topViewers,
    liveViewers,
    giftCatalog,
    autoScroll,
    unreadCount,
    streamContainerRef,
    resetEvents,
    scrollToBottom,
    handleToggleAutoScroll,
    setFilter,
    setSearchQuery,
    setStreamContainerRef,
    refresh,
  };
}
