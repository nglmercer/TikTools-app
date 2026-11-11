<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import CloudBundles from '@/features/cloud-bundles/ui/CloudBundles.vue';
import CloudOverlays from '@/features/cloud-overlays/ui/CloudOverlays.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';

const emit = defineEmits<{ back: [] }>();
const { t } = useI18n();
const capabilities = useCapabilityStore();

// Each Cloud feature appears only when the host announces it; an unknown answer keeps it hidden.
const overlaysAvailable = computed(() => capabilities.supports('bundles.overlays.create') === true);
const bundlesAvailable = computed(() => capabilities.supports('bundles.packages.preflight') === true);
</script>

<template>
  <BasePanel
    class="overlay-cloud"
    :title="t('overlays.cloud')"
  >
    <template #leading>
      <BaseIconButton
        :label="t('common.back')"
        size="sm"
        @click="emit('back')"
      >
        <IconChevronLeft />
      </BaseIconButton>
    </template>
    <div class="overlay-cloud__body">
      <CloudOverlays v-if="overlaysAvailable" />
      <CloudBundles v-if="bundlesAvailable" />
    </div>
  </BasePanel>
</template>

<style scoped>
.overlay-cloud {
  flex: 1;
  min-height: 0;
}

.overlay-cloud__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
  max-width: 960px;
}
</style>
