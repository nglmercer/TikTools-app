<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import type { LocalizedTutorialEntry } from '../../model/tutorial-catalog.ts';
import type { TourId } from '../../model/types.ts';
import TutorialDemoPreview from './TutorialDemoPreview.vue';
import { contextualTutorialTitle } from './tutorial-title.ts';
import { tutorialToneClass } from './tutorial-tone.ts';
import TutorialStepPreview from './TutorialStepPreview.vue';

const props = withDefaults(defineProps<{
  entry: LocalizedTutorialEntry;
  startable: boolean;
  previewUrl?: (id: TourId) => string;
  categoryLabel?: string;
}>(), { previewUrl: undefined, categoryLabel: undefined });
const emit = defineEmits<{
  back: [];
  start: [id: LocalizedTutorialEntry['id']];
  prepareRequirement: [id: LocalizedTutorialEntry['id']];
}>();
const { t } = useI18n();
const activeStep = ref(0);
const displayTitle = computed(() => contextualTutorialTitle(props.entry.title, props.categoryLabel));
const toneClass = computed(() => tutorialToneClass(props.entry.id));
</script>

<template>
  <section
    class="tutorial-catalog__detail tutorial-catalog__screen tutorial-catalog__screen--detail"
    :class="toneClass"
    :aria-label="entry.title"
    tabindex="-1"
  >
    <header class="tutorial-catalog__detail-head">
      <BaseButton
        variant="ghost"
        class="tutorial-catalog__back"
        :aria-label="t('guides.catalog.back')"
        @click="emit('back')"
      >
        <template #icon>
          <IconChevronLeft :size="14" />
        </template>
        {{ t('guides.catalog.back') }}
      </BaseButton>
      <div class="tutorial-catalog__detail-copy">
        <div class="tutorial-catalog__detail-title">
          <h3>{{ displayTitle }}</h3>
          <span class="tutorial-catalog__detail-icon">
            <component
              :is="entry.icon"
              :size="14"
            />
          </span>
        </div>
        <p>{{ entry.description }}</p>
        <p
          v-if="entry.requirementsMet === false"
          class="tutorial-catalog__requirement"
        >
          {{ t('guides.catalog.requirements.existingAction') }}
        </p>
      </div>
      <BaseButton
        v-if="entry.requirementsMet === false && props.startable"
        class="tutorial-catalog__requirement-action"
        @click="emit('prepareRequirement', entry.id)"
      >
        {{ t('guides.catalog.requirements.showCreation') }}
      </BaseButton>
      <BaseButton
        v-if="props.startable"
        variant="primary"
        class="tutorial-catalog__start"
        :disabled="entry.requirementsMet === false"
        @click="emit('start', entry.id)"
      >
        {{ t('guides.catalog.start') }}
      </BaseButton>
    </header>

    <div class="tutorial-catalog__detail-layout">
      <section
        class="tutorial-catalog__step-panel"
        :aria-label="t('guides.catalog.stepPreview')"
      >
        <h4>{{ t('guides.catalog.stepPreview') }}</h4>
        <TutorialStepPreview
          v-model="activeStep"
          :steps="entry.steps"
        />
      </section>
      <TutorialDemoPreview
        :tour-id="entry.id"
        :active-step="activeStep"
        :src="props.previewUrl?.(entry.id)"
      />
    </div>
  </section>
</template>
