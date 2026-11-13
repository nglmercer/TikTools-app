<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import { useData, withBase } from 'vitepress';
import { parseDemoOptions } from '../../../../src/web/demo/scenarios.ts';

const props = defineProps<{ scenario: string; locale?: string; route?: string; tour?: string; title: string }>();
const { lang } = useData();
const loaded = ref(false);
const instance = useId().replaceAll(/[^a-zA-Z0-9_-]/g, '_');
const es = computed(() => lang.value.startsWith('es'));
const url = computed(() => {
  const params = new URLSearchParams({ scenario: props.scenario, locale: props.locale ?? (es.value ? 'es' : 'en'), instance: `docs-${instance}` });
  if (props.route) params.set('route', props.route);
  if (props.tour) params.set('tour', props.tour);
  const valid = parseDemoOptions(params.toString());
  return `${withBase('/demo/')}?${params}${valid.route ? `#${valid.route}` : ''}`;
});
</script>

<template>
  <section class="demo-frame" :aria-label="title">
    <div class="demo-frame__actions">
      <button type="button" @click="loaded = !loaded">{{ loaded ? (es ? 'Cerrar demo' : 'Close demo') : (es ? 'Cargar demo interactiva' : 'Load interactive demo') }}</button>
      <a :href="url" target="_blank" rel="noopener">{{ es ? 'Abrir demo completa' : 'Open full demo' }}</a>
    </div>
    <iframe v-if="loaded" :src="url" :title="title" loading="lazy" sandbox="allow-scripts allow-same-origin" referrerpolicy="no-referrer" />
  </section>
</template>

<style scoped>
.demo-frame { margin: 1.5rem 0; border: 1px solid var(--vp-c-divider); }
.demo-frame__actions { display: flex; flex-wrap: wrap; gap: 1rem; align-items: center; padding: .75rem; }
.demo-frame button { padding: .3rem .6rem; border: 1px solid var(--vp-c-divider); border-radius: 3px; }
.demo-frame :focus-visible { outline: 2px solid var(--vp-c-brand-1); outline-offset: 3px; }
.demo-frame iframe { display: block; width: 100%; height: 650px; border: 0; border-top: 1px solid var(--vp-c-divider); }
</style>
