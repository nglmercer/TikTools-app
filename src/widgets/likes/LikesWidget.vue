<script setup lang="ts">
import WidgetStage from '../sdk/WidgetStage.vue';
import { useWidgetText } from '../sdk/text.ts';
const text = useWidgetText('likes');
import { computed, ref } from 'vue';
import type { LikesWidgetSettings } from '../shared/config.ts';
import type { LikeBoardEntry } from '../shared/likes-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';

type LikesWidgetProps = {
  entries: LikeBoardEntry[];
  status: GatewayStatus;
  settings: LikesWidgetSettings;
  debug: boolean;
  degraded?: boolean;
};

const props = defineProps<LikesWidgetProps>();

const failedAvatars = ref<Record<string, boolean>>({});

const title = computed(() => text('title', null));

function showAvatar(entry: LikeBoardEntry): boolean {
  return !!entry.avatarUrl && !failedAvatars.value[entry.userId || entry.uniqueId];
}

function initialsFor(entry: LikeBoardEntry): string {
  const clean = entry.uniqueId.replace(/^@+/, '');
  return clean.slice(0, 2).toUpperCase() || '•';
}

function rowText(field: 'name' | 'count', entry: LikeBoardEntry): string {
  if (field === 'name') return text('name', { displayName: entry.displayName, uniqueId: entry.uniqueId });
  return text('count', { displayName: entry.displayName, uniqueId: entry.uniqueId, count: entry.total });
}
</script>

<template>
  <WidgetStage :debug="props.debug" :status="props.status" class="likes-stage">
    <div class="likes-board" role="status" aria-label="Top likes">
      <p v-if="title" class="likes-title">{{ title }}</p>
      <p v-if="props.degraded" class="likes-empty">Ranking incomplete</p>
      <ol class="likes-list">
        <li
          v-for="(entry, index) in props.entries"
          :key="entry.userId || entry.uniqueId || entry.displayName"
          class="likes-row"
        >
          <span class="likes-rank" aria-hidden="true">{{ index + 1 }}</span>
          <span class="likes-avatar" aria-hidden="true">
            <img
              v-if="showAvatar(entry)"
              :src="entry.avatarUrl as string"
              :alt="entry.displayName"
              referrerpolicy="no-referrer"
              loading="lazy"
              @error="failedAvatars[entry.userId || entry.uniqueId] = true"
            />
            <span v-else>{{ initialsFor(entry) }}</span>
          </span>
          <span class="likes-name">{{ rowText('name', entry) }}</span>
          <span
            v-if="props.settings.showUniqueId && entry.uniqueId"
            class="likes-handle"
          >{{ entry.uniqueId }}</span>
          <span class="likes-count">{{ rowText('count', entry) }}</span>
        </li>
      </ol>
      <p v-if="!props.entries.length" class="likes-empty">No likes yet</p>
    </div>
  </WidgetStage>
</template>

<style scoped src="./likes.css"></style>
