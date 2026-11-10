<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, useId, watch } from 'vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconClose from '@/shared/ui/icon/icons/IconClose.vue';
import IconMinus from '@/shared/ui/icon/icons/IconMinus.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
const props = withDefaults(defineProps<{
  title: string;
  moveLabel: string;
  closeLabel: string;
  minimizeLabel: string;
  restoreLabel: string;
  resetLabel: string;
  width?: number;
  activationKey?: number;
}>(), { width: 620, activationKey: 0 });
const emit = defineEmits<{ viewportChange: [width: number] }>();
const open = defineModel<boolean>('open', { default: false });
const panel = ref<HTMLElement | null>(null);
const mover = ref<HTMLButtonElement | null>(null);
const titleId = useId();
const x = ref(24); const y = ref(72); const minimized = ref(false);
let returnFocus: HTMLElement | null = null;
let drag: { pointerId: number; x: number; y: number; left: number; top: number } | null = null;
function clamp(): void {
  x.value = Math.max(0, Math.min(x.value, window.innerWidth - Math.min(props.width, window.innerWidth)));
  const headerHeight = mover.value?.offsetHeight || 44;
  y.value = Math.max(0, Math.min(y.value, window.innerHeight - headerHeight - 16));
}
function stop(): void {
  if (drag && mover.value?.hasPointerCapture?.(drag.pointerId)) mover.value.releasePointerCapture(drag.pointerId);
  drag = null;
}
function start(event: PointerEvent): void {
  if (event.button !== 0) return;
  drag = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, left: x.value, top: y.value };
  mover.value?.setPointerCapture(event.pointerId);
}
function move(event: PointerEvent): void {
  if (!drag || drag.pointerId !== event.pointerId) return;
  x.value = drag.left + event.clientX - drag.x; y.value = drag.top + event.clientY - drag.y; clamp();
}
function key(event: KeyboardEvent): void {
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) return;
  event.preventDefault();
  const step = event.shiftKey ? 4 : 20;
  x.value += event.key === 'ArrowLeft' ? -step : event.key === 'ArrowRight' ? step : 0;
  y.value += event.key === 'ArrowUp' ? -step : event.key === 'ArrowDown' ? step : 0;
  clamp();
}
function reset(): void { x.value = 24; y.value = 72; clamp(); }
watch(() => [open.value, props.activationKey], async () => {
  if (!open.value) { stop(); await nextTick(); if (returnFocus?.isConnected) returnFocus.focus(); return; }
  const active = document.activeElement;
  if (active instanceof HTMLElement && !panel.value?.contains(active)) returnFocus = active;
  minimized.value = false;
  await nextTick(); clamp(); mover.value?.focus();
}, { immediate: true });
function resize(): void { clamp(); emit('viewportChange', window.innerWidth); }
onMounted(() => window.addEventListener('resize', resize));
onUnmounted(() => { stop(); window.removeEventListener('resize', resize); });
</script>
<template>
  <aside
    v-if="open"
    ref="panel"
    class="base-floating-panel"
    :aria-labelledby="titleId"
    :style="{ left: `${x}px`, top: `${y}px`, width: `min(${width}px, 100vw)`, maxHeight: `calc(100dvh - ${y + 8}px)` }"
  >
    <header class="base-floating-panel__header">
      <button
        ref="mover"
        type="button"
        class="base-floating-panel__move"
        :aria-label="moveLabel"
        @pointerdown="start"
        @pointermove="move"
        @pointerup="stop"
        @pointercancel="stop"
        @lostpointercapture="drag = null"
        @keydown="key"
      >
        <span aria-hidden="true">⠿</span><span :id="titleId">{{ title }}</span>
      </button>
      <BaseIconButton
        :label="resetLabel"
        @click="reset"
      >
        <IconRefresh />
      </BaseIconButton>
      <BaseIconButton
        :label="minimized ? restoreLabel : minimizeLabel"
        :pressed="minimized"
        @click="minimized = !minimized"
      >
        <IconPlus v-if="minimized" /><IconMinus v-else />
      </BaseIconButton>
      <BaseIconButton
        :label="closeLabel"
        @click="open = false"
      >
        <IconClose />
      </BaseIconButton>
    </header>
    <div
      v-show="!minimized"
      class="base-floating-panel__body"
    >
      <slot :minimized="minimized" />
    </div>
  </aside>
</template>
<style scoped>
.base-floating-panel { position: fixed; z-index: var(--z-floating); display: flex; flex-direction: column; background: var(--color-bg-panel); color: var(--color-text); border: var(--border-width) solid var(--color-border-strong); border-radius: var(--radius-md); box-shadow: var(--shadow-popup); overflow: hidden; }
.base-floating-panel__header { display: flex; flex: none; align-items: center; gap: var(--space-2); padding: var(--space-2); background: var(--color-bg-panel-header); border-bottom: var(--border-width) solid var(--color-border-subtle); }
.base-floating-panel__move { display: flex; align-items: center; gap: var(--space-3); flex: 1; min-width: 0; min-height: var(--control-height-lg); padding: var(--space-2) var(--space-3); border: 0; background: transparent; color: inherit; font: inherit; text-align: left; touch-action: none; cursor: move; }
.base-floating-panel__move:focus-visible { outline: var(--border-width) solid var(--color-border-focus); box-shadow: var(--focus-ring); }
.base-floating-panel__body { display: flex; flex-direction: column; min-height: 0; overflow: auto; }
@media (any-pointer: coarse) { .base-floating-panel__move { min-height: var(--control-height-touch); } }
</style>
