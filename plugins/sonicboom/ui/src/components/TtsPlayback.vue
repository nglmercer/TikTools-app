<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { PluginBroker } from '@tiktools/plugin-ui';
import TtsIcon from './TtsIcon.vue';
import { t, type Locale } from '../i18n/index.ts';
const props = defineProps<{ broker: PluginBroker; locale: Locale; enabled: boolean; connectionKey: string }>();
const emit = defineEmits<{ toggle: [enabled: boolean] }>();
type Snapshot = { state: 'pending' | 'ready' | 'error'; observedAt?: number; currentId?: string | null; queueLength?: number; playing?: boolean; paused?: boolean; durationSecs?: number | null; positionSecs?: number | null; error?: string };
const snapshot = ref<Snapshot | null>(null);
const error = ref('');
const actionError = ref('');
const pending = ref(false);
const polling = ref(false);
let epoch = 0;
let disposed = false;
let timer: ReturnType<typeof setInterval> | undefined;
let minimumObservation = 0;
const fresh = ref(false);
const current = computed(() => snapshot.value?.currentId);
const canControl = computed(() => props.enabled && fresh.value && snapshot.value?.state === 'ready' && !pending.value);
function seconds(value: number | null | undefined): string { return typeof value === 'number' && Number.isFinite(value) && value >= 0 ? `${value.toFixed(1)} s` : '—'; }
async function refresh() {
  if (disposed || !props.enabled || polling.value || document.hidden) return;
  const generation = epoch;
  polling.value = true;
  try {
    const outcome = await props.broker.executeAction('sonicboom.server.playback-status', {});
    if (!outcome.ok) throw new Error(outcome.error || outcome.summary);
    const value: unknown = JSON.parse(outcome.summary);
    if (!value || typeof value !== 'object' || !['ready', 'error', 'pending'].includes(String((value as Snapshot).state))) throw new Error(t(props.locale, 'ttsPlaybackInvalid'));
    if (disposed || generation !== epoch) return;
    const next = value as Snapshot;
    if (next.state === 'ready' && (!Number.isFinite(next.observedAt) || (next.observedAt ?? 0) < minimumObservation)) return;
    snapshot.value = next;
    fresh.value = next.state === 'ready' && Date.now() - (next.observedAt ?? 0) < 6000;
    error.value = next.state === 'error' ? t(props.locale, `ttsPlaybackError_${next.error}`) : '';
  } catch (failure) {
    if (!disposed && generation === epoch) { snapshot.value = null; fresh.value = false; error.value = failure instanceof Error ? failure.message : String(failure); }
  } finally { if (generation === epoch) polling.value = false; }
}
async function control(action: 'skip' | 'pause' | 'resume' | 'stop') {
  if (!canControl.value) return;
  const generation = epoch;
  pending.value = true; actionError.value = '';
  try {
    const outcome = await props.broker.executeAction(`sonicboom.server.${action}`, {});
    if (!outcome.ok) throw new Error(outcome.error || outcome.summary);
    if (disposed || generation !== epoch) return;
    // Never present the pre-control snapshot as confirmation of a new state.
    minimumObservation = Date.now(); snapshot.value = null; fresh.value = false;
  } catch (failure) {
    if (!disposed && generation === epoch) actionError.value = failure instanceof Error ? failure.message : String(failure);
  } finally { if (generation === epoch) { pending.value = false; void refresh(); } }
}
function reset() {
  epoch++; polling.value = false; pending.value = false; snapshot.value = null; fresh.value = false; error.value = ''; actionError.value = '';
  minimumObservation = Date.now();
  if (props.enabled) void refresh();
}
watch(() => [props.enabled, props.connectionKey], reset);
onMounted(() => {
  if (props.enabled) void refresh();
  timer = setInterval(() => {
    if (snapshot.value?.state === 'ready' && Date.now() - (snapshot.value.observedAt ?? 0) >= 6000) fresh.value = false;
    void refresh();
  }, 2000);
});
onBeforeUnmount(() => { disposed = true; epoch++; clearInterval(timer); });
</script>
<template>
  <section class="tts-card tts-playback" :aria-label="t(locale, 'ttsPlayback')">
    <div class="tts-playback__bar">
      <h4 class="tts-playback__title"><TtsIcon name="audio"/>{{ t(locale, 'ttsPlaybackMinimalTitle') }}</h4>
      <template v-if="enabled">
        <div v-if="snapshot?.state === 'ready' && fresh" class="tts-playback__metadata" role="status" :title="current || undefined">
          <span class="tts-playback__state">{{ t(locale, current ? (snapshot.paused ? 'ttsPlaybackPaused' : snapshot.playing ? 'ttsPlaybackPlaying' : 'ttsPlaybackWaiting') : 'ttsPlaybackIdle') }}</span>
          <span v-if="current" class="tts-playback__duration" :title="t(locale, snapshot.durationSecs == null ? 'ttsPlaybackDurationUnknown' : 'ttsPlaybackDuration')" :aria-label="t(locale, 'ttsPlaybackDuration') + ': ' + seconds(snapshot.durationSecs)"><template v-if="snapshot.positionSecs != null">{{ seconds(snapshot.positionSecs) }} / </template>{{ seconds(snapshot.durationSecs) }}</span>
          <span v-if="snapshot.queueLength" class="tts-playback__queue" :title="t(locale, 'ttsPlaybackQueue')" :aria-label="t(locale, 'ttsPlaybackQueue') + ': ' + snapshot.queueLength">+{{ snapshot.queueLength }}</span>
        </div>
        <span v-else-if="!error" class="tts-playback__state" role="status">{{ t(locale, 'ttsPlaybackConnecting') }}</span>
        <div class="tts-playback__controls">
          <button type="button" class="plg-btn plg-btn--sm" :disabled="!canControl || !current" :title="t(locale, snapshot?.paused ? 'ttsResumeAudio' : 'ttsPauseAudio')" :aria-label="t(locale, snapshot?.paused ? 'ttsResumeAudio' : 'ttsPauseAudio')" @click="control(snapshot?.paused ? 'resume' : 'pause')"><TtsIcon :name="snapshot?.paused ? 'play' : 'pause'"/></button>
          <button type="button" class="plg-btn plg-btn--sm" :disabled="!canControl || !current" :title="t(locale, 'ttsSkipAudio')" :aria-label="t(locale, 'ttsSkipAudio')" @click="control('skip')"><TtsIcon name="skip"/></button>
          <button type="button" class="plg-btn plg-btn--sm" :disabled="!canControl || (!current && !snapshot?.queueLength)" :title="t(locale, 'ttsStopAudio')" :aria-label="t(locale, 'ttsStopAudio')" @click="control('stop')"><TtsIcon name="stop"/></button>
        </div>
      </template>
      <label class="tts-playback__switch" :title="t(locale, 'ttsTrackPlayback') + ': ' + t(locale, 'ttsPlaybackScope')">
        <input type="checkbox" :checked="enabled" :aria-label="t(locale, 'ttsTrackPlayback')" :aria-description="t(locale, 'ttsPlaybackScope')" @change="emit('toggle', ($event.target as HTMLInputElement).checked)">
        <span aria-hidden="true"/>
      </label>
    </div>
    <p v-if="enabled && (error || actionError)" role="alert" class="plg-alert">{{ error || actionError }}</p>
  </section>
