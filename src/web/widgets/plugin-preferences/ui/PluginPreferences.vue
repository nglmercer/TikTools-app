<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import PluginSourcesEditor from '@/features/manage-plugin-sources/ui/PluginSourcesEditor.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconChevronLeft from '@/shared/ui/icon/icons/IconChevronLeft.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';

const emit = defineEmits<{ back: [] }>();
const { t } = useI18n();
</script>

<template>
  <!-- Plugin-wide preferences (update channel, automatic updates, local packages), opened from the catalog tools. -->
  <BasePanel
    class="plugin-preferences"
    :title="t('plugins.catalog.preferences')"
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
    <PluginSourcesEditor />
  </BasePanel>
</template>

<style scoped>
.plugin-preferences {
  flex: 1;
  min-height: 0;
}

.plugin-preferences :deep(.plugin-sources) {
  max-width: 560px;
}
</style>
