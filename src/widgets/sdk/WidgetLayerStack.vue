<script setup lang="ts">
import { ref } from 'vue';
import { interpolateText, type WidgetTextEvent } from './text.ts';
import type { WidgetLayer } from './template.ts';

const props = defineProps<{
  layers: readonly WidgetLayer[];
  event: WidgetTextEvent | null;
  avatarUrl?: string;
  artUrl?: string;
}>();

const failed = ref<Record<string, string>>({});

function imageUrl(layer: WidgetLayer): string | undefined {
  const url = layer.kind === 'avatar' ? props.avatarUrl : props.artUrl;
  return url && failed.value[layer.id] !== url ? url : undefined;
}

function content(layer: WidgetLayer): string {
  const event = props.event;
  return interpolateText(layer.text ?? '', {
    name: event?.displayName ?? '',
    username: event?.uniqueId ?? '',
    gift: event?.giftName ?? '',
    count: event?.count?.toLocaleString('en-US') ?? '',
    diamonds: event?.totalDiamonds?.toLocaleString('en-US') ?? '',
    message: event?.text ?? '',
  });
}

function initials(): string {
  return (props.event?.displayName || props.event?.uniqueId || '•').slice(0, 2).toUpperCase();
}
</script>

<template>
  <div class="widget-layer-stack">
    <template v-for="layer in props.layers" :key="layer.id">
      <div v-if="layer.kind === 'text' && content(layer)" class="widget-layer-text"
        :class="layer.field ? `is-${layer.field}` : 'is-custom'"
        :style="{
          color: layer.color || undefined,
          fontSize: layer.fontSize ? `${layer.fontSize}px` : undefined,
          fontWeight: layer.fontWeight || undefined,
        }">{{ content(layer) }}</div>
      <div v-else-if="layer.kind !== 'text'" class="widget-layer-media"
        :class="layer.kind === 'art' ? 'is-art' : 'is-avatar'"
        :style="layer.size ? { width: `${layer.size}px`, height: `${layer.size}px` } : undefined">
        <img v-if="imageUrl(layer)" :src="imageUrl(layer)" :alt="layer.name"
          referrerpolicy="no-referrer" @error="failed[layer.id] = imageUrl(layer) || ''" />
        <span v-else aria-hidden="true">{{ layer.kind === 'art' ? '🎁' : initials() }}</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.widget-layer-stack { display: flex; flex-direction: column; align-items: var(--widget-align-items, stretch); gap: 4px; width: 100%; min-width: 0; text-align: var(--widget-align, left); }
.widget-layer-text { max-width: 100%; color: var(--widget-textColor, #f5f5f7); font-size: 15px; line-height: 1.25; overflow-wrap: anywhere; white-space: pre-wrap; }
.widget-layer-text.is-title, .widget-layer-text.is-streakTitle { color: var(--widget-badge-color, var(--widget-accent, #22c55e)); font-size: var(--widget-badge-size, 11px); font-weight: var(--widget-badge-weight, 700); letter-spacing: var(--widget-badge-spacing, 0.14em); text-transform: uppercase; }
.widget-layer-text.is-name { font-size: 27px; font-weight: 800; }
.widget-layer-text.is-handle, .widget-layer-text.is-diamonds { color: #b9b9c4; font-size: 14px; }
.widget-layer-text.is-count { color: var(--widget-accent, #22c55e); font-size: 20px; font-weight: 700; }
.widget-layer-media { display: grid; flex: none; place-items: center; width: var(--widget-avatar-size, 56px); height: var(--widget-avatar-size, 56px); overflow: hidden; border-radius: var(--widget-avatar-radius, 50%); background: var(--widget-accent, #22c55e); color: #111; font-size: 17px; font-weight: 800; }
.widget-layer-media.is-art { width: 88px; height: 88px; border-radius: 14px; background: #1e1e26; }
.widget-layer-media img { width: 100%; height: 100%; object-fit: cover; }
</style>
