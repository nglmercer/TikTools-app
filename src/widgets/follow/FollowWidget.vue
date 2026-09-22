<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { FollowWidgetSettings } from '../shared/config.ts';
import type { FollowAlert } from '../shared/follow-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';

type FollowWidgetProps = {
  alert: FollowAlert | null;
  status: GatewayStatus;
  settings: FollowWidgetSettings;
  debug: boolean;
};

const props = defineProps<FollowWidgetProps>();

const avatarFailed = ref(false);

watch(
  () => props.alert?.id,
  () => {
    avatarFailed.value = false;
  },
);

const showAvatar = computed(() => !!props.alert?.avatarUrl && !avatarFailed.value);
</script>

<template>
  <div
    class="follow-stage"
    :style="{
      '--follow-enter-ms': `${props.settings.enterMs}ms`,
      '--follow-exit-ms': `${props.settings.exitMs}ms`,
    }"
  >
    <Transition name="follow" mode="out-in" :duration="{ enter: props.settings.enterMs, leave: props.settings.exitMs }">
      <div v-if="props.alert" :key="props.alert.id" class="follow-card" role="alert">
        <div :class="['follow-badge', { 'has-avatar': showAvatar }]" aria-hidden="true">
          <img
            v-if="showAvatar"
            class="follow-avatar"
            :src="props.alert.avatarUrl as string"
            :alt="props.alert.displayName"
            referrerpolicy="no-referrer"
            @error="avatarFailed = true"
          />
          <svg
            v-else
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M19 8v6" />
            <path d="M22 11h-6" />
          </svg>
        </div>
        <div class="follow-body">
          <span class="follow-kicker">New follower</span>
          <span class="follow-name">{{ props.alert.displayName }}</span>
          <span v-if="props.settings.showUniqueId && props.alert.uniqueId" class="follow-handle">
            {{ props.alert.uniqueId }}
          </span>
          <span class="follow-action">just followed!</span>
        </div>
      </div>
    </Transition>
    <div v-if="props.debug" class="widget-debug-status">gateway: {{ props.status }}</div>
  </div>
</template>
