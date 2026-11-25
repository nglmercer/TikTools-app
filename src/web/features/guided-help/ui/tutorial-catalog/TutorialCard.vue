<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import { tagIcons, type LocalizedTutorialEntry, type TutorialTag } from '../../model/tutorial-catalog.ts';
import { contextualTutorialTitle } from './tutorial-title.ts';
import { tutorialToneClass } from './tutorial-tone.ts';

const props = withDefaults(defineProps<{
  entry: LocalizedTutorialEntry;
  featured?: boolean;
  activeTag?: 'all' | TutorialTag;
  /** Lets the category screen omit the redundant category prefix from overview titles. */
  categoryLabel?: string;
}>(), { featured: false, activeTag: 'all', categoryLabel: undefined });

const emit = defineEmits<{ open: [id: LocalizedTutorialEntry['id']] }>();
const { t } = useI18n();
const visibleTags = computed(() => props.entry.tags.filter((tag) => tag !== props.entry.category && tag !== props.activeTag && tagIcons[tag] !== props.entry.icon));
const displayTitle = computed(() => contextualTutorialTitle(props.entry.title, props.categoryLabel));
const toneClass = computed(() => tutorialToneClass(props.entry.id));
</script>

<template>
  <button
    type="button"
    class="tutorial-catalog__card"
    :class="[toneClass, { 'tutorial-catalog__card--featured': props.featured }]"
    :data-tutorial-id="entry.id"
    @click="emit('open', entry.id)"
  >
    <span class="tutorial-catalog__card-icon">
      <component
        :is="entry.icon"
        :size="props.featured ? 20 : 18"
      />
    </span>
    <span class="tutorial-catalog__card-body">
      <strong>{{ displayTitle }}</strong>
      <span class="tutorial-catalog__card-description">{{ entry.description }}</span>
    </span>
    <span class="tutorial-catalog__card-aside">
      <span
        class="tutorial-catalog__steps"
        :aria-label="t('guides.catalog.steps', entry.steps.length)"
        :title="t('guides.catalog.steps', entry.steps.length)"
      >
        <IconFlow :size="14" />
        <strong>{{ entry.steps.length }}</strong>
      </span>
      <span
        v-if="visibleTags.length"
        class="tutorial-catalog__tag-icons"
      >
        <span
          v-for="tag in visibleTags"
          :key="tag"
          class="tutorial-catalog__tag-icon"
          :class="`tutorial-catalog__tag-icon--${tag}`"
          :aria-label="t(`guides.catalog.tags.${tag}`)"
          :title="t(`guides.catalog.tags.${tag}`)"
        >
          <component
            :is="tagIcons[tag]"
            :size="12"
          />
        </span>
      </span>
    </span>
  </button>
</template>
