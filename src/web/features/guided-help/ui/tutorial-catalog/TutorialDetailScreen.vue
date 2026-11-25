<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import type { LocalizedTutorialEntry } from '../../model/tutorial-catalog.ts';
import type { TourId } from '../../model/types.ts';
import TutorialDemoPreview from './TutorialDemoPreview.vue';
import { contextualTutorialTitle } from './tutorial-title.ts';
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
}>();
const { t } = useI18n();
const activeStep = ref(0);
const displayTitle = computed(() => contextualTutorialTitle(props.entry.title, props.categoryLabel));
</script>

<template>
  <section
    class="tutorial-catalog__detail tutorial-catalog__screen tutorial-catalog__screen--detail"
    :aria-label="entry.title"
  >
    <div class="tutorial-catalog__detail-bar">
      <BaseButton
        variant="ghost"
        size="sm"
        class="tutorial-catalog__back"
        @click="emit('back')"
      >
        <IconChevronLeft :size="14" />
        {{ t('guides.catalog.back') }}
      </BaseButton>
      <BaseButton
        v-if="props.startable"
        variant="primary"
        class="tutorial-catalog__start"
        @click="emit('start', entry.id)"
      >
        {{ t('guides.catalog.start') }}
      </BaseButton>
    </div>

    <header class="tutorial-catalog__detail-head">
      <span class="tutorial-catalog__detail-icon">
        <component
          :is="entry.icon"
          :size="20"
        />
      </span>
      <div>
        <h3>{{ displayTitle }}</h3>
        <p>{{ entry.description }}</p>
      </div>
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
