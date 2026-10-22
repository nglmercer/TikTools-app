<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { PluginBroker } from '../broker.ts';
import TtsIcon from './TtsIcon.vue';
import TtsToggle from './TtsToggle.vue';
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
  <section class="tts-card tts-playback">
    <h4 class="tts-card__title"><TtsIcon name="audio"/>{{ t(locale, 'ttsPlayback') }}</h4>
    <TtsToggle :label="t(locale, 'ttsTrackPlayback')" :hint="t(locale, 'ttsPlaybackScope')" :checked="enabled" @change="emit('toggle', $event)"/>
    <template v-if="enabled">
      <p class="tts-hint">{{ t(locale, 'ttsPlaybackScope') }}</p>
      <p v-if="error || actionError" role="alert" class="plg-alert">{{ error || actionError }}</p>
      <div v-if="snapshot?.state === 'ready' && fresh" class="tts-playback__status" role="status">
        <p>{{ t(locale, current ? (snapshot.paused ? 'ttsPlaybackPaused' : snapshot.playing ? 'ttsPlaybackPlaying' : 'ttsPlaybackWaiting') : 'ttsPlaybackIdle') }} <code v-if="current">{{ current }}</code></p>
        <p>{{ t(locale, 'ttsPlaybackQueue') }}: {{ snapshot.queueLength }} · {{ t(locale, 'ttsPlaybackDuration') }}: {{ seconds(snapshot.durationSecs) }}<template v-if="snapshot.positionSecs != null"> · {{ t(locale, 'ttsPlaybackPosition') }}: {{ seconds(snapshot.positionSecs) }}</template></p>
        <p v-if="current && snapshot.durationSecs == null" class="tts-hint">{{ t(locale, 'ttsPlaybackDurationUnknown') }}</p>
      </div>
      <p v-else-if="!error" role="status">{{ t(locale, 'ttsPlaybackPending') }}</p>
      <div class="tts-row tts-playback__controls">
        <button type="button" class="plg-btn plg-btn--sm" :disabled="!canControl || !current" :title="t(locale, 'ttsSkipAudio')" @click="control('skip')"><TtsIcon name="skip"/>{{ t(locale, 'ttsSkipAudio') }}</button>
        <button type="button" class="plg-btn plg-btn--sm" :disabled="!canControl || !current" @click="control(snapshot?.paused ? 'resume' : 'pause')"><TtsIcon :name="snapshot?.paused ? 'play' : 'pause'"/>{{ t(locale, snapshot?.paused ? 'ttsResumeAudio' : 'ttsPauseAudio') }}</button>
        <button type="button" class="plg-btn plg-btn--sm" :disabled="!canControl || (!current && !snapshot?.queueLength)" :title="t(locale, 'ttsStopAudio')" @click="control('stop')"><TtsIcon name="stop"/>{{ t(locale, 'ttsStopAudio') }}</button>
      </div>
    </template>
  </section>
</template>
<style scoped>
.tts-playback { margin-bottom: 12px; }
.tts-playback__toggle { display: flex; align-items: center; gap: 8px; }
.tts-playback__controls { justify-content: flex-start; flex-wrap: wrap; gap: 8px; }
</style>
