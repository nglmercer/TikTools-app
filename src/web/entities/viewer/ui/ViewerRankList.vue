<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { formatCount } from '@/shared/lib/intl.ts';
import type { ViewerIdentity } from '../lib/viewer-profile.ts';
import ViewerAvatar from './ViewerAvatar.vue';

export interface ViewerRankEntry {
  key: string;
  rank: number;
  uniqueId: string;
  nickname?: string;
  avatarUrl?: string;
  score: number;
}

withDefaults(
  defineProps<{
    entries: readonly ViewerRankEntry[];
    caption?: string;
    empty: string;
    /** Extra class prefix kept for callers and tests (`<prefix>__row`). */
    prefix?: string;
    images?: boolean;
  }>(),
  { caption: '', prefix: '', images: true },
);

const emit = defineEmits<{ select: [viewer: ViewerIdentity] }>();
const { locale } = useI18n();

const displayName = (entry: ViewerRankEntry): string =>
  entry.nickname && entry.nickname !== entry.uniqueId ? entry.nickname : `@${entry.uniqueId}`;
</script>

<template>
  <div
    class="viewer-rank"
    :class="prefix"
  >
    <p
      v-if="caption && entries.length"
      class="viewer-rank__caption"
    >
      {{ caption }}
    </p>
    <ol
      v-if="entries.length"
      class="viewer-rank__list"
    >
      <li
        v-for="entry in entries"
        :key="entry.key"
      >
        <button
          class="viewer-rank__row"
          :class="prefix && `${prefix}__row`"
          type="button"
          :title="entry.nickname || entry.uniqueId"
          @click="emit('select', { uniqueId: entry.uniqueId, nickname: entry.nickname, avatarUrl: entry.avatarUrl })"
        >
          <span
            class="viewer-rank__rank"
            :class="entry.rank <= 3 && `viewer-rank__rank--${entry.rank}`"
          >{{ entry.rank }}</span>
          <ViewerAvatar
            :handle="entry.uniqueId"
            :src="images ? entry.avatarUrl : undefined"
          />
          <span class="viewer-rank__name">{{ displayName(entry) }}</span>
          <span
            v-if="entry.score > 0"
            class="viewer-rank__score"
          >{{ formatCount(entry.score, locale) }}</span>
        </button>
      </li>
    </ol>
    <p
      v-else
      class="viewer-rank__empty"
    >
      {{ empty }}
    </p>
  </div>
</template>

<style scoped>
.viewer-rank {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.viewer-rank__caption {
  margin: 0;
  padding: 0 var(--space-3);
  color: var(--color-text-subtle);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.viewer-rank__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}

.viewer-rank__row {
  display: grid;
  grid-template-columns: 20px auto minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--space-4);
  width: 100%;
  height: 36px;
  padding: 0 var(--space-3);
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  text-align: left;
  cursor: pointer;
  outline: none;
  transform: none;
}

.viewer-rank__row:hover {
  background: var(--color-bg-hover);
}

.viewer-rank__row:focus-visible {
  box-shadow: var(--focus-ring);
}

.viewer-rank__rank {
  color: var(--color-text-subtle);
  font-variant-numeric: tabular-nums;
  text-align: center;
}

/* Podium: gold, silver, bronze. */
.viewer-rank__rank--1,
.viewer-rank__rank--2,
.viewer-rank__rank--3 {
  font-weight: var(--font-weight-semibold);
}

.viewer-rank__rank--1 { color: var(--color-chart-3); }
.viewer-rank__rank--2 { color: #c3c7d1; }
.viewer-rank__rank--3 { color: #c98a55; }

.viewer-rank__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.viewer-rank__score {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;
}

.viewer-rank__empty {
  margin: 0;
  padding: var(--space-4) var(--space-3);
  color: var(--color-text-subtle);
}
</style>
