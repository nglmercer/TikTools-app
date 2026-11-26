<script setup lang="ts">
import { computed, inject, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import type { TutorialCategory, TutorialTag } from '../model/tutorial-catalog.ts';
import { guideCatalogContextKey } from '../model/catalog-context.ts';
import type { TourCapabilities, TourId, TourTaskState } from '../model/types.ts';
import TutorialCatalogDialog from './TutorialCatalogDialog.vue';

const props = withDefaults(defineProps<{
  capabilities?: TourCapabilities;
  taskState?: TourTaskState | (() => TourTaskState);
  /** Preselect a category, e.g. `feed` when opened from the live feed. */
  category?: TutorialCategory;
  /** Preselect a topic inside that category, e.g. the active automation tab. */
  initialTag?: TutorialTag;
  /** Render a labeled call-to-action instead of the compact `?` button. */
  label?: string;
  /** Hide the trigger but keep the dialog mounted (e.g. when moved into an overflow menu). */
  hideButton?: boolean;
  tours?: readonly TourId[];
  onLaunch?: (id: TourId, trigger?: HTMLElement) => void;
}>(), { capabilities: undefined, taskState: undefined, category: undefined, label: undefined, hideButton: false, tours: undefined, onLaunch: undefined, initialTag: undefined });

const { t } = useI18n();
const hostCapabilities = useCapabilityStore();
const account = useAccountStore();
const automation = useAutomationStore();
const signedIn = computed(() => account.status?.state === 'signed-in');
const catalogContext = inject(guideCatalogContextKey, {});
const catalogOpen = ref(false);
defineExpose({ open: () => { catalogOpen.value = true; } });
watch(catalogOpen, (isOpen) => { if (isOpen) void account.ensureRestored(); });
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
  <Teleport to="body">
    <TutorialCatalogDialog
      v-model:open="catalogOpen"
      :capabilities="props.capabilities ?? hostCapabilities"
      :task-state="taskState"
      :initial-category="category"
      :initial-tag="initialTag"
      :tours="props.tours ?? catalogContext.tours"
      :on-launch="props.onLaunch ?? catalogContext.onLaunch"
      :preview-url="catalogContext.previewUrl"
      :signed-in="signedIn"
      :automation-snapshot="automation.behavior"
    />
  </Teleport>
</template>
