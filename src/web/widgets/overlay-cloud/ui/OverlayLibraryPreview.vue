<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, useTemplateRef } from 'vue';
import WidgetHost from '../../../../widgets/sdk/WidgetHost.vue';
import { widgetTemplates, type WidgetKind } from '../../../../widgets/sdk/templates.ts';
import type { WidgetStyle } from '../../../../widgets/sdk/template.ts';

defineProps<{ kind: WidgetKind; design?: WidgetStyle; label: string }>();
const frame = useTemplateRef('frame');
const scale = ref(0.5);
let observer: ResizeObserver | undefined;
onMounted(() => {
  observer = new ResizeObserver(([entry]) => { if (entry) scale.value = entry.contentRect.width / 640; });
  if (frame.value) observer.observe(frame.value);
});
onBeforeUnmount(() => observer?.disconnect());
</script>

<template>
  <div
    ref="frame"
    class="library-preview"
    role="img"
    :aria-label="label"
  >
    <div
      class="library-preview__stage"
      :style="{ transform: `scale(${scale})` }"
      aria-hidden="true"
      inert
    >
      <WidgetHost
        :template="widgetTemplates[kind]"
        mode="preview"
        :design="design"
      />
    </div>
  </div>
</template>

<style scoped>
.library-preview { position:relative; width:100%; aspect-ratio:16 / 9; overflow:hidden; background-color:var(--color-bg-app); background-image:conic-gradient(var(--color-bg-panel) 25%, transparent 0 50%, var(--color-bg-panel) 0 75%, transparent 0); background-size:20px 20px; }
.library-preview__stage { width:640px; height:360px; transform-origin:top left; pointer-events:none; }
</style>
