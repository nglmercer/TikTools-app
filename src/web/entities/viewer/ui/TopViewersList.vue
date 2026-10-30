<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatCount } from '@/shared/lib/intl.ts';
import type { TopViewerPayload, ViewerRecord } from '@/types.ts';
import { rankedViewers } from '../lib/top-viewers.ts';
import type { ViewerIdentity } from '../lib/viewer-profile.ts';
import ViewerAvatar from './ViewerAvatar.vue';

const props = withDefaults(
  defineProps<{
    topViewers: readonly TopViewerPayload[];
    leaderboard: readonly ViewerRecord[];
    liveViewers: number;
    images?: boolean;
  }>(),
  { images: true },
);

const emit = defineEmits<{ select: [viewer: ViewerIdentity] }>();
const { t, locale } = useI18n();
const entries = computed(() => rankedViewers(props.topViewers, props.leaderboard));
</script>

<template>
  <div class="top-viewers">
    <p class="top-viewers__live">
      {{ t('feed.viewers.live', { count: formatCount(liveViewers, locale) }) }}
    </p>
    <p
      v-if="entries.length"
      class="top-viewers__source"
    >
      {{ t(entries[0]?.provenance === 'tiktok_room' ? 'feed.viewers.sourceTikTok' : 'feed.viewers.sourcePoints') }}
    </p>
    <ul
      v-if="entries.length"
      class="top-viewers__list"
    >
      <li
        v-for="entry in entries"
        :key="entry.key"
      >
        <button
          class="top-viewers__row"
          type="button"
          :title="entry.nickname || entry.uniqueId"
          @click="emit('select', { uniqueId: entry.uniqueId, nickname: entry.nickname, avatarUrl: entry.avatarUrl })"
        >
          <span
            class="top-viewers__rank"
            :class="{ 'top-viewers__rank--podium': entry.rank <= 3 }"
          >{{ entry.rank }}</span>
          <ViewerAvatar
            :handle="entry.uniqueId"
            :src="images ? entry.avatarUrl : undefined"
          />
          <span class="top-viewers__name">
            {{ entry.nickname && entry.nickname !== entry.uniqueId ? entry.nickname : `@${entry.uniqueId}` }}
          </span>
          <span
            v-if="entry.score > 0"
            class="top-viewers__score"
          >{{ formatCount(entry.score, locale) }}</span>
        </button>
      </li>
    </ul>
    <p
      v-else
      class="top-viewers__empty"
    >
      {{ t('feed.viewers.empty') }}
    </p>
  </div>
</template>

<style scoped>
.top-viewers {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.top-viewers__live {
  margin: 0;
  color: var(--color-text-muted);
}

.top-viewers__source {
  margin: 0;
  color: var(--color-text-subtle);
  font-size: var(--font-size-sm);
}

.top-viewers__list {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  list-style: none;
}

.top-viewers__row {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: var(--space-4);
  width: 100%;
  height: 34px;
  padding: 0 var(--space-3);
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

.top-viewers__row:hover {
  background: var(--color-bg-hover);
}

.top-viewers__row:focus-visible {
  box-shadow: var(--focus-ring);
}

.top-viewers__rank {
  width: 16px;
  color: var(--color-text-subtle);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.top-viewers__rank--podium {
  color: var(--color-chart-3);
  font-weight: var(--font-weight-semibold);
}

.top-viewers__name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.top-viewers__score {
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
}

.top-viewers__empty {
  margin: 0;
  color: var(--color-text-subtle);
}
</style>
