<script setup lang="ts">
import { computed, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import type { TutorialStepText } from '../../model/tutorial-catalog.ts';

const props = defineProps<{
  steps: readonly TutorialStepText[];
}>();
const activeStep = defineModel<number>({ required: true });
const { t } = useI18n();
const id = useId();

const current = computed(() => props.steps[Math.min(activeStep.value, props.steps.length - 1)] ?? null);
const tabId = (index: number): string => `${id}-step-tab-${index}`;
const panelId = `${id}-step-panel`;

function select(index: number): void {
  activeStep.value = Math.max(0, Math.min(props.steps.length - 1, index));
}
function previous(): void { select(activeStep.value - 1); }
function next(): void { select(activeStep.value + 1); }
function onKeydown(event: KeyboardEvent): void {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
  event.preventDefault();
  select(activeStep.value + (event.key === 'ArrowRight' ? 1 : -1));
}
</script>

<template>
  <div class="tutorial-step-preview">
    <div
      class="tutorial-step-preview__tabs"
      role="tablist"
      :aria-label="t('guides.catalog.stepPreview')"
      @keydown="onKeydown"
    >
      <button
        v-for="(step, index) in steps"
        :id="tabId(index)"
        :key="step.id"
        type="button"
        role="tab"
        :aria-selected="index === activeStep"
        :aria-controls="panelId"
        :aria-label="`${index + 1}. ${step.title}`"
        :tabindex="index === activeStep ? 0 : -1"
        @click="select(index)"
      >{{ index + 1 }}</button>
    </div>

    <article
      v-if="current"
      :id="panelId"
      class="tutorial-step-preview__body"
      role="tabpanel"
      :aria-labelledby="tabId(activeStep)"
    >
      <strong>{{ activeStep + 1 }}. {{ current.title }}</strong>
      <p>{{ current.description }}</p>
    </article>

    <div class="tutorial-step-preview__controls">
      <BaseButton
        variant="ghost"
        size="sm"
        :disabled="activeStep === 0"
        @click="previous"
      >
        <template #icon>
          <IconChevronLeft :size="14" />
        </template>
        {{ t('guides.previous') }}
      </BaseButton>
      <span>{{ t('guides.progress', { current: activeStep + 1, total: steps.length }) }}</span>
      <BaseButton
        variant="ghost"
        size="sm"
        :disabled="activeStep >= steps.length - 1"
        @click="next"
      >
        {{ t('guides.next') }}
        <template #icon>
          <IconChevronRight :size="14" />
        </template>
      </BaseButton>
    </div>
  </div>
</template>
