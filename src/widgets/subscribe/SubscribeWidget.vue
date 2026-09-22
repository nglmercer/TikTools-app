<script setup lang="ts">
import WidgetStage from '../sdk/WidgetStage.vue';
import { useWidgetText } from '../sdk/text.ts';
const text = useWidgetText('subscribe');
import { computed, ref, watch } from 'vue';
import type { SubscribeWidgetSettings } from '../shared/config.ts';
import type { SubscribeAlert } from '../shared/subscribe-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';

type SubscribeWidgetProps = {
  alert: SubscribeAlert | null;
  status: GatewayStatus;
  settings: SubscribeWidgetSettings;
  debug: boolean;
};

const props = defineProps<SubscribeWidgetProps>();

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
    class="subscribe-stage"
    :style="{
      '--subscribe-enter-ms': `${props.settings.enterMs}ms`,
      '--subscribe-exit-ms': `${props.settings.exitMs}ms`,
    }"
  >
    <Transition name="subscribe" mode="out-in" :duration="{ enter: props.settings.enterMs, leave: props.settings.exitMs }">
      <div v-if="props.alert" :key="props.alert.id" class="subscribe-card" role="alert">
        <div :class="['subscribe-badge', { 'has-avatar': showAvatar }]" aria-hidden="true">
          <img
            v-if="showAvatar"
            class="subscribe-avatar"
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
            <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
          </svg>
        </div>
        <div class="subscribe-body">
          <span v-if="text('title', props.alert)" class="subscribe-kicker">{{ text('title', props.alert) }}</span>
          <span v-if="text('name', props.alert)" class="subscribe-name">{{ text('name', props.alert) }}</span>
          <span v-if="props.settings.showUniqueId && props.alert.uniqueId && text('handle', props.alert)" class="subscribe-handle">
            {{ text('handle', props.alert) }}
          </span>
          <span v-if="text('message', props.alert)" class="subscribe-action">{{ text('message', props.alert) }}</span>
        </div>
      </div>
    </Transition>
  </WidgetStage>
</template>

<style scoped src="./subscribe.css"></style>
