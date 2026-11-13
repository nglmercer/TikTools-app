<script setup lang="ts">
import { computed } from 'vue';
import { useData, withBase } from 'vitepress';
import { docLocales, docCopy, documentLocale } from '../../../locales.ts';

const props = defineProps<{ selector?: boolean }>();

const { lang } = useData();
const languages = computed(() => docLocales.filter((locale) => props.selector || locale !== documentLocale(lang.value)));
const demoHref = computed(() => `${withBase('/demo/')}?scenario=connection&locale=${documentLocale(lang.value)}`);
</script>

<template>
  <main class="docs-home" :class="{ 'docs-home--selector': selector }">
    <div class="docs-home__masthead">
      <div class="docs-home__eyebrow"><slot name="eyebrow" /></div>
      <div class="docs-home__language">
        <a v-for="locale in languages" :key="locale" :href="withBase(`/${locale}/`)" :lang="locale">{{ docCopy[locale].label }}</a>
      </div>
    </div>

    <div class="docs-home__hero">
      <div class="docs-home__intro">
        <slot name="intro" />
        <div class="docs-home__actions"><slot name="actions" :demo-href="demoHref" /></div>
      </div>
      <aside class="docs-home__start"><slot name="start" /></aside>
    </div>

    <section class="docs-home__directory">
      <div class="docs-home__section-heading"><slot name="directory" /></div>
      <div class="docs-home__paths">
        <section class="docs-home__path">
          <span class="docs-home__path-number" aria-hidden="true">01</span>
          <div><slot name="user" /></div>
        </section>
        <section class="docs-home__path">
          <span class="docs-home__path-number" aria-hidden="true">02</span>
          <div><slot name="developer" /></div>
        </section>
      </div>
    </section>

    <section class="docs-home__resources"><slot name="resources" /></section>
    <footer class="docs-home__footer"><slot name="footer" /></footer>
  </main>
</template>
