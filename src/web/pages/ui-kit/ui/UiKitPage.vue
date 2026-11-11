<script setup lang="ts">
import UiKitJson from './UiKitJson.vue';
import UiKitPatterns from './UiKitPatterns.vue';
import { ref } from 'vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import IconChevronUp from '@/shared/ui/icon/icons/IconChevronUp.vue';
import IconClose from '@/shared/ui/icon/icons/IconClose.vue';
import IconEye from '@/shared/ui/icon/icons/IconEye.vue';
import IconEyeOff from '@/shared/ui/icon/icons/IconEyeOff.vue';
import IconFilter from '@/shared/ui/icon/icons/IconFilter.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconLock from '@/shared/ui/icon/icons/IconLock.vue';
import IconMinus from '@/shared/ui/icon/icons/IconMinus.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconPopout from '@/shared/ui/icon/icons/IconPopout.vue';
import IconRecord from '@/shared/ui/icon/icons/IconRecord.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconStop from '@/shared/ui/icon/icons/IconStop.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import IconUnlock from '@/shared/ui/icon/icons/IconUnlock.vue';
import IconVolume from '@/shared/ui/icon/icons/IconVolume.vue';
import IconVolumeMuted from '@/shared/ui/icon/icons/IconVolumeMuted.vue';
import BaseRadio from '@/shared/ui/radio/BaseRadio.vue';
import BaseSlider from '@/shared/ui/slider/BaseSlider.vue';
import BaseSpinBox from '@/shared/ui/spin-box/BaseSpinBox.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseToolbar from '@/shared/ui/toolbar/BaseToolbar.vue';
import UiKitWorkspace from './UiKitWorkspace.vue';

const icons = [
  IconPlus,
  IconMinus,
  IconTrash,
  IconGear,
  IconChevronUp,
  IconChevronDown,
  IconChevronLeft,
  IconChevronRight,
  IconClose,
  IconCheck,
  IconEye,
  IconEyeOff,
  IconLock,
  IconUnlock,
  IconPopout,
  IconMoreVertical,
  IconSearch,
  IconFilter,
  IconPlay,
  IconStop,
  IconRecord,
  IconVolume,
  IconVolumeMuted,
];

const transitions = [
  { value: 'fade', label: 'Fade' },
  { value: 'cut', label: 'Cut' },
  { value: 'swipe', label: 'Swipe' },
  { value: 'slide', label: 'Slide', disabled: true },
  { value: 'stinger', label: 'Stinger' },
  { value: 'luma', label: 'Luma Wipe' },
];

const transition = ref('fade');
const duration = ref(300);
const name = ref('Display capture');
const search = ref('');
const captureCursor = ref(true);
const forceSdr = ref(false);
const method = ref<'auto' | 'dxgi' | 'wgc'>('auto');
const volume = ref(72);
const visible = ref(true);
</script>

