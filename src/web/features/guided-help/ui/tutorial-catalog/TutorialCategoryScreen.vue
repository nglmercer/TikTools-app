<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LocalizedTutorialEntry, TutorialTag } from '../../model/tutorial-catalog.ts';
import TutorialCard from './TutorialCard.vue';
import type { TutorialCategorySummary } from './types.ts';

const props = defineProps<{
  category: TutorialCategorySummary;
  activeTag: 'all' | TutorialTag;
}>();
const emit = defineEmits<{ openTutorial: [id: LocalizedTutorialEntry['id']] }>();
const { t } = useI18n();

const visibleEntries = computed(() => props.activeTag === 'all'
  ? props.category.entries
  : props.category.entries.filter((entry) => entry.tags.includes(props.activeTag as TutorialTag)));
const overviewEntries = computed(() => visibleEntries.value.filter((entry) => entry.overview));
const specializedEntries = computed(() => visibleEntries.value.filter((entry) => !entry.overview));
</script>

<template>
  <section
    class="tutorial-catalog__screen tutorial-catalog__screen--category"
    :aria-label="category.label"
  >
    <section
      v-if="overviewEntries.length"
      class="tutorial-catalog__section tutorial-catalog__section--overview"
      :aria-label="t('guides.catalog.overview')"
    >
      <h4>{{ t('guides.catalog.overview') }}</h4>
      <div class="tutorial-catalog__grid">
        <TutorialCard
          v-for="entry in overviewEntries"
          :key="entry.id"
          :entry="entry"
          :active-tag="activeTag"
          :featured="overviewEntries.length === 1"
          :category-label="category.label"
          @open="emit('openTutorial', $event)"
        />
      </div>
    </section>

    <section
      v-if="specializedEntries.length"
      class="tutorial-catalog__section tutorial-catalog__section--specialized"
      :aria-label="t('guides.catalog.specializedGuides')"
    >
      <h4>{{ t('guides.catalog.specializedGuides') }}</h4>
      <div class="tutorial-catalog__grid">
        <TutorialCard
          v-for="entry in specializedEntries"
          :key="entry.id"
          :entry="entry"
          :active-tag="activeTag"
          @open="emit('openTutorial', $event)"
        />
      </div>
    </section>
  </section>
</template>
