<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useId, watch } from 'vue';
import { useData, withBase } from 'vitepress';
import { docCopy, documentLocale } from '../../../locales.ts';
import { experienceCopy } from '../../../experience/copy.ts';
import type { Topic } from '../../../experience/types.ts';
import { parseDemoOptions } from '../../../../src/web/demo/scenarios.ts';
import { frameCopy } from '../../../experience/frame/index.ts';
import { toolbarCopy } from '../../../../src/web/demo/toolbar-copy.ts';
import { isDemoFrameState, type DemoFrameState, type FrameAction } from '../../../../src/web/demo/frame-messages.ts';
import IconPlay from '../../../../src/web/shared/ui/icon/icons/IconPlay.vue';
import IconPause from '../../../../src/web/shared/ui/icon/icons/IconPause.vue';
import IconRefresh from '../../../../src/web/shared/ui/icon/icons/IconRefresh.vue';
import IconChevronRight from '../../../../src/web/shared/ui/icon/icons/IconChevronRight.vue';
import IconPopout from '../../../../src/web/shared/ui/icon/icons/IconPopout.vue';
import BaseCompactSelect from '../../../../src/web/shared/ui/combo-box/BaseCompactSelect.vue';

const props = defineProps<{ scenario: string; locale?: string; route?: string; tour?: string; title: string; panel?: 'connection' | 'feed' | 'automation' | 'points' | 'overlays'; autoload?: boolean; minimal?: boolean; initialEvents?: number; scale?: 0.25 | 0.5 | 0.75 | 1; viewportHeight?: number }>();
const { lang, isDark } = useData();
const loaded = ref(false);
const ready = ref(false);
const mounted = ref(false);
const chromeTarget = ref(false);
const root = ref<HTMLElement | null>(null);
const iframe = ref<HTMLIFrameElement | null>(null);
const measuredWidth = ref(0);
const measuredHeight = ref(0);
const zoom = ref(props.scale ? String(props.scale) : 'fit');
const zoomLevels = [0.25, 0.5, 0.75, 1];
const runtime = ref<DemoFrameState | null>(null);
const logicalHeight = computed(() => {
  if (props.viewportHeight) return props.viewportHeight;
  if (!props.minimal && props.panel === 'connection') return 480;
  if (props.minimal && measuredHeight.value) return Math.max(400, Math.min(1200, measuredHeight.value));
  return 800;
});
const effectiveScale = computed(() => zoom.value === 'fit' ? Math.min(1, (measuredWidth.value || 1280) / 1280) : Number(zoom.value));
const frameStyle = computed(() => ({ '--frame-scale': String(effectiveScale.value), '--frame-height': String(logicalHeight.value) }));
const canRun = computed(() => runtime.value?.ready && !runtime.value.busy && runtime.value.status !== 'limit');
let observer: ResizeObserver | undefined;
const instance = ref(`docs-${useId().replaceAll(/[^a-zA-Z0-9_-]/g, '_')}`);
/** Fit the landing frame to its content instead of reserving a fixed 800px canvas. */
function measureHeight(): void {
  if (!props.minimal) return;
  const doc = iframe.value?.contentDocument;
  if (!doc) return;
  const content = doc.querySelector('.app-layout__content');
  if (!content) return;
  const top = content.getBoundingClientRect().top;
  const list = doc.querySelector('.live-stage__cards, .live-feed [role="list"]');
  let natural = list ? list.getBoundingClientRect().bottom - top + 24 : 0;
  if (!natural) {
    let bottom = 0;
    for (const element of content.querySelectorAll('*')) {
      if (element.children.length) continue;
      const rect = element.getBoundingClientRect();
      if (rect.height > 0 && rect.height < 160) bottom = Math.max(bottom, rect.bottom);
    }
    natural = bottom - top + 48;
  }
  const chrome = (doc.querySelector('.app-menu-bar')?.getBoundingClientRect().height ?? 0)
    + (doc.querySelector('.app-status-bar')?.getBoundingClientRect().height ?? 0);
  const height = Math.ceil(natural + chrome);
  if (height >= 400 && Math.abs(height - measuredHeight.value) > 8) measuredHeight.value = height;
}
function receiveState(event: MessageEvent) {
  if (event.origin !== location.origin || event.source !== iframe.value?.contentWindow || !isDemoFrameState(event.data)) return;
  runtime.value = event.data;
  measureHeight();
}
onMounted(() => {
  instance.value = `docs-${crypto.randomUUID()}`;
  measuredWidth.value = root.value?.clientWidth ?? 1280;
  observer = new ResizeObserver(entries => { measuredWidth.value = entries[0]?.contentRect.width ?? 1280; });
  if (root.value) observer.observe(root.value);
  window.addEventListener('message', receiveState);
  chromeTarget.value = Boolean(document.querySelector('#product-stage-tools'));
  mounted.value = true;
  if (props.autoload) loaded.value = true;
});
onBeforeUnmount(() => { observer?.disconnect(); window.removeEventListener('message', receiveState); });
const articleLocale = computed(() => documentLocale(lang.value));
const t = computed(() => docCopy[articleLocale.value]);
const copy = computed(() => experienceCopy[documentLocale(props.locale ?? lang.value)]);
const labels = computed(() => frameCopy[documentLocale(props.locale ?? lang.value)]);
const fitPercent = computed(() => `${Math.round(Math.min(1, (measuredWidth.value || 1280) / 1280) * 100)}%`);
const zoomOptions = computed(() => [
  { value: 'fit', label: `${labels.value.fit} (${fitPercent.value})`, short: fitPercent.value },
  ...zoomLevels.map((level) => ({ value: String(level), label: `${level * 100}%`, short: `${level * 100}%` })),
]);
const player = computed(() => toolbarCopy[documentLocale(props.locale ?? lang.value)]);
const topic = computed<Topic>(() => ['automation', 'overlays', 'points'].includes(props.scenario) ? props.scenario as Topic : 'connection');
// Snapshot only: changing the docs theme must not reload the iframe. Live
// changes are forwarded through the bounded frame command bridge.
const initialTheme = isDark.value ? 'dark' : 'light';
const url = computed(() => {
  const params = new URLSearchParams({ scenario: props.scenario, locale: props.locale ?? articleLocale.value, instance: instance.value, framed: '1', theme: initialTheme });
  if (props.route) params.set('route', props.route);
  if (props.tour) params.set('tour', props.tour);
  if (props.panel) { params.set('embed', props.panel); params.delete('tour'); }
  if (props.initialEvents !== undefined) params.set('events', String(props.initialEvents));
  const valid = parseDemoOptions(params.toString());
  return `${withBase('/demo/')}?${params}${valid.route ? `#${valid.route}` : ''}`;
});
const fullUrl = computed(() => { const value = url.value.split('#'); const clean = new URLSearchParams(value[0]!.split('?')[1]); clean.delete('embed'); clean.delete('instance'); clean.delete('events'); clean.delete('framed'); clean.delete('theme'); return `${withBase('/demo/')}?${clean}${value[1] ? `#${value[1]}` : ''}`; });
function toggle() { ready.value = false; runtime.value = null; loaded.value = !loaded.value; }
function command(action: FrameAction) {
  if (!runtime.value?.ready) return;
  iframe.value?.contentWindow?.postMessage({ type: 'tiktools.demo.command', instance: runtime.value.instance, ...action }, location.origin);
  if (action.action === 'reset') { ready.value = false; runtime.value = null; }
}
watch([() => runtime.value?.ready, isDark], ([frameReady, dark]) => {
  if (frameReady) command({ action: 'theme', value: dark ? 'dark' : 'light' });
});
</script>

<template>
  <section ref="root" class="demo-frame" :class="{ 'demo-frame--focused': panel, 'demo-frame--minimal': minimal, 'demo-frame--fit': zoom === 'fit' }" :style="frameStyle" :data-panel="panel" :aria-label="title">
    <div v-if="!minimal" class="demo-frame__heading"><span class="demo-frame__dot" /><strong class="demo-frame__title">{{ title }}<small class="demo-frame__demo-tag">demo:{{ props.scenario }}</small></strong></div>
    <div v-if="!minimal" class="demo-frame__actions">
      <button type="button" :aria-expanded="loaded" @click="toggle">{{ loaded ? t.closeDemo : t.loadDemo }}</button>
      <div v-if="loaded" class="demo-frame__tools" role="group" :aria-label="copy.interactive">
        <button type="button" data-testid="frame-play" :aria-label="runtime?.paused === false ? player.pause : player.play" :title="runtime?.paused === false ? player.pause : player.play" :aria-pressed="runtime?.paused === false" :disabled="!canRun" @click="command({ action:'play' })"><IconPause v-if="runtime?.paused === false" /><IconPlay v-else /></button>
        <button type="button" data-testid="frame-step" :aria-label="player.step" :title="player.step" :disabled="!canRun" @click="command({ action:'step' })"><IconChevronRight /></button>
        <button type="button" data-testid="frame-reset" :aria-label="copy.reset" :title="copy.reset" :disabled="!runtime?.ready" @click="command({ action:'reset' })"><IconRefresh /></button>
        <output v-if="runtime" class="demo-frame__count" :aria-label="player.events">{{ runtime.events }}</output>
        <select v-if="panel === 'connection'" class="demo-frame__case" :aria-label="copy.connectionStates.label" :value="runtime?.connectionCase ?? 'normal'" :disabled="!runtime?.ready" @change="command({ action:'connection-case', value:($event.target as HTMLSelectElement).value as 'normal' | 'offline' | 'auth' })"><option value="normal">{{ copy.connectionStates.normal }}</option><option value="offline">{{ copy.connectionStates.offline }}</option><option value="auth">{{ copy.connectionStates.auth }}</option></select>
      </div>
    </div>
    <p v-if="panel === 'connection' && runtime && runtime.connectionCase !== 'normal'" class="demo-frame__recovery" role="status">{{ runtime.connectionCase === 'offline' ? copy.connectionStates.offlineHelp : copy.connectionStates.authHelp }}</p>
    <template v-if="loaded">
      <div class="demo-frame__viewport">
        <p v-if="!ready" class="demo-frame__loading" role="status">{{ copy.loading }}</p>
        <Teleport
          to="#product-stage-tools"
          :disabled="!minimal || !chromeTarget"
        >
          <div
            class="demo-frame__overlay"
            :class="{ 'demo-frame__overlay--docked': minimal && chromeTarget }"
          >
            <a
              :href="fullUrl"
              target="_blank"
              rel="noopener"
              :aria-label="t.fullDemo"
              :title="t.fullDemo"
            ><IconPopout /><span class="demo-frame__overlay-label">demo</span></a>
            <BaseCompactSelect
              v-model="zoom"
              class="demo-frame__overlay-zoom"
              :options="zoomOptions"
              :ariaLabel="labels.zoom"
              :title="labels.desktop"
              test-id="frame-zoom"
            />
          </div>
        </Teleport>
        <div class="demo-frame__canvas"><iframe ref="iframe" :src="url" :title="title" :loading="autoload ? 'eager' : 'lazy'" :style="{ visibility:measuredWidth ? 'visible' : 'hidden' }" sandbox="allow-scripts allow-same-origin" referrerpolicy="no-referrer" @load="ready = true; measureHeight()" /></div>
      </div>
    </template>
    <div v-else-if="autoload && !mounted" class="demo-frame__waiting" role="status">{{ copy.loading }}</div>
    <button v-else class="demo-frame__preview" type="button" :aria-label="`${copy.tryIt}: ${title}`" @click="toggle"><span class="demo-frame__generic"><span aria-hidden="true">▤</span><b>{{ title }}</b><small>{{ copy.interactive }}</small></span><span class="demo-frame__play"><span aria-hidden="true">▶︎</span>{{ copy.tryIt }}</span></button>
    <div v-if="panel && !minimal" class="demo-frame__outcome"><span aria-hidden="true">↳</span><p><b>{{ panel === 'feed' ? copy.success : copy.task }}</b> {{ panel === 'feed' ? copy.topics[topic].result : copy.topics[topic].task }}</p></div>
  </section>
</template>

<style scoped>
.demo-frame { margin: 1.5rem 0; overflow:hidden; border: 1px solid var(--vp-c-divider); border-radius:12px; background:var(--vp-c-bg-soft); }
.demo-frame__heading { display:flex; align-items:center; gap:8px; padding:14px 18px 0; font-size:13px; }
.demo-frame__dot { width:7px; height:7px; border-radius:50%; background:var(--vp-c-brand-1); flex-shrink:0; }
.demo-frame__title { display:inline-flex; align-items:baseline; gap:8px; font-weight:600; }
.demo-frame__demo-tag { color:var(--vp-c-text-3); font:10px var(--vp-font-family-mono); font-weight:400; opacity:0; transition:opacity .15s; }
.demo-frame__heading:hover .demo-frame__demo-tag,
.demo-frame__title:focus-visible .demo-frame__demo-tag { opacity:1; }
.demo-frame__actions { display: flex; flex-wrap: wrap; gap: 8px 14px; align-items: center; padding: 10px 14px; font-size:12px; }
.demo-frame__actions button { padding:6px 10px; border:1px solid var(--vp-c-divider); border-radius:5px; background:var(--vp-c-bg); cursor:pointer; font-weight:600; }
.demo-frame__actions button:disabled { opacity:.45; cursor:default; }
.demo-frame__tools { display:flex; align-items:center; flex-wrap:wrap; gap:6px; flex:1; min-width:0; }
.demo-frame__tools > button { display:grid; place-items:center; width:30px; height:30px; padding:6px; flex:none; }
.demo-frame__tools svg { width:15px; height:15px; }
.demo-frame__count { padding:0 4px; color:var(--vp-c-text-2); font-size:10px; font-variant-numeric:tabular-nums; }
.demo-frame__case { padding:5px 8px; min-height:30px; max-width:190px; min-width:0; border:1px solid var(--vp-c-divider); border-radius:5px; background:var(--vp-c-bg); color:var(--vp-c-text-1); font:11px var(--vp-font-family-base); cursor:pointer; }
.demo-frame__recovery { margin:0; padding:10px 14px; font-size:12px; line-height:1.7; border-top:1px solid var(--vp-c-divider); }
.demo-frame :focus-visible { outline: 2px solid var(--vp-c-brand-1); outline-offset: 3px; }
/* Keep horizontal scroll inside the frame, but let vertical scroll chain to
   the article so the wheel never dies when the pointer is over the demo. */
.demo-frame__viewport { position:relative; overflow:auto; overscroll-behavior-x:contain; overscroll-behavior-y:auto; background:var(--vp-c-bg-soft); }
.demo-frame__overlay { position:absolute; z-index:3; top:10px; right:12px; display:flex; align-items:center; gap:8px; opacity:0; pointer-events:none; transition:opacity .15s; }
.demo-frame:hover .demo-frame__overlay,
.demo-frame:focus-within .demo-frame__overlay { opacity:1; pointer-events:auto; }
.demo-frame__overlay--docked { position:static; opacity:1; pointer-events:auto; transition:none; }
.demo-frame__overlay a { display:inline-flex; align-items:center; gap:6px; padding:5px 8px; border:1px solid var(--vp-c-divider); border-radius:6px; background:var(--vp-c-bg); color:var(--vp-c-text-1); text-decoration:none; }
.demo-frame__overlay svg { width:14px; height:14px; }
.demo-frame__overlay-label { color:var(--vp-c-text-3); font:10px var(--vp-font-family-mono); transition:opacity .15s; }
.demo-frame__canvas { position:relative; width:calc(1280px * var(--frame-scale)); height:calc(var(--frame-height) * 1px * var(--frame-scale)); margin-inline:auto; }
.demo-frame--fit .demo-frame__canvas { width:min(100%,1280px); height:auto; aspect-ratio:1280 / var(--frame-height); }
.demo-frame iframe { position:absolute; top:0; left:0; display:block; width:1280px; max-width:none; height:calc(var(--frame-height) * 1px); border:0; transform:scale(var(--frame-scale)); transform-origin:top left; }
.demo-frame--minimal { margin:0; border:0; border-radius:0; }
.demo-frame__preview { position:relative; display:block; width:100%; padding:18px; border:0; border-top:1px solid var(--vp-c-divider); cursor:pointer; text-align:left; background:var(--vp-c-bg-alt); color:var(--vp-c-text-1); }
.demo-frame__play { display:flex; gap:10px; align-items:center; justify-content:center; width:max-content; max-width:100%; padding:8px 16px; margin:16px auto 0; border-radius:6px; background:var(--vp-c-brand-3); color:white; font-size:13px; font-weight:600; }
.demo-frame__preview:hover .demo-frame__play { background:#2947bd; }
.demo-frame__generic { display:grid; justify-items:center; gap:6px; padding:24px; border:1px solid var(--vp-c-divider); border-radius:8px; background:var(--vp-c-bg); }
.demo-frame__generic > span { color:var(--vp-c-brand-1); font-size:32px; line-height:1.3; }
.demo-frame__generic > b { font-size:14px; }
.demo-frame__generic > small { color:var(--vp-c-text-2); font-size:12px; }
.demo-frame__outcome { display:flex; gap:10px; padding:12px 18px; border-top:1px solid var(--vp-c-divider); color:var(--vp-c-text-2); font-size:12px; line-height:1.6; }
.demo-frame__outcome p { margin:0; }
.demo-frame__loading { position:absolute; z-index:1; inset:0; display:grid; place-items:center; margin:0; padding:12px 18px; font-size:13px; background:var(--vp-c-bg-soft); color:var(--vp-c-text-2); pointer-events:none; }
.demo-frame__waiting { display:grid; place-items:center; width:min(100%,1280px); aspect-ratio:1280 / var(--frame-height); background:var(--vp-c-bg-soft); color:var(--vp-c-text-2); font-size:13px; }
@media(max-width:600px) { .demo-frame__case { max-width:155px; } .demo-frame__overlay-label { display:none; } }
</style>
