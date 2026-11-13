<script setup lang="ts">
import { computed } from 'vue';
import { useData, withBase } from 'vitepress';
import { docLocales, docCopy, documentLocale } from '../../../locales.ts';
import { experienceCopy } from '../../../experience/copy.ts';
import type { Topic } from '../../../experience/types.ts';
import DemoFrame from './DemoFrame.vue';
import ScenePreview from './ScenePreview.vue';

const props = defineProps<{ selector?: boolean }>();

const { lang } = useData();
const languages = computed(() => docLocales.filter((locale) => props.selector || locale !== documentLocale(lang.value)));
const demoHref = computed(() => `${withBase('/demo/')}?scenario=connection&locale=${documentLocale(lang.value)}`);
const locale = computed(() => documentLocale(lang.value));
const copy = computed(() => experienceCopy[locale.value]);
const topics: Topic[] = ['connection', 'automation', 'overlays', 'points'];
const guides = { connection: 'first-live', automation: 'automations/first-rule', overlays: 'overlays/', points: 'points/rules' };
const guideHref = (topic: Topic) => withBase(`/${locale.value}/user/${guides[topic]}${guides[topic].endsWith('/') ? '' : '.html'}`);
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
      <div class="docs-home__product"><DemoFrame scenario="connection" :locale="locale" panel="connection" :title="copy.topics.connection.title" /></div>
    </div>

    <aside class="docs-home__start"><slot name="start" /></aside>

    <section class="docs-home__topics" :aria-label="copy.topicsTitle">
      <div class="docs-home__section-heading"><h2>{{ copy.topicsTitle }}</h2><p>{{ copy.topicsDescription }}</p></div>
      <div class="docs-home__topic-grid"><a v-for="topic in topics" :key="topic" class="docs-topic" :href="guideHref(topic)"><ScenePreview :topic="topic" :copy="copy" /><div><h3>{{ copy.topics[topic].title }} <span aria-hidden="true">↗︎</span></h3><p>{{ copy.topics[topic].description }}</p><span class="docs-topic__link">{{ copy.readGuide }} →</span></div></a></div>
    </section>

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
