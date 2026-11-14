<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import IconPause from '@/shared/ui/icon/icons/IconPause.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import { demoLocales, scenarios, type ScenarioId, type DemoLocale } from './scenarios.ts';
import type { PlaybackStatus } from './playback.ts';
import { localeNames } from '@/shared/i18n/supported-locales.ts';
import { toolbarCopy } from './toolbar-copy.ts';
import type { EmbeddedPanel } from './scenarios.ts';
import { experienceCopy } from '../../../docs/experience/copy.ts';
const props = defineProps<{ scenario: ScenarioId; locale: DemoLocale; paused: boolean; busy: boolean; events: number; status: PlaybackStatus; guideState: string; failure: boolean; ready: boolean; tours: Array<{ value: string; label: string }>; initialTour: string; docsUrl: string; metrics: string; embed?: EmbeddedPanel | null; connectionCase?: 'normal' | 'offline' | 'auth' }>();
const emit = defineEmits<{ scenario: [value: ScenarioId]; locale: [value: DemoLocale]; play: []; step: []; reset: []; failure: []; guide: [id: string, trigger: HTMLElement]; catalog: []; cancelGuide: []; connectionCase: [value: 'normal' | 'offline' | 'auth'] }>();
const t = computed(() => toolbarCopy[props.locale]);
const copy = computed(() => experienceCopy[props.locale]);
const currentTask = computed(() => {
  const scenario = props.scenario;
  return scenario === 'connection' || scenario === 'automation' || scenario === 'points' || scenario === 'overlays' ? copy.value.topics[scenario] : copy.value.otherTasks[scenario];
});
const connectionOptions = computed(() => (['normal', 'offline', 'auth'] as const).map((value) => ({ value, label: copy.value.connectionStates[value] })));
const scenarioOptions = computed(() => Object.entries(scenarios).map(([value, scenario]) => ({ value, label: scenario[props.locale] })));
const languageOptions = computed(() => demoLocales.map((value) => ({ value, label: localeNames[value] })));
const selectedTour = ref(props.initialTour);
watch(selectedTour, () => emit('cancelGuide'));
const playbackLabel = computed(() => {
  const labels = { ready: t.value.ready, connecting: t.value.connecting, playing: t.value.playing, paused: t.value.paused, disconnected: t.value.disconnected, limit: t.value.limitReached, error: t.value.playbackError };
  return labels[props.status];
});
const guideLabel = computed(() => ({ preparing: t.value.preparingGuide, active: t.value.activeGuide, completed: t.value.guideComplete, dismissed: t.value.guideDismissed, unavailable: t.value.guideUnavailable })[props.guideState]);
const moreOpen = ref(false);
const menuAnchor = shallowRef<HTMLElement | null>(null);
const menuItems = computed<MenuEntry[]>(() => [{ id: 'components', label: t.value.catalog }, { id: 'failure', label: t.value.error, checked: props.failure }]);
function openMenu(event: MouseEvent) { menuAnchor.value = event.currentTarget as HTMLElement; moreOpen.value = !moreOpen.value; }
function selectAction(action: MenuAction) { if (action.id === 'failure') emit('failure'); else if (action.id === 'components') emit('catalog'); }
function guide(event: MouseEvent) { emit('guide', selectedTour.value, event.currentTarget as HTMLElement); }
</script>
<template>
  <div
    class="demo-toolbar"
    :class="{ 'demo-toolbar--embedded': embed }"
  >
    <div
      v-if="!embed"
      class="demo-toolbar__heading"
    >
      <div class="demo-toolbar__identity">
        <a
          class="demo-toolbar__brand"
          :href="docsUrl"
        >TikTools<span
          class="demo-toolbar__brand-divider"
          aria-hidden="true"
        >/</span><span>{{ t.demo }}</span></a>
        <span class="demo-toolbar__badge"><span aria-hidden="true" />{{ t.fictional }}</span>
      </div>
      <div class="demo-toolbar__utilities">
        <a
          class="demo-toolbar__docs"
          :href="docsUrl"
        >{{ t.docs }}<span aria-hidden="true">↗</span></a>
        <BaseComboBox
          class="demo-toolbar__language"
          :aria-label="t.language"
          :options="languageOptions"
          :model-value="locale"
          @update:model-value="emit('locale', $event as DemoLocale)"
        />
        <BaseButton
          variant="ghost"
          :disabled="!ready"
          :aria-label="t.more"
          :title="t.more"
          aria-haspopup="menu"
          :aria-expanded="moreOpen"
          data-testid="demo-more"
          @click="openMenu"
        >
          <template #icon>
            <IconMoreVertical />
          </template>
        </BaseButton>
      </div>
    </div>
    <p
      v-if="!embed"
      class="demo-toolbar__description"
      data-testid="demo-limit"
    >
      <strong>{{ copy.task }}:</strong> {{ currentTask.task }} <span class="demo-toolbar__limit">{{ t.limit }}</span>
    </p>
    <div class="demo-toolbar__controls">
      <label
        v-if="!embed"
        class="demo-toolbar__field demo-toolbar__scenario"
      ><span>{{ t.scenario }}</span><BaseComboBox
        :aria-label="t.scenario"
        :options="scenarioOptions"
        :model-value="scenario"
        @update:model-value="emit('scenario', $event as ScenarioId)"
      /></label>
      <label
        v-if="embed === 'connection'"
        class="demo-toolbar__field demo-toolbar__case"
      ><span>{{ copy.connectionStates.label }}</span><BaseComboBox
        :aria-label="copy.connectionStates.label"
        :options="connectionOptions"
        :model-value="connectionCase ?? 'normal'"
        :disabled="!ready || busy"
        @update:model-value="emit('connectionCase', $event as 'normal' | 'offline' | 'auth')"
      /></label>
      <div
        class="demo-toolbar__playback"
        role="group"
        :aria-label="t.demo"
      >
        <BaseButton
          variant="primary"
          data-testid="demo-play"
          :disabled="!ready || busy || status === 'limit' || ['preparing', 'active'].includes(guideState)"
          :pressed="!paused"
          @click="emit('play')"
        >
          <template #icon>
            <IconPlay v-if="paused" /><IconPause v-else />
          </template>{{ paused ? t.play : t.pause }}
        </BaseButton>
        <BaseButton
          data-testid="demo-step"
          :disabled="!ready || busy || status === 'limit' || ['preparing', 'active'].includes(guideState)"
          @click="emit('step')"
        >
          <template #icon>
            <IconChevronRight />
          </template>{{ t.step }}
        </BaseButton>
        <BaseButton
          variant="ghost"
          data-testid="demo-reset"
          :aria-label="t.reset"
          :title="t.reset"
          @click="emit('reset')"
        >
          <template #icon>
            <IconRefresh />
          </template>
        </BaseButton>
      </div>
      <div
        v-if="!embed"
        class="demo-toolbar__guides"
      >
        <label class="demo-toolbar__field"><span>{{ t.chooseGuide }}</span><BaseComboBox
          v-model="selectedTour"
          :aria-label="t.chooseGuide"
          :options="tours"
          :disabled="!ready || ['preparing', 'active'].includes(guideState)"
        /></label>
        <BaseButton
          data-testid="demo-guide"
          :disabled="!ready || busy || ['preparing', 'active'].includes(guideState)"
          @click="guide"
        >
          {{ guideState === 'active' ? t.activeGuide : t.guide }}
        </BaseButton>
      </div>
    </div>
    <p
      v-if="embed === 'connection' && connectionCase !== 'normal'"
      class="demo-toolbar__recovery"
      role="status"
    >
      {{ connectionCase === 'offline' ? copy.connectionStates.offlineHelp : copy.connectionStates.authHelp }}
    </p>
    <div
      class="demo-toolbar__status"
      role="status"
      aria-live="polite"
    >
      <span
        data-testid="demo-playback-status"
        :data-status="status"
      >{{ playbackLabel }}</span>
      <span
        class="demo-toolbar__event-count"
        data-testid="demo-event-count"
      >{{ t.events }}: {{ events }} / 300</span>
      <span
        v-if="guideLabel"
        data-testid="demo-guide-status"
        :data-status="guideState"
      >{{ guideLabel }}</span>
    </div>
    <details
      v-if="!embed"
      class="demo-toolbar__expected"
    >
      <summary>{{ copy.success }}</summary><p>{{ currentTask.result }}</p>
    </details>
    <output
      data-testid="demo-metrics"
      aria-live="off"
      hidden
    >{{ metrics }}</output>
    <BaseMenu
      v-model:open="moreOpen"
      :anchor="menuAnchor"
      :items="menuItems"
      placement="bottom-end"
      @select="selectAction"
    />
  </div>
</template>
