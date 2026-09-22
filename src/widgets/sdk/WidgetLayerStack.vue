<script setup lang="ts">
import { computed, ref } from 'vue';
import { interpolateText, type WidgetTextEvent } from './text.ts';
import type { WidgetLayer } from './template.ts';

const props = defineProps<{
  layers: readonly WidgetLayer[];
  event: WidgetTextEvent | null;
  variant: 'follow' | 'share' | 'subscribe' | 'gift' | 'chat';
  avatarUrl?: string;
  artUrl?: string;
}>();

const failed = ref<Record<string, string>>({});

type LayerRow = { id: string; media: WidgetLayer[]; text: WidgetLayer[] };

// A media layer starts a new row. Text after it stays beside it, while text
// before it forms its own full-width row. This keeps the original avatar and
// text arrangement as layers are inserted or removed.
const rows = computed<LayerRow[]>(() => {
  const result: LayerRow[] = [];
  let row: LayerRow | undefined;
  for (const layer of props.layers) {
    if (layer.kind === 'text') {
      if (!content(layer)) continue;
      if (!row) {
        row = { id: layer.id, media: [], text: [] };
        result.push(row);
      }
      row.text.push(layer);
    } else {
      if (!row || row.text.length > 0) {
        row = { id: layer.id, media: [], text: [] };
        result.push(row);
      }
      row.media.push(layer);
    }
  }
  return result;
});

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
  <div class="widget-layer-stack" :class="`is-${props.variant}`">
    <div v-for="row in rows" :key="row.id" class="widget-layer-row">
      <div v-if="row.media.length" class="widget-layer-media-group">
        <div v-for="layer in row.media" :key="layer.id" class="widget-layer-media"
          :class="[layer.kind === 'art' ? 'is-art' : 'is-avatar', { 'has-image': !!imageUrl(layer) }]"
          :style="layer.size ? { width: `${layer.size}px`, height: `${layer.size}px` } : undefined">
          <img v-if="imageUrl(layer)" :src="imageUrl(layer)" :alt="layer.name"
            referrerpolicy="no-referrer" @error="failed[layer.id] = imageUrl(layer) || ''" />
          <svg v-else-if="layer.kind === 'art'" aria-hidden="true" viewBox="0 0 24 24" fill="none"
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
          <span v-else aria-hidden="true">{{ initials() }}</span>
        </div>
      </div>
      <div v-if="row.text.length" class="widget-layer-text-group">
        <div v-for="layer in row.text" :key="layer.id" class="widget-layer-text"
          :class="layer.field ? `is-${layer.field}` : 'is-custom'"
          :style="{
            color: layer.color || undefined,
            fontSize: layer.fontSize ? `${layer.fontSize}px` : undefined,
            fontWeight: layer.fontWeight || undefined,
          }">{{ content(layer) }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.widget-layer-stack { display: flex; flex-direction: column; gap: 4px; width: max-content; max-width: 100%; min-width: 0; text-align: var(--widget-align, left); }
.widget-layer-row { display: flex; align-items: center; gap: var(--widget-gap, 16px); width: max-content; max-width: 100%; min-width: 0; }
.widget-layer-media-group { display: flex; align-items: center; flex: none; gap: 8px; }
.widget-layer-text-group { display: flex; flex: 0 1 auto; flex-direction: column; align-items: var(--widget-align-items, stretch); gap: 2px; min-width: 0; }
.widget-layer-text { max-width: 100%; color: var(--widget-textColor, #f5f5f7); font-size: 15px; line-height: 1.25; overflow-wrap: anywhere; white-space: pre-wrap; }
.widget-layer-text.is-title, .widget-layer-text.is-streakTitle { color: var(--widget-badge-color, var(--widget-accent, #22c55e)); font-size: var(--widget-badge-size, 11px); font-weight: var(--widget-badge-weight, 700); letter-spacing: var(--widget-badge-spacing, 0.14em); text-transform: uppercase; }
.widget-layer-text.is-name { font-size: 27px; font-weight: 800; }
.widget-layer-text.is-handle, .widget-layer-text.is-diamonds { color: #b9b9c4; font-size: 14px; }
.widget-layer-text.is-count { color: var(--widget-accent, #22c55e); font-size: 20px; font-weight: 700; }
.widget-layer-stack.is-follow .widget-layer-text.is-name,
.widget-layer-stack.is-share .widget-layer-text.is-name,
.widget-layer-stack.is-subscribe .widget-layer-text.is-name { font-size: 30px; line-height: 1.12; letter-spacing: -0.01em; }
.widget-layer-stack.is-follow .widget-layer-text.is-message,
.widget-layer-stack.is-share .widget-layer-text.is-message,
.widget-layer-stack.is-subscribe .widget-layer-text.is-message { font-size: 14px; color: #e8e8ec; }
.widget-layer-stack.is-gift .widget-layer-text.is-name { font-size: 26px; line-height: 1.15; }
.widget-layer-stack.is-gift .widget-layer-text.is-message { font-size: 20px; font-weight: 700; color: #e8e8ec; }
.widget-layer-stack.is-chat .widget-layer-text.is-name { font-size: 13px; color: var(--widget-accent, #f5f5f7); }
.widget-layer-stack.is-chat .widget-layer-text.is-message { font-size: 15px; line-height: 1.35; }
.widget-layer-stack.is-chat .widget-layer-media.is-avatar { background: linear-gradient(135deg, #ff0050, #00f2fe); color: #fff; font-size: 10px; }
.widget-layer-media { display: grid; flex: none; place-items: center; box-sizing: border-box; width: var(--widget-avatar-size, 56px); height: var(--widget-avatar-size, 56px); overflow: hidden; border-radius: var(--widget-avatar-radius, 50%); background: var(--widget-accent, #22c55e); color: #111; font-size: 17px; font-weight: 800; }
.widget-layer-media.has-image.is-avatar { background: transparent; border: var(--widget-avatar-borderWidth, 2px) solid var(--widget-avatar-borderColor, var(--widget-accent, #22c55e)); }
.widget-layer-media.is-art { width: 88px; height: 88px; border-radius: 14px; background: #1e1e26; }
.widget-layer-media svg { width: 50%; height: 50%; }
.widget-layer-media img { width: 100%; height: 100%; object-fit: cover; }
</style>
