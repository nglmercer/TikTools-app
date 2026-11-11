<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { overlayBoxStyle, overlayFrameStyle } from '../../../../shared/cloud-overlay-geometry.ts';
import type { OverlayDocument, OverlayElement } from '../../../../shared/cloud-overlay-schema.ts';

const props = defineProps<{ document: OverlayDocument }>();
const { t } = useI18n();

const frame = computed(() => overlayFrameStyle(props.document));

/** Element geometry comes from the shared overlay layout, so the preview matches OBS. */
function box(element: OverlayElement): Record<string, string> {
  return { ...overlayBoxStyle(props.document.canvas, element) };
}

/** Text shows its static value, or a sample for a live field. Art and avatars are placeholders. */
function label(element: OverlayElement): string {
  if (element.type === 'text') {
    return element.source.kind === 'static' ? element.source.text : `{${element.source.field}}`;
  }
  return element.type === 'avatar' ? t('cloudOverlays.preview.avatar') : t('cloudOverlays.preview.art');
}
</script>

<template>
  <div
    class="overlay-preview"
    :style="frame"
    role="img"
    :aria-label="t('cloudOverlays.preview.label')"
  >
    <div
      v-for="element in props.document.elements"
      :key="element.id"
      class="overlay-preview__box"
      :class="`overlay-preview__box--${element.type}`"
      :style="box(element)"
    >
      {{ label(element) }}
    </div>
  </div>
</template>

<style scoped>
.overlay-preview { position:relative; width:100%; max-width:640px; background:var(--color-surface-raised); border:1px solid var(--color-border); border-radius:var(--radius-md); overflow:hidden; }
.overlay-preview__box { position:absolute; display:flex; align-items:center; justify-content:center; padding:2px 4px; overflow:hidden; font-size:clamp(9px, 1.4vw, 14px); border:1px dashed var(--color-border); color:var(--color-text); overflow-wrap:anywhere; }
.overlay-preview__box--avatar { border-radius:50%; }
.overlay-preview__box--art { background:var(--color-surface); }
</style>
