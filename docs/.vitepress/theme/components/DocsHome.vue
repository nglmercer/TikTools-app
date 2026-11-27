<script setup lang="ts">
import { computed, ref } from 'vue';
import { useData, withBase } from 'vitepress';
import { documentLocale } from '../../../locales.ts';
import { experienceCopy } from '../../../experience/copy.ts';
import type { Topic } from '../../../experience/types.ts';
import DemoFrame from './DemoFrame.vue';
import { toolbarCopy } from '../../../../src/web/demo/toolbar-copy.ts';
import type { DemoFrameState, ShowcaseFrameState } from '../../../../src/web/demo/frame-messages.ts';
import IconEye from '../../../../src/web/shared/ui/icon/icons/IconEye.vue';
import IconPlay from '../../../../src/web/shared/ui/icon/icons/IconPlay.vue';
import IconPause from '../../../../src/web/shared/ui/icon/icons/IconPause.vue';
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
type StepId = 'feed' | 'inspector' | Exclude<Topic, 'connection'>;
// The guided tour visits these real views in order; connection stays a guide, not a tour step.
const steps: StepId[] = ['feed', 'inspector', 'automation', 'overlays', 'points'];
const icons = { feed: IconChat, inspector: IconEye, connection: IconLink, automation: IconFlow, overlays: IconLayers, points: IconCoins };
const guides: Record<Topic, string> = { connection: 'first-live', automation: 'automations/first-rule', overlays: 'overlays/', points: 'points/rules' };
const tour = ref<InstanceType<typeof DemoFrame> | null>(null);
const showcase = ref<ShowcaseFrameState>({ step: 0, count: steps.length, paused: false, done: false });
const activeStep = computed<StepId>(() => steps[Math.min(showcase.value.step, steps.length - 1)]!);
const stepTitle = (step: StepId) => step === 'feed' ? landing.value.feed : step === 'inspector' ? landing.value.inspector : copy.value.topics[step].title;
const activeTitle = computed(() => stepTitle(activeStep.value));
const activeText = computed(() => activeStep.value === 'feed' ? landing.value.stageHint : activeStep.value === 'inspector' ? landing.value.inspectorHint : copy.value.topics[activeStep.value].task);
const player = computed(() => toolbarCopy[locale.value]);
const stepNumber = computed(() => String(showcase.value.step + 1).padStart(2, '0'));
const stepCount = computed(() => String(showcase.value.count).padStart(2, '0'));
const guideHref = (section: Section | StepId) => section === 'feed' || section === 'inspector'
  ? withBase(`/${locale.value}/user/live/feed.html`)
  : withBase(`/${locale.value}/user/${guides[section]}${guides[section].endsWith('/') ? '' : '.html'}`);
function onTourState(state: DemoFrameState) { if (state.showcase) showcase.value = state.showcase; }
const goto = (index: number) => tour.value?.command({ action: 'showcase-goto', value: index });
const toggleTour = () => tour.value?.command({ action: showcase.value.paused ? 'showcase-play' : 'showcase-pause' });
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
            <button
              type="button"
              class="product-stage__tour-toggle"
              data-testid="stage-tour-toggle"
              :aria-label="showcase.paused ? player.play : player.pause"
              :title="showcase.paused ? player.play : player.pause"
              :aria-pressed="!showcase.paused"
              @click="toggleTour"
            ><IconPlay v-if="showcase.paused" /><IconPause v-else /></button>
            <a
              class="product-stage__guide"
              :href="guideHref(activeStep)"
            >{{ copy.readGuide }} <span aria-hidden="true">→</span></a>
          </div>
          <p class="product-stage__context">{{ stepNumber }} / {{ stepCount }} · {{ copy.fictional }}</p>
        </div>
      </header>
      <ol class="product-stage__steps" :aria-label="copy.interactive">
        <li v-for="(step, index) in steps" :key="step">
          <button
            type="button"
            class="product-stage__step"
            :aria-current="index === showcase.step ? 'step' : undefined"
            :data-testid="`stage-step-${step}`"
            @click="goto(index)"
          ><component :is="icons[step]" aria-hidden="true" /><span>{{ stepTitle(step) }}</span></button>
        </li>
      </ol>
      <div class="product-stage__window">
        <div class="product-stage__bar">
          <span class="product-stage__brand" title="demo:showcase">TikTools<small class="product-stage__demo-tag">demo:showcase</small></span>
          <div id="product-stage-tools" class="product-stage__tools"></div>
        </div>
        <div class="product-stage__body">
          <DemoFrame :key="locale" ref="tour" scenario="feed" :locale="locale" :title="landing.feed" showcase autoload minimal @state="onTourState" />
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
