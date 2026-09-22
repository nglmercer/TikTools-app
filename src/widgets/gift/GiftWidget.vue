<script setup lang="ts">
import WidgetStage from '../sdk/WidgetStage.vue';
import WidgetLayerStack from '../sdk/WidgetLayerStack.vue';
import { canRenderLegacyLayers } from '../sdk/layers.ts';
import { useWidgetText, useWidgetTextOrder, widgetDesignKey } from '../sdk/text.ts';
const text = useWidgetText('gift');
const order = useWidgetTextOrder('gift');
import { computed, inject, ref, watch } from 'vue';
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
const design = inject(widgetDesignKey, undefined);
const layered = computed(() => !!design?.value.layers && !canRenderLegacyLayers('gift', design.value));
const customTextOrder = computed(() => Boolean(design?.value.textOrder?.length));

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
const avatarVisible = computed(() => design?.value.avatar?.visible !== false);
const avatarInitials = computed(() => (props.alert?.displayName || props.alert?.uniqueId || '•')
  .trim().split(/\s+/).slice(0, 2).map((part) => part[0] ?? '').join('').toUpperCase());
</script>

<template>
  <WidgetStage :debug="props.debug" :status="props.status" class="gift-stage">
    <Transition name="gift" mode="out-in">
      <div v-if="props.alert" :key="props.alert.key" class="gift-card" role="alert">
        <WidgetLayerStack v-if="layered" variant="gift" :layers="design?.layers ?? []" :event="props.alert"
          :avatar-url="props.alert.avatarUrl ?? undefined" :art-url="props.settings.showImage ? props.alert.giftIconUrl ?? undefined : undefined" />
        <template v-else>
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
          <span v-if="text(props.alert.streaking ? 'streakTitle' : 'title', props.alert)" class="gift-kicker"
            :style="{ order: order(props.alert.streaking ? 'streakTitle' : 'title') }">{{ text(props.alert.streaking ? 'streakTitle' : 'title', props.alert) }}</span>
          <span class="gift-sender" :style="{ order: order('name') }">
            <span v-if="avatarVisible" class="gift-avatar">
              <img v-if="showAvatar" :src="props.alert.avatarUrl as string"
                :alt="props.alert.displayName" referrerpolicy="no-referrer" @error="avatarFailed = true" />
              <span v-else aria-hidden="true">{{ avatarInitials }}</span>
            </span>
            <span v-if="text('name', props.alert)" class="gift-name">{{ text('name', props.alert) }}</span>
          </span>
          <span v-if="text('message', props.alert) || (!customTextOrder && props.settings.showCount && text('count', props.alert))"
            class="gift-line" :style="{ order: order('message') }">
            {{ text('message', props.alert) }}
            <span
              v-if="!customTextOrder && props.settings.showCount && text('count', props.alert)"
              :key="props.alert.count"
              class="gift-count gift-count-pop"
            >
              {{ text('count', props.alert) }}
            </span>
          </span>
          <span v-if="customTextOrder && props.settings.showCount && text('count', props.alert)"
            class="gift-count-line" :style="{ order: order('count') }">
            <span :key="props.alert.count" class="gift-count gift-count-pop">{{ text('count', props.alert) }}</span>
          </span>
          <span v-if="props.settings.showDiamonds && text('diamonds', props.alert)" class="gift-diamonds"
            :style="{ order: order('diamonds') }">
            {{ text('diamonds', props.alert) }}
          </span>
        </div>
        </template>
      </div>
    </Transition>
  </WidgetStage>
</template>

<style scoped src="./gift.css"></style>
