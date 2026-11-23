<script setup lang="ts">
import { computed } from 'vue';
import { useData, withBase } from 'vitepress';
import { docCopy, docLocales, documentLocale, type DocLocale } from '../../../locales.ts';
import BaseIconSelect from '../../../../src/web/shared/ui/menu/BaseIconSelect.vue';

/** The one language control for the whole site, including the unlocalized root. */
withDefaults(defineProps<{ placement?: 'bar' | 'screen' }>(), { placement: 'bar' });

const { lang, page } = useData();
const locale = computed(() => documentLocale(lang.value || 'en'));
const label = computed(() => docCopy[locale.value].language);
const options = computed(() => docLocales.map((code) => ({ value: code, label: docCopy[code].label })));

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
  <BaseIconSelect
    class="site-language"
    :class="`site-language--${placement}`"
    :model-value="locale"
    :options="options"
    :label="label"
    @update:model-value="change"
  />
</template>

<!-- Unscoped: the classes are applied to BaseIconSelect's root through $attrs,
     so the ancestor scope id is not available here. -->
<style>
.site-language { display: inline-flex; align-items: center; margin-inline: auto }
.site-language.site-language--screen { width: 100%; justify-content: space-between; min-height: 40px; }

@media (min-width: 768px) {
  .site-language.site-language--screen { display: none; }
}

@media (max-width: 767px) {
  .site-language.site-language--bar { display: none; }
}
</style>
