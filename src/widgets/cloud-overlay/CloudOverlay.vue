<script setup lang="ts">
import { computed } from 'vue';
import type { OverlayDocument, OverlayElement } from '../../web/entities/overlay/lib/cloud-overlay-schema.ts';
import type { LiveFieldValues } from '../shared/cloud-overlay-fields.ts';

const props = defineProps<{ document: OverlayDocument | null; values: LiveFieldValues }>();

const frame = computed(() =>
  props.document ? { aspectRatio: `${props.document.canvas.width} / ${props.document.canvas.height}` } : {},
);

/** Position and size as percentages of the canvas, so the layout scales to any source size. */
function box(element: OverlayElement): Record<string, string> {
  const canvas = props.document?.canvas;
  if (!canvas) return {};
  return {
    left: `${(element.x / canvas.width) * 100}%`,
    top: `${(element.y / canvas.height) * 100}%`,
    width: `${(element.width / canvas.width) * 100}%`,
    height: `${(element.height / canvas.height) * 100}%`,
    opacity: String(element.opacity ?? 1),
  };
}

/** Static text as written; a live field shows its latest value and nothing until one arrives. */
function text(element: OverlayElement): string {
  if (element.type !== 'text') return '';
  return element.source.kind === 'static' ? element.source.text : (props.values[element.source.field] ?? '');
}

/** The avatar stands in with the viewer's initial; the real picture needs an image source OBS cannot read. */
function initial(): string {
  return (props.values['viewer.name'] ?? '').trim().charAt(0).toUpperCase();
}
</script>

<template>
  <div
    v-if="document"
    class="cloud-overlay"
    :style="frame"
  >
    <div
      v-for="element in document.elements"
      :key="element.id"
      class="cloud-overlay__box"
      :class="`cloud-overlay__box--${element.type}`"
      :style="box(element)"
    >
      <template v-if="element.type === 'text'">
        {{ text(element) }}
      </template>
      <template v-else-if="element.type === 'avatar'">
        {{ initial() }}
      </template>
    </div>
  </div>
</template>
