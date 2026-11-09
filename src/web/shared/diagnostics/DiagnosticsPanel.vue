<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import DiagnosticsLog from './DiagnosticsLog.vue';
import { diagnosticView, type DiagnosticSource } from './log.ts';
import { useDiagnosticNavigation } from './navigation.ts';
const { t } = useI18n();
const navigation = useDiagnosticNavigation();
const { open } = diagnosticView;
const panel = ref<HTMLElement | null>(null);
watch(() => [open.value, diagnosticView.request.value], async () => { if (!open.value) return; minimized.value = false; await nextTick(); panel.value?.querySelector<HTMLButtonElement>('button')?.focus(); });
const x = ref(24); const y = ref(72); const minimized = ref(false);
let drag: { x: number; y: number; left: number; top: number } | null = null;
function clamp(): void { x.value = Math.max(0, Math.min(x.value, window.innerWidth - Math.min(620, window.innerWidth))); y.value = Math.max(0, Math.min(y.value, window.innerHeight - 100)); }
function start(event: PointerEvent): void { drag = { x:event.clientX, y:event.clientY, left:x.value, top:y.value }; (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId); }
function move(event: PointerEvent): void { if (!drag) return; x.value = drag.left + event.clientX - drag.x; y.value = drag.top + event.clientY - drag.y; clamp(); }
function key(event: KeyboardEvent): void { if (!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(event.key)) return; event.preventDefault(); x.value += event.key === 'ArrowLeft' ? -20 : event.key === 'ArrowRight' ? 20 : 0; y.value += event.key === 'ArrowUp' ? -20 : event.key === 'ArrowDown' ? 20 : 0; clamp(); }
function resize(): void { clamp(); if (open.value && window.innerWidth < 768) navigation.open((diagnosticView.source.value || undefined) as DiagnosticSource | undefined, diagnosticView.entityId.value); }
onMounted(() => window.addEventListener('resize', resize));
onUnmounted(() => window.removeEventListener('resize', resize));
</script>
<template>
  <aside
    v-if="open"
    ref="panel"
    class="diagnostic-panel"
    :style="{ left: `${x}px`, top: `${y}px`, maxHeight: `calc(100dvh - ${y + 8}px)` }"
    :aria-label="t('diagnostics.title')"
  >
    <header>
      <button
        class="diagnostic-panel__drag"
        :aria-label="t('diagnostics.move')"
        @pointerdown="start"
        @pointermove="move"
        @pointerup="drag = null"
        @pointercancel="drag = null"
        @keydown="key"
      >
        ⠿ {{ t('diagnostics.title') }}
      </button>
      <button
        :aria-label="t('diagnostics.reset')"
        @click="x = 24; y = 72; clamp()"
      >
        ↺
      </button>
      <button
        :aria-label="t(minimized ? 'diagnostics.restore' : 'diagnostics.minimize')"
        @click="minimized = !minimized"
      >
        {{ minimized ? '+' : '−' }}
      </button>
      <button
        :aria-label="t('diagnostics.close')"
        @click="navigation.close"
      >
        ×
      </button>
    </header>
    <DiagnosticsLog v-if="!minimized" />
  </aside>
</template>
<style scoped>
.diagnostic-panel { position:fixed; z-index:100; width:min(620px,100vw); display:flex; flex-direction:column; background:var(--color-bg, #161922); color:var(--color-text); border:1px solid var(--color-border, #454955); border-radius:12px; box-shadow:0 16px 60px #0008; overflow:hidden; }
header { display:flex; padding:8px; gap:6px; border-bottom:1px solid var(--color-border, #454955); }
header button { color:inherit; background:transparent; border:0; padding:8px; font:inherit; cursor:pointer; }
header button:focus-visible { outline:2px solid #a69bff; }
.diagnostic-panel__drag { flex:1; text-align:left; touch-action:none; cursor:move; }
</style>
