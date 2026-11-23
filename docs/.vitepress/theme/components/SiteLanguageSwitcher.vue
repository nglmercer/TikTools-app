<script setup lang="ts">
import { computed } from 'vue';
import { useData, withBase } from 'vitepress';
import { docCopy, docLocales, documentLocale, type DocLocale } from '../../../locales.ts';
import BaseCompactSelect from '../../../../src/web/shared/ui/combo-box/BaseCompactSelect.vue';

/** The one language control for the whole site, including the unlocalized root. */
withDefaults(defineProps<{ placement?: 'bar' | 'screen' }>(), { placement: 'bar' });

const { lang, page } = useData();
const locale = computed(() => documentLocale(lang.value || 'en'));
const label = computed(() => docCopy[locale.value].language);
const options = computed(() => docLocales.map((code) => ({ value: code, label: docCopy[code].label, short: code.toUpperCase() })));

function hrefFor(target: DocLocale): string {
  const tail = page.value.relativePath.replace(/^[^/]+\//, '');
  const path = tail === 'index.md' ? `/${target}/` : `/${target}/${tail.replace(/\.md$/, '.html')}`;
  const hash = typeof window === 'undefined' ? '' : window.location.hash;
  return withBase(path) + hash;
}

function change(target: string): void {
  location.assign(hrefFor(target as DocLocale));
}
</script>

<template>
  <BaseCompactSelect
    class="site-language"
    :class="`site-language--${placement}`"
    :model-value="locale"
    :options="options"
    :ariaLabel="label"
    :title="label"
    @update:model-value="change"
  />
</template>

<style scoped>
.site-language { display: inline-flex; align-items: center; }
.site-language--screen { width: 100%; justify-content: space-between; min-height: 40px; }

@media (min-width: 768px) {
  .site-language--screen { display: none; }
}

@media (max-width: 767px) {
  .site-language--bar { display: none; }
}
</style>
