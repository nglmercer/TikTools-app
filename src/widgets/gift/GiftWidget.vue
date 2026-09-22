<script setup lang="ts">
import WidgetStage from '../sdk/WidgetStage.vue';
import { useWidgetText } from '../sdk/text.ts';
const text = useWidgetText('gift');
import { computed, ref, watch } from 'vue';
import type { GiftWidgetSettings } from '../shared/config.ts';
import type { GiftAlertView } from '../shared/gift-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';

type GiftWidgetProps = {
  alert: GiftAlertView | null;
  status: GatewayStatus;
  settings: GiftWidgetSettings;
  debug: boolean;
};

const props = defineProps<GiftWidgetProps>();

const imageFailed = ref(false);
const avatarFailed = ref(false);

// A new alert key resets the image state; count updates on the same streak
// must not restart the artwork.
watch(
  () => props.alert?.key,
  () => {
    imageFailed.value = false;
    avatarFailed.value = false;
  },
);

const showArtwork = computed(() => props.settings.showImage && !!props.alert?.giftIconUrl && !imageFailed.value);
const showAvatar = computed(() => !!props.alert?.avatarUrl && !avatarFailed.value);
</script>

<template>
  <WidgetStage :debug="props.debug" :status="props.status" class="gift-stage">
    <Transition name="gift" mode="out-in">
      <div v-if="props.alert" :key="props.alert.key" class="gift-card" role="alert">
        <div class="gift-art" aria-hidden="true">
          <img
            v-if="showArtwork"
            :src="props.alert.giftIconUrl as string"
            :alt="props.alert.giftName"
            @error="imageFailed = true"
          />
          <svg
            v-else
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="3" y="8" width="18" height="4" rx="1" />
            <path d="M12 8v13" />
            <path d="M19 12v7a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-7" />
            <path d="M7.5 8a2.5 2.5 0 0 1 0-5C11 3 12 8 12 8s1-5 4.5-5a2.5 2.5 0 0 1 0 5" />
          </svg>
        </div>
        <div class="gift-body">
          <span v-if="text(props.alert.streaking ? 'streakTitle' : 'title', props.alert)" class="gift-kicker">{{ text(props.alert.streaking ? 'streakTitle' : 'title', props.alert) }}</span>
          <span class="gift-sender">
            <img
              v-if="showAvatar"
              class="gift-avatar"
              :src="props.alert.avatarUrl as string"
              :alt="props.alert.displayName"
              referrerpolicy="no-referrer"
              @error="avatarFailed = true"
            />
            <span v-if="text('name', props.alert)" class="gift-name">{{ text('name', props.alert) }}</span>
          </span>
          <span v-if="text('message', props.alert) || (props.settings.showCount && text('count', props.alert))" class="gift-line">
            {{ text('message', props.alert) }}
            <span
              v-if="props.settings.showCount && text('count', props.alert)"
              :key="props.alert.count"
              class="gift-count gift-count-pop"
            >
              {{ text('count', props.alert) }}
            </span>
          </span>
          <span v-if="props.settings.showDiamonds && text('diamonds', props.alert)" class="gift-diamonds">
            {{ text('diamonds', props.alert) }}
          </span>
        </div>
      </div>
    </Transition>
  </WidgetStage>
</template>

<style scoped src="./gift.css"></style>
