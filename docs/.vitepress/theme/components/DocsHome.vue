<script setup lang="ts">
import { computed, ref } from 'vue';
import { useData, withBase } from 'vitepress';
import { documentLocale } from '../../../locales.ts';
import { experienceCopy } from '../../../experience/copy.ts';
import type { Topic } from '../../../experience/types.ts';
import DemoFrame from './DemoFrame.vue';
import BaseIconSelect from '../../../../src/web/shared/ui/menu/BaseIconSelect.vue';
import { landingCopy } from '../../../experience/landing/index.ts';
import IconChat from '../../../../src/web/shared/ui/icon/icons/IconChat.vue';
import IconLink from '../../../../src/web/shared/ui/icon/icons/IconLink.vue';
import IconFlow from '../../../../src/web/shared/ui/icon/icons/IconFlow.vue';
import IconLayers from '../../../../src/web/shared/ui/icon/icons/IconLayers.vue';
import IconCoins from '../../../../src/web/shared/ui/icon/icons/IconCoins.vue';

defineProps<{ selector?: boolean }>();

const { lang } = useData();
const demoHref = '#playground';
const locale = computed(() => documentLocale(lang.value));
const copy = computed(() => experienceCopy[locale.value]);
const landing = computed(() => landingCopy[locale.value]);
const topics: Topic[] = ['connection', 'automation', 'overlays', 'points'];
type Section = 'feed' | Topic;
const sections: Section[] = ['feed', 'connection', 'automation', 'overlays', 'points'];
const icons = { feed: IconChat, connection: IconLink, automation: IconFlow, overlays: IconLayers, points: IconCoins };
const guides: Record<Topic, string> = { connection: 'first-live', automation: 'automations/first-rule', overlays: 'overlays/', points: 'points/rules' };
const active = ref<Section>('feed');
const activeTitle = computed(() => active.value === 'feed' ? landing.value.feed : copy.value.topics[active.value].title);
const activeText = computed(() => active.value === 'feed' ? landing.value.stageHint : copy.value.topics[active.value].task);
const sectionLabel = (section: Section) => section === 'feed' ? landing.value.feed : copy.value.topics[section].title;
const sectionOptions = computed(() => sections.map((section) => ({ value: section, label: sectionLabel(section), icon: icons[section] })));
const sectionNumber = computed(() => String(sections.indexOf(active.value) + 1).padStart(2, '0'));
const guideHref = (section: Section) => section === 'feed'
  ? withBase(`/${locale.value}/user/live/feed.html`)
  : withBase(`/${locale.value}/user/${guides[section]}${guides[section].endsWith('/') ? '' : '.html'}`);
</script>

<template>
  <main class="docs-home" :class="{ 'docs-home--selector': selector }">
    <div class="docs-home__masthead">
      <div class="docs-home__eyebrow site-eyebrow"><slot name="eyebrow" /></div>
    </div>

    <div class="docs-home__hero">
      <div class="docs-home__intro">
        <slot name="intro" />
        <div class="docs-home__actions"><slot name="actions" :demo-href="demoHref" /></div>
      </div>
      <div class="docs-home__availability"><span><i aria-hidden="true" />{{ landing.desktop }}</span><span>{{ landing.localDemo }}</span></div>
    </div>

    <section id="playground" class="product-stage" :aria-label="landing.feed">
      <header class="product-stage__intro">
        <div class="product-stage__lesson">
          <h3>{{ activeTitle }}</h3>
          <p>{{ activeText }}</p>
        </div>
        <div class="product-stage__aside">
          <div class="product-stage__controls">
            <BaseIconSelect
              class="product-stage__sections"
              :model-value="active"
              :options="sectionOptions"
              :label="copy.interactive"
              placement="bottom-end"
              test-id="stage-section"
              @update:model-value="active = $event as Section"
            />
            <a
              class="product-stage__guide"
              :href="guideHref(active)"
            >{{ copy.readGuide }} <span aria-hidden="true">→</span></a>
          </div>
          <p class="product-stage__context">{{ sectionNumber }} / 05 · {{ copy.fictional }}</p>
        </div>
      </header>
      <div class="product-stage__window">
        <div class="product-stage__bar">
          <span class="product-stage__brand" :title="`demo:${active}`">TikTools<small class="product-stage__demo-tag">demo:{{ active }}</small></span>
          <div id="product-stage-tools" class="product-stage__tools"></div>
        </div>
        <div class="product-stage__body">
          <DemoFrame :key="`${locale}-${active}`" :scenario="active" :locale="locale" :panel="active" :title="activeTitle" :initial-events="active === 'feed' ? 9 : undefined" autoload minimal />
        </div>
      </div>
    </section>

    <section class="docs-home__next" :aria-label="copy.topicsTitle">
      <div class="docs-home__section-heading"><div><span class="docs-home__section-kicker">{{ landing.featuresEyebrow }}</span><h2>{{ landing.featuresTitle }}</h2></div><p>{{ landing.featuresDescription }}</p></div>
      <div class="docs-home__topic-grid"><a v-for="(topic,index) in topics" :key="topic" class="docs-topic" :href="guideHref(topic)"><div class="docs-topic__icon"><component :is="icons[topic]" aria-hidden="true" /><span>0{{ index + 1 }}</span></div><div><h3>{{ copy.topics[topic].title }}</h3><p>{{ copy.topics[topic].description }}</p><span class="docs-topic__link">{{ copy.readGuide }} <span aria-hidden="true">→</span></span></div></a></div>
      <aside v-if="$slots.start" class="docs-home__start"><slot name="start" /></aside>
    </section>

    <section class="docs-home__directory">
      <div class="docs-home__section-heading"><slot name="directory" /></div>
      <div class="docs-home__paths">
        <section class="docs-home__path site-card">
          <span class="docs-home__path-number" aria-hidden="true">01</span>
          <div><slot name="user" /></div>
        </section>
        <section class="docs-home__path site-card">
          <span class="docs-home__path-number" aria-hidden="true">02</span>
          <div><slot name="developer" /></div>
        </section>
      </div>
    </section>

    <section class="docs-home__resources"><slot name="resources" /></section>
    <footer class="docs-home__footer"><slot name="footer" /></footer>
  </main>
</template>
