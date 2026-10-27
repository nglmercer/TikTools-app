<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { usePointsStore } from '@/entities/points/model/points-store.ts';
import { enrichViewer, sameHandle, type ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import { loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import type { DisplayEvent } from '@/types.ts';
import type { InspectorTab } from '@/widgets/feed-inspector/model/inspector-tab.ts';
import FeedInspector from '@/widgets/feed-inspector/ui/FeedInspector.vue';
import LiveFeedPanel from '@/widgets/live-feed/ui/LiveFeedPanel.vue';

const MESSAGE_HISTORY_LIMIT = 20;

const connection = useConnectionStore();
const liveStore = useLiveFeedStore();
const { events, topViewers, likeLeaders, liveViewers, imagesEnabled } = storeToRefs(liveStore);
const { leaderboard } = storeToRefs(usePointsStore());

const inspectorOpen = ref(loadViewState().aside?.open !== false);
const tab = ref<InspectorTab>('viewers');
const selectedEventId = ref<number | null>(null);
const selectedEventSnapshot = shallowRef<DisplayEvent | null>(null);
const selectedViewer = shallowRef<ViewerIdentity | null>(null);

const selectedEvent = computed(() => {
  if (selectedEventId.value === null) return null;
  // Keep showing the selected row even after retention evicts it from the
  // visible buffer, instead of flashing "ya no está en el búfer".
  return events.value.find((event) => event.id === selectedEventId.value) ?? selectedEventSnapshot.value;
});
const viewer = computed(() => (selectedViewer.value ? enrichViewer(selectedViewer.value, leaderboard.value) : null));
const viewerMessages = computed<DisplayEvent[]>(() => {
  const current = selectedViewer.value;
  if (!current) return [];
  return events.value
    .filter((event) => event.kind === 'chat' && sameHandle(event.author, current.uniqueId))
    .slice(-MESSAGE_HISTORY_LIMIT)
    .reverse();
});

function selectEvent(event: DisplayEvent): void {
  selectedEventId.value = event.id;
  selectedEventSnapshot.value = { ...event };
  tab.value = 'event';
  inspectorOpen.value = true;
}

function openViewer(identity: ViewerIdentity): void {
  selectedViewer.value = identity;
  tab.value = 'user';
  inspectorOpen.value = true;
}

watch(inspectorOpen, (open) => saveViewState('aside', { open }));

watch(events, (list) => {
  if (selectedEventId.value === null) return;
  const live = list.find((event) => event.id === selectedEventId.value);
  if (live) selectedEventSnapshot.value = { ...live };
});

watch(
  () => connection.activeCreator,
  () => {
    selectedEventId.value = null;
    selectedEventSnapshot.value = null;
    selectedViewer.value = null;
    tab.value = 'viewers';
  },
);

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && inspectorOpen.value && !event.defaultPrevented) inspectorOpen.value = false;
}

onMounted(() => window.addEventListener('keydown', onKeydown));
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <div class="feed-page">
    <LiveFeedPanel
      class="feed-page__stream"
      :selected-event-id="selectedEventId"
      :inspector-open="inspectorOpen"
      @select="selectEvent"
      @open-viewer="openViewer"
      @toggle-inspector="inspectorOpen = !inspectorOpen"
    />
    <FeedInspector
      v-if="inspectorOpen"
      v-model:tab="tab"
      class="feed-page__inspector"
      :event="selectedEvent"
      :viewer="viewer"
      :messages="viewerMessages"
      :top-viewers="topViewers"
      :like-leaders="likeLeaders"
      :leaderboard="leaderboard"
      :live-viewers="liveViewers"
      :images="imagesEnabled"
      @close="inspectorOpen = false"
      @open-viewer="openViewer"
      @select-event="selectEvent"
    />
  </div>
</template>

<style scoped>
.feed-page {
  display: flex;
  flex: 1;
  gap: var(--space-4);
  min-width: 0;
  min-height: 0;
  padding: var(--space-4);
}

.feed-page__stream {
  flex: 1;
}

.feed-page__inspector {
  flex: none;
  width: 340px;
}

@media (width < 1024px) {
  .feed-page {
    gap: var(--space-3);
    padding: var(--space-3);
  }

  .feed-page__inspector {
    width: 300px;
  }
}

@media (width < 900px) {
  .feed-page {
    position: relative;
    flex-direction: column;
    overflow-y: auto;
  }

  .feed-page__stream {
    width: 100%;
    min-width: 0;
    min-height: min(52dvh, 480px);
  }

  .feed-page__inspector {
    position: absolute;
    inset: 0 0 0 auto;
    z-index: 5;
    width: min(340px, 100%);
    background: var(--color-bg-panel);
    box-shadow: var(--shadow-popup);
  }
}

@media (width < 700px) {
  .feed-page {
    padding: var(--space-2);
  }

  .feed-page__inspector {
    position: fixed;
    inset: auto 0 0 0;
    width: 100%;
    max-height: 72dvh;
    border-top: var(--border-width) solid var(--color-border-subtle);
  }
}
</style>
