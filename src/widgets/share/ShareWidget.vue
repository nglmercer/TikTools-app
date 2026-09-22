<script setup lang="ts">
import WidgetStage from '../sdk/WidgetStage.vue';
import { computed, ref, watch } from 'vue';
import type { ShareWidgetSettings } from '../shared/config.ts';
import type { ShareAlert } from '../shared/share-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';

type ShareWidgetProps = {
  alert: ShareAlert | null;
  status: GatewayStatus;
  settings: ShareWidgetSettings;
  debug: boolean;
};

const props = defineProps<ShareWidgetProps>();

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
  <WidgetStage :debug="props.debug" :status="props.status"
    class="share-stage"
    :style="{
      '--share-enter-ms': `${props.settings.enterMs}ms`,
      '--share-exit-ms': `${props.settings.exitMs}ms`,
    }"
  >
    <Transition name="share" mode="out-in" :duration="{ enter: props.settings.enterMs, leave: props.settings.exitMs }">
      <div v-if="props.alert" :key="props.alert.id" class="share-card" role="alert">
        <div :class="['share-badge', { 'has-avatar': showAvatar }]" aria-hidden="true">
          <img
            v-if="showAvatar"
            class="share-avatar"
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
            <circle cx="18" cy="5" r="3" />
            <circle cx="6" cy="12" r="3" />
            <circle cx="18" cy="19" r="3" />
            <path d="M8.6 13.5l6.8 4" />
            <path d="M15.4 6.5l-6.8 4" />
          </svg>
        </div>
        <div class="share-body">
          <span class="share-kicker">Shared</span>
          <span class="share-name">{{ props.alert.displayName }}</span>
          <span v-if="props.settings.showUniqueId && props.alert.uniqueId" class="share-handle">
            {{ props.alert.uniqueId }}
          </span>
          <span class="share-action">shared the LIVE!</span>
        </div>
      </div>
    </Transition>
  </WidgetStage>
</template>

<style scoped src="./share.css"></style>
