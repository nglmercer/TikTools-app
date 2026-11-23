<script setup lang="ts">
import { computed } from 'vue';
import { useData, withBase } from 'vitepress';
import { docCopy, documentLocale, type DocLocale } from '../../../locales.ts';
import LanguageSwitcher from '../../../../src/web/shared/ui/language-switcher/LanguageSwitcher.vue';

/** The one language control for the whole site, including the unlocalized root. */
withDefaults(defineProps<{ placement?: 'bar' | 'screen' }>(), { placement: 'bar' });

const { lang, page } = useData();
const locale = computed(() => documentLocale(lang.value || 'en'));
const label = computed(() => docCopy[locale.value].language);

function hrefFor(target: DocLocale): string {
  const tail = page.value.relativePath.replace(/^[^/]+\//, '');
  const path = tail === 'index.md' ? `/${target}/` : `/${target}/${tail.replace(/\.md$/, '.html')}`;
  const hash = typeof window === 'undefined' ? '' : window.location.hash;
  return withBase(path) + hash;
}

function change(target: DocLocale): void {
  location.assign(hrefFor(target));
}
</script>

<template>
  <LanguageSwitcher
    class="site-language"
    :class="`site-language--${placement}`"
    variant="select"
    :locale="locale"
    :label="label"
    :href-for="hrefFor"
    @change="change"
  />
</template>

<style scoped>
.site-language {
  display: inline-flex;
  align-items: center;
}

.site-language :deep(select) {
  min-width: 104px;
  min-height: 32px;
  padding: 4px 26px 4px 10px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  background-color: var(--vp-c-bg-soft);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12'%3E%3Cpath d='M2.5 4.5 6 8l3.5-3.5' fill='none' stroke='%23888' stroke-width='1.2'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 12px;
  color: var(--vp-c-text-1);
  font-size: 12px;
  cursor: pointer;
  appearance: none;
}

.site-language :deep(select:hover) {
  border-color: var(--vp-c-brand-1);
}

.site-language--screen { display: block; }

.site-language--screen :deep(select) {
  width: 100%;
  min-height: 40px;
  border-color: var(--vp-c-divider);
  background-color: var(--vp-c-bg-soft);
  color: var(--vp-c-text-1);
  font-size: 14px;
}

@media (min-width: 768px) {
  .site-language--screen { display: none; }
}

@media (max-width: 767px) {
  .site-language--bar { display: none; }
}
</style>
