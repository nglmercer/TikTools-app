<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, useTemplateRef, watch } from 'vue';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { useLiveFeedStore } from '@/entities/live-event/model/live-feed-store.ts';
import { usePointsStore } from '@/entities/points/model/points-store.ts';
import { registerGuideActions } from '@/features/guided-help/model/guide-actions.ts';
import { guideAction, guideScope as guideScopeNames } from '@/features/guided-help/model/guide-vocabulary.ts';
import { enrichViewer, sameHandle, type ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import { loadViewState, saveViewState } from '@/shared/lib/view-state.ts';
import type { DisplayEvent } from '@/types.ts';
import type { InspectorTab } from '@/widgets/feed-inspector/model/inspector-tab.ts';
import FeedInspector from '@/widgets/feed-inspector/ui/FeedInspector.vue';
import LiveFeedPanel from '@/widgets/live-feed/ui/LiveFeedPanel.vue';

const MESSAGE_HISTORY_LIMIT = 20;

const connection = useConnectionStore();
const liveStore = useLiveFeedStore();
const { events, topViewers, likeLeaders, topRanking, likesRanking, imagesEnabled } = storeToRefs(liveStore);
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

const guideScope = useTemplateRef<HTMLElement>('guideScope');
let unregisterGuideActions: (() => void) | undefined;
onMounted(() => {
  unregisterGuideActions = registerGuideActions(guideScopeNames.feed, {
    [guideAction.feed.openInspector]: () => {
      if (inspectorOpen.value) return false;
      inspectorOpen.value = true;
      return true;
    },
    [guideAction.feed.showViewers]: () => {
      if (!inspectorOpen.value) { inspectorOpen.value = true; return true; }
      if (tab.value === 'viewers') return false;
      tab.value = 'viewers';
      return true;
    },
    [guideAction.feed.showEvent]: () => {
      if (!inspectorOpen.value) { inspectorOpen.value = true; return true; }
      if (!selectedEvent.value) {
        const first = events.value[0];
        if (!first) return false;
        selectEvent(first);
        return true;
      }
      if (tab.value === 'event') return false;
      tab.value = 'event';
      return true;
    },
    [guideAction.feed.showViewer]: () => {
      if (!inspectorOpen.value) { inspectorOpen.value = true; return true; }
      if (!viewer.value) {
        const top = topViewers.value[0] ?? likeLeaders.value[0];
        const first = events.value[0];
        const identity = top
          ? { uniqueId: top.uniqueId, nickname: top.nickname, ...(top.avatarUrl ? { avatarUrl: top.avatarUrl } : {}) }
          : first ? { uniqueId: first.author, ...(first.nickname ? { nickname: first.nickname } : {}), ...(first.avatarUrl ? { avatarUrl: first.avatarUrl } : {}) } : null;
        if (!identity) return false;
        openViewer(identity);
        return true;
      }
      if (tab.value === 'user') return false;
      tab.value = 'user';
      return true;
    },
  }, guideScope.value ?? undefined);
  window.addEventListener('keydown', onKeydown);
});
onBeforeUnmount(() => {
  unregisterGuideActions?.();
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <div
    ref="guideScope"
    class="feed-page"
    data-guide-scope="feed"
  >
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
      :top-ranking="topRanking"
      :likes-ranking="likesRanking"
      :leaderboard="leaderboard"
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

@media (width < 700px) {
  .feed-page {
    flex-direction: column;
    padding: var(--space-2);
  }

  .feed-page__stream {
    min-height: 0;
  }

  .feed-page__inspector {
    flex: 0 1 35%;
    width: 100%;
    min-height: 160px;
  }
}
</style>
