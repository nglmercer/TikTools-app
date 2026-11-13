<script setup lang="ts">
import { computed, ref, shallowRef } from 'vue';
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
import { toolbarCopy } from './toolbar-copy.ts';
const props = defineProps<{ scenario: ScenarioId; locale: DemoLocale; paused: boolean; failure: boolean; ready: boolean; tours: Array<{ value: string; label: string }>; initialTour: string; docsUrl: string; metrics: string }>();
const emit = defineEmits<{ scenario: [value: ScenarioId]; locale: [value: DemoLocale]; play: []; step: []; reset: []; failure: []; guide: [id: string, trigger: HTMLElement]; catalog: [] }>();
const t = computed(() => toolbarCopy[props.locale]);
const scenarioOptions = computed(() => Object.entries(scenarios).map(([value, scenario]) => ({ value, label: scenario[props.locale] })));
const languageOptions = computed(() => demoLocales.map((value) => ({ value, label: ({ en: 'English', es: 'Español', pt: 'Português', fr: 'Français' })[value] })));
const selectedTour = ref(props.initialTour);
const moreOpen = ref(false);
const menuAnchor = shallowRef<HTMLElement | null>(null);
const menuItems = computed<MenuEntry[]>(() => [{ id: 'components', label: t.value.catalog }, { id: 'failure', label: t.value.error, checked: props.failure }]);
function openMenu(event: MouseEvent) { menuAnchor.value = event.currentTarget as HTMLElement; moreOpen.value = !moreOpen.value; }
function selectAction(action: MenuAction) { if (action.id === 'failure') emit('failure'); else if (action.id === 'components') emit('catalog'); }
function guide(event: MouseEvent) { emit('guide', selectedTour.value, event.currentTarget as HTMLElement); }
</script>
<template>
  <div class="demo-toolbar">
    <div class="demo-toolbar__heading">
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
      class="demo-toolbar__description"
      data-testid="demo-limit"
    >
      {{ t.limit }}
    </p>
    <div class="demo-toolbar__controls">
      <label class="demo-toolbar__field demo-toolbar__scenario"><span>{{ t.scenario }}</span><BaseComboBox
        :aria-label="t.scenario"
        :options="scenarioOptions"
        :model-value="scenario"
        @update:model-value="emit('scenario', $event as ScenarioId)"
      /></label>
      <div
        class="demo-toolbar__playback"
        role="group"
        :aria-label="t.demo"
      >
        <BaseButton
          variant="primary"
          data-testid="demo-play"
          :pressed="!paused"
          @click="emit('play')"
        >
          <template #icon>
            <IconPlay v-if="paused" /><IconPause v-else />
          </template>{{ paused ? t.play : t.pause }}
        </BaseButton>
        <BaseButton
          data-testid="demo-step"
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
      <div class="demo-toolbar__guides">
        <label class="demo-toolbar__field"><span>{{ t.chooseGuide }}</span><BaseComboBox
          v-model="selectedTour"
          :aria-label="t.chooseGuide"
          :options="tours"
        /></label>
        <BaseButton
          data-testid="demo-guide"
          :disabled="!ready"
          @click="guide"
        >
          {{ t.guide }}<template #icon>
            <span aria-hidden="true">?</span>
          </template>
        </BaseButton>
      </div>
    </div>
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
