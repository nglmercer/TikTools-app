<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { formatCount } from '@/shared/lib/intl.ts';
import type { LikeLeaderPayload } from '@/types.ts';
import type { ViewerIdentity } from '../lib/viewer-profile.ts';
import ViewerAvatar from './ViewerAvatar.vue';

withDefaults(
  defineProps<{ leaders: readonly LikeLeaderPayload[]; images?: boolean }>(),
  { images: true },
);

const emit = defineEmits<{ select: [viewer: ViewerIdentity] }>();
const { t, locale } = useI18n();
</script>

<template>
  <div class="top-likes">
    <ul
      v-if="leaders.length"
      class="top-likes__list"
    >
      <li
        v-for="leader in leaders"
        :key="leader.userId || leader.uniqueId"
      >
        <button
          class="top-likes__row"
          type="button"
          :title="leader.nickname || leader.uniqueId"
          @click="emit('select', { uniqueId: leader.uniqueId, nickname: leader.nickname, avatarUrl: leader.avatarUrl })"
        >
          <span
            class="top-likes__rank"
            :class="{ 'top-likes__rank--podium': leader.rank <= 3 }"
          >{{ leader.rank }}</span>
          <ViewerAvatar
            :handle="leader.uniqueId"
            :src="images ? leader.avatarUrl : undefined"
          />
          <span class="top-likes__name">
            {{ leader.nickname && leader.nickname !== leader.uniqueId ? leader.nickname : `@${leader.uniqueId}` }}
          </span>
          <span class="top-likes__score">{{ formatCount(leader.score, locale) }}</span>
        </button>
      </li>
    </ul>
    <p
      v-else
      class="top-likes__empty"
    >
      {{ t('feed.viewers.emptyLikes') }}
    </p>
  </div>
</template>

<style scoped>
.top-likes__list {
  display: flex;
  flex-direction: column;
  gap: 0;
  margin: 0;
  padding: 0;
  list-style: none;
}

.top-likes__row {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: var(--space-4);
  width: 100%;
  min-height: 34px;
  padding: 0 var(--space-3);
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  text-align: left;
  cursor: pointer;
}

.top-likes__row:hover {
  background: var(--color-bg-hover);
}

.top-likes__row:focus-visible {
  box-shadow: var(--focus-ring);
}

.top-likes__rank {
  width: 16px;
  color: var(--color-text-subtle);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.top-likes__rank--podium {
  color: var(--color-chart-3);
  font-weight: var(--font-weight-semibold);
}

.top-likes__name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  font-weight: var(--font-weight-medium);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.top-likes__score {
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
}

.top-likes__empty {
  margin: 0;
  color: var(--color-text-subtle);
}
</style>
