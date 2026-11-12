<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import PluginUpdatePolicyEditor from '@/features/manage-plugin-update-policy/ui/PluginUpdatePolicyEditor.vue';
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
    <PluginUpdatePolicyEditor />
  </BasePanel>
</template>

<style scoped>
.plugin-preferences {
  flex: 1;
  min-height: 0;
}

.plugin-preferences :deep(.plugin-update-policy) {
  max-width: 560px;
}
</style>
