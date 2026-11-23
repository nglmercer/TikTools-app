<script setup lang="ts">
import { computed } from 'vue';
import { useData, withBase } from 'vitepress';
import { docCopy, documentLocale, type DocLocale } from '../../../locales.ts';
import { experienceCopy } from '../../../experience/copy.ts';
import type { Topic } from '../../../experience/types.ts';
import LanguageSwitcher from '../../../../src/web/shared/ui/language-switcher/LanguageSwitcher.vue';
import DemoFrame from './DemoFrame.vue';
import { landingCopy } from '../../../experience/landing/index.ts';
import IconLink from '../../../../src/web/shared/ui/icon/icons/IconLink.vue';
import IconFlow from '../../../../src/web/shared/ui/icon/icons/IconFlow.vue';
import IconLayers from '../../../../src/web/shared/ui/icon/icons/IconLayers.vue';
import IconCoins from '../../../../src/web/shared/ui/icon/icons/IconCoins.vue';

defineProps<{ selector?: boolean }>();

const { lang, page } = useData();
const demoHref = '#playground';
const locale = computed(() => documentLocale(lang.value));
/** Switch languages on the equivalent page, keeping the reader's section. */
function languageHref(target: DocLocale): string {
  const tail = page.value.relativePath.replace(/^[^/]+\//, '');
  const path = tail === 'index.md' ? `/${target}/` : `/${target}/${tail.replace(/\.md$/, '.html')}`;
  const hash = typeof window === 'undefined' ? '' : window.location.hash;
  return withBase(path) + hash;
}
const copy = computed(() => experienceCopy[locale.value]);
const landing = computed(() => landingCopy[locale.value]);
const topics: Topic[] = ['connection', 'automation', 'overlays', 'points'];
const icons = { connection: IconLink, automation: IconFlow, overlays: IconLayers, points: IconCoins };
const guides = { connection: 'first-live', automation: 'automations/first-rule', overlays: 'overlays/', points: 'points/rules' };
const guideHref = (topic: Topic) => withBase(`/${locale.value}/user/${guides[topic]}${guides[topic].endsWith('/') ? '' : '.html'}`);
</script>

<template>
  <main class="docs-home" :class="{ 'docs-home--selector': selector }">
    <div class="docs-home__masthead">
      <div class="docs-home__eyebrow"><slot name="eyebrow" /></div>
      <LanguageSwitcher
        class="docs-home__language"
        variant="links"
        :locale="locale"
        :label="docCopy[locale].language"
        :href-for="languageHref"
      />
    </div>

    <div class="docs-home__hero">
      <div class="docs-home__intro">
        <slot name="intro" />
        <div class="docs-home__actions"><slot name="actions" :demo-href="demoHref" /></div>
      </div>
      <div class="docs-home__availability"><span><i aria-hidden="true" />{{ landing.desktop }}</span><span>{{ landing.localDemo }}</span></div>
    </div>

    <section id="playground" class="product-stage" :aria-label="landing.feed">
      <div class="product-stage__window">
        <div class="product-stage__bar"><span class="product-stage__brand">TikTools</span></div>
        <div class="product-stage__body">
          <aside class="product-stage__lesson">
            <h3>{{ landing.feed }}</h3>
            <p>{{ landing.stageHint }}</p>
            <a :href="withBase(`/${locale}/user/live/feed.html`)">{{ copy.readGuide }} <span aria-hidden="true">→</span></a>
          </aside>
          <DemoFrame :key="locale" scenario="feed" :locale="locale" panel="feed" :title="landing.feed" :initial-events="9" autoload minimal />
        </div>
      </div>
    </section>

    <section class="docs-home__topics" :aria-label="copy.topicsTitle">
      <div class="docs-home__section-heading"><div><span class="docs-home__section-kicker">{{ landing.featuresEyebrow }}</span><h2>{{ landing.featuresTitle }}</h2></div><p>{{ landing.featuresDescription }}</p></div>
      <div class="docs-home__topic-grid"><a v-for="(topic,index) in topics" :key="topic" class="docs-topic" :href="guideHref(topic)"><div class="docs-topic__icon"><component :is="icons[topic]" aria-hidden="true" /><span>0{{ index + 1 }}</span></div><div><h3>{{ copy.topics[topic].title }}</h3><p>{{ copy.topics[topic].description }}</p><span class="docs-topic__link">{{ copy.readGuide }} <span aria-hidden="true">→</span></span></div></a></div>
    </section>

    <aside class="docs-home__start"><slot name="start" /></aside>

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
