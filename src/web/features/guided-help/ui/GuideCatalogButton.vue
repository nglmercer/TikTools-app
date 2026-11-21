<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import type { TutorialTag } from '../model/tutorial-catalog.ts';
import type { TourCapabilities, TourTaskState } from '../model/types.ts';
import TutorialCatalogDialog from './TutorialCatalogDialog.vue';

withDefaults(defineProps<{
  capabilities?: TourCapabilities;
  taskState?: TourTaskState | (() => TourTaskState);
  /** Preselect a category, e.g. `feed` when opened from the live feed. */
  category?: TutorialTag;
  /** Render a labeled call-to-action instead of the compact `?` button. */
  label?: string;
  /** Hide the trigger but keep the dialog mounted (e.g. when moved into an overflow menu). */
  hideButton?: boolean;
}>(), { capabilities: undefined, taskState: undefined, category: undefined, label: undefined, hideButton: false });

const { t } = useI18n();
const catalogOpen = ref(false);
defineExpose({ open: () => { catalogOpen.value = true; } });
</script>

<template>
  <span v-show="!hideButton">
    <BaseButton
      v-if="label"
      size="sm"
      data-guide-catalog
      @click="catalogOpen = true"
    >
      {{ label }}
    </BaseButton>
    <BaseIconButton
      v-else
      :label="t('guides.catalog.button')"
      data-guide-catalog
      size="sm"
      @click="catalogOpen = true"
    >
      ?
    </BaseIconButton>
  </span>
  <TutorialCatalogDialog
    v-model:open="catalogOpen"
    :capabilities="capabilities"
    :task-state="taskState"
    :initial-category="category"
  />
</template>
