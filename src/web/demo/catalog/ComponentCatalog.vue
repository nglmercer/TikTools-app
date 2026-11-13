<script setup lang="ts">
import { computed, defineAsyncComponent, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import { exampleText, familyTitle } from '../../../../docs/examples/text.ts';
import { catalogCopy } from './copy.ts';
import { demoLocales, type DemoLocale } from '../scenarios.ts';
const examples = import.meta.glob('../../../../docs/examples/*.vue');
const components = Object.fromEntries(Object.entries(examples).map(([path, loader]) => [path.split('/').at(-1)!.replace('.vue', ''), defineAsyncComponent(loader as Parameters<typeof defineAsyncComponent>[0])]));
const source = import.meta.glob('../../../../docs/examples/*.vue', { query: '?raw', import: 'default', eager: true });
const route = useRoute(); const router = useRouter(); const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const copy = computed(() => catalogCopy[demoLocales.includes(locale.value as DemoLocale) ? locale.value as DemoLocale : 'en']);
const selected = computed({ get: () => typeof route.params.family === 'string' && Object.hasOwn(components, route.params.family) ? route.params.family : 'buttons', set: (family: string) => { void router.replace(`/components/${family}`); } });
const state = ref('normal');
const text = computed(() => source[`../../../../docs/examples/${selected.value}.vue`]);
const states = computed(() => ['normal', 'loading', 'empty', 'error', 'disabled', 'long', 'mobile'].map((value) => ({ value, label: ({ normal: t.value.ready, loading: t.value.loading, empty: t.value.empty, error: t.value.error, disabled: t.value.disabled, long: t.value.long, mobile: copy.value.mobile })[value]! })));
const guideUrl = computed(() => new URL(`../${locale.value}/developer/components/${selected.value}.html`, location.href).href);
</script>
<template>
  <main class="component-catalog">
    <header class="component-catalog__header">
      <div>
        <h1>{{ copy.title }}</h1>
        <p>{{ copy.description }}</p>
      </div>
      <BaseButton @click="router.push('/feed')">
        {{ t.back }}
      </BaseButton>
    </header>
    <div class="component-catalog__toolbar">
      <label><span>{{ copy.family }}</span><BaseComboBox
        v-model="selected"
        :options="Object.keys(components).map(value => ({ value, label: familyTitle(value, locale) }))"
        :aria-label="copy.family"
      /></label>
      <label><span>{{ copy.state }}</span><BaseComboBox
        v-model="state"
        :options="states"
        :aria-label="copy.state"
      /></label>
      <a :href="guideUrl">{{ copy.guide }} ↗</a>
    </div>
    <section
      class="component-catalog__preview"
      :style="{ maxWidth: state === 'mobile' ? '360px' : '100%' }"
      :aria-label="copy.preview"
    >
      <component
        :is="components[selected]"
        :key="selected"
        :state="state"
        :locale="locale"
      />
    </section>
    <details><summary>{{ copy.source }}</summary><pre><code>{{ text }}</code></pre></details>
  </main>
</template>
<style scoped>
.component-catalog { height:100%; overflow:auto; padding:24px; background:var(--color-bg-app); color:var(--color-text); }
.component-catalog__header { display:flex; justify-content:space-between; align-items:flex-start; gap:16px; margin-bottom:24px; }
.component-catalog h1 { font-size:22px; margin:0 0 6px; font-weight:600; }
.component-catalog p { font-size:13px; color:var(--color-text-muted); margin:0; max-width:64ch; }
.component-catalog__toolbar { display:flex; align-items:flex-end; flex-wrap:wrap; gap:16px; padding-bottom:20px; border-bottom:1px solid var(--color-border); }
.component-catalog__toolbar label { display:grid; gap:6px; min-width:160px; flex:0 1 260px; }
.component-catalog__toolbar label > span { font-size:12px; color:var(--color-text-muted); }
.component-catalog__toolbar a { margin-left:auto; align-self:center; color:var(--color-text); font-size:13px; text-underline-offset:3px; }
.component-catalog__preview { margin:24px 0; min-height:90px; }
.component-catalog details { border-top:1px solid var(--color-border); padding:16px 0; }
.component-catalog summary { cursor:pointer; font-size:13px; }
.component-catalog pre { overflow:auto; max-height:460px; font-size:12px; line-height:1.6; padding:16px; background:var(--color-bg-panel); border:1px solid var(--color-border); }
.component-catalog a:focus-visible,.component-catalog summary:focus-visible { outline:2px solid var(--color-border-focus); outline-offset:4px; }
@media(max-width:600px) { .component-catalog { padding:16px 12px; } .component-catalog__header { flex-wrap:wrap; } .component-catalog__toolbar label { flex:1 1 180px; min-width:0; } .component-catalog__toolbar a { margin-left:0; } }
</style>
