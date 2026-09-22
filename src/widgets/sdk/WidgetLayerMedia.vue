<script setup lang="ts">
import { computed, ref } from 'vue';
import type { WidgetLayer } from './template.ts';
import type { WidgetTextEvent } from './text.ts';

const props = defineProps<{
  layer: WidgetLayer;
  variant: 'follow' | 'share' | 'subscribe' | 'gift' | 'chat';
  event: WidgetTextEvent | null;
  avatarUrl?: string;
  artUrl?: string;
}>();

const failedUrl = ref('');
const imageUrl = computed(() => {
  const url = props.layer.kind === 'avatar' ? props.avatarUrl : props.artUrl;
  return url && failedUrl.value !== url ? url : undefined;
});
const initials = computed(() => (props.event?.displayName || props.event?.uniqueId || '•').slice(0, 2).toUpperCase());
</script>

<template>
  <div class="widget-layer-media"
    :class="[props.layer.kind === 'art' ? 'is-art' : 'is-avatar', { 'has-image': !!imageUrl, 'is-chat': props.variant === 'chat' }]"
    :style="props.layer.size ? { width: `${props.layer.size}px`, height: `${props.layer.size}px` } : undefined">
    <img v-if="imageUrl" :src="imageUrl" :alt="props.layer.name"
      referrerpolicy="no-referrer" @error="failedUrl = imageUrl || ''" />
    <svg v-else-if="props.layer.kind === 'art'" aria-hidden="true" viewBox="0 0 24 24" fill="none"
      stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3" y="8" width="18" height="4" rx="1" />
      <path d="M12 8v13M19 12v7a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-7M7.5 8a2.5 2.5 0 0 1 0-5C11 3 12 8 12 8s1-5 4.5-5a2.5 2.5 0 0 1 0 5" />
    </svg>
    <svg v-else-if="props.variant === 'follow'" aria-hidden="true" viewBox="0 0 24 24" fill="none"
      stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M19 8v6M22 11h-6" />
      <circle cx="9" cy="7" r="4" />
    </svg>
    <svg v-else-if="props.variant === 'share'" aria-hidden="true" viewBox="0 0 24 24" fill="none"
      stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="18" cy="5" r="3" /><circle cx="6" cy="12" r="3" /><circle cx="18" cy="19" r="3" />
      <path d="M8.6 13.5l6.8 4M15.4 6.5l-6.8 4" />
    </svg>
    <svg v-else-if="props.variant === 'subscribe'" aria-hidden="true" viewBox="0 0 24 24" fill="none"
      stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
    </svg>
    <span v-else aria-hidden="true">{{ initials }}</span>
  </div>
</template>

<style>
.widget-layer-media { display: grid; flex: none; place-items: center; box-sizing: border-box; width: var(--widget-avatar-size, 56px); height: var(--widget-avatar-size, 56px); overflow: hidden; border-radius: var(--widget-avatar-radius, 50%); background: var(--widget-accent, #22c55e); color: #111; font-size: 17px; font-weight: 800; }
.widget-layer-media.has-image.is-avatar { background: transparent; border: var(--widget-avatar-borderWidth, 2px) solid var(--widget-avatar-borderColor, var(--widget-accent, #22c55e)); }
.widget-layer-media.is-art { width: 88px; height: 88px; border-radius: 14px; background: #1e1e26; }
.widget-layer-media.is-art svg { color: var(--widget-accent, #f5c518); }
.widget-layer-media.is-chat.is-avatar { background: linear-gradient(135deg, #ff0050, #00f2fe); color: #fff; font-size: 10px; }
.widget-layer-media svg { width: 50%; height: 50%; }
.widget-layer-media img { width: 100%; height: 100%; object-fit: cover; }
</style>
