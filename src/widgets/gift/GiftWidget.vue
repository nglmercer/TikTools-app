<script setup lang="ts">
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
const formattedDiamonds = computed(() => (props.alert ? props.alert.totalDiamonds.toLocaleString('en-US') : ''));
const formattedCount = computed(() => (props.alert ? `×${props.alert.count.toLocaleString('en-US')}` : ''));
</script>

<template>
  <div class="gift-stage">
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
          <span class="gift-kicker">{{ props.alert.streaking ? 'Gift streak' : 'Gift received' }}</span>
          <span class="gift-sender">
            <img
              v-if="showAvatar"
              class="gift-avatar"
              :src="props.alert.avatarUrl as string"
              :alt="props.alert.displayName"
              referrerpolicy="no-referrer"
              @error="avatarFailed = true"
            />
            <span class="gift-name">{{ props.alert.displayName }}</span>
          </span>
          <span class="gift-line">
            sent {{ props.alert.giftName }}
            <span
              v-if="props.settings.showCount"
              :key="props.alert.count"
              class="gift-count gift-count-pop"
            >
              {{ formattedCount }}
            </span>
          </span>
          <span v-if="props.settings.showDiamonds" class="gift-diamonds">
            {{ formattedDiamonds }} diamonds
          </span>
        </div>
      </div>
    </Transition>
    <div v-if="props.debug" class="widget-debug-status">gateway: {{ props.status }}</div>
  </div>
</template>