<template>
  <div class="ui-kit">
    <UiKitPatterns class="ui-kit__panel--wide" />
    <UiKitWorkspace class="ui-kit__panel--wide" />
    <UiKitJson class="ui-kit__panel--wide" />
    <section class="ui-kit__panel">
      <header class="ui-kit__header">
        Buttons
      </header>
      <div class="ui-kit__body ui-kit__stack">
        <BaseButton block>
          Start Streaming
        </BaseButton>
        <BaseButton
          block
          :pressed="true"
        >
          Stop Recording
        </BaseButton>
        <div class="ui-kit__row">
          <BaseButton variant="primary">
            OK
          </BaseButton>
          <BaseButton>Cancel</BaseButton>
          <BaseButton variant="danger">
            Delete
          </BaseButton>
          <BaseButton disabled>
            Disabled
          </BaseButton>
          <BaseButton size="sm">
            <template #icon>
              <IconGear />
            </template>
            Small
          </BaseButton>
        </div>
      </div>
      <BaseToolbar
        class="ui-kit__toolbar"
        aria-label="Scene actions"
      >
        <BaseIconButton label="Add">
          <IconPlus />
        </BaseIconButton>
        <BaseIconButton label="Remove">
          <IconTrash />
        </BaseIconButton>
        <BaseIconButton label="Properties">
          <IconGear />
        </BaseIconButton>
        <BaseIconButton label="Move up">
          <IconChevronUp />
        </BaseIconButton>
        <BaseIconButton label="Move down">
          <IconChevronDown />
        </BaseIconButton>
        <BaseIconButton
          label="Toggle visibility"
          :pressed="visible"
          @click="visible = !visible"
        >
          <IconEye v-if="visible" />
          <IconEyeOff v-else />
        </BaseIconButton>
        <BaseIconButton
          label="Disabled"
          disabled
        >
          <IconLock />
        </BaseIconButton>
      </BaseToolbar>
    </section>

    <section class="ui-kit__panel">
      <header class="ui-kit__header">
        Properties
      </header>
      <div class="ui-kit__body ui-kit__form">
        <label
          class="ui-kit__label"
          for="kit-transition"
        >Transition</label>
        <BaseComboBox
          id="kit-transition"
          v-model="transition"
          :options="transitions"
        />
        <label
          class="ui-kit__label"
          for="kit-duration"
        >Duration</label>
        <BaseSpinBox
          id="kit-duration"
          v-model="duration"
          :min="50"
          :max="20000"
          :step="50"
          suffix="ms"
        />
        <label
          class="ui-kit__label"
          for="kit-name"
        >Name</label>
        <BaseTextField
          id="kit-name"
          v-model="name"
        />
        <label
          class="ui-kit__label"
          for="kit-search"
        >Search</label>
        <BaseTextField
          id="kit-search"
          v-model="search"
          type="search"
          placeholder="Filter sources"
        >
          <template #prefix>
            <IconSearch />
          </template>
        </BaseTextField>
        <span class="ui-kit__label">Invalid</span>
        <BaseTextField
          model-value="rtmp://"
          aria-label="Server"
          invalid
        />
        <span class="ui-kit__label">Disabled</span>
        <BaseComboBox
          :options="transitions"
          model-value="cut"
          aria-label="Disabled transition"
          disabled
        />
        <span />
        <BaseCheckbox v-model="captureCursor">
          Capture cursor
        </BaseCheckbox>
        <span />
        <BaseCheckbox v-model="forceSdr">
          Force SDR
        </BaseCheckbox>
        <span />
        <BaseCheckbox
          :model-value="true"
          disabled
        >
          Disabled option
        </BaseCheckbox>
        <span class="ui-kit__label">Capture method</span>
        <div
          class="ui-kit__row"
          role="radiogroup"
        >
          <BaseRadio
            v-model="method"
            value="auto"
            name="method"
          >
            Automatic
          </BaseRadio>
          <BaseRadio
            v-model="method"
            value="dxgi"
            name="method"
          >
            DXGI
          </BaseRadio>
          <BaseRadio
            v-model="method"
            value="wgc"
            name="method"
            disabled
          >
            WGC
          </BaseRadio>
        </div>
        <label
          class="ui-kit__label"
          for="kit-volume"
        >Volume</label>
        <div class="ui-kit__row ui-kit__row--nowrap">
          <IconVolume />
          <BaseSlider
            id="kit-volume"
            v-model="volume"
            class="ui-kit__grow"
          />
          <span class="ui-kit__value">{{ volume }}%</span>
        </div>
      </div>
      <footer class="ui-kit__actions">
        <BaseButton>Defaults</BaseButton>
        <span class="ui-kit__spacer" />
        <BaseButton variant="primary">
          OK
        </BaseButton>
        <BaseButton>Cancel</BaseButton>
      </footer>
    </section>

    <section class="ui-kit__panel ui-kit__panel--wide">
      <header class="ui-kit__header">
        Icons
      </header>
      <div class="ui-kit__body ui-kit__icons">
        <component
          :is="icon"
          v-for="(icon, index) in icons"
          :key="index"
        />
      </div>
    </section>
  </div>
</template>

<style scoped>
.ui-kit {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(360px, 100%), 1fr));
  gap: var(--space-4);
  align-content: start;
  align-items: start;
  height: 100%;
  padding: var(--space-4);
  overflow: auto;
  background: var(--color-bg-app);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
}

.ui-kit__panel {
  display: flex;
  flex-direction: column;
  border: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel);
}

.ui-kit__panel--wide {
  grid-column: 1 / -1;
}

.ui-kit__header {
  padding: var(--space-3) var(--space-4);
  background: var(--color-bg-panel-header);
  font-weight: var(--font-weight-semibold);
}

.ui-kit__body {
  flex: 1;
  padding: var(--space-4);
}

.ui-kit__stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.ui-kit__row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
}

.ui-kit__row--nowrap {
  flex-wrap: nowrap;
}

.ui-kit__grow {
  flex: 1;
}

.ui-kit__form {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: var(--space-4) var(--space-5);
  align-items: center;
}

.ui-kit__label {
  justify-self: end;
  color: var(--color-text);
}

.ui-kit__value {
  min-width: 4ch;
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
}

.ui-kit__toolbar {
  display: flex;
  gap: var(--space-1);
  padding: var(--space-2);
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.ui-kit__actions {
  display: flex;
  gap: var(--space-3);
  padding: var(--space-4);
}

.ui-kit__spacer {
  flex: 1;
}

.ui-kit__icons {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-5);
}
</style>
