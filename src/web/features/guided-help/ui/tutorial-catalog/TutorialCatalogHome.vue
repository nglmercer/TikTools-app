<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { LocalizedTutorialEntry, TutorialCategory } from '../../model/tutorial-catalog.ts';
import TutorialCard from './TutorialCard.vue';
import { tutorialToneClass } from './tutorial-tone.ts';
import type { TutorialCategorySummary } from './types.ts';

defineProps<{
  categories: readonly TutorialCategorySummary[];
  searching: boolean;
  results: readonly LocalizedTutorialEntry[];
}>();
const emit = defineEmits<{
  openCategory: [category: TutorialCategory];
  openTutorial: [id: LocalizedTutorialEntry['id']];
}>();
const { t } = useI18n();
</script>

<template>
  <section
    class="tutorial-catalog__screen tutorial-catalog__screen--catalog"
    :aria-label="t('guides.catalog.screens.catalog')"
  >
    <div
      v-if="searching"
      class="tutorial-catalog__grid"
    >
      <TutorialCard
        v-for="entry in results"
        :key="entry.id"
        :entry="entry"
        @open="emit('openTutorial', $event)"
      />
      <p
        v-if="!results.length"
        class="tutorial-catalog__empty"
      >
        {{ t('guides.catalog.empty') }}
      </p>
    </div>

    <div
      v-else
      class="tutorial-catalog__categories"
    >
      <button
        v-for="category in categories"
        :key="category.id"
        type="button"
        class="tutorial-catalog__category-card"
        :class="tutorialToneClass(category.id)"
        @click="emit('openCategory', category.id)"
      >
        <span class="tutorial-catalog__card-icon">
          <component
            :is="category.icon"
            :size="20"
          />
        </span>
        <span class="tutorial-catalog__category-body">
          <strong>{{ category.label }}</strong>
          <span class="tutorial-catalog__category-count">{{ t('guides.catalog.guides', category.count) }}</span>
        </span>
      </button>
    </div>
  </section>
</template>
