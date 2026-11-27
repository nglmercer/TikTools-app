<script setup lang="ts">
import BaseBackButton from '@/shared/ui/button/BaseBackButton.vue';
import { useI18n } from 'vue-i18n';
import PluginUpdatePolicyEditor from '@/features/manage-plugin-update-policy/ui/PluginUpdatePolicyEditor.vue';
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
      <BaseBackButton
        @back="emit('back')"
      />
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
