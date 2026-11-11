<script setup lang="ts">
import { computed } from 'vue';
import { overlayBoxStyle, overlayFrameStyle } from '../../shared/cloud-overlay-geometry.ts';
import type { OverlayDocument, OverlayElement } from '../../shared/cloud-overlay-schema.ts';
import type { LiveFieldValues } from '../shared/cloud-overlay-fields.ts';

const props = defineProps<{ document: OverlayDocument | null; values: LiveFieldValues }>();

const frame = computed(() => overlayFrameStyle(props.document));

/** Element geometry comes from the shared overlay layout, so OBS matches the editor preview. */
function box(element: OverlayElement): Record<string, string> {
  return { ...overlayBoxStyle(props.document?.canvas, element) };
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
