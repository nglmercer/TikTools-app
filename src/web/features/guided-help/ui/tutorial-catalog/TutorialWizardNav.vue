<script setup lang="ts">
import { computed, type Component } from 'vue';
import { useI18n } from 'vue-i18n';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import type { TutorialCatalogScreen } from './types.ts';

const props = withDefaults(defineProps<{
  screen: TutorialCatalogScreen;
  categoryLabel?: string;
  categoryIcon?: Component;
  categoryCount?: number;
}>(), { categoryLabel: undefined, categoryIcon: undefined, categoryCount: undefined });
const emit = defineEmits<{ navigate: [screen: 'catalog' | 'category'] }>();
const { t } = useI18n();

const steps = computed(() => [
  { screen: 'catalog' as const, label: t('guides.catalog.screens.catalog'), enabled: true },
  { screen: 'category' as const, label: props.categoryLabel || t('guides.catalog.screens.category'), enabled: props.screen !== 'catalog' && Boolean(props.categoryLabel) },
  { screen: 'detail' as const, label: t('guides.catalog.screens.guide'), enabled: false },
]);
const activeIndex = computed(() => steps.value.findIndex((item) => item.screen === props.screen));
// Future steps are positional placeholders, not useful navigation information.
const items = computed(() => steps.value.slice(0, activeIndex.value + 1));
function navigate(screen: 'catalog' | 'category' | 'detail'): void {
  if (screen !== 'detail') emit('navigate', screen);
}
</script>

<template>
  <nav
    class="tutorial-catalog__wizard"
    :aria-label="t('guides.catalog.wizardLabel')"
  >
    <ol>
      <template
        v-for="(item, index) in items"
        :key="item.screen"
      >
        <li>
          <button
            v-if="index < activeIndex && item.enabled"
            type="button"
            @click="navigate(item.screen)"
          >
            <component
              :is="props.categoryIcon"
              v-if="item.screen === 'category' && props.categoryIcon"
              :size="13"
            />
            <span class="tutorial-catalog__wizard-label">{{ item.label }}</span>
            <span
              v-if="item.screen === 'category' && typeof props.categoryCount === 'number'"
              class="tutorial-catalog__wizard-count"
              :title="t('guides.catalog.guides', props.categoryCount)"
            >{{ props.categoryCount }}</span>
          </button>
          <span
            v-else
            :aria-current="index === activeIndex ? 'step' : undefined"
            :class="{ 'tutorial-catalog__wizard-current': index === activeIndex }"
          >
            <component
              :is="props.categoryIcon"
              v-if="item.screen === 'category' && props.categoryIcon && props.categoryLabel"
              :size="13"
            />
            <span class="tutorial-catalog__wizard-label">{{ item.label }}</span>
            <span
              v-if="item.screen === 'category' && props.categoryLabel && typeof props.categoryCount === 'number'"
              class="tutorial-catalog__wizard-count"
              :title="t('guides.catalog.guides', props.categoryCount)"
            >{{ props.categoryCount }}</span>
          </span>
        </li>
        <li
          v-if="index < items.length - 1"
          class="tutorial-catalog__wizard-separator"
          aria-hidden="true"
        ><IconChevronRight :size="12" /></li>
      </template>
    </ol>
  </nav>
</template>