</template>
<style scoped>
.tts-playback { gap: var(--space-4); padding-bottom: 0; }
.tts-playback__bar { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-5); min-height: var(--control-height-lg); margin: 0 calc(-1 * var(--space-5)); padding: var(--space-2) var(--space-5); background: var(--color-bg-panel-header); }
.tts-playback > .plg-alert { margin-bottom: var(--space-5); }
.tts-playback__title { display: flex; align-items: center; gap: var(--space-4); margin: 0; font-size: var(--font-size-md); font-weight: var(--font-weight-semibold); }
.tts-playback__title .tts-icon { color: var(--color-text-muted); }
.tts-playback__metadata { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-5); min-width: 0; font-size: var(--font-size-sm); }
.tts-playback__state { color: var(--color-text-muted); font-size: var(--font-size-sm); }
.tts-playback__duration { color: var(--color-text-muted); font-variant-numeric: tabular-nums; }
.tts-playback__queue { color: var(--color-text-subtle); font-variant-numeric: tabular-nums; }
.tts-playback__controls { display: flex; gap: var(--space-2); margin-left: auto; }
.tts-playback__controls .plg-btn { width: var(--control-height-sm); padding: 0; }
.tts-playback__controls .tts-icon { width: 14px; height: 14px; }
/* Mirrors the app's BaseSwitch. */
.tts-playback__switch { position: relative; display: inline-flex; align-items: center; margin-left: auto; cursor: pointer; }
.tts-playback__controls + .tts-playback__switch { margin-left: 0; }
.tts-playback__switch input { position: absolute; z-index: 1; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
.tts-playback__switch > span { display: inline-flex; align-items: center; width: 32px; height: 18px; padding: 2px; border: var(--border-width) solid var(--color-border-strong); border-radius: 999px; background: var(--color-bg-field); }
.tts-playback__switch > span::after { content: ''; width: 12px; height: 12px; border-radius: 50%; background: var(--color-text-muted); transition: transform var(--duration-fast); }
.tts-playback__switch input:checked + span { border-color: var(--color-border-selected); background: var(--color-accent); }
.tts-playback__switch input:checked + span::after { transform: translateX(14px); background: var(--color-text-on-accent); }
.tts-playback__switch:has(input:focus-visible) > span { box-shadow: var(--focus-ring); }
@media (max-width: 480px) {
  .tts-playback__bar { gap: var(--space-4); }
  .tts-playback__title { flex: 1; }
  .tts-playback__metadata { order: 3; flex: 1; }
  .tts-playback__controls { order: 4; margin-left: 0; }
}
@media (prefers-reduced-motion: reduce) { .tts-playback__switch > span::after { transition: none; } }
@media (forced-colors: active) {
  .tts-playback__switch input { position: static; opacity: 1; appearance: auto; }
  .tts-playback__switch > span { display: none; }
}
</style>
