<script setup lang="ts">
import { computed, defineAsyncComponent, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import { exampleText, familyTitle } from '../../../../docs/examples/text.ts';
const examples = import.meta.glob('../../../../docs/examples/*.vue');
const components = Object.fromEntries(Object.entries(examples).map(([path, loader]) => [path.split('/').at(-1)!.replace('.vue', ''), defineAsyncComponent(loader as Parameters<typeof defineAsyncComponent>[0])]));
const source = import.meta.glob('../../../../docs/examples/*.vue', { query: '?raw', import: 'default', eager: true });
const route = useRoute(); const router = useRouter(); const { locale } = useI18n();
const t = computed(() => exampleText(locale.value));
const selected = computed({ get: () => typeof route.params.family === 'string' && Object.hasOwn(components, route.params.family) ? route.params.family : 'buttons', set: (family: string) => { void router.replace(`/components/${family}`); } });
const state = ref('normal');
const text = computed(() => source[`../../../../docs/examples/${selected.value}.vue`]);
const states = computed(() => ['normal', 'loading', 'empty', 'error', 'disabled', 'long', 'mobile'].map((value) => ({ value, label: ({ normal: t.value.ready, loading: t.value.loading, empty: t.value.empty, error: t.value.error, disabled: t.value.disabled, long: t.value.long, mobile: locale.value === 'es' ? 'Móvil' : locale.value === 'fr' ? 'Mobile' : locale.value === 'pt' ? 'Celular' : 'Mobile' })[value]! })));
</script>
<template>
  <main class="component-catalog">
    <h1>{{ locale === 'es' ? 'Componentes' : locale === 'pt' ? 'Componentes' : locale === 'fr' ? 'Composants' : 'Components' }}</h1>
    <div class="component-catalog__toolbar">
      <BaseComboBox
        v-model="selected"
        :options="Object.keys(components).map(value => ({ value, label: familyTitle(value, locale) }))"
        :aria-label="t.name"
      />
      <BaseComboBox
        v-model="state"
        :options="states"
        :aria-label="t.enabled"
      />
      <BaseButton @click="router.push('/feed')">
        {{ t.back }}
      </BaseButton>
    </div>
    <section
      :style="{ maxWidth: state === 'mobile' ? '360px' : '100%' }"
      :aria-label="t.item"
    >
      <component
        :is="components[selected]"
        :key="selected"
        :state="state"
        :locale="locale"
      />
    </section>
    <details><summary>{{ locale === 'es' ? 'Código del ejemplo' : locale === 'pt' ? 'Código do exemplo' : locale === 'fr' ? 'Code de l’exemple' : 'Example source' }}</summary><pre><code>{{ text }}</code></pre></details>
  </main>
</template>
<style scoped>
.component-catalog { height: 100%; overflow: auto; padding: 1rem; background: var(--color-bg); color: var(--color-text); }
.component-catalog h1 { font-size: 1.5rem; margin: 0 0 1rem; }
.component-catalog__toolbar { display: flex; flex-wrap: wrap; gap: .5rem; margin-bottom: 1rem; }
.component-catalog pre { overflow: auto; font-size: .8rem; }
</style>
