<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { eventText, formatEventTime } from '@/entities/live-event/lib/event-text.ts';
import EventDetails from '@/entities/live-event/ui/EventDetails.vue';
import type { ViewerIdentity } from '@/entities/viewer/lib/viewer-profile.ts';
import TopViewersList from '@/entities/viewer/ui/TopViewersList.vue';
import TopLikesList from '@/entities/viewer/ui/TopLikesList.vue';
import ViewerProfile from '@/entities/viewer/ui/ViewerProfile.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconClose from '@/shared/ui/icon/icons/IconClose.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import type { DisplayEvent, LikeLeaderPayload, TopViewerPayload, UserCardUser, ViewerRecord } from '@/types.ts';
import type { InspectorTab } from '../model/inspector-tab.ts';

const props = withDefaults(
  defineProps<{
    event: DisplayEvent | null;
    viewer: UserCardUser | null;
    messages: readonly DisplayEvent[];
    topViewers: readonly TopViewerPayload[];
    likeLeaders: readonly LikeLeaderPayload[];
    leaderboard: readonly ViewerRecord[];
    liveViewers: number;
    images?: boolean;
  }>(),
  { images: true },
);

const tab = defineModel<InspectorTab>('tab', { required: true });

const emit = defineEmits<{
  close: [];
  'open-viewer': [viewer: ViewerIdentity];
  'select-event': [event: DisplayEvent];
}>();

const { t, locale } = useI18n();
const viewerMode = ref<'top' | 'likes'>('top');
const viewerModes = computed(() => [
  { value: 'top' as const, label: t('feed.viewers.filterTop') },
  { value: 'likes' as const, label: t('feed.viewers.filterLikes') },
]);

const tabs = computed(() => [
  { value: 'viewers' as const, label: t('feed.inspector.viewers') },
  { value: 'event' as const, label: t('feed.inspector.details'), disabled: !props.event },
  { value: 'user' as const, label: t('feed.inspector.profile'), disabled: !props.viewer },
]);
</script>

<template>
  <BasePanel
    class="feed-inspector"
    :title="t('feed.inspector.title')"
    flush
  >
    <template #actions>
      <BaseIconButton
        :label="t('feed.inspector.hide')"
        size="sm"
        @click="emit('close')"
      >
        <IconClose />
      </BaseIconButton>
    </template>
    <BaseTabs
      v-model="tab"
      class="feed-inspector__tabs"
      :tabs="tabs"
    >
      <template #default="{ value }">
        <div class="feed-inspector__body">
          <template v-if="value === 'viewers'">
            <BaseSegmented
              v-model="viewerMode"
              class="feed-inspector__viewer-filter"
              :options="viewerModes"
              size="sm"
            />
            <TopLikesList
              v-if="viewerMode === 'likes'"
              :leaders="likeLeaders"
              :images="images"
              @select="emit('open-viewer', $event)"
            />
            <TopViewersList
              v-else
              :top-viewers="topViewers"
              :leaderboard="leaderboard"
              :live-viewers="liveViewers"
              :images="images"
              @select="emit('open-viewer', $event)"
            />
          </template>
          <template v-else-if="value === 'event'">
            <EventDetails
              v-if="event"
              :event="event"
              :images="images"
              @open-viewer="emit('open-viewer', $event)"
            />
            <p
              v-else
              class="feed-inspector__empty"
            >
              {{ t('feed.inspector.emptyEvent') }}
            </p>
          </template>
          <template v-else>
            <ViewerProfile
              v-if="viewer"
              :viewer="viewer"
              :images="images"
            >
              <section class="feed-inspector__messages">
                <h3 class="feed-inspector__heading">
                  {{ t('feed.profile.recentMessages') }}
                  <span class="feed-inspector__count">{{ messages.length }}</span>
                </h3>
                <ol
                  v-if="messages.length"
                  class="feed-inspector__list"
                >
                  <li
                    v-for="message in messages"
                    :key="message.id"
                  >
                    <button
                      class="feed-inspector__message"
                      type="button"
                      @click="emit('select-event', message)"
                    >
                      <span class="feed-inspector__message-text">{{ eventText(message, t) }}</span>
                      <time class="feed-inspector__message-time">{{ formatEventTime(message.receivedAt, locale) }}</time>
                    </button>
                  </li>
                </ol>
                <p
                  v-else
                  class="feed-inspector__empty"
                >
                  {{ t('feed.profile.noMessages') }}
                </p>
              </section>
            </ViewerProfile>
            <p
              v-else
              class="feed-inspector__empty"
            >
              {{ t('feed.inspector.emptyUser') }}
            </p>
          </template>
        </div>
      </template>
    </BaseTabs>
  </BasePanel>
</template>

<style scoped>
.feed-inspector__tabs {
  height: 100%;
  padding-top: var(--space-2);
}

.feed-inspector__body {
  height: 100%;
  padding: var(--space-4);
  overflow-y: auto;
}

.feed-inspector__viewer-filter {
  margin-bottom: var(--space-3);
}

.feed-inspector__empty {
  margin: 0;
  color: var(--color-text-subtle);
}

.feed-inspector__messages {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.feed-inspector__heading {
  display: flex;
  justify-content: space-between;
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
}

.feed-inspector__count {
  color: var(--color-text-muted);
}

.feed-inspector__list {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  list-style: none;
}

.feed-inspector__message {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: var(--space-4);
  width: 100%;
  min-height: 28px;
  padding: var(--space-1) var(--space-3);
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  text-align: left;
  cursor: pointer;
  outline: none;
  transform: none;
}

.feed-inspector__message:hover {
  background: var(--color-bg-hover);
}

.feed-inspector__message:focus-visible {
  box-shadow: var(--focus-ring);
}

.feed-inspector__message-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.feed-inspector__message-time {
  color: var(--color-text-subtle);
  font-size: var(--font-size-xs);
}
</style>
