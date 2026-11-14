<script setup lang="ts">
import { computed, onMounted, ref, useId } from 'vue';
import { useData, withBase } from 'vitepress';
import { docCopy, documentLocale } from '../../../locales.ts';
import { experienceCopy } from '../../../experience/copy.ts';
import type { Topic } from '../../../experience/types.ts';
import { parseDemoOptions } from '../../../../src/web/demo/scenarios.ts';

const props = defineProps<{ scenario: string; locale?: string; route?: string; tour?: string; title: string; panel?: 'connection' | 'feed' | 'automation' | 'points' | 'overlays'; autoload?: boolean; minimal?: boolean; initialEvents?: number }>();
const { lang } = useData();
const loaded = ref(false);
const ready = ref(false);
const mounted = ref(false);
const instance = ref(`docs-${useId().replaceAll(/[^a-zA-Z0-9_-]/g, '_')}`);
onMounted(() => { instance.value = `docs-${crypto.randomUUID()}`; mounted.value = true; if (props.autoload) loaded.value = true; });
const articleLocale = computed(() => documentLocale(lang.value));
const t = computed(() => docCopy[articleLocale.value]);
const copy = computed(() => experienceCopy[documentLocale(props.locale ?? lang.value)]);
const topic = computed<Topic>(() => ['automation', 'overlays', 'points'].includes(props.scenario) ? props.scenario as Topic : 'connection');
const url = computed(() => {
  const params = new URLSearchParams({ scenario: props.scenario, locale: props.locale ?? articleLocale.value, instance: instance.value });
  if (props.route) params.set('route', props.route);
  if (props.tour) params.set('tour', props.tour);
  if (props.panel) { params.set('embed', props.panel); params.delete('tour'); }
  if (props.initialEvents !== undefined) params.set('events', String(props.initialEvents));
  const valid = parseDemoOptions(params.toString());
  return `${withBase('/demo/')}?${params}${valid.route ? `#${valid.route}` : ''}`;
});
const fullUrl = computed(() => { const value = url.value.split('#'); const clean = new URLSearchParams(value[0]!.split('?')[1]); clean.delete('embed'); clean.delete('instance'); clean.delete('events'); return `${withBase('/demo/')}?${clean}${value[1] ? `#${value[1]}` : ''}`; });
function toggle() { ready.value = false; loaded.value = !loaded.value; }
</script>

<template>
  <section class="demo-frame" :class="{ 'demo-frame--focused': panel, 'demo-frame--minimal': minimal }" :data-panel="panel" :aria-label="title">
    <div v-if="!minimal" class="demo-frame__heading"><span class="demo-frame__dot" /><strong>{{ title }}</strong><span class="demo-frame__badge">{{ copy.fictional }}</span></div>
    <div v-if="!minimal" class="demo-frame__actions">
      <button type="button" :aria-expanded="loaded" @click="toggle">{{ loaded ? t.closeDemo : t.loadDemo }}</button>
      <a :href="fullUrl" target="_blank" rel="noopener">{{ t.fullDemo }} ↗︎</a>
    </div>
    <template v-if="loaded">
      <div class="demo-frame__viewport">
        <p v-if="!ready" class="demo-frame__loading" role="status">{{ copy.loading }}</p>
        <iframe :src="url" :title="title" :loading="autoload ? 'eager' : 'lazy'" sandbox="allow-scripts allow-same-origin" referrerpolicy="no-referrer" @load="ready = true" />
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
.demo-frame__badge { margin-left:auto; color:var(--vp-c-text-2); font:10px var(--vp-font-family-mono); }
.demo-frame__actions { display: flex; flex-wrap: wrap; gap: 8px 18px; align-items: center; padding: 12px 18px; font-size:12px; }
.demo-frame__actions button { padding:6px 10px; border:1px solid var(--vp-c-divider); border-radius:5px; background:var(--vp-c-bg); cursor:pointer; font-weight:600; }
.demo-frame :focus-visible { outline: 2px solid var(--vp-c-brand-1); outline-offset: 3px; }
.demo-frame__viewport { position:relative; }
.demo-frame iframe { display: block; width: 100%; height: var(--demo-frame-height,650px); border: 0; border-top: 1px solid var(--vp-c-divider); }
.demo-frame--focused iframe { height:var(--demo-frame-height,510px); }
.demo-frame[data-panel="connection"] iframe { height:var(--demo-frame-height,370px); }
.demo-frame--minimal { margin:0; border:0; border-radius:0; }
.demo-frame--minimal iframe { border:0; }
.demo-frame__preview { position:relative; display:block; width:100%; padding:18px; border:0; border-top:1px solid var(--vp-c-divider); cursor:pointer; text-align:left; background:var(--vp-c-bg-alt); color:var(--vp-c-text-1); }
.demo-frame__play { display:flex; gap:10px; align-items:center; justify-content:center; width:max-content; max-width:100%; padding:8px 16px; margin:16px auto 0; border-radius:6px; background:var(--vp-c-brand-3); color:white; font-size:13px; font-weight:600; }
.demo-frame__preview:hover .demo-frame__play { background:#2947bd; }
.demo-frame__generic { display:grid; justify-items:center; gap:6px; padding:24px; border:1px solid var(--vp-c-divider); border-radius:8px; background:var(--vp-c-bg); }
.demo-frame__generic > span { color:var(--vp-c-brand-1); font-size:32px; line-height:1.3; }
.demo-frame__generic > b { font-size:14px; }
.demo-frame__generic > small { color:var(--vp-c-text-2); font-size:12px; }
.demo-frame__outcome { display:flex; gap:10px; padding:12px 18px; border-top:1px solid var(--vp-c-divider); color:var(--vp-c-text-2); font-size:12px; line-height:1.6; }
.demo-frame__outcome p { margin:0; }
.demo-frame__loading { position:absolute; z-index:1; inset:0; display:grid; place-items:center; margin:0; padding:12px 18px; font-size:13px; background:#1c1d24; color:#d9dce7; pointer-events:none; }
.demo-frame__waiting { display:grid; place-items:center; min-height:var(--demo-frame-height,370px); background:#1c1d24; color:#d9dce7; font-size:13px; }
@media(max-width:600px) { .demo-frame__badge { display:none; } .demo-frame--focused iframe { height:var(--demo-frame-height,560px); } .demo-frame[data-panel="connection"] iframe { height:var(--demo-frame-height,430px); } }
</style>
