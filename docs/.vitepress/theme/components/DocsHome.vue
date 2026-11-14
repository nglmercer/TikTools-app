<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { useData, withBase } from 'vitepress';
import { docLocales, docCopy, documentLocale } from '../../../locales.ts';
import { experienceCopy } from '../../../experience/copy.ts';
import type { Topic } from '../../../experience/types.ts';
import DemoFrame from './DemoFrame.vue';
import { landingCopy } from '../../../experience/landing/index.ts';
import IconLink from '../../../../src/web/shared/ui/icon/icons/IconLink.vue';
import IconFlow from '../../../../src/web/shared/ui/icon/icons/IconFlow.vue';
import IconLayers from '../../../../src/web/shared/ui/icon/icons/IconLayers.vue';
import IconCoins from '../../../../src/web/shared/ui/icon/icons/IconCoins.vue';
import IconChat from '../../../../src/web/shared/ui/icon/icons/IconChat.vue';

const props = defineProps<{ selector?: boolean }>();

const { lang } = useData();
const languages = computed(() => docLocales.filter((locale) => props.selector || locale !== documentLocale(lang.value)));
const demoHref = '#playground';
const locale = computed(() => documentLocale(lang.value));
const copy = computed(() => experienceCopy[locale.value]);
const landing = computed(() => landingCopy[locale.value]);
const active = ref<'feed' | Topic>('feed');
const views = ['feed', 'connection', 'automation', 'overlays', 'points'] as const;
const icons = { feed: IconChat, connection: IconLink, automation: IconFlow, overlays: IconLayers, points: IconCoins };
const tabs = ref<HTMLButtonElement[]>([]);
const activeTitle = computed(() => active.value === 'feed' ? landing.value.feed : copy.value.topics[active.value].title);
const activeTask = computed(() => active.value === 'feed' ? copy.value.otherTasks.feed : copy.value.topics[active.value]);
const fullDemo = computed(() => `${withBase('/demo/')}?scenario=${active.value}&locale=${locale.value}`);
async function navigateTabs(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
  event.preventDefault();
  const index = views.indexOf(active.value);
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? views.length - 1 : (index + (event.key === 'ArrowRight' ? 1 : views.length - 1)) % views.length;
  active.value = views[next]!;
  await nextTick(); tabs.value[next]?.focus();
}
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
      <div class="docs-home__availability"><span><i aria-hidden="true" />{{ landing.desktop }}</span><span>{{ landing.localDemo }}</span></div>
    </div>

    <section id="playground" class="product-stage" :aria-labelledby="`stage-title-${locale}`">
      <header class="product-stage__heading"><div><span class="product-stage__eyebrow"><i aria-hidden="true" />{{ copy.interactive }}</span><h2 :id="`stage-title-${locale}`">{{ landing.stageTitle }}</h2></div><p>{{ landing.stageDescription }}</p></header>
      <div class="product-stage__window">
        <div class="product-stage__bar"><span class="product-stage__brand">TikTools<span class="product-stage__sample">{{ copy.fictional }}</span></span><a :href="fullDemo" target="_blank" rel="noopener">{{ landing.openDemo }} <span aria-hidden="true">↗︎</span></a></div>
        <div class="product-stage__tabs" role="tablist" :aria-label="landing.chooseView" @keydown="navigateTabs"><button v-for="(view,index) in views" :id="`stage-${view}-tab`" :key="view" :ref="(element) => { if(element) tabs[index] = element as HTMLButtonElement; }" type="button" role="tab" :aria-selected="active === view" :tabindex="active === view ? 0 : -1" aria-controls="stage-panel" @click="active = view"><component :is="icons[view]" aria-hidden="true" />{{ view === 'feed' ? landing.feed : copy.topics[view].title }}</button></div>
        <div id="stage-panel" class="product-stage__body" role="tabpanel" :aria-labelledby="`stage-${active}-tab`">
          <aside class="product-stage__lesson"><span class="product-stage__lesson-number">0{{ views.indexOf(active) + 1 }} / 05</span><h3>{{ activeTitle }}</h3><p>{{ activeTask.task }}</p><div><strong>{{ copy.success }}</strong><p>{{ activeTask.result }}</p></div><a :href="active === 'feed' ? withBase(`/${locale}/user/live/feed.html`) : guideHref(active)">{{ copy.readGuide }} <span aria-hidden="true">→</span></a></aside>
          <DemoFrame :key="`${locale}-${active}`" :scenario="active" :locale="locale" :panel="active" :title="activeTitle" :initial-events="active === 'feed' ? 9 : undefined" autoload minimal />
        </div>
      </div>
      <div class="product-stage__flow" aria-hidden="true"><span><b>01</b>{{ copy.previewLabels.creator }}</span><i>→</i><span><b>02</b>{{ copy.previewLabels.event }}</span><i>→</i><span><b>03</b>{{ copy.previewLabels.action }}</span><i>→</i><span><b>04</b>{{ copy.previewLabels.output }}</span></div>
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
