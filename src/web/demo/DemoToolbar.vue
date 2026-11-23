<script setup lang="ts">
import { computed, ref, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import IconPause from '@/shared/ui/icon/icons/IconPause.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import LanguageSwitcher from '@/shared/ui/language-switcher/LanguageSwitcher.vue';
import TutorialCatalogDialog from '@/features/guided-help/ui/TutorialCatalogDialog.vue';
import type { TourId } from '@/features/guided-help/model/types.ts';
import type { PlaybackStatus } from './playback.ts';
import { demoTutorialIds, type DemoTourId } from './tours.ts';
import type { DemoLocale, EmbeddedPanel } from './scenarios.ts';
import { toolbarCopy } from './toolbar-copy.ts';
import { experienceCopy } from '../../../docs/experience/copy.ts';

const props = defineProps<{
  locale: DemoLocale;
  paused: boolean;
  busy: boolean;
  events: number;
  status: PlaybackStatus;
  guideState: string;
  failure: boolean;
  ready: boolean;
  docsUrl: string;
  metrics: string;
  embed?: EmbeddedPanel | null;
  connectionCase?: 'normal' | 'offline' | 'auth';
  /** Tour matching the section currently open in the workspace. */
  recommendedTour?: DemoTourId;
}>();
const emit = defineEmits<{
  locale: [value: DemoLocale];
  play: [];
  step: [];
  reset: [];
  failure: [];
  guide: [id: DemoTourId];
  cancelGuide: [];
  catalog: [];
  connectionCase: [value: 'normal' | 'offline' | 'auth'];
}>();

const { t: translate } = useI18n();
const t = computed(() => toolbarCopy[props.locale]);
const copy = computed(() => experienceCopy[props.locale]);
const recommended = computed(() => props.recommendedTour ?? 'connection');
const guidesLabel = computed(() => translate('guides.catalog.button'));
const connectionOptions = computed(() => (['normal', 'offline', 'auth'] as const).map((value) => ({ value, label: copy.value.connectionStates[value] })));
const playbackLabel = computed(() => {
  const labels = { ready: t.value.ready, connecting: t.value.connecting, playing: t.value.playing, paused: t.value.paused, disconnected: t.value.disconnected, limit: t.value.limitReached, error: t.value.playbackError };
  return labels[props.status];
});
const guideLabel = computed(() => ({ preparing: t.value.preparingGuide, active: t.value.activeGuide, completed: t.value.guideComplete, dismissed: t.value.guideDismissed, unavailable: t.value.guideUnavailable })[props.guideState]);
const playbackDisabled = computed(() => !props.ready || props.busy || props.status === 'limit' || ['preparing', 'active'].includes(props.guideState));
const catalogOpen = ref(false);
const moreOpen = ref(false);
const menuAnchor = shallowRef<HTMLElement | null>(null);
const menuItems = computed<MenuEntry[]>(() => [{ id: 'components', label: t.value.catalog }, { id: 'failure', label: t.value.error, checked: props.failure }]);
function openMenu(event: MouseEvent) { menuAnchor.value = event.currentTarget as HTMLElement; moreOpen.value = !moreOpen.value; }
function selectAction(action: MenuAction) { if (action.id === 'failure') emit('failure'); else if (action.id === 'components') emit('catalog'); }
function launchGuide(id: TourId) { if (demoTutorialIds.includes(id as DemoTourId)) emit('guide', id as DemoTourId); }
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
        <LanguageSwitcher
          class="demo-toolbar__language"
          variant="select"
          :locale="locale"
          :label="t.language"
          @change="emit('locale', $event)"
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
    <div class="demo-toolbar__controls">
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
          :disabled="playbackDisabled"
          :pressed="!paused"
          @click="emit('play')"
        >
          <template #icon>
            <IconPlay v-if="paused" /><IconPause v-else />
          </template>{{ paused ? t.play : t.pause }}
        </BaseButton>
        <BaseButton
          data-testid="demo-step"
          :disabled="playbackDisabled"
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
        <p class="demo-toolbar__recommendation">
          <span>{{ t.recommended }}</span>
          <strong>{{ t.tourTitles[recommended] }}</strong>
        </p>
        <BaseButton
          data-testid="demo-guide"
          :disabled="!ready"
          @click="catalogOpen = true"
        >
          {{ guidesLabel }}
        </BaseButton>
        <BaseButton
          v-if="['preparing', 'active'].includes(guideState)"
          variant="ghost"
          data-testid="demo-close-guide"
          :aria-label="t.closeGuide"
          @click="emit('cancelGuide')"
        >
          {{ t.closeGuide }}
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
        v-if="guideLabel"
        data-testid="demo-guide-status"
        :data-status="guideState"
      >{{ guideLabel }}</span>
    </div>
    <output
      data-testid="demo-event-count"
      aria-live="off"
      hidden
    >{{ events }} / 300</output>
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
    <TutorialCatalogDialog
      v-if="!embed"
      v-model:open="catalogOpen"
      :tours="demoTutorialIds"
      :initial-tour="recommended"
      :on-launch="launchGuide"
    />
  </div>
</template>
