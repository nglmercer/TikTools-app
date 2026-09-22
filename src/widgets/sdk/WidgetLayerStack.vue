<script setup lang="ts">
import { computed } from 'vue';
import { interpolateText, type WidgetTextEvent } from './text.ts';
import type { WidgetLayer } from './template.ts';
import WidgetLayerMedia from './WidgetLayerMedia.vue';

const props = defineProps<{
  layers: readonly WidgetLayer[];
  event: WidgetTextEvent | null;
  variant: 'follow' | 'share' | 'subscribe' | 'gift' | 'chat';
  avatarUrl?: string;
  artUrl?: string;
}>();

type LayerRow = { id: string; media: WidgetLayer[]; text: WidgetLayer[] };
type MediaPlacement = NonNullable<WidgetLayer['placement']>;

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

function mediaAt(row: LayerRow, placement: MediaPlacement): WidgetLayer[] {
  return row.media.filter((layer) => (layer.placement ?? 'left') === placement);
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

</script>

<template>
  <div class="widget-layer-stack" :class="`is-${props.variant}`">
    <div v-for="row in rows" :key="row.id" class="widget-layer-row">
      <div v-if="mediaAt(row, 'top').length" class="widget-layer-vertical-media">
        <WidgetLayerMedia v-for="layer in mediaAt(row, 'top')" :key="layer.id" :layer="layer"
          :variant="props.variant" :event="props.event" :avatar-url="props.avatarUrl" :art-url="props.artUrl" />
      </div>
      <div v-if="mediaAt(row, 'left').length || row.text.length || mediaAt(row, 'right').length" class="widget-layer-main">
        <WidgetLayerMedia v-for="layer in mediaAt(row, 'left')" :key="layer.id" :layer="layer"
          :variant="props.variant" :event="props.event" :avatar-url="props.avatarUrl" :art-url="props.artUrl" />
        <div v-if="row.text.length" class="widget-layer-text-group">
          <div v-for="layer in row.text" :key="layer.id" class="widget-layer-text"
            :class="layer.field ? `is-${layer.field}` : 'is-custom'"
            :style="{
              color: layer.color || undefined,
              fontSize: layer.fontSize ? `${layer.fontSize}px` : undefined,
              fontWeight: layer.fontWeight || undefined,
            }">{{ content(layer) }}</div>
        </div>
        <WidgetLayerMedia v-for="layer in mediaAt(row, 'right')" :key="layer.id" :layer="layer"
          :variant="props.variant" :event="props.event" :avatar-url="props.avatarUrl" :art-url="props.artUrl" />
      </div>
      <div v-if="mediaAt(row, 'bottom').length" class="widget-layer-vertical-media">
        <WidgetLayerMedia v-for="layer in mediaAt(row, 'bottom')" :key="layer.id" :layer="layer"
          :variant="props.variant" :event="props.event" :avatar-url="props.avatarUrl" :art-url="props.artUrl" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.widget-layer-stack { display: flex; flex-direction: column; gap: 4px; width: max-content; max-width: 100%; min-width: 0; text-align: var(--widget-align, left); }
.widget-layer-row { display: flex; flex-direction: column; align-items: stretch; gap: var(--widget-gap, 16px); width: max-content; max-width: 100%; min-width: 0; }
.widget-layer-main { display: flex; align-self: center; align-items: center; gap: var(--widget-gap, 16px); width: max-content; max-width: 100%; min-width: 0; }
.widget-layer-vertical-media { display: flex; justify-content: center; gap: 8px; }
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
</style>
