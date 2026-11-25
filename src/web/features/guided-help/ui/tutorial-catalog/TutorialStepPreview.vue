<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import type { TutorialStepText } from '../../model/tutorial-catalog.ts';

const props = defineProps<{
  steps: readonly TutorialStepText[];
}>();
const activeStep = defineModel<number>({ required: true });
const numbers = ref<HTMLElement | null>(null);
const { t } = useI18n();

const current = computed(() => props.steps[Math.min(activeStep.value, props.steps.length - 1)] ?? null);

function select(index: number): void {
  activeStep.value = Math.max(0, Math.min(props.steps.length - 1, index));
}
function previous(): void { select(activeStep.value - 1); }
function next(): void { select(activeStep.value + 1); }

async function focusNumber(index: number): Promise<void> {
  await nextTick();
  numbers.value?.querySelectorAll<HTMLButtonElement>('button')[index]?.focus();
}
async function onNumbersKeydown(event: KeyboardEvent): Promise<void> {
  const buttons = [...(numbers.value?.querySelectorAll<HTMLButtonElement>('button') ?? [])];
  const focusedIndex = buttons.indexOf(document.activeElement as HTMLButtonElement);
  const currentIndex = focusedIndex >= 0 ? focusedIndex : activeStep.value;
  let target = currentIndex;

  if (event.key === 'ArrowLeft') target -= 1;
  else if (event.key === 'ArrowRight') target += 1;
  else if (event.key === 'Home') target = 0;
  else if (event.key === 'End') target = props.steps.length - 1;
  else return;

  target = Math.max(0, Math.min(props.steps.length - 1, target));
  event.preventDefault();
  select(target);
  await focusNumber(target);
}
</script>

<template>
  <div class="tutorial-step-preview">
    <article
      v-if="current"
      class="tutorial-step-preview__body"
      aria-live="polite"
    >
      <strong>{{ current.title }}</strong>
      <p>{{ current.description }}</p>
    </article>

    <div class="tutorial-step-preview__controls">
      <BaseButton
        variant="ghost"
        size="sm"
        :disabled="activeStep === 0"
        :aria-label="t('guides.previous')"
        @click="previous"
      >
        <template #icon>
          <IconChevronLeft :size="14" />
        </template>
        {{ t('guides.previous') }}
      </BaseButton>
      <div
        ref="numbers"
        class="tutorial-step-preview__numbers"
        :aria-label="t('guides.catalog.stepPreview')"
        @keydown="onNumbersKeydown"
      >
        <button
          v-for="(step, index) in steps"
          :key="step.id"
          type="button"
          :aria-current="index === activeStep ? 'step' : undefined"
          :aria-label="`${index + 1}. ${step.title}`"
          :title="`${index + 1}. ${step.title}`"
          @click="select(index)"
        >{{ index + 1 }}</button>
      </div>
      <BaseButton
        variant="ghost"
        size="sm"
        :disabled="activeStep >= steps.length - 1"
        :aria-label="t('guides.next')"
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
